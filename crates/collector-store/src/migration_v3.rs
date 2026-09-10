//! Additive schema-2 to schema-3 archive migration.
//!
//! Every Wave 1 byte remains in place. Schema-3 sidecars remove compatibility
//! Capture/Asset copies from canonical Item revisions and establish standalone
//! append-only Evidence plus revisioned collection-level Asset records.

use super::*;
use crate::archive_v3::{
    AssetRecordV3, CaptureRecordV3, EvidenceRecordV3, EvidenceReferenceV3, ItemRecordV3,
    LegacyCaptureRootV3, LegacyPayloadV3, RevisionEnvelopeV3,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize)]
struct V3Manifest {
    schema_version: u32,
    entity_type: String,
    entity_id: String,
    collection_id: CollectionId,
    revision: Revision,
    record_sha256: String,
}

pub(crate) fn read_item(
    revision_dir: &Path,
    collection_id: CollectionId,
) -> Result<(ItemRecordV3, Vec<EvidenceReferenceV3>)> {
    let manifest_path = revision_dir.join("manifest.v3.toml");
    let record_path = revision_dir.join("record.v3.json");
    let manifest: V3Manifest =
        toml::from_str(&fs::read_to_string(manifest_path).map_err(storage)?).map_err(storage)?;
    let bytes = fs::read(record_path).map_err(storage)?;
    verify_manifest(&manifest, "item", collection_id, &bytes)?;
    let envelope: RevisionEnvelopeV3<ItemRecordV3> =
        serde_json::from_slice(&bytes).map_err(storage)?;
    if envelope.schema_version != 3
        || envelope.record.id.to_string() != manifest.entity_id
        || envelope.record.collection_id != collection_id
        || envelope.record.revision != manifest.revision
    {
        return Err(Error::Storage(
            "schema-3 Item envelope identity mismatch".into(),
        ));
    }
    Ok((envelope.record, envelope.evidence))
}

pub(crate) fn read_evidence(
    collection: &Path,
    collection_id: CollectionId,
) -> Result<Vec<EvidenceRecordV3>> {
    let root = collection.join("evidence");
    if !root.exists() {
        return Ok(vec![]);
    }
    let mut records = Vec::new();
    for directory in directories(&root)? {
        if directory.join("revisions").exists() {
            let mut revisions = directories(&directory.join("revisions"))?;
            revisions.sort();
            let revision = revisions.last().ok_or_else(|| {
                Error::Storage("schema-3 Evidence has no committed revision".into())
            })?;
            let manifest: V3Manifest = toml::from_str(
                &fs::read_to_string(revision.join("manifest.v3.toml")).map_err(storage)?,
            )
            .map_err(storage)?;
            let bytes = fs::read(revision.join("record.v3.json")).map_err(storage)?;
            verify_manifest(&manifest, "evidence", collection_id, &bytes)?;
            let envelope: RevisionEnvelopeV3<EvidenceRecordV3> =
                serde_json::from_slice(&bytes).map_err(storage)?;
            envelope.record.validate()?;
            if envelope.schema_version != 3
                || envelope.record.id.to_string() != manifest.entity_id
                || envelope.record.collection_id != collection_id
            {
                return Err(Error::Storage(
                    "schema-3 Evidence envelope identity mismatch".into(),
                ));
            }
            records.push(envelope.record);
            continue;
        }
        let manifest: V3Manifest = toml::from_str(
            &fs::read_to_string(directory.join("manifest.v3.toml")).map_err(storage)?,
        )
        .map_err(storage)?;
        let bytes = fs::read(directory.join("record.v3.json")).map_err(storage)?;
        verify_manifest(&manifest, "evidence", collection_id, &bytes)?;
        let record: EvidenceRecordV3 = serde_json::from_slice(&bytes).map_err(storage)?;
        record.validate()?;
        if record.id.to_string() != manifest.entity_id || record.collection_id != collection_id {
            return Err(Error::Storage("schema-3 Evidence identity mismatch".into()));
        }
        records.push(record);
    }
    Ok(records)
}

