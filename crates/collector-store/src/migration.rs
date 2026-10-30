//! One-time schema-1 to schema-2 archive conversion. This is deliberately narrower
//! than P01's future publication transaction: immutable per-revision sidecars are
//! checkpointed and the completion marker is written last.
use super::*;
use serde::Deserialize;
use serde_json::{Map, Value, json};
use sha2::{Digest as _, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Deserialize)]
struct RawRevision {
    item: RawItem,
}
#[derive(Deserialize)]
struct RawItem {
    domain: ModuleData,
    metadata: RawMetadata,
}
#[derive(Deserialize)]
struct RawMetadata {
    candidates: Vec<RawCandidate>,
    selected: Option<String>,
    overrides: BTreeMap<String, Value>,
}
#[derive(Deserialize)]
struct RawCandidate {
    id: String,
    provider: ModuleId,
    title: String,
    subtitle: String,
    language: LanguageTag,
    strength: MatchStrength,
    observation: RawJson,
}

pub(crate) fn migrate_schema_one(archive: &Path) -> Result<()> {
    let checkpoint = archive.join(".archive-migrations").join("schema-1-to-2");
    let completed = checkpoint.join("complete");
    if completed.try_exists().map_err(storage)? {
        return Ok(());
    }
    fs::create_dir_all(checkpoint.join("records")).map_err(storage)?;

    let mut referenced_works = BTreeMap::<WorkId, (CollectionId, BTreeSet<ItemId>)>::new();
    let mut referenced_editions =
        BTreeMap::<EditionId, (CollectionId, Option<WorkId>, BTreeSet<ItemId>)>::new();
    let mut capture_states = BTreeMap::<CaptureId, (CollectionId, Vec<u8>, u64)>::new();
    let mut latest_items = BTreeMap::<ItemId, Item>::new();
    let mut item_owners = BTreeMap::<ItemId, CollectionId>::new();
    let mut specimen_owners = BTreeMap::<SpecimenId, CollectionId>::new();
    let mut component_owners = BTreeMap::<ComponentId, CollectionId>::new();
    let mut carrier_owners = BTreeMap::<CarrierId, CollectionId>::new();
    let mut representation_owners = BTreeMap::<RepresentationId, CollectionId>::new();
    let mut asset_owners = BTreeMap::<AssetId, CollectionId>::new();
    let mut evidence_owners = BTreeMap::<EvidenceId, CollectionId>::new();
    for collection_dir in directories(&archive.join("collections"))? {
        let collection: Collection = toml::from_str(
            &fs::read_to_string(collection_dir.join("collection.toml")).map_err(storage)?,
        )
        .map_err(storage)?;
        let items_dir = collection_dir.join("items");
        if !items_dir.try_exists().map_err(storage)? {
            continue;
        }
        for item_dir in directories(&items_dir)? {
            let revisions_dir = item_dir.join("revisions");
            if !revisions_dir.try_exists().map_err(storage)? {
                continue;
            }
            let mut versions = directories(&revisions_dir)?;
            versions.sort();
            for version in versions {
                let manifest_path = version.join("manifest.toml");
                if !manifest_path.try_exists().map_err(storage)? {
                    continue;
                }
                let manifest: Manifest =
                    toml::from_str(&fs::read_to_string(&manifest_path).map_err(storage)?)
                        .map_err(storage)?;
                if manifest.schema_version == 2 {
                    continue;
                }
                if manifest.schema_version != 1 {
                    return Err(Error::Unavailable(format!(
                        "unsupported archive revision schema {}",
                        manifest.schema_version
                    )));
                }
                let original_path = version.join("record.json");
                let original = fs::read(&original_path).map_err(storage)?;
                if Digest::sha256(&original).hex != manifest.record_sha256 {
                    return Err(Error::Storage(
                        "schema-1 record digest mismatch during migration".into(),
                    ));
                }
                let raw: RawRevision = serde_json::from_slice(&original).map_err(storage)?;
                let mut root: Value = serde_json::from_slice(&original).map_err(storage)?;
                let item = root
                    .get_mut("item")
                    .and_then(Value::as_object_mut)
                    .ok_or_else(|| Error::Invalid("schema-1 revision has no item object".into()))?;
                let work_id = optional_id::<WorkId>(item.remove("work_id"))?;
                let edition_id = optional_id::<EditionId>(item.remove("edition_id"))?;
                if let Some(id) = work_id {
                    match referenced_works.get(&id) {
                        Some((owner, _)) if *owner != collection.id => {
                            return Err(Error::Invalid("work reused across collections".into()));
                        }
                        Some(_) => {
                            referenced_works
                                .get_mut(&id)
                                .unwrap()
                                .1
                                .insert(manifest.item_id);
                        }
                        None => {
                            referenced_works
                                .insert(id, (collection.id, BTreeSet::from([manifest.item_id])));
                        }
                    }
                }
                if let Some(id) = edition_id {
                    let value = (collection.id, work_id, BTreeSet::from([manifest.item_id]));
                    match referenced_editions.get(&id) {
                        Some((owner, existing_work, _))
                            if *owner != collection.id
                                || (*existing_work).is_some()
                                    && work_id.is_some()
                                    && *existing_work != work_id =>
                        {
                            return Err(Error::Invalid("edition is reused across collections or associated with conflicting works".into()));
                        }
                        Some((_, None, items)) if work_id.is_some() => {
                            let mut items = items.clone();
                            items.insert(manifest.item_id);
                            referenced_editions.insert(id, (collection.id, work_id, items));
                        }
                        Some(_) => {
                            referenced_editions
                                .get_mut(&id)
                                .unwrap()
                                .2
                                .insert(manifest.item_id);
                        }
                        None => {
                            referenced_editions.insert(id, value);
                        }
                    }
                }
                item.insert(
                    "identity_link".into(),
                    match (edition_id, work_id) {
                        (Some(edition_id), _) => json!({"kind":"edition","edition_id":edition_id}),
                        (None, Some(work_id)) => json!({"kind":"work","work_id":work_id}),
                        _ => json!({"kind":"none"}),
                    },
                );
                migrate_specimens(item)?;
                migrate_captures(item, collection.id, manifest.item_id)?;
                migrate_assets(item, manifest.item_id)?;
                let mut record: RevisionRecord = serde_json::from_value(root).map_err(storage)?;
                record.item.domain = raw.item.domain;
                record.item.metadata = MetadataState {
                    candidates: raw
                        .item
                        .metadata
                        .candidates
                        .into_iter()
                        .map(|candidate| MetadataCandidate {
                            id: candidate.id,
                            provider: candidate.provider,
                            title: candidate.title,
                            subtitle: candidate.subtitle,
                            language: candidate.language,
                            strength: candidate.strength,
                            observation: candidate.observation,
                        })
                        .collect(),
                    selected: raw.item.metadata.selected,
                    overrides: raw.item.metadata.overrides,
                };
                for capture in &mut record.item.captures {
                    capture.revision = Revision(0);
                    let signature = serde_json::to_vec(capture).map_err(storage)?;
                    let revision = match capture_states.get(&capture.id) {
                        Some((owner, _, _)) if *owner != collection.id => {
                            return Err(Error::Invalid(
                                "capture identity is reused across collections".into(),
                            ));
                        }
                        Some((_, previous, revision)) if *previous == signature => *revision,
                        Some((_, _, revision)) => revision
                            .checked_add(1)
                            .ok_or_else(|| Error::Invalid("capture revision overflow".into()))?,
                        None => 1,
                    };
                    capture.revision = Revision(revision);
                    let changed = capture_states
                        .get(&capture.id)
                        .is_none_or(|(_, previous, _)| *previous != signature);
                    capture_states.insert(capture.id, (collection.id, signature, revision));
                    if changed {
                        emit_typed_record(
                            &collection_dir,
                            "captures",
                            capture.id.to_string(),
                            capture.collection_id,
                            revision,
                            capture,
                        )?;
                    }
                }
                record.item.validate()?;
                claim_owner(&mut item_owners, record.item.id, collection.id)?;
                for specimen in &record.item.specimens {
                    claim_owner(&mut specimen_owners, specimen.id, collection.id)?;
                    for component in &specimen.components {
                        claim_owner(&mut component_owners, component.id, collection.id)?;
                        if let Some(carrier) = &component.carrier {
                            claim_owner(&mut carrier_owners, carrier.id, collection.id)?;
                        }
                    }
                }
                for representation in record
                    .item
                    .captures
                    .iter()
                    .flat_map(|capture| &capture.representations)
                {
                    claim_owner(&mut representation_owners, representation.id, collection.id)?;
                }
                for asset in &record.item.assets {
                    claim_owner(&mut asset_owners, asset.id, collection.id)?;
                }
                for evidence in &record.evidence {
                    claim_owner(&mut evidence_owners, evidence.id, collection.id)?;
                }
                latest_items.insert(record.item.id, record.item.clone());
                record.legacy = Some(LegacyRevisionLocator {
                    relative_path: relative(archive, &original_path)?,
                    record_sha256: manifest.record_sha256.clone(),
                });
                let canonical = serde_json::to_vec_pretty(&record).map_err(storage)?;
                let migrated_manifest = Manifest {
                    schema_version: 2,
                    entity_type: Some("item".into()),
                    item_id: manifest.item_id,
                    collection_id: manifest.collection_id,
                    revision: manifest.revision,
                    record_sha256: Digest::sha256(&canonical).hex,
                };
                let checkpoint_path = checkpoint.join("records").join(format!(
                    "{}-{:020}.done",
                    manifest.item_id, manifest.revision.0
                ));
                let manifest_bytes = toml::to_string_pretty(&migrated_manifest).map_err(storage)?;
                if checkpoint_path.try_exists().map_err(storage)? {
                    // Once checkpointed, every output must still match. Never repair tampering.
                    for (path, expected) in [
                        (version.join("record.v2.json"), canonical.as_slice()),
                        (version.join("manifest.v2.toml"), manifest_bytes.as_bytes()),
                        (
                            checkpoint_path.clone(),
                            migrated_manifest.record_sha256.as_bytes(),
                        ),
                    ] {
                        if fs::read(&path).map_err(storage)? != expected {
                            return Err(Error::Storage(format!(
                                "committed migration output differs: {}",
                                path.display()
                            )));
                        }
                    }
                } else {
                    atomic_replace(&version.join("record.v2.json"), &canonical)?;
                    atomic_replace(&version.join("manifest.v2.toml"), manifest_bytes.as_bytes())?;
                    write_immutable(&checkpoint_path, migrated_manifest.record_sha256.as_bytes())?;
                }
            }
        }
        emit_identity_records(
            &collection_dir,
            &collection,
            &referenced_works,
            &referenced_editions,
        )?;
    }
    sync_directory(&checkpoint.join("records"))?;
    for collection_dir in directories(&archive.join("collections"))? {
        let path = collection_dir.join("collection.toml");
        if !path.try_exists().map_err(storage)? {
            continue;
        }
        let collection: Collection =
            toml::from_str(&fs::read_to_string(path).map_err(storage)?).map_err(storage)?;
        CollectionGraph {
            works: super::read_latest_records(&collection_dir.join("works"), collection.id)?,
            editions: super::read_latest_records(&collection_dir.join("editions"), collection.id)?,
            items: latest_items
                .values()
                .filter(|item| item.collection_id == collection.id)
                .cloned()
                .collect(),
            captures: super::migration_v3::read_captures(&collection_dir, collection.id)?
                .into_iter()
                .map(|record| record.capture)
                .collect(),
            assets: latest_items
                .values()
                .filter(|item| item.collection_id == collection.id)
                .flat_map(|item| item.assets.clone())
                .collect(),
        }
        .validate(collection.id)?;
    }
    write_immutable(&completed, b"schema-1-to-2 complete\n")?;
    sync_directory(&checkpoint)?;
    sync_directory(checkpoint.parent().unwrap())?;
    sync_directory(archive)?;
    Ok(())
}

