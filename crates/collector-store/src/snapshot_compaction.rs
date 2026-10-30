//! Stopped-server duplicate-payload conversion. Snapshot envelopes and evidence IDs remain immutable.
use super::*;
use crate::backup::{backup_inventory, read_bounded_json, verify_backup, verify_backup_roots};
use collector_app::{FrozenJobInput, SnapshotRecord};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SnapshotDuplicate {
    pub snapshot: SnapshotId,
    pub canonical: SnapshotId,
    pub collection: CollectionId,
    pub bytes: ByteLength,
    pub digest: Digest,
    pub input_key: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SnapshotDuplicatePlan {
    pub version: u32,
    pub duplicates: Vec<SnapshotDuplicate>,
    pub skipped: Vec<SnapshotId>,
    pub saved_bytes: u64,
    pub reference_bytes: u64,
}
pub(crate) type SnapshotInputReferences = std::collections::BTreeMap<
    CollectionId,
    std::collections::BTreeMap<SnapshotId, SnapshotDuplicate>,
>;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PayloadReference {
    version: u32,
    mapping: SnapshotDuplicate,
    checksum: Digest,
}

impl Store {
    fn original_snapshot_payload(&self, record: &SnapshotRecord) -> Result<PathBuf> {
        let db = lock(&self.db)?;
        Ok(collection_path(&db, &self.archive, record.collection_id)?
            .join("snapshots")
            .join(record.id.to_string())
            .join("payload/source.bin"))
    }
    pub(crate) fn snapshot_payload_path(&self, record: &SnapshotRecord) -> Result<PathBuf> {
        self.resolve_snapshot_payload(record, &self.original_snapshot_payload(record)?)
    }
    pub(crate) fn resolve_snapshot_payload(
        &self,
        record: &SnapshotRecord,
        original: &Path,
    ) -> Result<PathBuf> {
        let reference = original.with_file_name("reference.v1.json");
        if !reference.try_exists().map_err(storage)? {
            return Ok(original.to_owned());
        }
        let data = read_bounded_json(&reference, 16 * 1024)?;
        let reference: PayloadReference = serde_json::from_slice(&data).map_err(storage)?;
        let mapping = &reference.mapping;
        if reference.version != 1
            || mapping.snapshot != record.id
            || mapping.canonical == record.id
            || mapping.collection != record.collection_id
            || mapping.bytes != record.bytes
            || mapping.digest != record.digest
            || reference.checksum != Digest::sha256(&serde_json::to_vec(mapping).map_err(storage)?)
        {
            return Err(Error::Storage("invalid snapshot payload reference".into()));
        }
        let snapshots = original
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .ok_or(Error::Conflict)?;
        let target = snapshots
            .join(mapping.canonical.to_string())
            .join("payload/source.bin");
        if target
            .with_file_name("reference.v1.json")
            .try_exists()
            .map_err(storage)?
        {
            return Err(Error::Storage(
                "chained snapshot references are unsupported".into(),
            ));
        }
        Ok(target)
    }
    pub(crate) fn snapshot_record_page(
        &self,
        collection: Option<CollectionId>,
        after: Option<SnapshotId>,
    ) -> Result<Vec<(SnapshotId, Option<String>)>> {
        let db = lock(&self.db)?;
        let mut stmt = db.prepare("SELECT id,CASE WHEN length(record)<=2097152 THEN record ELSE NULL END FROM snapshots WHERE (? IS NULL OR collection_id=?) AND (? IS NULL OR id>?) ORDER BY id LIMIT 32").map_err(storage)?;
        let collection = collection.map(|v| v.to_string());
        let after = after.map(|v| v.to_string());
        stmt.query_map(params![collection, collection, after, after], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
        })
        .map_err(storage)?
        .map(|row| {
            let (id, raw) = row.map_err(storage)?;
            Ok((id.parse().map_err(storage)?, raw))
        })
        .collect()
    }

    fn known_snapshot_layout(original: &Path) -> Result<bool> {
        let payload = original.parent().ok_or(Error::Conflict)?;
        let root = payload.parent().ok_or(Error::Conflict)?;
        for entry in fs::read_dir(root).map_err(storage)? {
            let entry = entry.map_err(storage)?;
            if !entry.file_type().map_err(storage)?.is_dir()
                || !matches!(entry.file_name().to_str(), Some("payload" | "revisions"))
            {
                return Ok(false);
            }
        }
        for entry in fs::read_dir(payload).map_err(storage)? {
            let entry = entry.map_err(storage)?;
            if !entry.file_type().map_err(storage)?.is_file()
                || !matches!(
                    entry.file_name().to_str(),
                    Some("source.bin" | "reference.v1.json")
                )
            {
                return Ok(false);
            }
        }
        for revision in fs::read_dir(root.join("revisions")).map_err(storage)? {
            let revision = revision.map_err(storage)?;
            let name = revision.file_name();
            if !revision.file_type().map_err(storage)?.is_dir()
                || !name.to_str().is_some_and(|name| {
                    name.len() == 20 && name.bytes().all(|b| b.is_ascii_digit())
                })
            {
                return Ok(false);
            }
            for entry in fs::read_dir(revision.path()).map_err(storage)? {
                let entry = entry.map_err(storage)?;
                if !entry.file_type().map_err(storage)?.is_file()
                    || !matches!(
                        entry.file_name().to_str(),
                        Some("record.v4.json" | "manifest.v4.toml")
                    )
                {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    pub fn inspect_snapshot_duplicates(&self) -> Result<SnapshotDuplicatePlan> {
        let unresolved = fs::read_dir(self.archive.join(".transactions"))
            .map_err(storage)?
            .next()
            .is_some();
        let mut groups: std::collections::BTreeMap<
            String,
            Vec<(chrono::DateTime<chrono::Utc>, SnapshotDuplicate)>,
        > = std::collections::BTreeMap::new();
        let mut plan = SnapshotDuplicatePlan {
            version: 1,
            duplicates: vec![],
            skipped: vec![],
            saved_bytes: 0,
            reference_bytes: 0,
        };
        let mut after = None;
        let mut references = SnapshotInputReferences::new();
        loop {
            let rows = self.snapshot_record_page(None, after)?;
            if rows.is_empty() {
                break;
            }
            after = rows.last().map(|row| row.0);
            for (id, raw) in rows {
                let Some(raw) = raw else {
                    plan.skipped.push(id);
                    continue;
                };
                let value: serde_json::Value = serde_json::from_str(&raw).map_err(storage)?;
                let record: SnapshotRecord = serde_json::from_str(&raw).map_err(storage)?;

                if !collector_app::reuses_equivalent_provider_responses(&record.provider) {
                    continue;
                }
                if unresolved {
                    plan.skipped.push(record.id);
                    continue;
                }
                let original = self.original_snapshot_payload(&record)?;
                if original
                    .with_file_name("reference.v1.json")
                    .try_exists()
                    .map_err(storage)?
                {
                    continue;
                }
                if !Self::known_snapshot_layout(&original)? {
                    plan.skipped.push(record.id);
                    continue;
                }
                // Unknown fields may describe a distinct interpretation. Do not rewrite them.
                if serde_json::to_value(&record).map_err(storage)? != value {
                    plan.skipped.push(record.id);
                    continue;
                }
                let input = match record.acquisition_key.clone() {
                    Some(key) => Some(key),
                    None => self.legacy_snapshot_key_checked(
                        record.id,
                        &mut || Ok(false),
                        &mut references,
                    )?,
                };
                let Some(input) = input else {
                    plan.skipped.push(record.id);
                    continue;
                };
                let busy:bool=lock(&self.db)?.query_row("SELECT EXISTS(SELECT 1 FROM private.processing_jobs p JOIN private.jobs j ON j.id=p.job_id WHERE (json_extract(p.frozen_input,'$.input.snapshot_id')=? OR json_extract(p.frozen_input,'$.input.acquisition.snapshot_id')=?) AND j.state IN ('queued','running','cancel_requested'))",params![record.id.to_string(),record.id.to_string()],|r|r.get(0)).map_err(storage)?;
                if busy {
                    plan.skipped.push(record.id);
                    continue;
                }
                let (bytes, digest) = crate::files::hash_file(&original)?;
                if bytes != record.bytes || digest != record.digest {
                    return Err(Error::Storage(
                        "snapshot payload is corrupt; conversion refused".into(),
                    ));
                }
                let mut comparison = record.clone();
                comparison.acquisition_key = Some(input.clone());
                let identity = collector_app::snapshot_response_key(&comparison)?;
                groups.entry(identity).or_default().push((
                    record.created_at,
                    SnapshotDuplicate {
                        snapshot: record.id,
                        canonical: record.id,
                        collection: record.collection_id,
                        bytes: record.bytes,
                        digest: record.digest,
                        input_key: input,
                    },
                ));
            }
        }
        for mut members in groups.into_values() {
            members.sort_by_key(|(created, mapping)| (*created, mapping.snapshot));
            let canonical = members[0].1.snapshot;
            for (_, mut mapping) in members.into_iter().skip(1) {
                mapping.canonical = canonical;
                plan.saved_bytes = plan
                    .saved_bytes
                    .checked_add(mapping.bytes.0)
                    .ok_or(Error::Conflict)?;
                plan.reference_bytes = plan
                    .reference_bytes
                    .checked_add(payload_reference_bytes(&mapping)?.len() as u64)
                    .ok_or(Error::Conflict)?;
                plan.duplicates.push(mapping);
            }
        }
        Ok(plan)
    }
    pub(crate) fn legacy_snapshot_key(&self, id: SnapshotId) -> Result<Option<String>> {
        self.legacy_snapshot_key_checked(id, &mut || Ok(false), &mut SnapshotInputReferences::new())
    }
    pub(crate) fn legacy_snapshot_key_checked(
        &self,
        id: SnapshotId,
        cancelled: &mut dyn FnMut() -> Result<bool>,
        references: &mut SnapshotInputReferences,
    ) -> Result<Option<String>> {
        if cancelled()? {
            return Err(Error::JobCancelled);
        }
        let rows = {
            let db = lock(&self.db)?;
            let mut stmt=db.prepare("SELECT CASE WHEN length(frozen_input)<=16777216 THEN frozen_input ELSE NULL END FROM private.processing_jobs WHERE json_extract(frozen_input,'$.input.snapshot_id')=? OR json_extract(frozen_input,'$.input.acquisition.snapshot_id')=?").map_err(storage)?;
            stmt.query_map(params![id.to_string(), id.to_string()], |r| {
                r.get::<_, Option<String>>(0)
            })
            .map_err(storage)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(storage)?
        };
        let mut keys = std::collections::BTreeSet::new();
        for raw in rows {
            if cancelled()? {
                return Err(Error::JobCancelled);
            }
            let Some(raw) = raw else {
                return Ok(None);
            };
            let value: serde_json::Value = serde_json::from_str(&raw).map_err(storage)?;
            let Ok(input) = serde_json::from_str::<FrozenJobInput>(&raw) else {
                return Ok(None);
            };
            if serde_json::to_value(&input).map_err(storage)? != value {
                return Ok(None);
            }
            let frozen = match input {
                FrozenJobInput::Acquisition(f) => f,
                FrozenJobInput::MetadataResolution(f) => f.acquisition,
                _ => continue,
            };
            keys.insert(collector_app::acquisition_key(&frozen)?);
        }
        if keys.len() == 1 {
            return Ok(keys.into_iter().next());
        }
        if !keys.is_empty() {
            return Ok(None);
        }
        // After archive-only rebuild, conversion sidecars retain the proven input
        // identity even when private acquisition receipts no longer exist.
        let record = {
            let db = lock(&self.db)?;
            let raw: String = db
                .query_row(
                    "SELECT record FROM snapshots WHERE id=?",
                    [id.to_string()],
                    |r| r.get(0),
                )
                .map_err(storage)?;
            serde_json::from_str::<SnapshotRecord>(&raw).map_err(storage)?
        };
        if let std::collections::btree_map::Entry::Vacant(entry) =
            references.entry(record.collection_id)
        {
            entry.insert(self.portable_snapshot_input_keys(record.collection_id, cancelled)?);
        }
        let Some(mapping) = references
            .get(&record.collection_id)
            .and_then(|keys| keys.get(&id))
        else {
            return Ok(None);
        };
        if mapping.bytes != record.bytes || mapping.digest != record.digest {
            return Err(Error::Conflict);
        }
        Ok(Some(mapping.input_key.clone()))
    }

    fn portable_snapshot_input_keys(
        &self,
        collection: CollectionId,
        cancelled: &mut dyn FnMut() -> Result<bool>,
    ) -> Result<std::collections::BTreeMap<SnapshotId, SnapshotDuplicate>> {
        let root = {
            let db = lock(&self.db)?;
            collection_path(&db, &self.archive, collection)?.join("snapshots")
        };
        let mut keys = std::collections::BTreeMap::<SnapshotId, SnapshotDuplicate>::new();
        let mut after: Option<String> = None;
        loop {
            if cancelled()? {
                return Err(Error::JobCancelled);
            }
            let ids = {
                let db = lock(&self.db)?;
                let mut stmt = db.prepare("SELECT id FROM snapshots WHERE collection_id=? AND (? IS NULL OR id>?) ORDER BY id LIMIT 256").map_err(storage)?;
                stmt.query_map(params![collection.to_string(), after, after], |r| {
                    r.get::<_, String>(0)
                })
                .map_err(storage)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(storage)?
            };
            if ids.is_empty() {
                break;
            }
            after = ids.last().cloned();
            for id in ids {
                if cancelled()? {
                    return Err(Error::JobCancelled);
                }
                let path = root.join(&id).join("payload/reference.v1.json");
                if !path.try_exists().map_err(storage)? {
                    continue;
                }
                let reference: PayloadReference =
                    serde_json::from_slice(&read_bounded_json(&path, 16 * 1024)?)
                        .map_err(storage)?;
                let mapping = reference.mapping;
                if reference.version != 1
                    || mapping.collection != collection
                    || mapping.snapshot.to_string() != id
                    || mapping.snapshot == mapping.canonical
                    || reference.checksum
                        != Digest::sha256(&serde_json::to_vec(&mapping).map_err(storage)?)
                    || root
                        .join(mapping.canonical.to_string())
                        .join("payload/reference.v1.json")
                        .try_exists()
                        .map_err(storage)?
                {
                    return Err(Error::Conflict);
                }
                for id in [mapping.snapshot, mapping.canonical] {
                    if let Some(old) = keys.get(&id)
                        && (old.input_key != mapping.input_key
                            || old.digest != mapping.digest
                            || old.bytes != mapping.bytes)
                    {
                        return Err(Error::Conflict);
                    }
                    keys.insert(id, mapping.clone());
                }
            }
        }
        Ok(keys)
    }

    pub fn apply_snapshot_duplicates(
        &self,
        plan: &SnapshotDuplicatePlan,
        backup: &Path,
    ) -> Result<()> {
        if plan.version != 1 {
            return Err(Error::Invalid("unsupported duplicate plan".into()));
        }
        verify_backup(backup)?;
        let inventory = backup_inventory(backup)?;
        if Path::new(&inventory.archive_root) != self.archive.canonicalize().map_err(storage)?
            || Path::new(&inventory.state_root) != self.state.canonicalize().map_err(storage)?
        {
            return Err(Error::Conflict);
        }
        let plan_bytes = serde_json::to_vec(plan).map_err(storage)?;
        let journal_root = self.state.join("snapshot-conversions");
        let journal = journal_root.join(format!("{}.json", Digest::sha256(&plan_bytes).hex));
        let encoded=serde_json::to_vec(&serde_json::json!({"plan":plan,"backup_sha256":String::from_utf8(read_bounded_json(&backup.join("complete.sha256"),128)?).map_err(storage)?})).map_err(storage)?;
        if journal.try_exists().map_err(storage)? {
            if read_bounded_json(&journal, 16 * 1024 * 1024)? != encoded {
                return Err(Error::Conflict);
            }
        } else {
            if self.inspect_snapshot_duplicates()? != *plan {
                return Err(Error::Conflict);
            }
            verify_backup_roots(self, backup)?;
            fs::create_dir_all(&journal_root).map_err(storage)?;
            write_new(&journal, &encoded)?;
            sync_directory(&journal_root)?;
            sync_directory(&self.state)?;
        }
        self.publication_fault
            .check(PublicationFaultPoint::Prepared)?;
        for mapping in &plan.duplicates {
            let record = collector_app::SnapshotPort::snapshot(
                self,
                collector_app::SnapshotQuery {
                    collection_id: mapping.collection,
                    snapshot_id: mapping.snapshot,
                },
            )?;
            let canonical = collector_app::SnapshotPort::snapshot(
                self,
                collector_app::SnapshotQuery {
                    collection_id: mapping.collection,
                    snapshot_id: mapping.canonical,
                },
            )?;
            if record.bytes != mapping.bytes
                || record.digest != mapping.digest
                || canonical.bytes != mapping.bytes
                || canonical.digest != mapping.digest
            {
                return Err(Error::Conflict);
            }
            let record_key = record
                .acquisition_key
                .clone()
                .or(self.legacy_snapshot_key(record.id)?)
                .ok_or(Error::Conflict)?;
            let canonical_key = canonical
                .acquisition_key
                .clone()
                .or(self.legacy_snapshot_key(canonical.id)?)
                .ok_or(Error::Conflict)?;
            let mut original_comparison = record.clone();
            original_comparison.acquisition_key = Some(record_key.clone());
            let mut canonical_comparison = canonical.clone();
            canonical_comparison.acquisition_key = Some(canonical_key.clone());
            if record_key != mapping.input_key
                || canonical_key != mapping.input_key
                || collector_app::snapshot_response_key(&original_comparison)?
                    != collector_app::snapshot_response_key(&canonical_comparison)?
            {
                return Err(Error::Conflict);
            }
            let target = self.original_snapshot_payload(&canonical)?;
            if target
                .with_file_name("reference.v1.json")
                .try_exists()
                .map_err(storage)?
                || crate::files::hash_file(&target)? != (mapping.bytes, mapping.digest.clone())
            {
                return Err(Error::Conflict);
            }
            let original = self.original_snapshot_payload(&record)?;
            let sidecar = original.with_file_name("reference.v1.json");
            let bytes = payload_reference_bytes(mapping)?;
            let staged = sidecar.with_extension("tmp");
            if staged.try_exists().map_err(storage)? {
                // A crash may leave this conversion's complete staging file. Unknown
                // or partial contents require operator restoration, not deletion.
                if read_bounded_json(&staged, 16 * 1024)? != bytes {
                    return Err(Error::Conflict);
                }
                fs::remove_file(&staged).map_err(storage)?;
                sync_directory(original.parent().ok_or(Error::Conflict)?)?;
            }
            if !Self::known_snapshot_layout(&original)? || !Self::known_snapshot_layout(&target)? {
                return Err(Error::Conflict);
            }
            if sidecar.try_exists().map_err(storage)? {
                if read_bounded_json(&sidecar, 16 * 1024)? != bytes {
                    return Err(Error::Conflict);
                }
            } else {
                if crate::files::hash_file(&original)? != (mapping.bytes, mapping.digest.clone()) {
                    return Err(Error::Conflict);
                }
                let staged = sidecar.with_extension("tmp");
                if staged.try_exists().map_err(storage)? {
                    fs::remove_file(&staged).map_err(storage)?;
                }
                write_new(&staged, &bytes)?;
                fs::rename(&staged, &sidecar).map_err(storage)?;
                sync_directory(original.parent().ok_or(Error::Conflict)?)?;
            }
            self.publication_fault
                .check(PublicationFaultPoint::BytesStaged)?;
            self.resolve_snapshot_payload(&record, &original)?;
            if original.try_exists().map_err(storage)? {
                fs::remove_file(&original).map_err(storage)?;
                sync_directory(original.parent().ok_or(Error::Conflict)?)?;
            }
            self.publication_fault
                .check(PublicationFaultPoint::PayloadPublished)?;
        }
        self.reindex()?;
        self.publication_fault
            .check(PublicationFaultPoint::ProjectionApplied)?;
        // Keep the journal as the immutable conversion receipt; rerunning is safe.
        Ok(())
    }
}

fn payload_reference_bytes(mapping: &SnapshotDuplicate) -> Result<Vec<u8>> {
    serde_json::to_vec(&PayloadReference {
        version: 1,
        mapping: mapping.clone(),
        checksum: Digest::sha256(&serde_json::to_vec(mapping).map_err(storage)?),
    })
    .map_err(storage)
}