pub(crate) fn read_assets(
    collection: &Path,
    collection_id: CollectionId,
) -> Result<Vec<AssetRecordV3>> {
    let root = collection.join("assets");
    if !root.exists() {
        return Ok(vec![]);
    }
    let mut records = Vec::new();
    for entity in directories(&root)? {
        let revisions_root = entity.join("revisions");
        if !revisions_root.exists() {
            continue;
        }
        let mut revisions = directories(&revisions_root)?;
        revisions.sort();
        let Some(revision) = revisions.last() else {
            continue;
        };
        let manifest: V3Manifest = toml::from_str(
            &fs::read_to_string(revision.join("manifest.v3.toml")).map_err(storage)?,
        )
        .map_err(storage)?;
        let bytes = fs::read(revision.join("record.v3.json")).map_err(storage)?;
        verify_manifest(&manifest, "asset", collection_id, &bytes)?;
        let envelope: RevisionEnvelopeV3<AssetRecordV3> =
            serde_json::from_slice(&bytes).map_err(storage)?;
        if envelope.schema_version != 3
            || envelope.record.asset.id.to_string() != manifest.entity_id
            || envelope.record.collection_id != collection_id
            || envelope.record.revision != manifest.revision
        {
            return Err(Error::Storage(
                "schema-3 Asset envelope identity mismatch".into(),
            ));
        }
        for locator in &envelope.record.legacy_payloads {
            let path = archive_root(collection)?.join(&locator.relative_path);
            if Digest::sha256(&fs::read(path).map_err(storage)?).hex != locator.sha256 {
                return Err(Error::Storage(
                    "legacy Asset locator digest mismatch".into(),
                ));
            }
        }
        records.push(envelope.record);
    }
    Ok(records)
}

pub(crate) fn read_captures(
    collection: &Path,
    collection_id: CollectionId,
) -> Result<Vec<CaptureRecordV3>> {
    let root = collection.join("captures");
    if !root.exists() {
        return Ok(vec![]);
    }
    let mut records = Vec::new();
    for entity in directories(&root)? {
        let revisions_root = entity.join("revisions");
        if !revisions_root.exists() {
            continue;
        }
        let mut revisions = directories(&revisions_root)?;
        revisions.sort();
        let Some(revision) = revisions.last() else {
            continue;
        };
        if !revision.join("manifest.v3.toml").exists() {
            #[derive(Deserialize)]
            struct V2Manifest {
                schema_version: u32,
                entity_id: String,
                collection_id: CollectionId,
                revision: Revision,
                record_sha256: String,
            }
            let manifest: V2Manifest = toml::from_str(
                &fs::read_to_string(revision.join("manifest.toml")).map_err(storage)?,
            )
            .map_err(storage)?;
            let bytes = fs::read(revision.join("record.json")).map_err(storage)?;
            let capture: Capture = serde_json::from_slice(&bytes).map_err(storage)?;
            if manifest.schema_version != 2
                || manifest.collection_id != collection_id
                || manifest.entity_id != capture.id.to_string()
                || manifest.revision != capture.revision
                || Digest::sha256(&bytes).hex != manifest.record_sha256
            {
                return Err(Error::Storage(
                    "schema-2 Capture fallback identity mismatch".into(),
                ));
            }
            let mut legacy_root = None;
            for prior in revisions.iter().rev().skip(1) {
                if prior.join("record.v3.json").exists() {
                    let prior_bytes = fs::read(prior.join("record.v3.json")).map_err(storage)?;
                    let prior: RevisionEnvelopeV3<CaptureRecordV3> =
                        serde_json::from_slice(&prior_bytes).map_err(storage)?;
                    legacy_root = prior.record.legacy_root;
                    break;
                }
            }
            records.push(CaptureRecordV3 {
                capture,
                legacy_root,
            });
            continue;
        }
        let manifest: V3Manifest = toml::from_str(
            &fs::read_to_string(revision.join("manifest.v3.toml")).map_err(storage)?,
        )
        .map_err(storage)?;
        let bytes = fs::read(revision.join("record.v3.json")).map_err(storage)?;
        verify_manifest(&manifest, "capture", collection_id, &bytes)?;
        let envelope: RevisionEnvelopeV3<CaptureRecordV3> =
            serde_json::from_slice(&bytes).map_err(storage)?;
        envelope.record.capture.validate()?;
        if envelope.schema_version != 3
            || envelope.record.capture.id.to_string() != manifest.entity_id
            || envelope.record.capture.collection_id != collection_id
            || envelope.record.capture.revision != manifest.revision
        {
            return Err(Error::Storage(
                "schema-3 Capture envelope identity mismatch".into(),
            ));
        }
        if let Some(locator) = &envelope.record.legacy_root
            && locator.relative_path != "files"
        {
            return Err(Error::Storage(
                "unsupported schema-3 legacy Capture root".into(),
            ));
        }
        records.push(envelope.record);
    }
    Ok(records)
}