fn migrate_specimens(item: &mut Map<String, Value>) -> Result<()> {
    let Some(specimens) = item.get_mut("specimens").and_then(Value::as_array_mut) else {
        return Ok(());
    };
    for specimen in specimens {
        let object = specimen
            .as_object_mut()
            .ok_or_else(|| Error::Invalid("legacy specimen is not an object".into()))?;
        let carriers = object.remove("carriers").unwrap_or_else(|| json!([]));
        let values = carriers
            .as_array()
            .ok_or_else(|| Error::Invalid("legacy carriers are not an array".into()))?;
        let mut positions = BTreeSet::new();
        let mut components = Vec::new();
        for (index, value) in values.iter().enumerate() {
            let carrier = value
                .as_object()
                .ok_or_else(|| Error::Invalid("legacy carrier is not an object".into()))?;
            let legacy_id: CarrierId = serde_json::from_value(
                carrier
                    .get("id")
                    .cloned()
                    .ok_or_else(|| Error::Invalid("legacy carrier has no id".into()))?,
            )
            .map_err(storage)?;
            let legacy_kind = carrier
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("other");
            let requested = carrier
                .get("position")
                .and_then(Value::as_u64)
                .map(|value| value as u16)
                .unwrap_or(index as u16 + 1);
            let mut position = requested;
            while !positions.insert(position) {
                position = position
                    .checked_add(1)
                    .ok_or_else(|| Error::Invalid("component position overflow".into()))?;
            }
            let component_kind = match legacy_kind {
                "cartridge" => "cartridge",
                "optical_disc" => "optical_disc",
                "tape" => "tape",
                "card" => "memory_card",
                "case" => "case",
                "booklet" => "booklet",
                _ => "other_data",
            };
            let data_bearing = !matches!(legacy_kind, "case" | "booklet");
            components.push(json!({
                "id": deterministic_component(legacy_id),
                "label": carrier.get("label").and_then(Value::as_str).unwrap_or_default(),
                "kind": component_kind,
                "position": position,
                "carrier": if data_bearing { json!({"id":legacy_id,"label":carrier.get("label").and_then(Value::as_str).unwrap_or_default(),"kind":legacy_kind}) } else { Value::Null },
                "migration_provenance": {"legacy_carrier_id":legacy_id,"legacy_kind":legacy_kind,"legacy_position":carrier.get("position").cloned().unwrap_or(Value::Null)}
            }));
        }
        object.insert("components".into(), Value::Array(components));
    }
    Ok(())
}

