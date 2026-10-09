//! Wave 4 archive records and package-reader mechanisms.
use super::*;
use collector_app::{
    MetadataAuthorityRecord, PortableProcessingRecord, ProcessingPort, ProcessingQuery,
    ProcessingReadPort, ProvisionalImportPort, SnapshotPort, SnapshotQuery, SnapshotRecord,
};
use rusqlite::TransactionBehavior;
use sha2::{Digest as _, Sha256};
use std::io::{Read, Seek, Write};

const STREAM_CHUNK: usize = 1024 * 1024;
const SNAPSHOT_LIMIT: u64 = 512 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
pub(crate) struct ManifestV4 {
    pub(crate) schema_version: u32,
    pub(crate) entity_type: String,
    pub(crate) entity_id: String,
    pub(crate) collection_id: CollectionId,
    pub(crate) revision: Revision,
    pub(crate) record_sha256: String,
    pub(crate) transaction_id: TransactionId,
}

struct V4Target<'a> {
    transaction_id: TransactionId,
    collection_id: CollectionId,
    kind: &'a str,
    id: String,
    revision: Revision,
}

pub(super) fn representation_fingerprint(
    representation: &Representation,
) -> Result<SourceFingerprint> {
    let source = PackageScope::Representation(representation.id);
    SourceFingerprint::calculate(
        source,
        representation
            .files
            .iter()
            .enumerate()
            .map(|(index, file)| PackageFileFingerprint {
                id: stable_file_id(source, index),
                bytes: file.bytes,
                sha256: file.digest.clone(),
            })
            .collect(),
        None,
        std::collections::BTreeMap::new(),
    )
}

fn stable_file_id(scope: PackageScope, index: usize) -> PackageFileId {
    let (kind, id) = scope_columns(scope);
    PackageFileId(stable_private_uuid(&format!(
        "package-file:v1:{kind}:{id}:{index}"
    )))
}

fn scope_columns(scope: PackageScope) -> (&'static str, String) {
    match scope {
        PackageScope::Import(id) => ("import", id.to_string()),
        PackageScope::Representation(id) => ("representation", id.to_string()),
    }
}

impl Store {
    pub(crate) fn publish_processing_record_under_writer(
        &self,
        job: JobId,
        record: PortableProcessingRecord,
        cancelled: &mut dyn FnMut() -> Result<bool>,
    ) -> Result<()> {
        record.validate()?;
        {
            let db = lock(&self.db)?;
            validate_processing_reference(&db, &record)?;
        }
        if let Ok(existing) = self.processing_record(record.observation.id) {
            return if serde_json::to_value(existing).map_err(storage)?
                == serde_json::to_value(&record).map_err(storage)?
            {
                Ok(())
            } else {
                Err(Error::Storage(
                    "immutable processing observation identity conflicts".into(),
                ))
            };
        }
        let txid = TransactionId(job.0);
        let result_key = collector_app::processing_result_key(&record)?;
        let canonical: Option<String> = lock(&self.db)?
            .query_row(
                "SELECT id FROM processing_groups WHERE result_key=?",
                [&result_key],
                |r| r.get(0),
            )
            .optional()
            .map_err(storage)?;
        if let Some(canonical) = canonical {
            let occurrence = collector_app::ProcessingOccurrence {
                id: record.observation.id,
                collection_id: record.observation.collection_id,
                result_id: canonical.parse().map_err(storage)?,
                result_key,
                snapshot_id: record.observation.snapshot_id,
                import_rebinding: record.import_rebinding.clone(),
                checked_at: record.observation.created_at,
            };
            self.publish_v4(
                V4Target {
                    transaction_id: txid,
                    collection_id: occurrence.collection_id,
                    kind: "processing_occurrence",
                    id: occurrence.id.to_string(),
                    revision: Revision(1),
                },
                &occurrence,
                None,
                cancelled,
            )?;
        } else {
            self.publish_v4(
                V4Target {
                    transaction_id: txid,
                    collection_id: record.observation.collection_id,
                    kind: "processing_observation",
                    id: record.observation.id.to_string(),
                    revision: Revision(1),
                },
                &record,
                None,
                cancelled,
            )?;
        }
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        project_processing(&tx, &record)?;
        tx.execute(
            "UPDATE private.processing_jobs SET output_references=?,checkpoint='published' WHERE job_id=?",
            params![serde_json::to_string(&vec![record.observation.id]).map_err(storage)?,job.to_string()],
        ).map_err(storage)?;
        tx.commit().map_err(storage)?;
        drop(db);
        finish_v3_publication(self, txid)
    }