fn verify_manifest(
    manifest: &V3Manifest,
    kind: &str,
    collection_id: CollectionId,
    bytes: &[u8],
) -> Result<()> {
    if manifest.schema_version != 3
        || manifest.entity_type != kind
        || manifest.collection_id != collection_id
        || Digest::sha256(bytes).hex != manifest.record_sha256
    {
        return Err(Error::Storage(format!(
            "schema-3 {kind} manifest or digest mismatch"
        )));
    }
    Ok(())
}

fn archive_root(collection: &Path) -> Result<&Path> {
    collection
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| Error::Storage("collection has no archive parent".into()))
}

pub(crate) fn migrate_schema_two(archive: &Path) -> Result<()> {
    let checkpoint = archive.join(".archive-migrations/schema-2-to-3");
    let complete = checkpoint.join("complete");
    if complete.exists() {
        verify_completed_migration(archive, &checkpoint)?;
        return Ok(());
    }
    fs::create_dir_all(checkpoint.join("records")).map_err(storage)?;

    let mut evidence = BTreeMap::<EvidenceId, Vec<u8>>::new();
    let mut assets = BTreeMap::<AssetId, (CollectionId, Vec<u8>, u64)>::new();
    for collection_dir in directories(&archive.join("collections"))? {
        let collection: Collection = toml::from_str(
            &fs::read_to_string(collection_dir.join("collection.toml")).map_err(storage)?,
        )
        .map_err(storage)?;
        migrate_capture_records(archive, &collection_dir, collection.id)?;
        let items_dir = collection_dir.join("items");
        if !items_dir.exists() {
            continue;
        }
        for item_dir in directories(&items_dir)? {
            let revisions_dir = item_dir.join("revisions");
            if !revisions_dir.exists() {
                continue;
            }
            let mut revisions = directories(&revisions_dir)?;
            revisions.sort();
            for revision_dir in revisions {
                let manifest_path = if revision_dir.join("manifest.v2.toml").exists() {
                    revision_dir.join("manifest.v2.toml")
                } else {
                    revision_dir.join("manifest.toml")
                };
                let record_path = if revision_dir.join("record.v2.json").exists() {
                    revision_dir.join("record.v2.json")
                } else {
                    revision_dir.join("record.json")
                };
                let manifest: Manifest =
                    toml::from_str(&fs::read_to_string(&manifest_path).map_err(storage)?)
                        .map_err(storage)?;
                if manifest.schema_version != 2 {
                    return Err(Error::Unavailable(format!(
                        "unsupported item archive schema {} during schema-3 migration",
                        manifest.schema_version
                    )));
                }
                let source = fs::read(&record_path).map_err(storage)?;
                if Digest::sha256(&source).hex != manifest.record_sha256 {
                    return Err(Error::Storage(
                        "schema-2 item digest mismatch during schema-3 migration".into(),
                    ));
                }
                let source_record: RevisionRecord =
                    serde_json::from_slice(&source).map_err(storage)?;
                if source_record.item.id != manifest.item_id
                    || source_record.item.collection_id != collection.id
                    || source_record.item.revision != manifest.revision
                {
                    return Err(Error::Storage(
                        "schema-2 item identity mismatch during schema-3 migration".into(),
                    ));
                }

                let mut references = Vec::new();
                for legacy in &source_record.evidence {
                    let converted = EvidenceRecordV3::from_wave1(collection.id, legacy);
                    converted.validate()?;
                    let canonical = serde_json::to_vec_pretty(&converted).map_err(storage)?;
                    match evidence.get(&converted.id) {
                        Some(previous) if previous != &canonical => {
                            return Err(Error::Storage(format!(
                                "Evidence identity {} has conflicting schema-2 bodies",
                                converted.id
                            )));
                        }
                        Some(_) => {}
                        None => {
                            write_standalone_evidence(
                                archive,
                                &collection_dir,
                                &converted,
                                &canonical,
                            )?;
                            evidence.insert(converted.id, canonical.clone());
                        }
                    }
                    references.push(EvidenceReferenceV3 {
                        id: converted.id,
                        sha256: Digest::sha256(&canonical).hex,
                    });
                }

                for asset in &source_record.item.assets {
                    let legacy_payloads = asset_payloads(archive, &item_dir, asset)?;
                    let probe = serde_json::to_vec(&(asset, &legacy_payloads)).map_err(storage)?;
                    let asset_revision = match assets.get(&asset.id) {
                        Some((owner, _, _)) if *owner != collection.id => {
                            return Err(Error::Storage(
                                "Asset identity is reused across collections".into(),
                            ));
                        }
                        Some((_, previous, revision)) if previous == &probe => *revision,
                        Some((_, _, revision)) => revision.checked_add(1).ok_or_else(|| {
                            Error::Invalid("Asset revision overflow during migration".into())
                        })?,
                        None => 1,
                    };
                    let changed = assets
                        .get(&asset.id)
                        .is_none_or(|(_, previous, _)| previous != &probe);
                    if changed {
                        let record = AssetRecordV3 {
                            collection_id: collection.id,
                            revision: Revision(asset_revision),
                            asset: asset.clone(),
                            deleted: false,
                            updated_at: source_record.item.updated_at,
                            legacy_payloads,
                        };
                        write_revisioned_record(
                            archive,
                            &collection_dir,
                            "assets",
                            asset.id.to_string(),
                            collection.id,
                            Revision(asset_revision),
                            &record,
                        )?;
                    }
                    assets.insert(asset.id, (collection.id, probe, asset_revision));
                }

                let transaction_id = deterministic_transaction(&format!(
                    "item:{}:{}:{}",
                    collection.id, source_record.item.id, source_record.item.revision.0
                ));
                let envelope = RevisionEnvelopeV3::new(
                    transaction_id,
                    ItemRecordV3::from(&source_record.item),
                    references,
                );
                write_item_sidecar(&checkpoint, &revision_dir, &manifest, &envelope)?;
            }
        }
    }
    sync_directory(&checkpoint.join("records"))?;
    migration::write_immutable(&complete, b"schema-2-to-3 complete\n")?;
    sync_directory(&checkpoint)?;
    sync_directory(checkpoint.parent().unwrap())?;
    sync_directory(archive)
}