fn migrate_captures(
    item: &mut Map<String, Value>,
    collection: CollectionId,
    item_id: ItemId,
) -> Result<()> {
    let Some(captures) = item.get_mut("captures").and_then(Value::as_array_mut) else {
        return Ok(());
    };
    for capture in captures {
        let object = capture
            .as_object_mut()
            .ok_or_else(|| Error::Invalid("legacy capture is not an object".into()))?;
        let carrier = object.remove("carrier_id").unwrap_or(Value::Null);
        object.insert("collection_id".into(), json!(collection));
        object.insert("revision".into(), json!(1));
        object.insert(
            "association".into(),
            json!({"item_id":item_id,"specimen_id":null,"component_id":null,"carrier_id":carrier}),
        );
        object.insert(
            "legacy_payload_locator".into(),
            json!({"item_id":item_id,"relative_path":"files"}),
        );
    }
    Ok(())
}

fn migrate_assets(item: &mut Map<String, Value>, item_id: ItemId) -> Result<()> {
    let Some(assets) = item.get_mut("assets").and_then(Value::as_array_mut) else {
        return Ok(());
    };
    for asset in assets {
        asset
            .as_object_mut()
            .ok_or_else(|| Error::Invalid("legacy asset is not an object".into()))?
            .insert("subject".into(), json!({"kind":"item","item_id":item_id}));
    }
    Ok(())
}

