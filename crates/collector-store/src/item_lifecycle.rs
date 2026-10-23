//! Archive-backed Item visibility and private-work settlement.
//!
//! The Item revision is authoritative. A private-state interruption after its
//! publication is repaired by `reconcile_removed_item_work` on every recovery
//! sweep; publication/admission guards consult the same tombstone immediately.
use super::*;
use collector_app::{ProcessingPort, ProvisionalImportPort};
use rusqlite::TransactionBehavior;

impl Store {
    pub(crate) fn change_item_removed(
        &self,
        id: ItemId,
        expected: Revision,
        removed: bool,
    ) -> Result<Item> {
        let _writer = lock(&self.writer)?;
        let _publication_gate = lock(&self.job_publication_gate)?;
        let mut item = Catalog::item(self, id)?;
        if item.lifecycle.merged_into.is_some() {
            return Err(Error::Invalid(
                "Use guarded group undo to restore merged entries.".into(),
            ));
        }
        if item.revision != expected || item.deleted == removed {
            return Err(Error::Conflict);
        }
        // A restore can win the race against the periodic recovery sweep after
        // a crash between tombstone publication and private work settlement.
        if !removed {
            self.settle_removed_item_work(id)?;
        }
        if removed {
            item.lifecycle.generation = item
                .lifecycle
                .generation
                .checked_add(1)
                .ok_or(Error::Conflict)?;
        }
        item.deleted = removed;
        let saved = self.publish_under_writer(item, expected, vec![])?;
        if removed {
            self.settle_removed_item_work(id)?;
        }
        Ok(saved)
    }