fn migrate_capture_records(
    archive: &Path,
    collection: &Path,
    collection_id: CollectionId,
) -> Result<()> {
    let root = collection.join("captures");
    if !root.exists() {
        return Ok(());
    }
    for entity in directories(&root)? {
        let revisions_root = entity.join("revisions");
        if !revisions_root.exists() {
            continue;
        }
        for revision in directories(&revisions_root)? {
            let manifest: V3Manifest = {
                #[derive(Deserialize)]
                struct V2Manifest {
                    schema_version: u32,
                    entity_type: String,
                    entity_id: String,
                    collection_id: CollectionId,
                    revision: Revision,
                    record_sha256: String,
                }
                let legacy: V2Manifest = toml::from_str(
                    &fs::read_to_string(revision.join("manifest.toml")).map_err(storage)?,
                )
                .map_err(storage)?;
                if legacy.schema_version != 2
                    || legacy.entity_type != "capture"
                    || legacy.collection_id != collection_id
                {
                    return Err(Error::Storage(
                        "invalid schema-2 Capture manifest during migration".into(),
                    ));
                }
                V3Manifest {
                    schema_version: 3,
                    entity_type: "capture".into(),
                    entity_id: legacy.entity_id,
                    collection_id: legacy.collection_id,
                    revision: legacy.revision,
                    record_sha256: legacy.record_sha256,
                }
            };
            let source = fs::read(revision.join("record.json")).map_err(storage)?;
            if Digest::sha256(&source).hex != manifest.record_sha256 {
                return Err(Error::Storage(
                    "schema-2 Capture digest mismatch during migration".into(),
                ));
            }
            let value: serde_json::Value = serde_json::from_slice(&source).map_err(storage)?;
            let capture: Capture = serde_json::from_value(value.clone()).map_err(storage)?;
            if capture.id.to_string() != manifest.entity_id
                || capture.collection_id != collection_id
                || capture.revision != manifest.revision
            {
                return Err(Error::Storage(
                    "schema-2 Capture identity mismatch during migration".into(),
                ));
            }
            let mut legacy_root = value
                .get("legacy_payload_locator")
                .cloned()
                .map(serde_json::from_value::<LegacyCaptureRootV3>)
                .transpose()
                .map_err(storage)?;
            if legacy_root.is_none()
                && let Some(item_id) = capture.association.item_id
                && find_item_directory(collection, item_id)
                    .is_some_and(|item| item.join("files").exists())
            {
                legacy_root = Some(LegacyCaptureRootV3 {
                    item_id,
                    relative_path: "files".into(),
                });
            }
            let record = CaptureRecordV3 {
                capture,
                legacy_root,
            };
            write_revisioned_record(
                archive,
                collection,
                "captures",
                manifest.entity_id,
                collection_id,
                manifest.revision,
                &record,
            )?;
        }
    }
    Ok(())
}