fn emit_identity_records(
    collection_dir: &Path,
    collection: &Collection,
    works: &BTreeMap<WorkId, (CollectionId, BTreeSet<ItemId>)>,
    editions: &BTreeMap<EditionId, (CollectionId, Option<WorkId>, BTreeSet<ItemId>)>,
) -> Result<()> {
    for (id, (_, items)) in works
        .iter()
        .filter(|(_, (owner, _))| *owner == collection.id)
    {
        emit_typed_record(
            collection_dir,
            "works",
            id.to_string(),
            collection.id,
            1,
            &Work {
                id: *id,
                collection_id: collection.id,
                revision: Revision(1),
                provisional: true,
                title: String::new(),
                translations: vec![],
                identifiers: vec![],
                migration_provenance: items
                    .iter()
                    .map(|item_id| LegacyRelationshipProvenance {
                        item_id: *item_id,
                        legacy_field: "work_id".into(),
                    })
                    .collect(),
                deleted: false,
                updated_at: collection.created_at,
            },
        )?;
    }
    for (id, (_, work_id, items)) in editions
        .iter()
        .filter(|(_, (owner, _, _))| *owner == collection.id)
    {
        emit_typed_record(
            collection_dir,
            "editions",
            id.to_string(),
            collection.id,
            1,
            &Edition {
                id: *id,
                collection_id: collection.id,
                revision: Revision(1),
                work_id: *work_id,
                provisional: true,
                title: String::new(),
                expected_components: vec![],
                identifiers: vec![],
                migration_provenance: items
                    .iter()
                    .map(|item_id| LegacyRelationshipProvenance {
                        item_id: *item_id,
                        legacy_field: "edition_id".into(),
                    })
                    .collect(),
                deleted: false,
                updated_at: collection.created_at,
            },
        )?;
    }
    Ok(())
}

