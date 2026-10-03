//! Indexed graph neighborhoods; domain validation remains in CollectionGraph.
use super::*;
use std::collections::{BTreeSet, HashSet};

pub(crate) fn install(db: &Connection) -> Result<()> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS entity_owners(kind TEXT NOT NULL,id TEXT NOT NULL,collection_id TEXT NOT NULL,root_kind TEXT NOT NULL,root_id TEXT NOT NULL,PRIMARY KEY(kind,id));
         CREATE INDEX IF NOT EXISTS entity_owner_root ON entity_owners(root_kind,root_id);
         CREATE TABLE IF NOT EXISTS entity_references(root_kind TEXT NOT NULL,root_id TEXT NOT NULL,target_kind TEXT NOT NULL,target_id TEXT NOT NULL,collection_id TEXT NOT NULL,PRIMARY KEY(root_kind,root_id,target_kind,target_id));
         CREATE INDEX IF NOT EXISTS entity_reference_target ON entity_references(target_kind,target_id,root_kind,root_id);",
    ).map_err(storage)?;
    for (table, kind) in [
        ("items", "item"),
        ("works", "work"),
        ("editions", "edition"),
        ("captures", "capture"),
        ("assets", "asset"),
    ] {
        let mut insert = format!(
            "INSERT INTO entity_owners VALUES('{kind}',NEW.id,NEW.collection_id,'{kind}',NEW.id);"
        );
        if kind == "item" {
            for (nested, source, id) in [
                (
                    "specimen",
                    "json_each(NEW.record,'$.specimens') s",
                    "json_extract(s.value,'$.id')",
                ),
                (
                    "loan",
                    "json_each(NEW.record,'$.specimens') s,json_each(s.value,'$.loans') l",
                    "json_extract(l.value,'$.id')",
                ),
                (
                    "component",
                    "json_each(NEW.record,'$.specimens') s,json_each(s.value,'$.components') c",
                    "json_extract(c.value,'$.id')",
                ),
                (
                    "carrier",
                    "json_each(NEW.record,'$.specimens') s,json_each(s.value,'$.components') c",
                    "json_extract(c.value,'$.carrier.id')",
                ),
            ] {
                insert.push_str(&format!("INSERT INTO entity_owners SELECT '{nested}',{id},NEW.collection_id,'item',NEW.id FROM {source} WHERE {id} IS NOT NULL;"));
            }
        }
        if kind == "capture" {
            insert.push_str("INSERT INTO entity_owners SELECT 'representation',json_extract(r.value,'$.id'),NEW.collection_id,'capture',NEW.id FROM json_each(NEW.record,'$.representations') r;");
        }
        let references: Vec<(&str, String)> = match kind {
            "item" => ["work", "edition"]
                .into_iter()
                .map(|target| {
                    (
                        target,
                        format!("json_extract(NEW.record,'$.identity_link.{target}_id')"),
                    )
                })
                .collect(),
            "edition" => vec![("work", "NEW.work_id".into())],
            "capture" => ["item", "specimen", "component", "carrier"]
                .into_iter()
                .map(|target| {
                    (
                        target,
                        format!("json_extract(NEW.record,'$.association.{target}_id')"),
                    )
                })
                .collect(),
            "asset" => [
                "item",
                "specimen",
                "component",
                "capture",
                "work",
                "edition",
            ]
            .into_iter()
            .map(|target| {
                (
                    target,
                    format!("json_extract(NEW.record,'$.asset.subject.{target}_id')"),
                )
            })
            .collect(),
            _ => vec![],
        };
        for (target, value) in references {
            let live = if kind == "asset" {
                " AND NEW.deleted=0"
            } else {
                ""
            };
            insert.push_str(&format!("INSERT INTO entity_references SELECT '{kind}',NEW.id,'{target}',{value},NEW.collection_id WHERE {value} IS NOT NULL{live};"));
        }
        let delete = format!(
            "DELETE FROM entity_references WHERE root_kind='{kind}' AND root_id=OLD.id; DELETE FROM entity_owners WHERE root_kind='{kind}' AND root_id=OLD.id;"
        );
        for (op, body) in [
            ("INSERT", insert.clone()),
            ("UPDATE", format!("{delete}{insert}")),
            ("DELETE", delete),
        ] {
            db.execute_batch(&format!("CREATE TRIGGER IF NOT EXISTS references_{table}_{op} AFTER {op} ON {table} BEGIN {body} END;")).map_err(storage)?;
        }
    }
    Ok(())
}