    pub(crate) fn reconcile_removed_item_work(&self) -> Result<()> {
        let mut after = String::new();
        loop {
            let ids = {
                let db = lock(&self.db)?;
                let mut statement = db
                .prepare(
                    "SELECT id FROM items WHERE id>? AND deleted=1 AND (
                   EXISTS(SELECT 1 FROM private.guided_ingest_intents g
                          WHERE g.target_item_id=items.id AND g.cancelled=0 AND g.settled=0)
                   OR EXISTS(SELECT 1 FROM private.imports p
                          WHERE p.item_id=items.id AND p.phase NOT IN ('published','cancelled'))
                   OR EXISTS(SELECT 1 FROM private.jobs j
                          WHERE j.item_id=items.id AND j.state IN ('queued','running')
                            AND NOT EXISTS(SELECT 1 FROM private.scan_requests r WHERE r.job_id=j.id) AND NOT EXISTS(SELECT 1 FROM private.derivative_direct_requests r WHERE r.job_id=j.id)))
                 ORDER BY id LIMIT 1000",
                )
                .map_err(storage)?;
                statement
                    .query_map([&after], |row| row.get::<_, String>(0))
                    .map_err(storage)?
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(storage)?
            };
            if ids.is_empty() {
                break;
            }
            for id in ids {
                after = id.clone();
                self.settle_removed_item_work(id.parse().map_err(storage)?)?;
            }
        }
        Ok(())
    }

    fn settle_removed_item_work(&self, id: ItemId) -> Result<()> {
        let now = self.clock.now().timestamp();
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let removed: bool = tx
            .query_row(
                "SELECT deleted FROM items WHERE id=?",
                [id.to_string()],
                |row| row.get(0),
            )
            .map_err(storage)?;
        if !removed {
            return Ok(());
        }
        Self::stop_item_work_in_tx(&tx, id, now)?;
        tx.commit().map_err(storage)
    }
    pub(crate) fn stop_item_work_in_tx(
        tx: &rusqlite::Transaction<'_>,
        id: ItemId,
        now: i64,
    ) -> Result<()> {
        tx.execute(
            "DELETE FROM private.scan_requests WHERE kind='import' AND owner IN
             (SELECT import_id FROM private.guided_ingest_intents WHERE target_item_id=?
              UNION SELECT id FROM private.imports WHERE item_id=?)",
            params![id.to_string(), id.to_string()],
        )
        .map_err(storage)?;
        tx.execute(
            "UPDATE private.guided_ingest_intents
             SET cancelled=1,settled=1,updated_at=? WHERE target_item_id=? AND cancelled=0 AND settled=0",
            params![now, id.to_string()],
        )
        .map_err(storage)?;
        tx.execute(
            "UPDATE private.imports SET phase='cancelled',revision=revision+1,updated_at=?
             WHERE (item_id=? OR id IN (SELECT import_id FROM private.guided_ingest_intents WHERE target_item_id=?))
               AND phase<>'published' AND phase<>'cancelled'
               AND NOT EXISTS(SELECT 1 FROM captures c WHERE c.id=private.imports.capture_id)
               AND NOT EXISTS(SELECT 1 FROM private.scan_requests r
                              JOIN private.jobs j ON j.id=r.job_id
                              WHERE r.kind='direct' AND j.kind='import_analysis'
                                AND json_extract(j.request,'$.import_id')=private.imports.id
                                AND j.state IN ('queued','running'))",
            params![now,id.to_string(),id.to_string()],
        ).map_err(storage)?;
        // A direct owner keeps shared scan computation alive. Only item-owned
        // requests without another reference are stopped.
        tx.execute(
            "UPDATE private.jobs SET state='cancelled',phase='cancelled',
               cancellation_requested=1,updated_at=?
             WHERE state='queued' AND (item_id=? OR json_extract(request,'$.import_id') IN
                (SELECT import_id FROM private.guided_ingest_intents WHERE target_item_id=?
                 UNION SELECT id FROM private.imports WHERE item_id=?))
               AND NOT EXISTS(SELECT 1 FROM private.imports p JOIN captures c ON c.id=p.capture_id
                              WHERE p.id=json_extract(private.jobs.request,'$.import_id')
                                AND private.jobs.kind='import_publication')
               AND NOT EXISTS(SELECT 1 FROM private.scan_requests r WHERE r.job_id=private.jobs.id) AND NOT EXISTS(SELECT 1 FROM private.derivative_direct_requests r WHERE r.job_id=private.jobs.id)",
            params![now,id.to_string(),id.to_string(),id.to_string()],
        ).map_err(storage)?;
        tx.execute(
            "UPDATE private.jobs SET state='cancel_requested',phase='cancel_requested',
               cancellation_requested=1,updated_at=?
             WHERE state='running' AND (item_id=? OR json_extract(request,'$.import_id') IN
                (SELECT import_id FROM private.guided_ingest_intents WHERE target_item_id=?
                 UNION SELECT id FROM private.imports WHERE item_id=?))
               AND NOT EXISTS(SELECT 1 FROM private.imports p JOIN captures c ON c.id=p.capture_id
                              WHERE p.id=json_extract(private.jobs.request,'$.import_id')
                                AND private.jobs.kind='import_publication')
               AND NOT EXISTS(SELECT 1 FROM private.scan_requests r WHERE r.job_id=private.jobs.id) AND NOT EXISTS(SELECT 1 FROM private.derivative_direct_requests r WHERE r.job_id=private.jobs.id)",
            params![now,id.to_string(),id.to_string(),id.to_string()],
        ).map_err(storage)?;
        Ok(())
    }
}

impl Store {
    /// Caller holds the archive writer. The portable generation fences old jobs
    /// even if interrupted before private admission/settlement commits.
    pub(crate) fn begin_clean_reimport(
        &self,
        record: &mut collector_app::ImportRecord,
        intent: &collector_app::GuidedIngestIntent,
    ) -> Result<()> {
        let options = intent.clean_reimport.as_ref().ok_or(Error::Conflict)?;
        let mut item = Catalog::item(self, intent.target_item_id)?;
        if item.deleted {
            return Err(Error::Conflict);
        }
        if item
            .lifecycle
            .reimport
            .as_ref()
            .is_some_and(|pending| pending.import_id == record.view.id)
        {
            record.view.expected_item_revision = Some(item.revision);
            return Ok(());
        }
        let metadata_revision = self
            .metadata_authority(item.collection_id, item.id)?
            .map(|record| record.revision)
            .unwrap_or(Revision(0));
        item = collector_app::prepare_clean_reimport(item, record, options, metadata_revision)?;
        let expected = item.revision;
        // This preparation is recoverable through the deterministic Import identity;
        // only the admission transaction may complete the caller's receipt.
        let preparation = Self {
            receipt: None,
            command_result: None,
            ..self.clone()
        };
        let saved = preparation.publish_under_writer(item, expected, vec![])?;
        record.view.expected_item_revision = Some(saved.revision);
        Ok(())
    }