pub(crate) fn emit_typed_record<T: Serialize>(
    collection: &Path,
    kind: &str,
    id: String,
    collection_id: CollectionId,
    revision: u64,
    value: &T,
) -> Result<()> {
    let directory = collection
        .join(kind)
        .join(&id)
        .join("revisions")
        .join(format!("{revision:020}"));
    let archive = collection
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| Error::Storage("collection has no archive parent".into()))?;
    // Older migration attempts wrote directly into the final directory. Retain
    // unfinished records for diagnosis, then republish atomically from the source.
    if directory.try_exists().map_err(storage)?
        && !directory
            .join("manifest.toml")
            .try_exists()
            .map_err(storage)?
    {
        if archive
            .join(".archive-migrations/schema-1-to-2/complete")
            .try_exists()
            .map_err(storage)?
        {
            return Err(Error::Storage(
                "committed typed revision is missing its manifest".into(),
            ));
        }
        fs::rename(
            &directory,
            archive
                .join(".staging")
                .join(format!("migration-incomplete-{}", TransactionId::new())),
        )
        .map_err(storage)?;
        sync_directory(directory.parent().unwrap())?;
    }
    publish_typed_revision(
        archive,
        collection,
        kind,
        id,
        collection_id,
        Revision(revision),
        value,
    )
}

pub(crate) fn emit_capture_records(collection: &Path, item: &Item) -> Result<()> {
    for capture in &item.captures {
        emit_typed_record(
            collection,
            "captures",
            capture.id.to_string(),
            capture.collection_id,
            capture.revision.0.max(1),
            capture,
        )?;
    }
    Ok(())
}

fn deterministic_component(carrier: CarrierId) -> ComponentId {
    let mut bytes: [u8; 16] = Sha256::digest(format!("collector-component:{carrier}")).as_slice()
        [..16]
        .try_into()
        .unwrap();
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    ComponentId(uuid::Uuid::from_bytes(bytes))
}

fn optional_id<T: std::str::FromStr>(value: Option<Value>) -> Result<Option<T>>
where
    T::Err: std::fmt::Display,
{
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => value.parse().map(Some).map_err(storage),
        _ => Err(Error::Invalid("legacy identity is not a string".into())),
    }
}
fn claim_owner<K: Ord + Copy>(
    owners: &mut BTreeMap<K, CollectionId>,
    id: K,
    collection: CollectionId,
) -> Result<()> {
    if owners.get(&id).is_some_and(|owner| *owner != collection) {
        return Err(Error::Invalid(
            "identity is reused across collections".into(),
        ));
    }
    owners.entry(id).or_insert(collection);
    Ok(())
}
pub(crate) fn write_immutable(path: &Path, bytes: &[u8]) -> Result<()> {
    if path.try_exists().map_err(storage)? {
        if fs::read(path).map_err(storage)? == bytes {
            return Ok(());
        }
        return Err(Error::Storage(format!(
            "migration output differs from durable checkpoint: {}",
            path.display()
        )));
    }
    atomic_replace(path, bytes)
}

/// Write and sync a sibling temporary file before publishing its name. Callers
/// decide whether the destination is an uncommitted output or immutable checkpoint.
pub(crate) fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::Storage("migration output has no parent".into()))?;
    let temporary = parent.join(format!(".migration-{}.tmp", TransactionId::new()));
    write_new(&temporary, bytes)?;
    fs::rename(&temporary, path).map_err(storage)?;
    sync_directory(parent)
}