fn find_item_directory(collection: &Path, id: ItemId) -> Option<PathBuf> {
    directories(&collection.join("items"))
        .ok()?
        .into_iter()
        .find(|path| {
            path.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.ends_with(&id.to_string()))
        })
}

fn write_item_sidecar(
    checkpoint: &Path,
    revision_dir: &Path,
    source_manifest: &Manifest,
    envelope: &RevisionEnvelopeV3<ItemRecordV3>,
) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(envelope).map_err(storage)?;
    let manifest = V3Manifest {
        schema_version: 3,
        entity_type: "item".into(),
        entity_id: source_manifest.item_id.to_string(),
        collection_id: source_manifest.collection_id,
        revision: source_manifest.revision,
        record_sha256: Digest::sha256(&bytes).hex,
    };
    let manifest_bytes = toml::to_string_pretty(&manifest).map_err(storage)?;
    let marker = checkpoint.join("records").join(format!(
        "{}-{:020}.done",
        source_manifest.item_id, source_manifest.revision.0
    ));
    checked_output(&revision_dir.join("record.v3.json"), &bytes)?;
    checked_output(
        &revision_dir.join("manifest.v3.toml"),
        manifest_bytes.as_bytes(),
    )?;
    migration::write_immutable(&marker, manifest.record_sha256.as_bytes())
}

fn write_standalone_evidence(
    archive: &Path,
    collection: &Path,
    record: &EvidenceRecordV3,
    bytes: &[u8],
) -> Result<()> {
    let directory = collection.join("evidence").join(record.id.to_string());
    fs::create_dir_all(&directory).map_err(storage)?;
    let manifest = V3Manifest {
        schema_version: 3,
        entity_type: "evidence".into(),
        entity_id: record.id.to_string(),
        collection_id: record.collection_id,
        revision: Revision(1),
        record_sha256: Digest::sha256(bytes).hex,
    };
    checked_output(&directory.join("record.v3.json"), bytes)?;
    checked_output(
        &directory.join("manifest.v3.toml"),
        toml::to_string_pretty(&manifest)
            .map_err(storage)?
            .as_bytes(),
    )?;
    sync_parents(&directory, archive)
}

fn write_revisioned_record<T: Serialize>(
    archive: &Path,
    collection: &Path,
    kind: &str,
    id: String,
    collection_id: CollectionId,
    revision: Revision,
    record: &T,
) -> Result<()> {
    let transaction_id =
        deterministic_transaction(&format!("{kind}:{collection_id}:{id}:{}", revision.0));
    let envelope = RevisionEnvelopeV3::new(transaction_id, record, vec![]);
    let bytes = serde_json::to_vec_pretty(&envelope).map_err(storage)?;
    let manifest = V3Manifest {
        schema_version: 3,
        entity_type: kind.trim_end_matches('s').into(),
        entity_id: id.clone(),
        collection_id,
        revision,
        record_sha256: Digest::sha256(&bytes).hex,
    };
    let directory = collection
        .join(kind)
        .join(id)
        .join("revisions")
        .join(format!("{:020}", revision.0));
    fs::create_dir_all(&directory).map_err(storage)?;
    checked_output(&directory.join("record.v3.json"), &bytes)?;
    checked_output(
        &directory.join("manifest.v3.toml"),
        toml::to_string_pretty(&manifest)
            .map_err(storage)?
            .as_bytes(),
    )?;
    sync_parents(&directory, archive)
}

