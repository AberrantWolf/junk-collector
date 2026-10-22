//! Group publication adapter: stage every affected revision, commit one marker,
//! then update ownership and all projections in one checkpoint-sealed transaction.
use super::*;
use collector_app::{
    AlbumGroupPublication, AlbumGroupResult, AlbumGroupingPort, AutomaticAlbumGroupCandidate,
};
use std::collections::BTreeSet;

#[allow(clippy::too_many_arguments)]
fn revision<T: Serialize>(
    transaction: TransactionId,
    collection: CollectionId,
    path: String,
    kind: &str,
    id: String,
    revision: Revision,
    value: T,
    evidence: Vec<archive_v3::EvidenceReferenceV3>,
) -> Result<group_publication::GroupRevision> {
    let bytes = serde_json::to_vec_pretty(&archive_v3::RevisionEnvelopeV3::new(
        transaction,
        value,
        evidence,
    ))
    .map_err(storage)?;
    let manifest = V3TypedManifest {
        schema_version: 3,
        entity_type: kind.into(),
        entity_id: id,
        collection_id: collection,
        revision,
        record_sha256: Digest::sha256(&bytes).hex,
        transaction_id: Some(transaction),
    };
    Ok(group_publication::GroupRevision {
        path,
        files: vec![
            ("record.v3.json".into(), bytes),
            (
                "manifest.v3.toml".into(),
                toml::to_string_pretty(&manifest)
                    .map_err(storage)?
                    .into_bytes(),
            ),
        ],
    })
}
fn same<T: Serialize>(left: &T, right: &T) -> Result<bool> {
    Ok(serde_json::to_vec(left).map_err(storage)? == serde_json::to_vec(right).map_err(storage)?)
}
impl AlbumGroupingPort for Store {
    fn automatic_album_group_candidates(
        &self,
        after: Option<ItemId>,
        limit: u16,
    ) -> Result<Vec<AutomaticAlbumGroupCandidate>> {
        if !(1..=200).contains(&limit) {
            return Err(Error::Invalid(
                "Use a 1–200 automatic grouping page.".into(),
            ));
        }
        let db = lock(&self.db)?;
        let mut statement = db.prepare("WITH cohorts AS (
            SELECT MIN(m.item_id) AS anchor,m.collection_id FROM album_group_members m
            JOIN items i ON i.id=m.item_id AND i.deleted=0
            GROUP BY m.collection_id,m.album_id HAVING COUNT(*)>1
          ) SELECT anchor,(SELECT MIN(g.user_id) FROM private.grants g JOIN private.users u ON u.id=g.user_id
              WHERE g.collection_id=c.collection_id AND g.role>=1 AND u.enabled=1)
          FROM cohorts c WHERE (?1 IS NULL OR anchor>?1) AND EXISTS(
            SELECT 1 FROM private.grants g JOIN private.users u ON u.id=g.user_id
            WHERE g.collection_id=c.collection_id AND g.role>=1 AND u.enabled=1)
          ORDER BY anchor LIMIT ?2").map_err(storage)?;
        statement
            .query_map(params![after.map(|id| id.to_string()), limit], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(storage)?
            .map(|row| {
                let (item, user) = row.map_err(storage)?;
                Ok(AutomaticAlbumGroupCandidate {
                    item_id: item.parse().map_err(storage)?,
                    user_id: user.parse().map_err(storage)?,
                })
            })
            .collect()
    }

    fn automatic_album_group_idle(&self, items: &[ItemId]) -> Result<bool> {
        automatic_group_idle(&*lock(&self.db)?, items)
    }

    fn album_group_publication(&self, group: TransactionId) -> Result<AlbumGroupPublication> {
        let db = lock(&self.db)?;
        let mut statement = db
            .prepare("SELECT path FROM collections ORDER BY id")
            .map_err(storage)?;
        let paths = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(storage)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(storage)?;
        drop(statement);
        drop(db);
        for path in paths {
            let path = self
                .archive
                .join(path)
                .join("publication-groups")
                .join(format!("{group}.json"));
            if path.exists() {
                let manifest: group_publication::GroupManifest =
                    serde_json::from_slice(&fs::read(path).map_err(storage)?).map_err(storage)?;
                if manifest.transaction_id != group {
                    return Err(Error::Storage("group identity mismatch".into()));
                }
                return manifest.decision.parse();
            }
        }
        Err(Error::NotFound)
    }
    fn publish_album_group(
        &self,
        user: UserId,
        mut publication: AlbumGroupPublication,
    ) -> Result<AlbumGroupResult> {
        let _writer = lock(&self.writer)?;
        let _gate = lock(&self.job_publication_gate)?;
        let collection = publication
            .entries
            .first()
            .ok_or(Error::Conflict)?
            .before
            .collection_id;
        if publication.entries.len() > 32 {
            return Err(Error::Invalid("album group exceeds its entry limit".into()));
        }
        let prefix = {
            let db = lock(&self.db)?;
            relative(
                &self.archive,
                &collection_path(&db, &self.archive, collection)?,
            )?
        };
        let mut revisions = Vec::new();
        let mut captures = Vec::new();
        let mut assets = Vec::new();
        let mut item_paths = Vec::new();
        for entry in &mut publication.entries {
            if entry.before.collection_id != collection
                || entry.after.collection_id != collection
                || entry.before.id != entry.after.id
                || entry.after.revision.0
                    != entry
                        .before
                        .revision
                        .0
                        .checked_add(1)
                        .ok_or(Error::Conflict)?
            {
                return Err(Error::Invalid("invalid grouped item revision".into()));
            }
            let item_path = self.item_path(entry.before.id)?;
            let relative_path = relative(&self.archive, &item_path)?;
            for capture in &mut entry.after.captures {
                let current = Catalog::capture(self, capture.id)?;
                if !same(capture, &current)? {
                    capture.revision =
                        Revision(current.revision.0.checked_add(1).ok_or(Error::Conflict)?);
                    let legacy_root = {
                        let db = lock(&self.db)?;
                        db.query_row("SELECT item_id,relative_path FROM legacy_payload_locators WHERE capture_id=?",[capture.id.to_string()],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?))).optional().map_err(storage)?.map(|(item_id,relative_path)|Ok::<_,Error>(archive_v3::LegacyCaptureRootV3{item_id:item_id.parse().map_err(storage)?,relative_path})).transpose()?
                    };
                    revisions.push(revision(
                        publication.result.group_id,
                        collection,
                        format!(
                            "{prefix}/captures/{}/revisions/{:020}",
                            capture.id, capture.revision.0
                        ),
                        "capture",
                        capture.id.to_string(),
                        capture.revision,
                        archive_v3::CaptureRecordV3 {
                            capture: capture.clone(),
                            legacy_root,
                        },
                        vec![],
                    )?);
                    captures.push(capture.clone());
                }
            }
            for asset in &entry.after.assets {
                let current: archive_v3::AssetRecordV3 =
                    projected_record(&*lock(&self.db)?, "assets", &asset.id.to_string())?;
                if !same(&current.asset, asset)? {
                    let mut record = current.clone();
                    record.asset = asset.clone();
                    record.revision =
                        Revision(record.revision.0.checked_add(1).ok_or(Error::Conflict)?);
                    record.updated_at = entry.after.updated_at;
                    revisions.push(revision(
                        publication.result.group_id,
                        collection,
                        format!(
                            "{prefix}/assets/{}/revisions/{:020}",
                            asset.id, record.revision.0
                        ),
                        "asset",
                        asset.id.to_string(),
                        record.revision,
                        record.clone(),
                        vec![],
                    )?);
                    assets.push(record);
                }
            }
            let evidence = {
                let db = lock(&self.db)?;
                let mut statement = db
                    .prepare("SELECT record FROM evidence WHERE item_id=? ORDER BY id")
                    .map_err(storage)?;
                statement
                    .query_map([entry.after.id.to_string()], |row| row.get::<_, String>(0))
                    .map_err(storage)?
                    .map(|raw| {
                        let record: archive_v3::EvidenceRecordV3 =
                            serde_json::from_str(&raw.map_err(storage)?).map_err(storage)?;
                        record.validate()?;
                        Ok(archive_v3::EvidenceReferenceV3 {
                            id: record.id,
                            sha256: Digest::sha256(
                                &serde_json::to_vec_pretty(&record).map_err(storage)?,
                            )
                            .hex,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?
            };
            revisions.push(revision(
                publication.result.group_id,
                collection,
                format!(
                    "{prefix}/items/{}/revisions/{:020}",
                    entry.after.id, entry.after.revision.0
                ),
                "item",
                entry.after.id.to_string(),
                entry.after.revision,
                archive_v3::ItemRecordV3::from(&entry.after),
                evidence,
            )?);
            if let Some(record) = &entry.metadata_after {
                record.validate()?;
                if record.item_id != entry.after.id
                    || record.collection_id != collection
                    || record.revision.0
                        != entry
                            .metadata_before
                            .as_ref()
                            .map_or(0, |record| record.revision.0)
                            .checked_add(1)
                            .ok_or(Error::Conflict)?
                {
                    return Err(Error::Invalid(
                        "invalid grouped metadata authority revision".into(),
                    ));
                }
                let bytes = serde_json::to_vec_pretty(record).map_err(storage)?;
                let manifest = processing::ManifestV4 {
                    schema_version: 4,
                    entity_type: "metadata_authority".into(),
                    entity_id: record.item_id.to_string(),
                    collection_id: collection,
                    revision: record.revision,
                    record_sha256: Digest::sha256(&bytes).hex,
                    transaction_id: publication.result.group_id,
                };
                revisions.push(group_publication::GroupRevision {
                    path: format!(
                        "{prefix}/metadata_authorities/{}/revisions/{:020}",
                        record.item_id, record.revision.0
                    ),
                    files: vec![
                        ("record.v4.json".into(), bytes),
                        (
                            "manifest.v4.toml".into(),
                            toml::to_string_pretty(&manifest)
                                .map_err(storage)?
                                .into_bytes(),
                        ),
                    ],
                });
            }
            item_paths.push(relative_path);
        }
        let guard = || -> Result<()> {
            let db = lock(&self.db)?;
            if publication.result.automatic
                && !automatic_group_idle(
                    &db,
                    &publication
                        .entries
                        .iter()
                        .map(|entry| entry.before.id)
                        .collect::<Vec<_>>(),
                )?
            {
                return Err(Error::Conflict);
            }
            let allowed:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM private.users u JOIN private.grants g ON g.user_id=u.id WHERE u.id=? AND u.enabled=1 AND g.collection_id=? AND g.role>=1)",params![user.to_string(),collection.to_string()],|row|row.get(0)).map_err(storage)?;
            if publication.membership_guards.len() > 200 {
                return Err(Error::Invalid(
                    "automatic grouping read set exceeds its limit".into(),
                ));
            }
            if publication.result.automatic {
                let guard_ids = publication
                    .membership_guards
                    .iter()
                    .map(|(id, _, _)| id.to_string())
                    .collect::<Vec<_>>();
                let unseen: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM album_group_members m JOIN items i ON i.id=m.item_id AND i.deleted=0 WHERE m.collection_id=?1 AND m.album_id IN(SELECT value FROM json_each(?2)) AND m.item_id NOT IN(SELECT value FROM json_each(?3)))",
                    params![collection.to_string(), serde_json::to_string(&publication.membership_groups).map_err(storage)?, serde_json::to_string(&guard_ids).map_err(storage)?], |row| row.get(0)).map_err(storage)?;
                if unseen {
                    return Err(Error::Conflict);
                }
            }
            drop(db);
            if !allowed {
                return Err(Error::Forbidden);
            }
            for (id, item_digest, metadata_digest) in &publication.membership_guards {
                let item = Catalog::item(self, *id)?;
                let metadata =
                    collector_app::ProcessingPort::metadata_authority(self, collection, *id)?;
                if item.collection_id != collection
                    || Digest::sha256(&serde_json::to_vec(&item).map_err(storage)?) != *item_digest
                    || Digest::sha256(&serde_json::to_vec(&metadata).map_err(storage)?)
                        != *metadata_digest
                {
                    return Err(Error::Conflict);
                }
            }
            for entry in &publication.entries {
                if !same(&Catalog::item(self, entry.before.id)?, &entry.before)? {
                    return Err(Error::Conflict);
                }
                let metadata = collector_app::ProcessingPort::metadata_authority(
                    self,
                    collection,
                    entry.before.id,
                )?;
                if !same(&metadata, &entry.metadata_before)? {
                    return Err(Error::Conflict);
                }
            }
            Ok(())
        };
        guard()?;
        let mut graph = CollectionGraph {
            items: publication
                .entries
                .iter()
                .map(|entry| entry.after.clone())
                .collect(),
            captures: publication
                .entries
                .iter()
                .flat_map(|entry| entry.after.captures.clone())
                .collect(),
            assets: publication
                .entries
                .iter()
                .flat_map(|entry| entry.after.assets.clone())
                .collect(),
            ..Default::default()
        };
        let mut assets_seen = BTreeSet::new();
        graph.assets.retain(|asset| assets_seen.insert(asset.id));
        let roots = publication
            .entries
            .iter()
            .map(|entry| entry.before.id.to_string())
            .collect::<BTreeSet<_>>();
        reference_index::validate_group_change(&*lock(&self.db)?, graph, collection, &roots)?;
        let mut candidate_history = std::collections::BTreeMap::new();
        for entry in &publication.entries {
            let mut candidates = collector_app::ProcessingPort::metadata_candidates(
                self,
                collection,
                entry.after.id,
            )?;
            if let Some(record) = &entry.metadata_after {
                for candidate in record
                    .observations
                    .iter()
                    .flat_map(|value| &value.candidates)
                {
                    if !candidates
                        .iter()
                        .any(|value| value.identity == candidate.identity)
                    {
                        candidates.push(candidate.clone());
                    }
                }
                record.validate_display(&candidates)?;
            }
            candidate_history.insert(entry.after.id, candidates);
        }
        let decision = RawJson::from_json(serde_json::to_string(&publication).map_err(storage)?)?;
        let scoped = self.with_command_result(&publication.result)?;
        let paths = group_publication::publish(
            &scoped,
            publication.result.group_id,
            collection,
            revisions,
            decision,
            &mut || guard(),
        )?;
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        for root in &roots {
            tx.execute(
                "DELETE FROM entity_owners WHERE root_kind='item' AND root_id=?",
                [root],
            )
            .map_err(storage)?;
            tx.execute("DELETE FROM components WHERE item_id=?", [root])
                .map_err(storage)?;
        }
        for capture in captures {
            project_capture(&tx, &capture)?;
        }
        for asset in assets {
            tx.execute(
                "UPDATE assets SET revision=?,record=?,deleted=? WHERE id=?",
                params![
                    asset.revision.0,
                    serde_json::to_string(&asset).map_err(storage)?,
                    asset.deleted,
                    asset.asset.id.to_string()
                ],
            )
            .map_err(storage)?;
        }
        for (entry, path) in publication.entries.iter().zip(item_paths) {
            project(
                &tx,
                &RevisionRecord {
                    item: entry.after.clone(),
                    evidence: vec![],
                    legacy: None,
                },
                &path,
            )?;
        }
        for entry in &publication.entries {
            if let Some(record) = &entry.metadata_after {
                processing::project_authority_revision(
                    &tx,
                    record,
                    candidate_history
                        .get(&entry.after.id)
                        .ok_or(Error::Conflict)?,
                )?;
            }
        }
        self.record_command(&tx, &publication.result)?;
        index_checkpoint::published_in_transaction(&tx, &self.archive, &paths)?;
        tx.commit().map_err(storage)?;
        drop(db);
        finish_v3_publication(self, publication.result.group_id)?;
        Ok(publication.result)
    }
}

fn automatic_group_idle(db: &Connection, items: &[ItemId]) -> Result<bool> {
    let ids = serde_json::to_string(items).map_err(storage)?;
    let busy: bool = db.query_row("SELECT
      EXISTS(SELECT 1 FROM private.jobs WHERE item_id IN (SELECT value FROM json_each(?1))
        AND state IN ('queued','running','cancel_requested')) OR
      EXISTS(SELECT 1 FROM private.guided_ingest_intents WHERE target_item_id IN (SELECT value FROM json_each(?1))
        AND settled=0 AND cancelled=0) OR
      EXISTS(SELECT 1 FROM private.bulk_library_members m JOIN private.bulk_library b ON b.id=m.batch_id
        WHERE m.item_id IN (SELECT value FROM json_each(?1)) AND b.state='accepted' AND m.state IN ('ready','queued','running'))",
      [ids], |row| row.get(0)).map_err(storage)?;
    Ok(!busy)
}