type Key = (&'static str, uuid::Uuid, &'static str, uuid::Uuid);
fn keys(graph: &CollectionGraph) -> Vec<Key> {
    let mut values = Vec::new();
    for item in &graph.items {
        values.push(("item", item.id.0, "item", item.id.0));
        for specimen in &item.specimens {
            values.push(("specimen", specimen.id.0, "item", item.id.0));
            values.extend(
                specimen
                    .loans
                    .iter()
                    .map(|loan| ("loan", loan.id.0, "item", item.id.0)),
            );
            for component in &specimen.components {
                values.push(("component", component.id.0, "item", item.id.0));
                if let Some(carrier) = &component.carrier {
                    values.push(("carrier", carrier.id.0, "item", item.id.0));
                }
            }
        }
    }
    values.extend(
        graph
            .works
            .iter()
            .map(|value| ("work", value.id.0, "work", value.id.0)),
    );
    values.extend(
        graph
            .editions
            .iter()
            .map(|value| ("edition", value.id.0, "edition", value.id.0)),
    );
    for capture in &graph.captures {
        values.push(("capture", capture.id.0, "capture", capture.id.0));
        values.extend(
            capture
                .representations
                .iter()
                .map(|value| ("representation", value.id.0, "capture", capture.id.0)),
        );
    }
    values.extend(
        graph
            .assets
            .iter()
            .map(|value| ("asset", value.id.0, "asset", value.id.0)),
    );
    values
}

fn references(graph: &CollectionGraph) -> BTreeSet<(&'static str, uuid::Uuid)> {
    let mut refs = BTreeSet::new();
    for item in &graph.items {
        match item.identity_link {
            ItemIdentityLink::Work { work_id } => {
                refs.insert(("work", work_id.0));
            }
            ItemIdentityLink::Edition { edition_id } => {
                refs.insert(("edition", edition_id.0));
            }
            ItemIdentityLink::None => {}
        }
    }
    for edition in &graph.editions {
        if let Some(work) = edition.work_id {
            refs.insert(("work", work.0));
        }
    }
    for capture in &graph.captures {
        for (kind, id) in [
            ("item", capture.association.item_id.map(|id| id.0)),
            ("specimen", capture.association.specimen_id.map(|id| id.0)),
            ("component", capture.association.component_id.map(|id| id.0)),
            ("carrier", capture.association.carrier_id.map(|id| id.0)),
        ] {
            if let Some(id) = id {
                refs.insert((kind, id));
            }
        }
    }
    for asset in &graph.assets {
        refs.insert(match asset.subject {
            AssetSubject::Item { item_id } => ("item", item_id.0),
            AssetSubject::Specimen { specimen_id } => ("specimen", specimen_id.0),
            AssetSubject::Component { component_id } => ("component", component_id.0),
            AssetSubject::Capture { capture_id } => ("capture", capture_id.0),
            AssetSubject::Work { work_id } => ("work", work_id.0),
            AssetSubject::Edition { edition_id } => ("edition", edition_id.0),
        });
    }
    refs
}

fn add_root(
    db: &Connection,
    graph: &mut CollectionGraph,
    kind: &str,
    id: &str,
    collection: CollectionId,
) -> Result<bool> {
    let exists = match kind {
        "item" => graph.items.iter().any(|value| value.id.to_string() == id),
        "work" => graph.works.iter().any(|value| value.id.to_string() == id),
        "edition" => graph
            .editions
            .iter()
            .any(|value| value.id.to_string() == id),
        "capture" => graph
            .captures
            .iter()
            .any(|value| value.id.to_string() == id),
        "asset" => graph.assets.iter().any(|value| value.id.to_string() == id),
        _ => return Err(Error::Storage("invalid indexed graph root".into())),
    };
    if exists {
        return Ok(false);
    }
    let table = match kind {
        "item" => "items",
        "work" => "works",
        "edition" => "editions",
        "capture" => "captures",
        _ => "assets",
    };
    let raw: Option<String> = db
        .query_row(
            &format!("SELECT record FROM {table} WHERE id=? AND collection_id=?"),
            params![id, collection.to_string()],
            |row| row.get(0),
        )
        .optional()
        .map_err(storage)?;
    let Some(raw) = raw else {
        return Ok(false);
    };
    match kind {
        "item" => {
            let mut item: Item = serde_json::from_str(&raw).map_err(storage)?;
            item.captures.clear();
            item.assets.clear();
            graph.items.push(item);
        }
        "work" => graph
            .works
            .push(serde_json::from_str(&raw).map_err(storage)?),
        "edition" => graph
            .editions
            .push(serde_json::from_str(&raw).map_err(storage)?),
        "capture" => graph
            .captures
            .push(serde_json::from_str(&raw).map_err(storage)?),
        _ => {
            let record: archive_v3::AssetRecordV3 = serde_json::from_str(&raw).map_err(storage)?;
            if !record.deleted {
                graph.assets.push(record.asset);
            }
        }
    }
    Ok(true)
}