fn asset_payloads(archive: &Path, item_dir: &Path, asset: &Asset) -> Result<Vec<LegacyPayloadV3>> {
    let mut payloads = Vec::new();
    for (role, file) in std::iter::once(("original", &asset.file))
        .chain(asset.thumbnail.as_ref().map(|file| ("thumbnail", file)))
    {
        let path = item_dir
            .join("assets")
            .join(asset.id.to_string())
            .join(&file.name);
        if !path.is_file() {
            return Err(Error::Storage(format!(
                "legacy Asset payload is missing: {}",
                path.display()
            )));
        }
        payloads.push(LegacyPayloadV3 {
            role: role.into(),
            relative_path: relative(archive, &path)?,
            sha256: Digest::sha256(&fs::read(path).map_err(storage)?).hex,
        });
    }
    Ok(payloads)
}

fn checked_output(path: &Path, bytes: &[u8]) -> Result<()> {
    if path.exists() {
        if fs::read(path).map_err(storage)? == bytes {
            return Ok(());
        }
        return Err(Error::Storage(format!(
            "schema-3 migration output differs: {}",
            path.display()
        )));
    }
    migration::atomic_replace(path, bytes)
}

fn sync_parents(start: &Path, archive: &Path) -> Result<()> {
    let mut current = Some(start);
    while let Some(path) = current {
        sync_directory(path)?;
        if path == archive {
            break;
        }
        current = path.parent();
    }
    Ok(())
}

fn deterministic_transaction(label: &str) -> TransactionId {
    let mut bytes: [u8; 16] = Sha256::digest(label.as_bytes())[..16].try_into().unwrap();
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    TransactionId(uuid::Uuid::from_bytes(bytes))
}

fn verify_completed_migration(archive: &Path, checkpoint: &Path) -> Result<()> {
    for marker in directories_or_files(&checkpoint.join("records"))? {
        if !marker.is_file() {
            return Err(Error::Storage(
                "schema-3 migration checkpoint contains an unknown directory".into(),
            ));
        }
        let name = marker
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| Error::Storage("invalid schema-3 checkpoint name".into()))?;
        let (id, revision) = name
            .strip_suffix(".done")
            .and_then(|value| value.rsplit_once('-'))
            .ok_or_else(|| Error::Storage("invalid schema-3 checkpoint name".into()))?;
        let item_id: ItemId = id.parse().map_err(storage)?;
        let revision: u64 = revision.parse().map_err(storage)?;
        let mut found = None;
        for collection in directories(&archive.join("collections"))? {
            let items = collection.join("items");
            if !items.exists() {
                continue;
            }
            for item in directories(&items)? {
                if item
                    .file_name()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.ends_with(&item_id.to_string()))
                {
                    found = Some(
                        item.join("revisions")
                            .join(format!("{revision:020}"))
                            .join("manifest.v3.toml"),
                    );
                }
            }
        }
        let manifest_path = found
            .ok_or_else(|| Error::Storage("schema-3 checkpoint refers to a missing Item".into()))?;
        let manifest: V3Manifest =
            toml::from_str(&fs::read_to_string(&manifest_path).map_err(storage)?)
                .map_err(storage)?;
        if fs::read(marker).map_err(storage)? != manifest.record_sha256.as_bytes() {
            return Err(Error::Storage(
                "schema-3 checkpoint does not match its Item manifest".into(),
            ));
        }
    }
    Ok(())
}

fn directories_or_files(path: &Path) -> Result<Vec<PathBuf>> {
    let mut values = fs::read_dir(path)
        .map_err(storage)?
        .map(|entry| entry.map(|entry| entry.path()).map_err(storage))
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    Ok(values)
}