    pub(crate) fn archive_old_item_work(
        tx: &rusqlite::Transaction<'_>,
        item: ItemId,
        current: ImportId,
    ) -> Result<()> {
        tx.execute("INSERT INTO private.work_history(user_id,target,archived,pending,action,error)
            SELECT user_id,'import:'||id,1,0,'dismiss',NULL FROM private.imports WHERE item_id=?1 AND id<>?2 AND NOT EXISTS(SELECT 1 FROM private.scan_requests r JOIN private.jobs j ON j.id=r.job_id WHERE r.kind='direct' AND json_extract(j.request,'$.import_id')=private.imports.id)
            ON CONFLICT(user_id,target) DO UPDATE SET archived=1,pending=0,action='dismiss',error=NULL",params![item.to_string(),current.to_string()]).map_err(storage)?;
        tx.execute("INSERT INTO private.work_history(user_id,target,archived,pending,action,error)
            SELECT user_id,'job:'||id,1,0,'dismiss',NULL FROM private.jobs WHERE item_id=?1
              AND NOT EXISTS(SELECT 1 FROM private.scan_requests r WHERE r.job_id=private.jobs.id) AND NOT EXISTS(SELECT 1 FROM private.derivative_direct_requests r WHERE r.job_id=private.jobs.id)
            ON CONFLICT(user_id,target) DO UPDATE SET archived=1,pending=0,action='dismiss',error=NULL",[item.to_string()]).map_err(storage)?;
        Ok(())
    }

    pub(crate) fn finish_clean_reimport_under_writer(&self, id: ImportId) -> Result<()> {
        let record = self.import_record(id)?;
        let Some(item_id) = record.view.association.item_id else {
            return Ok(());
        };
        let mut item = Catalog::item(self, item_id)?;
        if item.deleted {
            return Ok(());
        }
        let Some(pending) = &mut item.lifecycle.reimport else {
            return Ok(());
        };
        if pending.import_id != id || pending.completed {
            return Ok(());
        }
        if self.capture(record.view.capture_id).is_err() {
            return Err(Error::Conflict);
        }
        for old in &pending.replaced_captures {
            if !item.lifecycle.historical_captures.contains(old) {
                item.lifecycle.historical_captures.push(*old);
            }
        }
        pending.completed = true;
        let expected = item.revision;
        self.publish_under_writer(item, expected, vec![])?;
        Ok(())
    }

    pub(crate) fn copy_retained_import(
        &self,
        id: ImportId,
        job: JobId,
        claim: &str,
        representation: RepresentationId,
    ) -> Result<collector_app::ImportRecord> {
        use sha2::{Digest as _, Sha256};
        use std::io::Read;
        let intent = self.guided_ingest_intent(id)?.ok_or(Error::NotFound)?;
        if intent.cancelled
            || intent
                .clean_reimport
                .as_ref()
                .and_then(|value| value.retained_representation)
                != Some(representation)
        {
            return Err(Error::Conflict);
        }
        let mut record = self.import_record(id)?;
        let scope = collector_core::PackageScope::Representation(representation);
        let inventory = self.package_inventory(record.view.collection_id, scope)?;
        if inventory.files.len() != record.view.files.len() {
            return Err(Error::Conflict);
        }
        for (index, source) in inventory.files.iter().enumerate() {
            let declared = record.view.files[index].clone();
            let mut reader = self.package_file(record.view.collection_id, scope, source.id)?;
            let mut hash = Sha256::new();
            let mut offset = 0;
            let mut buffer = vec![0u8; 1024 * 1024];
            loop {
                self.ensure_job_continues(job, Some(claim))?;
                let read = reader.read(&mut buffer).map_err(storage)?;
                if read == 0 {
                    break;
                }
                if offset + read as u64 > declared.bytes.0 {
                    return Err(Error::Invalid("retained original length changed".into()));
                }
                hash.update(&buffer[..read]);
                if record.view.phase == collector_app::ImportPhase::Staging {
                    record = self.stage_import_chunk_checked(
                        id,
                        index,
                        offset,
                        &buffer[..read],
                        &mut || self.ensure_job_continues(job, Some(claim)),
                    )?;
                }
                offset += read as u64;
                self.heartbeat_job_record(
                    job,
                    claim,
                    "validating_retained_original",
                    record
                        .view
                        .files
                        .iter()
                        .map(|file| file.received_bytes.0)
                        .sum(),
                    30,
                )?;
            }
            let digest = Digest::new(HashAlgorithm::Sha256, hex::encode(hash.finalize()))?;
            if offset != declared.bytes.0 || declared.digest.as_ref() != Some(&digest) {
                return Err(Error::Invalid(
                    "retained original does not match its recorded digest".into(),
                ));
            }
        }
        self.ensure_job_continues(job, Some(claim))?;
        Ok(record)
    }
}

impl Store {
    /// Caller holds archive writer and publication gate through the durable write.
    pub(crate) fn validate_item_effect(
        &self,
        context: collector_app::ItemEffectContext,
        collection: CollectionId,
    ) -> Result<()> {
        let item = Catalog::item(self, context.item_id)?;
        if item.deleted
            || item.collection_id != collection
            || item.lifecycle.generation != context.generation
        {
            return Err(Error::Conflict);
        }
        let db = lock(&self.db)?;
        let valid = match context.source {
            collector_app::ItemEffectSource::Command => true,
            collector_app::ItemEffectSource::Job { id, claim } => db.query_row(
                "SELECT EXISTS(SELECT 1 FROM private.jobs j WHERE j.id=?1 AND j.item_id=?2 AND j.collection_id=?3 AND j.state='running' AND j.cancellation_requested=0 AND j.claim_token=?4 AND COALESCE((SELECT generation FROM private.item_job_generations g WHERE g.job_id=j.id),0)=?5)",
                params![id.to_string(),context.item_id.to_string(),collection.to_string(),claim.to_string(),context.generation], |row|row.get::<_,bool>(0)
            ).map_err(storage)?,
            collector_app::ItemEffectSource::GuidedImport(import) => db.query_row(
                "SELECT EXISTS(SELECT 1 FROM private.guided_ingest_intents g
                 JOIN private.imports p ON p.id=g.import_id
                 WHERE g.import_id=?1 AND g.target_item_id=?2 AND g.cancelled=0
                 AND p.collection_id=?3 AND p.phase<>'cancelled'
                 AND COALESCE((SELECT generation FROM private.item_import_generations v WHERE v.import_id=p.id),0)=?4)",
                params![import.to_string(),context.item_id.to_string(),collection.to_string(),context.generation], |row|row.get::<_,bool>(0)
            ).map_err(storage)?,
            collector_app::ItemEffectSource::Snapshot(snapshot) => db.query_row(
                "SELECT EXISTS(SELECT 1 FROM private.snapshot_bindings b JOIN snapshots s ON s.id=b.snapshot_id
                 WHERE b.snapshot_id=?1 AND b.item_id=?2 AND s.collection_id=?3 AND b.generation=?4)",
                params![snapshot.to_string(),context.item_id.to_string(),collection.to_string(),context.generation], |row|row.get::<_,bool>(0)
            ).map_err(storage)?,
        };
        if valid { Ok(()) } else { Err(Error::Conflict) }
    }
}