pub(crate) fn validate_change(
    db: &Connection,
    mut graph: CollectionGraph,
    collection: CollectionId,
) -> Result<()> {
    let owner: Collection = projected_record(db, "collections", &collection.to_string())?;
    if matches!(owner.binding, CollectionBinding::LegacyUnassigned) {
        return Err(Error::Invalid(
            "legacy unassigned collections are content-read-only".into(),
        ));
    }
    let candidate_keys = keys(&graph);
    for (kind, id, root_kind, root_id) in &candidate_keys {
        let existing: Option<(String, String, String)> = db
            .query_row(
                "SELECT collection_id,root_kind,root_id FROM entity_owners WHERE kind=?1 AND id=?2",
                params![kind, id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(storage)?;
        if let Some((owner, existing_kind, existing_id)) = existing {
            if owner != collection.to_string() {
                return Err(Error::Invalid(
                    "entity identity is reused across collections".into(),
                ));
            }
            if existing_kind != *root_kind || existing_id != root_id.to_string() {
                return Err(Error::Invalid(
                    "entity identity is reused by another owner".into(),
                ));
            }
        }
    }
    // Check inbound references to old AND new physical identities, so removing
    // or moving a nested part cannot leave an unattached Capture/Asset dangling.
    let mut inbound = BTreeSet::new();
    for item in &graph.items {
        let mut statement = db.prepare("SELECT DISTINCT r.root_kind,r.root_id FROM entity_references r JOIN entity_owners o ON o.kind=r.target_kind AND o.id=r.target_id WHERE o.root_kind='item' AND o.root_id=? AND r.root_kind IN ('capture','asset')").map_err(storage)?;
        inbound.extend(
            statement
                .query_map([item.id.to_string()], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(storage)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(storage)?,
        );
    }
    for (kind, id) in inbound {
        add_root(db, &mut graph, &kind, &id, collection)?;
    }
    loop {
        let present = keys(&graph)
            .into_iter()
            .map(|(kind, id, _, _)| (kind, id))
            .collect::<HashSet<_>>();
        let mut added = false;
        for (kind, id) in references(&graph)
            .into_iter()
            .filter(|key| !present.contains(key))
        {
            let root: Option<(String,String)> = db.query_row("SELECT root_kind,root_id FROM entity_owners WHERE kind=? AND id=? AND collection_id=?",params![kind,id.to_string(),collection.to_string()], |row| Ok((row.get(0)?,row.get(1)?))).optional().map_err(storage)?;
            if let Some((kind, id)) = root {
                added |= add_root(db, &mut graph, &kind, &id, collection)?;
            }
        }
        if !added {
            break;
        }
    }
    graph.validate_for_collection(&owner)
}

pub(crate) fn capture_for_representation(
    db: &Connection,
    collection: CollectionId,
    id: RepresentationId,
) -> Result<Capture> {
    let raw: String = db.query_row("SELECT c.record FROM entity_owners o JOIN captures c ON c.id=o.root_id WHERE o.kind='representation' AND o.id=? AND o.collection_id=? AND o.root_kind='capture'",params![id.to_string(),collection.to_string()], |row| row.get(0)).optional().map_err(storage)?.ok_or(Error::NotFound)?;
    serde_json::from_str(&raw).map_err(storage)
}

pub(crate) fn assets_for_item(
    db: &Connection,
    item: &Item,
    work: Option<WorkId>,
) -> Result<Vec<archive_v3::AssetRecordV3>> {
    let mut subjects = BTreeSet::from([("item", item.id.0)]);
    if let Some(work) = work {
        subjects.insert(("work", work.0));
    }
    if let ItemIdentityLink::Edition { edition_id } = item.identity_link {
        subjects.insert(("edition", edition_id.0));
    }
    for specimen in &item.specimens {
        subjects.insert(("specimen", specimen.id.0));
        for component in &specimen.components {
            subjects.insert(("component", component.id.0));
        }
    }
    subjects.extend(
        item.captures
            .iter()
            .map(|capture| ("capture", capture.id.0)),
    );
    let mut assets = std::collections::BTreeMap::new();
    let mut statement = db.prepare("SELECT a.record FROM entity_references r JOIN assets a ON a.id=r.root_id WHERE r.target_kind=? AND r.target_id=? AND r.root_kind='asset' AND a.collection_id=?").map_err(storage)?;
    for (kind, id) in subjects {
        let rows = statement
            .query_map(
                params![kind, id.to_string(), item.collection_id.to_string()],
                |row| row.get::<_, String>(0),
            )
            .map_err(storage)?;
        for row in rows {
            let record: archive_v3::AssetRecordV3 =
                serde_json::from_str(&row.map_err(storage)?).map_err(storage)?;
            assets.insert(record.asset.id, record);
        }
    }
    // Tombstones are omitted from references but still remove legacy snapshots.
    for id in item.assets.iter().map(|asset| asset.id) {
        if let Some(raw) = db
            .query_row(
                "SELECT record FROM assets WHERE id=?",
                [id.to_string()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(storage)?
        {
            let record: archive_v3::AssetRecordV3 = serde_json::from_str(&raw).map_err(storage)?;
            assets.insert(record.asset.id, record);
        }
    }
    Ok(assets.into_values().collect())
}