    fn package_inventory_record(
        &self,
        collection_id: CollectionId,
        source: PackageScope,
    ) -> Result<PackageInventory> {
        let files = match source {
            PackageScope::Import(id) => {
                let record = self.import_record(id)?;
                if record.view.collection_id != collection_id {
                    return Err(Error::Forbidden);
                }
                record
                    .view
                    .files
                    .iter()
                    .enumerate()
                    .map(|(index, file)| {
                        Ok(PackageInventoryFile {
                            id: stable_file_id(source, index),
                            display_name: PackageFileName::new(file.name.clone())?,
                            bytes: file.bytes,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?
            }
            PackageScope::Representation(id) => {
                let capture = reference_index::capture_for_representation(
                    &*lock(&self.db)?,
                    collection_id,
                    id,
                )?;
                let representation = capture
                    .representations
                    .iter()
                    .find(|value| value.id == id)
                    .ok_or(Error::NotFound)?;
                representation
                    .files
                    .iter()
                    .enumerate()
                    .map(|(index, file)| {
                        Ok(PackageInventoryFile {
                            id: stable_file_id(source, index),
                            display_name: PackageFileName::new(file.name.clone())?,
                            bytes: file.bytes,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?
            }
        };
        PackageInventory::new(collection_id, source, files)
    }

    fn package_file_record(
        &self,
        collection_id: CollectionId,
        source: PackageScope,
        file_id: PackageFileId,
    ) -> Result<Box<dyn collector_app::MediaReader>> {
        let inventory = self.package_inventory_record(collection_id, source)?;
        let index = inventory
            .files
            .iter()
            .position(|file| file.id == file_id)
            .ok_or(Error::NotFound)?;
        let path = match source {
            PackageScope::Import(id) => self
                .state
                .join("imports")
                .join(id.to_string())
                .join(format!("{index}.part")),
            PackageScope::Representation(id) => {
                let capture = reference_index::capture_for_representation(
                    &*lock(&self.db)?,
                    collection_id,
                    id,
                )?;
                if let Some(item_id) = capture.association.item_id {
                    self.representation_path(item_id, id, index)?.0
                } else {
                    let collection = {
                        let db = lock(&self.db)?;
                        collection_path(&db, &self.archive, collection_id)?
                    };
                    collection
                        .join("captures")
                        .join(capture.id.to_string())
                        .join("payload")
                        .join(id.to_string())
                        .join(inventory.files[index].display_name.as_str())
                }
            }
        };
        let canonical = fs::canonicalize(&path).map_err(storage)?;
        let root = match source {
            PackageScope::Import(_) => &self.state,
            PackageScope::Representation(_) => &self.archive,
        };
        if !canonical.starts_with(fs::canonicalize(root).map_err(storage)?) {
            return Err(Error::Storage("package path escapes storage root".into()));
        }
        let metadata = canonical.metadata().map_err(storage)?;
        if !metadata.is_file() || metadata.len() != inventory.files[index].bytes.0 {
            return Err(Error::Storage(
                "package file no longer matches its inventory".into(),
            ));
        }
        Ok(Box::new(File::open(canonical).map_err(storage)?))
    }

    fn publish_v4<T: Serialize>(
        &self,
        target: V4Target<'_>,
        value: &T,
        payload: Option<(&mut dyn Read, u64, &Digest)>,
        cancelled: &mut dyn FnMut() -> Result<bool>,
    ) -> Result<Option<String>> {
        let V4Target {
            transaction_id,
            collection_id,
            kind,
            id,
            revision,
        } = target;
        let collection = {
            let db = lock(&self.db)?;
            collection_path(&db, &self.archive, collection_id)?
        };
        let record_bytes = serde_json::to_vec_pretty(value).map_err(storage)?;
        let manifest = ManifestV4 {
            schema_version: 4,
            entity_type: kind.into(),
            entity_id: id.clone(),
            collection_id,
            revision,
            record_sha256: Digest::sha256(&record_bytes).hex,
            transaction_id,
        };
        let manifest_bytes = toml::to_string_pretty(&manifest)
            .map_err(storage)?
            .into_bytes();
        let stage_relative = format!(".staging/{transaction_id}");
        let directory = match kind {
            "metadata_authority" => "metadata_authorities",
            _ => {
                if kind.ends_with('y') {
                    return Err(Error::Invalid("unsupported archive record kind".into()));
                } else {
                    ""
                }
            }
        };
        let directory = if directory.is_empty() {
            format!("{kind}s")
        } else {
            directory.into()
        };
        let destination_relative = format!(
            "{}/{directory}/{id}/revisions/{:020}",
            relative(&self.archive, &collection)?,
            revision.0
        );
        if let Some(result) = &self.command_result {
            self.prepare_command(
                "archive",
                &format!("{destination_relative}/record.v4.json"),
                &Digest::sha256(&record_bytes).hex,
                result,
            )?;
        }
        let payload_relative = payload.as_ref().map(|_| {
            format!(
                "{}/{directory}/{id}/payload/source.bin",
                relative(&self.archive, &collection).expect("validated collection path")
            )
        });
        let mut artifacts = Vec::new();
        if let (Some(path), Some((_, _, digest))) = (&payload_relative, &payload) {
            artifacts.push(publication::JournalArtifact {
                kind: publication::ArtifactKind::Payload,
                staged_path: format!("{stage_relative}/source.bin"),
                published_path: path.clone(),
                sha256: digest.hex.clone(),
            });
        }
        artifacts.extend([
            publication::JournalArtifact {
                kind: publication::ArtifactKind::Revision,
                staged_path: format!("{stage_relative}/record.v4.json"),
                published_path: format!("{destination_relative}/record.v4.json"),
                sha256: Digest::sha256(&record_bytes).hex,
            },
            publication::JournalArtifact {
                kind: publication::ArtifactKind::Revision,
                staged_path: format!("{stage_relative}/manifest.v4.toml"),
                published_path: format!("{destination_relative}/manifest.v4.toml"),
                sha256: Digest::sha256(&manifest_bytes).hex,
            },
        ]);
        let journal = publication::PublicationJournal::new(
            transaction_id,
            kind,
            id,
            Some(collection_id),
            Revision(revision.0.saturating_sub(1)),
            revision,
            artifacts,
        )?;
        let journals = publication::PublicationJournalStore::new(&self.archive);
        if let Some(phase) = journals.phase(transaction_id)? {
            if phase >= publication::PublicationPhase::RevisionCommitted {
                return Ok(payload_relative);
            }
            journals.abort(transaction_id)?;
        }
        journals.write_new(&journal)?;
        self.publication_fault
            .check(PublicationFaultPoint::Prepared)?;
        let stage = self.archive.join(&stage_relative);
        fs::create_dir(&stage).map_err(storage)?;
        if let Some((reader, expected_length, expected_digest)) = payload {
            let mut output = File::create(stage.join("source.bin")).map_err(storage)?;
            let mut hasher = Sha256::new();
            let mut length = 0_u64;
            let mut buffer = vec![0; STREAM_CHUNK];
            loop {
                if cancelled()? {
                    return Err(Error::JobCancelled);
                }
                let read = reader.read(&mut buffer).map_err(storage)?;
                if read == 0 {
                    break;
                }
                length = length
                    .checked_add(read as u64)
                    .ok_or_else(|| Error::Invalid("snapshot length overflow".into()))?;
                if length > SNAPSHOT_LIMIT || length > expected_length {
                    return Err(Error::Invalid("snapshot exceeds its frozen length".into()));
                }
                hasher.update(&buffer[..read]);
                output.write_all(&buffer[..read]).map_err(storage)?;
            }
            output.sync_all().map_err(storage)?;
            let digest = Digest::new(HashAlgorithm::Sha256, hex::encode(hasher.finalize()))?;
            if length != expected_length || &digest != expected_digest {
                return Err(Error::Invalid(
                    "streamed snapshot length or digest mismatch".into(),
                ));
            }
        }
        write_new(&stage.join("record.v4.json"), &record_bytes)?;
        write_new(&stage.join("manifest.v4.toml"), &manifest_bytes)?;
        sync_directory(&stage)?;
        journals.advance(transaction_id, publication::PublicationPhase::BytesStaged)?;
        self.publication_fault
            .check(PublicationFaultPoint::BytesStaged)?;
        // Cancellation and the first irreversible rename share a gate. A request
        // that wins the gate aborts staged bytes; once publication wins, callers
        // observe the durable commit and report cancellation as too late.
        let _publication_gate = lock(&self.job_publication_gate)?;
        if cancelled()? {
            return Err(Error::JobCancelled);
        }
        if let Some(path) = &payload_relative {
            let destination = self.archive.join(path);
            fs::create_dir_all(destination.parent().unwrap()).map_err(storage)?;
            fs::rename(stage.join("source.bin"), &destination).map_err(storage)?;
            sync_directory(destination.parent().unwrap())?;
        }
        journals.advance(
            transaction_id,
            publication::PublicationPhase::PayloadPublished,
        )?;
        self.publication_fault
            .check(PublicationFaultPoint::PayloadPublished)?;
        let destination = self.archive.join(&destination_relative);
        fs::create_dir_all(destination.parent().unwrap()).map_err(storage)?;
        fs::rename(&stage, &destination).map_err(storage)?;
        sync_directory(destination.parent().unwrap())?;
        journals.advance(
            transaction_id,
            publication::PublicationPhase::RevisionCommitted,
        )?;
        self.publication_fault
            .check(PublicationFaultPoint::RevisionCommitted)?;
        Ok(payload_relative)
    }

    pub(crate) fn project_v4_records(&self, tx: &rusqlite::Transaction<'_>) -> Result<()> {
        let mut observations = Vec::new();
        let mut occurrences = Vec::new();
        let mut snapshots = Vec::new();
        let mut recipes = Vec::new();
        let mut authorities = std::collections::BTreeMap::new();
        let mut refreshes = std::collections::BTreeMap::new();
        let mut metadata_candidates = std::collections::BTreeMap::<
            (CollectionId, collector_core::ItemId),
            std::collections::BTreeMap<
                collector_core::MetadataCandidateIdentity,
                collector_core::MetadataCandidateObservation,
            >,
        >::new();
        let mut candidate_versions = std::collections::BTreeMap::<
            (
                CollectionId,
                collector_core::ItemId,
                collector_core::MetadataCandidateIdentity,
            ),
            collector_core::MetadataCandidateObservation,
        >::new();
        for collection_dir in directories(&self.archive.join("collections"))? {
            let collection: Collection = toml::from_str(
                &fs::read_to_string(collection_dir.join("collection.toml")).map_err(storage)?,
            )
            .map_err(storage)?;
            for revision in latest_v4(&collection_dir.join("processing_observations"))? {
                let record: PortableProcessingRecord =
                    read_v4(&revision, collection.id, "processing_observation")?;
                record.validate()?;
                observations.push(record);
            }
            for revision in latest_v4(&collection_dir.join("processing_occurrences"))? {
                let occurrence: collector_app::ProcessingOccurrence =
                    read_v4(&revision, collection.id, "processing_occurrence")?;
                occurrences.push(occurrence);
            }
            for revision in latest_v4(&collection_dir.join("snapshots"))? {
                let record: SnapshotRecord = read_v4(&revision, collection.id, "snapshot")?;
                record.validate()?;
                let payload = revision
                    .parent()
                    .and_then(Path::parent)
                    .unwrap()
                    .join("payload/source.bin");
                let (bytes, digest) = crate::files::hash_file(&payload)?;
                if bytes != record.bytes || digest != record.digest {
                    return Err(Error::Storage("archived snapshot payload mismatch".into()));
                }
                snapshots.push((record, relative(&self.archive, &payload)?));
            }
            for revision in latest_v4(&collection_dir.join("conversion_recipes"))? {
                let record: ConversionRecipe =
                    read_v4(&revision, collection.id, "conversion_recipe")?;
                record.validate()?;
                recipes.push((collection.id, record));
            }
            for revision in all_v4(&collection_dir.join("metadata_authorities"))? {
                let record: MetadataAuthorityRecord =
                    read_v4(&revision, collection.id, "metadata_authority")?;
                record.validate()?;
                if record.collection_id != collection.id {
                    return Err(Error::Storage(
                        "metadata authority collection mismatch".into(),
                    ));
                }
                for observation in &record.observations {
                    let value = (record.collection_id, record.item_id, observation.clone());
                    if let Some(old) = refreshes.insert(observation.id, value.clone())
                        && old != value
                    {
                        return Err(Error::Conflict);
                    }
                    for candidate in &observation.candidates {
                        let version_key = (
                            record.collection_id,
                            record.item_id,
                            candidate.identity.clone(),
                        );
                        if let Some(old) = candidate_versions.insert(version_key, candidate.clone())
                            && old != *candidate
                        {
                            return Err(Error::Conflict);
                        }
                        let candidates = metadata_candidates
                            .entry((record.collection_id, record.item_id))
                            .or_default();
                        if let Some(old) = candidates.get(&candidate.identity)
                            && old != candidate
                        {
                            return Err(Error::Conflict);
                        }
                        candidates.insert(candidate.identity.clone(), candidate.clone());
                        if candidates.len() > 4096 {
                            return Err(Error::Invalid(
                                "metadata candidate history exceeds its bound".into(),
                            ));
                        }
                    }
                }
                let candidates = metadata_candidates
                    .get(&(record.collection_id, record.item_id))
                    .map(|values| values.values().cloned().collect::<Vec<_>>())
                    .unwrap_or_default();
                record.validate_display(&candidates)?;
                authorities.insert(record.item_id, record);
            }
        }
        tx.execute_batch("DELETE FROM processing_members; DELETE FROM processing_groups; DELETE FROM processing_observations; DELETE FROM snapshots; DELETE FROM conversion_recipes; DELETE FROM metadata_authorities; DELETE FROM metadata_candidates; DELETE FROM metadata_refreshes;")
            .map_err(storage)?;
        for (record, path) in snapshots {
            tx.execute(
                "INSERT INTO snapshots(id,collection_id,record,payload_path,created_at) VALUES(?,?,?,?,?)",
                params![
                    record.id.to_string(),
                    record.collection_id.to_string(),
                    serde_json::to_string(&record).map_err(storage)?,
                    path,
                    record.created_at.timestamp()
                ],
            )
            .map_err(storage)?;
        }
        observations.sort_by_key(|record| (record.observation.created_at, record.observation.id));
        for record in observations {
            project_processing(tx, &record)?;
        }
        for occurrence in occurrences {
            let raw: String = tx
                .query_row(
                    "SELECT record FROM processing_observations WHERE id=?",
                    [occurrence.result_id.to_string()],
                    |r| r.get(0),
                )
                .map_err(storage)?;
            let mut record: PortableProcessingRecord =
                serde_json::from_str(&raw).map_err(storage)?;
            if record.observation.collection_id != occurrence.collection_id
                || occurrence.id == occurrence.result_id
            {
                return Err(Error::Storage(
                    "Invalid processing occurrence reference".into(),
                ));
            }
            if collector_app::processing_result_key(&record)? != occurrence.result_key {
                return Err(Error::Storage(
                    "Processing occurrence does not reference an equivalent result".into(),
                ));
            }
            record.observation.id = occurrence.id;
            record.observation.created_at = occurrence.checked_at;
            record.observation.snapshot_id = occurrence.snapshot_id;
            record.import_rebinding = occurrence.import_rebinding;
            record.validate()?;
            if collector_app::processing_result_key(&record)? != occurrence.result_key {
                return Err(Error::Storage(
                    "Processing occurrence result differs from its reference".into(),
                ));
            }
            project_processing(tx, &record)?;
        }
        for (collection, record) in recipes {
            tx.execute(
                "INSERT INTO conversion_recipes(id,collection_id,record) VALUES(?,?,?)",
                params![
                    record.id.to_string(),
                    collection.to_string(),
                    serde_json::to_string(&record).map_err(storage)?
                ],
            )
            .map_err(storage)?;
        }
        for ((collection, item), candidates) in metadata_candidates {
            for candidate in candidates.into_values() {
                project_metadata_candidate(tx, collection, item, &candidate)?;
            }
        }
        for (collection, item, observation) in refreshes.into_values() {
            tx.execute(
                "INSERT INTO metadata_refreshes(id,collection_id,item_id,record) VALUES(?,?,?,?)",
                params![
                    observation.id.to_string(),
                    collection.to_string(),
                    item.to_string(),
                    serde_json::to_string(&observation).map_err(storage)?
                ],
            )
            .map_err(storage)?;
        }
        for record in authorities.into_values() {
            tx.execute("INSERT INTO metadata_authorities(id,collection_id,revision,record) VALUES(?,?,?,?)",params![record.item_id.to_string(),record.collection_id.to_string(),record.revision.0,serde_json::to_string(&record).map_err(storage)?]).map_err(storage)?;
            project_effective_metadata_search(tx, &record)?;
        }
        Ok(())
    }
}

fn latest_v4(root: &Path) -> Result<Vec<PathBuf>> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    for entity in directories(root)? {
        let revisions = entity.join("revisions");
        let mut values = directories(&revisions)?;
        values.sort();
        if let Some(latest) = values.pop() {
            result.push(latest);
        }
    }
    Ok(result)
}

fn all_v4(root: &Path) -> Result<Vec<PathBuf>> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    for entity in directories(root)? {
        let revisions = entity.join("revisions");
        let mut values = directories(&revisions)?;
        values.sort();
        result.extend(values);
    }
    Ok(result)
}

pub(crate) fn read_v4<T: serde::de::DeserializeOwned>(
    path: &Path,
    collection: CollectionId,
    kind: &str,
) -> Result<T> {
    let manifest: ManifestV4 =
        toml::from_str(&fs::read_to_string(path.join("manifest.v4.toml")).map_err(storage)?)
            .map_err(storage)?;
    let bytes = fs::read(path.join("record.v4.json")).map_err(storage)?;
    let entity_id = path
        .parent()
        .and_then(Path::parent)
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        .ok_or_else(|| Error::Storage("invalid schema-4 entity path".into()))?;
    let revision = path
        .file_name()
        .and_then(|value| value.to_str())
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| Error::Storage("invalid schema-4 revision path".into()))?;
    if manifest.schema_version != 4
        || manifest.collection_id != collection
        || manifest.entity_type != kind
        || manifest.entity_id != entity_id
        || manifest.revision != Revision(revision)
        || manifest.record_sha256 != Digest::sha256(&bytes).hex
    {
        return Err(Error::Storage("invalid schema-4 archive envelope".into()));
    }
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(storage)?;
    let body = if kind == "processing_observation" {
        &value["observation"]
    } else {
        &value
    };
    let id_field = if kind == "metadata_authority" {
        "item_id"
    } else {
        "id"
    };
    if body[id_field].as_str() != Some(entity_id)
        || (kind != "conversion_recipe"
            && body["collection_id"].as_str() != Some(collection.to_string().as_str()))
        || (kind == "metadata_authority" && body["revision"].as_u64() != Some(revision))
        || (kind != "metadata_authority" && revision != 1)
    {
        return Err(Error::Storage(
            "archive record identity differs from envelope".into(),
        ));
    }
    // Deserialize the original bytes to preserve lexical raw module envelopes.
    serde_json::from_slice(&bytes).map_err(storage)
}

fn validate_processing_reference(
    tx: &rusqlite::Connection,
    record: &PortableProcessingRecord,
) -> Result<()> {
    if let Some(inputs) = &record.inputs {
        match (&inputs.reference, record.observation.snapshot_id) {
            (Some(reference), Some(id)) => {
                let raw: String = tx
                    .query_row(
                        "SELECT record FROM snapshots WHERE id=? AND collection_id=?",
                        params![id.to_string(), record.observation.collection_id.to_string()],
                        |r| r.get(0),
                    )
                    .map_err(storage)?;
                let snapshot: SnapshotRecord = serde_json::from_str(&raw).map_err(storage)?;
                if snapshot.provider != reference.provider
                    || snapshot.bytes != reference.bytes
                    || snapshot.digest != reference.digest
                    || snapshot.parsed_observation.value().map_err(storage)?
                        != reference.parsed_observation.value().map_err(storage)?
                {
                    return Err(Error::Storage(
                        "Processing reference content does not match its snapshot".into(),
                    ));
                }
            }
            (None, None) => {}
            _ => {
                return Err(Error::Storage(
                    "Processing reference provenance is incomplete".into(),
                ));
            }
        }
    }
    Ok(())
}

fn project_processing(
    tx: &rusqlite::Transaction<'_>,
    record: &PortableProcessingRecord,
) -> Result<()> {
    validate_processing_reference(tx, record)?;
    let (scope, id) = scope_columns(record.observation.source_fingerprint.source);
    tx.execute(
        "INSERT INTO processing_observations(id,collection_id,source_scope,source_id,operation,created_at,record) VALUES(?,?,?,?,?,?,?)",
        params![record.observation.id.to_string(),record.observation.collection_id.to_string(),scope,id,record.observation.operation.to_string(),record.observation.created_at.timestamp(),serde_json::to_string(record).map_err(storage)?],
    ).map_err(storage)?;
    let key = collector_app::processing_result_key(record)?;
    let scan_key = record.inputs.as_ref().map(|v| v.key()).transpose()?;
    tx.execute("INSERT INTO processing_groups(id,result_key,scan_key,first_checked_at,last_checked_at,occurrences) VALUES(?,?,?,?,?,1) ON CONFLICT(result_key) DO UPDATE SET first_checked_at=MIN(first_checked_at,excluded.first_checked_at),last_checked_at=MAX(last_checked_at,excluded.last_checked_at),occurrences=occurrences+1",params![record.observation.id.to_string(),key,scan_key,record.observation.created_at.timestamp_millis(),record.observation.created_at.timestamp_millis()]).map_err(storage)?;
    tx.execute("INSERT INTO processing_members(id,result_id) SELECT ?,id FROM processing_groups WHERE result_key=?",params![record.observation.id.to_string(),key]).map_err(storage)?;
    if let PackageScope::Representation(source) = record.observation.source_fingerprint.source {
        let item:Option<String>=tx.query_row("SELECT c.item_id FROM entity_owners o JOIN captures c ON c.id=o.root_id WHERE o.kind='representation' AND o.id=?",[source.to_string()],|r|r.get(0)).optional().map_err(storage)?.flatten();
        if let Some(item) = item {
            library_care::project_preparation(tx, item.parse().map_err(storage)?)?;
            music_search::refresh(tx, item.parse().map_err(storage)?)?;
        }
    }
    Ok(())
}

fn project_metadata_candidate(
    tx: &rusqlite::Transaction<'_>,
    collection: CollectionId,
    item: collector_core::ItemId,
    candidate: &collector_core::MetadataCandidateObservation,
) -> Result<()> {
    let key = serde_json::to_string(&candidate.identity).map_err(storage)?;
    let raw = serde_json::to_string(candidate).map_err(storage)?;
    let existing: Option<String> = tx
        .query_row(
            "SELECT record FROM metadata_candidates WHERE item_id=? AND candidate_key=?",
            params![item.to_string(), key],
            |r| r.get(0),
        )
        .optional()
        .map_err(storage)?;
    if let Some(existing) = existing {
        if existing != raw {
            return Err(Error::Conflict);
        }
        return Ok(());
    }
    tx.execute(
        "INSERT INTO metadata_candidates(item_id,collection_id,candidate_key,record) VALUES(?,?,?,?)
         ON CONFLICT(item_id,candidate_key) DO NOTHING",
        params![
            item.to_string(),
            collection.to_string(),
            key,
            serde_json::to_string(candidate).map_err(storage)?
        ],
    )
    .map_err(storage)?;
    Ok(())
}

impl ProcessingReadPort for Store {
    fn scan_result(&self, key: &str) -> Result<Option<collector_app::ProcessingResultGroup>> {
        let db = lock(&self.db)?;
        db.query_row("SELECT o.record,g.first_checked_at,g.last_checked_at,g.occurrences FROM processing_groups g JOIN processing_observations o ON o.id=g.id WHERE g.scan_key=? ORDER BY g.last_checked_at DESC,g.id DESC LIMIT 1",[key],read_group).optional().map_err(storage)?.map(parse_group).transpose()
    }
    fn grouped_observations(
        &self,
        query: &ProcessingQuery,
    ) -> Result<Page<collector_app::ProcessingResultGroup>> {
        query.validate()?;
        let (scope, source) = scope_columns(query.source);
        let db = lock(&self.db)?;
        let cursor=query.after.map(|id|db.query_row("SELECT o.created_at,o.id FROM processing_observations o JOIN processing_groups g ON o.id=g.id WHERE o.id=? AND o.collection_id=? AND o.source_scope=? AND o.source_id=? AND (? IS NULL OR o.operation=?)",params![id.to_string(),query.collection_id.to_string(),scope,source,query.operation.as_ref().map(|op|op.as_str()),query.operation.as_ref().map(|op|op.as_str())],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?))).optional().map_err(storage)?.ok_or(Error::NotFound)).transpose()?;
        let mut stmt=db.prepare("SELECT o.record,g.first_checked_at,g.last_checked_at,g.occurrences FROM processing_observations o JOIN processing_groups g ON o.id=g.id WHERE o.collection_id=? AND o.source_scope=? AND o.source_id=? AND (? IS NULL OR o.operation=?) AND (? IS NULL OR (o.created_at,o.id)<(?,?)) ORDER BY o.created_at DESC,o.id DESC LIMIT ?").map_err(storage)?;
        let rows = stmt
            .query_map(
                params![
                    query.collection_id.to_string(),
                    scope,
                    source,
                    query.operation.as_ref().map(ToString::to_string),
                    query.operation.as_ref().map(ToString::to_string),
                    cursor.as_ref().map(|v| v.0),
                    cursor.as_ref().map(|v| v.0),
                    cursor.as_ref().map(|v| v.1.as_str()),
                    i64::from(query.limit) + 1
                ],
                read_group,
            )
            .map_err(storage)?;
        let mut items = rows
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(storage)?
            .into_iter()
            .map(parse_group)
            .collect::<Result<Vec<_>>>()?;
        let next_cursor = if items.len() > usize::from(query.limit) {
            items.truncate(usize::from(query.limit));
            items.last().map(|v| v.observation.id.to_string())
        } else {
            None
        };
        Ok(Page { items, next_cursor })
    }

    fn observations(&self, query: &ProcessingQuery) -> Result<Page<ProcessingObservation>> {
        query.validate()?;
        let (scope, source_id) = scope_columns(query.source);
        let db = lock(&self.db)?;
        let cursor = query.after.map(|after| {
            db.query_row(
                "SELECT created_at,id FROM processing_observations WHERE id=? AND collection_id=? AND source_scope=? AND source_id=? AND (? IS NULL OR operation=?)",
                params![after.to_string(), query.collection_id.to_string(), scope, source_id,
                    query.operation.as_ref().map(ToString::to_string), query.operation.as_ref().map(ToString::to_string)],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            ).optional().map_err(storage)?.ok_or_else(|| Error::Invalid("invalid processing cursor".into()))
        }).transpose()?;
        let mut statement = db.prepare(
            "SELECT record FROM processing_observations WHERE collection_id=? AND source_scope=? AND source_id=? AND (? IS NULL OR operation=?) AND (? IS NULL OR (created_at,id) < (?,?)) ORDER BY created_at DESC,id DESC LIMIT ?"
        ).map_err(storage)?;
        let rows = statement
            .query_map(
                params![
                    query.collection_id.to_string(),
                    scope,
                    source_id,
                    query.operation.as_ref().map(ToString::to_string),
                    query.operation.as_ref().map(ToString::to_string),
                    cursor.as_ref().map(|value| value.0),
                    cursor.as_ref().map(|value| value.0),
                    cursor.as_ref().map(|value| value.1.as_str()),
                    i64::from(query.limit) + 1
                ],
                |row| row.get::<_, String>(0),
            )
            .map_err(storage)?;
        let mut items = rows
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(storage)?
            .into_iter()
            .map(|value| {
                serde_json::from_str::<PortableProcessingRecord>(&value)
                    .map(|value| value.observation)
                    .map_err(storage)
            })
            .collect::<Result<Vec<_>>>()?;
        let next_cursor = if items.len() > usize::from(query.limit) {
            items.truncate(usize::from(query.limit));
            items.last().map(|value| value.id.to_string())
        } else {
            None
        };
        Ok(Page { items, next_cursor })
    }
}

impl SnapshotPort for Store {
    fn snapshot(&self, query: SnapshotQuery) -> Result<SnapshotRecord> {
        let record: String = lock(&self.db)?
            .query_row(
                "SELECT record FROM snapshots WHERE id=? AND collection_id=?",
                params![
                    query.snapshot_id.to_string(),
                    query.collection_id.to_string()
                ],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?
            .ok_or(Error::NotFound)?;
        serde_json::from_str(&record).map_err(storage)
    }
    fn snapshot_bytes(&self, query: SnapshotQuery) -> Result<Box<dyn Read + Send>> {
        let path: String = lock(&self.db)?
            .query_row(
                "SELECT payload_path FROM snapshots WHERE id=? AND collection_id=?",
                params![
                    query.snapshot_id.to_string(),
                    query.collection_id.to_string()
                ],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?
            .ok_or(Error::NotFound)?;
        Ok(Box::new(
            File::open(self.archive.join(path)).map_err(storage)?,
        ))
    }

    fn snapshots(
        &self,
        collection_id: CollectionId,
        after: Option<SnapshotId>,
        limit: u16,
    ) -> Result<Page<SnapshotRecord>> {
        if !(1..=200).contains(&limit) {
            return Err(Error::Invalid("snapshot page limit must be 1..=200".into()));
        }
        let db = lock(&self.db)?;
        let cursor = after
            .map(|id| {
                db.query_row(
                    "SELECT created_at,id FROM snapshots WHERE id=? AND collection_id=?",
                    params![id.to_string(), collection_id.to_string()],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()
                .map_err(storage)?
                .ok_or_else(|| Error::Invalid("invalid snapshot cursor".into()))
            })
            .transpose()?;
        let mut statement = db
            .prepare(
                "SELECT record FROM snapshots WHERE collection_id=? AND (? IS NULL OR (created_at,id) < (?,?)) ORDER BY created_at DESC,id DESC LIMIT ?",
            )
            .map_err(storage)?;
        let rows = statement
            .query_map(
                params![
                    collection_id.to_string(),
                    cursor.as_ref().map(|value| value.0),
                    cursor.as_ref().map(|value| value.0),
                    cursor.as_ref().map(|value| value.1.as_str()),
                    i64::from(limit) + 1
                ],
                |row| row.get::<_, String>(0),
            )
            .map_err(storage)?;
        let mut items = rows
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(storage)?
            .into_iter()
            .map(|record| serde_json::from_str(&record).map_err(storage))
            .collect::<Result<Vec<SnapshotRecord>>>()?;
        let next_cursor = if items.len() > usize::from(limit) {
            items.truncate(usize::from(limit));
            items.last().map(|snapshot| snapshot.id.to_string())
        } else {
            None
        };
        Ok(Page { items, next_cursor })
    }

    fn active_snapshot(
        &self,
        collection_id: CollectionId,
        provider: &ModuleId,
    ) -> Result<Option<SnapshotId>> {
        let value: Option<String> = lock(&self.db)?
            .query_row(
                "SELECT snapshot_id FROM private.active_catalog_snapshots WHERE collection_id=? AND provider=?",
                params![collection_id.to_string(), provider.0],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        value
            .map(|value| value.parse().map_err(storage))
            .transpose()
    }
}

impl ProcessingPort for Store {
    fn ensure_snapshot_current(&self, item: ItemId, snapshot: SnapshotId) -> Result<()> {
        let stale: bool=lock(&self.db)?.query_row("SELECT EXISTS(SELECT 1 FROM private.jobs j JOIN items i ON i.id=j.item_id WHERE j.item_id=?1 AND COALESCE(json_extract(j.result,'$.completion.snapshot_id'),json_extract(j.result,'$.snapshot_id'))=?2 AND COALESCE((SELECT generation FROM private.item_job_generations g WHERE g.job_id=j.id),0)<>COALESCE(json_extract(i.record,'$.lifecycle.generation'),0))",params![item.to_string(),snapshot.to_string()],|row|row.get(0)).map_err(storage)?;
        if stale { Err(Error::Conflict) } else { Ok(()) }
    }

    fn conversion_scratch(&self) -> Result<Box<dyn collector_app::ConversionScratch>> {
        crate::derivatives::scratch(self)
    }
    fn derivative_committed(&self, frozen: &collector_app::FrozenDerivativeJob) -> Result<bool> {
        self.derivative_is_committed(frozen)
    }
    fn publish_derivative(
        &self,
        claim: &JobClaim,
        frozen: &collector_app::FrozenDerivativeJob,
        validation: &collector_app::ConversionValidation,
        scratch: &mut dyn collector_app::ConversionScratch,
        checkpoint: &mut dyn FnMut() -> Result<()>,
    ) -> Result<()> {
        self.publish_music_derivative(claim, frozen, validation, scratch, checkpoint)
    }

    fn package_inventory(
        &self,
        collection: CollectionId,
        source: PackageScope,
    ) -> Result<PackageInventory> {
        self.package_inventory_record(collection, source)
    }
    fn recorded_source(
        &self,
        collection: CollectionId,
        source: PackageScope,
    ) -> Result<SourceFingerprint> {
        if let PackageScope::Representation(id) = source {
            let capture =
                reference_index::capture_for_representation(&*lock(&self.db)?, collection, id)?;
            let representation = capture
                .representations
                .iter()
                .find(|v| v.id == id)
                .ok_or(Error::NotFound)?;
            return representation_fingerprint(representation);
        }
        let inventory = self.package_inventory_record(collection, source)?;
        let digests = match source {
            PackageScope::Import(id) => self
                .import_record(id)?
                .view
                .files
                .into_iter()
                .map(|file| file.digest.ok_or(Error::Conflict))
                .collect::<Result<Vec<_>>>()?,
            PackageScope::Representation(id) => {
                reference_index::capture_for_representation(&*lock(&self.db)?, collection, id)?
                    .representations
                    .into_iter()
                    .find(|value| value.id == id)
                    .ok_or(Error::NotFound)?
                    .files
                    .into_iter()
                    .map(|file| file.digest)
                    .collect::<Vec<_>>()
            }
        };
        SourceFingerprint::calculate(
            source,
            inventory
                .files
                .into_iter()
                .zip(digests)
                .map(|(file, sha256)| PackageFileFingerprint {
                    id: file.id,
                    bytes: file.bytes,
                    sha256,
                })
                .collect(),
            None,
            std::collections::BTreeMap::new(),
        )
    }
    fn package_file(
        &self,
        collection: CollectionId,
        source: PackageScope,
        file: PackageFileId,
    ) -> Result<Box<dyn collector_app::MediaReader>> {
        self.package_file_record(collection, source, file)
    }
    fn save_provisional_observation(
        &self,
        import: ImportId,
        job: JobId,
        record: &ProcessingObservation,
    ) -> Result<()> {
        record.validate()?;
        if record.source_fingerprint.source != PackageScope::Import(import) {
            return Err(Error::Invalid(
                "provisional observation is not Import-scoped".into(),
            ));
        }
        let value = serde_json::to_string(record).map_err(storage)?;
        let db = lock(&self.db)?;
        let existing: Option<String> = db.query_row("SELECT record FROM private.provisional_processing_observations WHERE observation_id=?",[record.id.to_string()],|row|row.get(0)).optional().map_err(storage)?;
        if existing.as_ref().is_some_and(|existing| existing != &value) {
            return Err(Error::Storage(
                "immutable provisional observation conflicts".into(),
            ));
        }
        db.execute("INSERT OR IGNORE INTO private.provisional_processing_observations(observation_id,import_id,job_id,record) VALUES(?,?,?,?)",params![record.id.to_string(),import.to_string(),job.to_string(),value]).map_err(storage)?;
        Ok(())
    }
    fn provisional_observations(&self, import: ImportId) -> Result<Vec<ProcessingObservation>> {
        let db = lock(&self.db)?;
        let mut statement=db.prepare("SELECT record FROM private.provisional_processing_observations WHERE import_id=? ORDER BY observation_id").map_err(storage)?;
        let rows = statement
            .query_map([import.to_string()], |row| row.get::<_, String>(0))
            .map_err(storage)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(storage)?
            .into_iter()
            .map(|value| serde_json::from_str(&value).map_err(storage))
            .collect()
    }
    fn publish_processing_record(
        &self,
        claim: &JobClaim,
        record: PortableProcessingRecord,
    ) -> Result<()> {
        let _writer = lock(&self.writer)?;
        self.validate_publication_claim(claim, record.observation.collection_id)?;
        let mut cancelled = || self.cancellation_requested_record(claim.id, &claim.claim_token);
        self.publish_processing_record_under_writer(claim.id, record, &mut cancelled)
    }
    fn publish_snapshot(
        &self,
        claim: &JobClaim,
        record: SnapshotRecord,
        reader: &mut dyn Read,
        cancelled: &mut dyn FnMut() -> Result<bool>,
    ) -> Result<()> {
        record.validate()?;
        if let Ok(existing) = self.snapshot(SnapshotQuery {
            collection_id: record.collection_id,
            snapshot_id: record.id,
        }) {
            return if serde_json::to_value(existing).map_err(storage)?
                == serde_json::to_value(&record).map_err(storage)?
            {
                Ok(())
            } else {
                Err(Error::Storage(
                    "immutable snapshot identity conflicts".into(),
                ))
            };
        }
        let job = claim.id;
        self.validate_publication_claim(claim, record.collection_id)?;
        let mut guarded = || {
            Ok(self.cancellation_requested_record(claim.id, &claim.claim_token)? || cancelled()?)
        };
        let cancelled: &mut dyn FnMut() -> Result<bool> = &mut guarded;
        // Drain potentially network-backed input before taking the archive writer.
        // The journal publisher reads only this local bounded spool and verifies it.
        let mut spool = tempfile::tempfile_in(&self.state).map_err(storage)?;
        let mut length = 0_u64;
        let mut buffer = vec![0; STREAM_CHUNK];
        loop {
            if cancelled()? {
                return Err(Error::JobCancelled);
            }
            let count = reader.read(&mut buffer).map_err(storage)?;
            if count == 0 {
                break;
            }
            length = length
                .checked_add(count as u64)
                .ok_or_else(|| Error::Invalid("snapshot length overflow".into()))?;
            if length > record.bytes.0 {
                return Err(Error::Invalid("snapshot exceeds declared length".into()));
            }
            spool.write_all(&buffer[..count]).map_err(storage)?;
        }
        if length != record.bytes.0 {
            return Err(Error::Invalid("snapshot length mismatch".into()));
        }
        spool.rewind().map_err(storage)?;
        let _writer = lock(&self.writer)?;
        let txid = TransactionId(job.0);
        let result = self.publish_v4(
            V4Target {
                transaction_id: txid,
                collection_id: record.collection_id,
                kind: "snapshot",
                id: record.id.to_string(),
                revision: Revision(1),
            },
            &record,
            Some((&mut spool, record.bytes.0, &record.digest)),
            cancelled,
        );
        let path = match result {
            Ok(path) => path.unwrap(),
            Err(error) => {
                let journals = publication::PublicationJournalStore::new(&self.archive);
                if journals
                    .phase(txid)?
                    .is_some_and(|phase| phase < publication::PublicationPhase::RevisionCommitted)
                {
                    journals.abort(txid)?;
                }
                return Err(error);
            }
        };
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        tx.execute("INSERT INTO snapshots(id,collection_id,record,payload_path,created_at) VALUES(?,?,?,?,?) ON CONFLICT(id) DO NOTHING",params![record.id.to_string(),record.collection_id.to_string(),serde_json::to_string(&record).map_err(storage)?,path,record.created_at.timestamp()]).map_err(storage)?;
        tx.execute("UPDATE private.processing_jobs SET output_references=?,checkpoint='snapshot_committed' WHERE job_id=?",params![serde_json::to_string(&vec![record.id]).map_err(storage)?,job.to_string()]).map_err(storage)?;
        tx.commit().map_err(storage)?;
        drop(db);
        finish_v3_publication(self, txid)
    }
    fn processing_record(&self, id: ObservationId) -> Result<PortableProcessingRecord> {
        let value: String = lock(&self.db)?
            .query_row(
                "SELECT record FROM processing_observations WHERE id=?",
                [id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?
            .ok_or(Error::NotFound)?;
        serde_json::from_str(&value).map_err(storage)
    }
    fn recipe(&self, id: RecipeId) -> Result<ConversionRecipe> {
        let db = lock(&self.db)?;
        projected_record(&db, "conversion_recipes", &id.to_string())
    }
    fn publish_recipe(
        &self,
        job: JobId,
        collection: CollectionId,
        recipe: ConversionRecipe,
    ) -> Result<()> {
        recipe.validate()?;
        if !matches!(
            recipe.source_fingerprint.source,
            PackageScope::Representation(_)
        ) {
            return Err(Error::Invalid(
                "portable recipe requires a Representation source".into(),
            ));
        }
        let _writer = lock(&self.writer)?;
        match self.recipe(recipe.id) {
            Ok(existing) => {
                return if serde_json::to_value(existing).map_err(storage)?
                    == serde_json::to_value(&recipe).map_err(storage)?
                {
                    Ok(())
                } else {
                    Err(Error::Conflict)
                };
            }
            Err(Error::NotFound) => {}
            Err(error) => return Err(error),
        }
        self.package_inventory_record(collection, recipe.source_fingerprint.source)?;
        let mut never = || Ok(false);
        let txid = TransactionId(job.0);
        self.publish_v4(
            V4Target {
                transaction_id: txid,
                collection_id: collection,
                kind: "conversion_recipe",
                id: recipe.id.to_string(),
                revision: Revision(1),
            },
            &recipe,
            None,
            &mut never,
        )?;
        lock(&self.db)?.execute("INSERT INTO conversion_recipes(id,collection_id,record) VALUES(?,?,?) ON CONFLICT(id) DO NOTHING",params![recipe.id.to_string(),collection.to_string(),serde_json::to_string(&recipe).map_err(storage)?]).map_err(storage)?;
        finish_v3_publication(self, txid)
    }
    fn publish_metadata_authority(
        &self,
        transaction: TransactionId,
        record: MetadataAuthorityRecord,
    ) -> Result<()> {
        self.publish_metadata_record(transaction, record, None)
    }
    fn publish_metadata_for_job(
        &self,
        claim: &JobClaim,
        record: MetadataAuthorityRecord,
    ) -> Result<()> {
        self.publish_metadata_record(TransactionId::new(), record, Some(claim))
    }

    fn metadata_authority(
        &self,
        collection: CollectionId,
        item: collector_core::ItemId,
    ) -> Result<Option<MetadataAuthorityRecord>> {
        let record = lock(&self.db)?
            .query_row(
                "SELECT record FROM metadata_authorities WHERE id=? AND collection_id=?",
                params![item.to_string(), collection.to_string()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(storage)?;
        record
            .map(|value| serde_json::from_str(&value).map_err(storage))
            .transpose()
    }

    fn metadata_refresh_observed(
        &self,
        collection: CollectionId,
        item: collector_core::ItemId,
        observation: collector_core::ObservationId,
    ) -> Result<bool> {
        lock(&self.db)?.query_row("SELECT EXISTS(SELECT 1 FROM metadata_refreshes WHERE id=? AND collection_id=? AND item_id=?)", params![observation.to_string(), collection.to_string(), item.to_string()], |row| row.get(0)).map_err(storage)
    }

    fn metadata_candidates(
        &self,
        collection: CollectionId,
        item: collector_core::ItemId,
    ) -> Result<Vec<collector_core::MetadataCandidateObservation>> {
        let db = lock(&self.db)?;
        let mut statement = db
            .prepare(
                "SELECT record FROM metadata_candidates WHERE collection_id=? AND item_id=? ORDER BY candidate_key LIMIT 4097",
            )
            .map_err(storage)?;
        let rows = statement
            .query_map(params![collection.to_string(), item.to_string()], |row| {
                row.get::<_, String>(0)
            })
            .map_err(storage)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(storage)?
            .into_iter()
            .map(|value| serde_json::from_str(&value).map_err(storage))
            .collect()
    }
}

pub(crate) fn project_effective_metadata_search(
    tx: &rusqlite::Connection,
    record: &MetadataAuthorityRecord,
) -> Result<()> {
    let raw: String = tx
        .query_row(
            "SELECT record FROM items WHERE id=?",
            [record.item_id.to_string()],
            |row| row.get(0),
        )
        .map_err(storage)?;
    let item: collector_core::Item = serde_json::from_str(&raw).map_err(storage)?;
    let mut statement = tx.prepare("SELECT record FROM metadata_candidates WHERE collection_id=? AND item_id=? ORDER BY candidate_key LIMIT 4097").map_err(storage)?;
    let rows = statement
        .query_map(
            params![record.collection_id.to_string(), record.item_id.to_string()],
            |row| row.get::<_, String>(0),
        )
        .map_err(storage)?;
    let candidates = rows
        .map(|row| serde_json::from_str(&row.map_err(storage)?).map_err(storage))
        .collect::<Result<Vec<_>>>()?;
    let display = collector_app::resolve_record_metadata(&item, record, &candidates)?;
    let artist = collector_app::music_search_artist_override(&item, record)?;
    music_search::project(tx, &item, Some(&display), &candidates, artist.as_deref())?;
    tx.execute(
        "UPDATE browse_items SET title_key=? WHERE id=?",
        params![display.title.value.to_lowercase(), item.id.to_string()],
    )
    .map_err(storage)?;
    tx.execute(
        "DELETE FROM search WHERE id=?",
        [record.item_id.to_string()],
    )
    .map_err(storage)?;
    tx.execute(
        "INSERT INTO search VALUES (?,?,?)",
        params![
            item.id.to_string(),
            display.title.value,
            format!(
                "{} {} {}",
                display
                    .subtitle
                    .as_ref()
                    .map(|value| value.value.as_str())
                    .unwrap_or(""),
                collector_app::browse_sort_fields(&item).0,
                collector_app::browse_sort_fields(&item).1
            )
        ],
    )
    .map_err(storage)?;
    library_care::project_health(tx, &item)
}

fn read_group(row: &rusqlite::Row<'_>) -> rusqlite::Result<(String, i64, i64, u64)> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
}
fn parse_group(
    (raw, first, last, occurrences): (String, i64, i64, u64),
) -> Result<collector_app::ProcessingResultGroup> {
    let record: PortableProcessingRecord = serde_json::from_str(&raw).map_err(storage)?;
    Ok(collector_app::ProcessingResultGroup {
        observation: record.observation,
        first_checked_at: chrono::DateTime::from_timestamp_millis(first)
            .ok_or_else(|| Error::Storage("Invalid check date".into()))?,
        last_checked_at: chrono::DateTime::from_timestamp_millis(last)
            .ok_or_else(|| Error::Storage("Invalid check date".into()))?,
        occurrences,
    })
}

impl Store {
    fn validate_publication_claim(&self, claim: &JobClaim, collection: CollectionId) -> Result<()> {
        let record = self.job_record(claim.id, claim.user_id)?;
        if claim.collection_id != Some(collection)
            || record.collection_id != claim.collection_id
            || record.item_id != claim.item_id
            || record.attempt != claim.attempt
            || record.kind != claim.kind
        {
            return Err(Error::Conflict);
        }
        if self.cancellation_requested_record(claim.id, &claim.claim_token)? {
            return Err(Error::JobCancelled);
        }
        Ok(())
    }
}

impl Store {
    fn publish_metadata_record(
        &self,
        transaction: TransactionId,
        record: MetadataAuthorityRecord,
        claim: Option<&JobClaim>,
    ) -> Result<()> {
        record.validate()?;
        if self.item(record.item_id)?.collection_id != record.collection_id {
            return Err(Error::NotFound);
        }
        let _writer = lock(&self.writer)?;
        if let Some(claim) = claim {
            self.validate_publication_claim(claim, record.collection_id)?;
            self.ensure_job_continues(claim.id, Some(&claim.claim_token))?;
        }
        let current = lock(&self.db)?
            .query_row(
                "SELECT revision FROM metadata_authorities WHERE id=?",
                [record.item_id.to_string()],
                |row| row.get::<_, u64>(0),
            )
            .optional()
            .map_err(storage)?
            .unwrap_or(0);
        if record.revision.0 != current.saturating_add(1) {
            return Err(Error::Conflict);
        }
        let item = self.item(record.item_id)?;
        if item.deleted
            || record.inventory.as_ref().is_some_and(|baseline| {
                baseline.revision != item.revision
                    || baseline.title != item.title
                    || baseline.subtitle != item.subtitle
            })
        {
            return Err(Error::Conflict);
        }
        let mut candidates = self.metadata_candidates(record.collection_id, record.item_id)?;
        collector_app::merge_metadata_candidates(
            &mut candidates,
            record
                .observations
                .iter()
                .flat_map(|observation| &observation.candidates),
        )?;
        if !record.observations.is_empty() {
            let db = lock(&self.db)?;
            let mut statement = db
                .prepare(
                    "SELECT record FROM metadata_refreshes WHERE collection_id=? AND item_id=?",
                )
                .map_err(storage)?;
            let rows = statement
                .query_map(
                    params![record.collection_id.to_string(), record.item_id.to_string()],
                    |row| row.get::<_, String>(0),
                )
                .map_err(storage)?;
            for raw in rows {
                let historical: collector_core::MetadataRefreshObservation =
                    serde_json::from_str(&raw.map_err(storage)?).map_err(storage)?;
                for prior in &historical.candidates {
                    if record
                        .observations
                        .iter()
                        .flat_map(|observation| &observation.candidates)
                        .any(|incoming| incoming.identity == prior.identity && incoming != prior)
                    {
                        return Err(Error::Conflict);
                    }
                }
            }
        }
        record.validate_display(&candidates)?;
        for observation in &record.observations {
            let existing: Option<(String, String, String)> = lock(&self.db)?
                .query_row(
                    "SELECT collection_id,item_id,record FROM metadata_refreshes WHERE id=?",
                    [observation.id.to_string()],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()
                .map_err(storage)?;
            if let Some((collection, item, raw)) = existing {
                let old: collector_core::MetadataRefreshObservation =
                    serde_json::from_str(&raw).map_err(storage)?;
                if collection != record.collection_id.to_string()
                    || item != record.item_id.to_string()
                    || old != *observation
                {
                    return Err(Error::Conflict);
                }
            }
        }
        let mut guarded = || {
            if let Some(claim) = claim {
                self.validate_publication_claim(claim, record.collection_id)?;
                let db = lock(&self.db)?;
                let allowed:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM private.users u JOIN private.grants g ON g.user_id=u.id WHERE u.id=? AND u.enabled=1 AND g.collection_id=? AND g.role>=1)",params![claim.user_id.to_string(),record.collection_id.to_string()],|row|row.get(0)).map_err(storage)?;
                if !allowed {
                    return Err(Error::Forbidden);
                }
                drop(db);
                if let Some(collector_app::FrozenJobInput::MetadataResolution(frozen)) =
                    &claim.frozen_input
                {
                    self.validate_item_effect(
                        collector_app::ItemEffectContext {
                            item_id: frozen.item_id,
                            generation: frozen.generation,
                            source: collector_app::ItemEffectSource::Job {
                                id: claim.id,
                                claim: claim.claim_token.parse().map_err(storage)?,
                            },
                        },
                        record.collection_id,
                    )?;
                }
                return self.cancellation_requested_record(claim.id, &claim.claim_token);
            }
            Ok(false)
        };
        self.with_command_result(&record)?.publish_v4(
            V4Target {
                transaction_id: transaction,
                collection_id: record.collection_id,
                kind: "metadata_authority",
                id: record.item_id.to_string(),
                revision: record.revision,
            },
            &record,
            None,
            &mut guarded,
        )?;
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        project_authority_revision(&tx, &record, &candidates)?;

        tx.commit().map_err(storage)?;
        drop(db);
        finish_v3_publication(self, transaction)
    }
}

pub(crate) fn project_authority_revision(
    tx: &rusqlite::Transaction<'_>,
    record: &MetadataAuthorityRecord,
    candidates: &[MetadataCandidateObservation],
) -> Result<()> {
    for observation in &record.observations {
        tx.execute("INSERT OR IGNORE INTO metadata_refreshes(id,collection_id,item_id,record) VALUES(?,?,?,?)", params![observation.id.to_string(),record.collection_id.to_string(),record.item_id.to_string(),serde_json::to_string(observation).map_err(storage)?]).map_err(storage)?;
    }
    for candidate in candidates {
        project_metadata_candidate(tx, record.collection_id, record.item_id, candidate)?;
    }
    for observation in &record.observations {
        for candidate in &observation.candidates {
            project_metadata_candidate(tx, record.collection_id, record.item_id, candidate)?;
        }
    }
    tx.execute("INSERT INTO metadata_authorities(id,collection_id,revision,record) VALUES(?,?,?,?) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,record=excluded.record",params![record.item_id.to_string(),record.collection_id.to_string(),record.revision.0,serde_json::to_string(&record).map_err(storage)?]).map_err(storage)?;
    project_effective_metadata_search(tx, record)?;

    Ok(())
}

impl Store {
    /// Sync a content-addressed private spool before committing its Job reference.
    pub fn stage_uploaded_catalog(&self, bytes: &[u8]) -> Result<Digest> {
        if bytes.is_empty() || bytes.len() > collector_app::MAX_DAT_UPLOAD_BYTES {
            return Err(Error::Invalid(
                "Upload an extracted DAT file between 1 byte and 64 MiB.".into(),
            ));
        }
        let digest = Digest::new(HashAlgorithm::Sha256, hex::encode(Sha256::digest(bytes)))?;
        let directory = self.state.join("catalog-uploads");
        fs::create_dir_all(&directory).map_err(storage)?;
        let target = directory.join(&digest.hex);
        let mut staged = tempfile::NamedTempFile::new_in(&directory).map_err(storage)?;
        staged.write_all(bytes).map_err(storage)?;
        staged.as_file().sync_all().map_err(storage)?;
        match staged.persist_noclobber(&target) {
            Ok(_) => {}
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                let file = File::open(&target).map_err(storage)?;
                let valid = if file.metadata().map_err(storage)?.len() == bytes.len() as u64 {
                    let mut existing = Vec::with_capacity(bytes.len());
                    file.take(bytes.len() as u64 + 1)
                        .read_to_end(&mut existing)
                        .map_err(storage)?;
                    existing == bytes
                } else {
                    false
                };
                if !valid {
                    // Repair only private staging from freshly supplied bytes with
                    // this exact digest. Immutable archive snapshots are untouched.
                    error
                        .file
                        .persist(&target)
                        .map_err(|error| storage(error.error))?;
                }
            }
            Err(error) => return Err(storage(error.error)),
        }
        File::open(&directory)
            .and_then(|file| file.sync_all())
            .map_err(storage)?;
        Ok(digest)
    }
}
impl collector_app::UploadedCatalogPort for Store {
    fn stage_uploaded_catalog(&self, bytes: &[u8]) -> Result<Digest> {
        Store::stage_uploaded_catalog(self, bytes)
    }
    fn uploaded_catalog(
        &self,
        upload: &collector_app::UploadedDatCatalog,
    ) -> Result<Box<dyn collector_app::MediaReader>> {
        upload.validate()?;
        let file = File::open(self.state.join("catalog-uploads").join(&upload.digest.hex))
            .map_err(storage)?;
        if file.metadata().map_err(storage)?.len() != upload.bytes.0 {
            return Err(Error::Invalid(
                "Saved DAT upload length changed; upload the original DAT again.".into(),
            ));
        }
        Ok(Box::new(file))
    }
}
