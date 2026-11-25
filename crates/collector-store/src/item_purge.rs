//! Permanent deletion of library data: a whole removed entry, or chosen
//! originals (discs or purchases) of an entry that stays.
//!
//! A whole-entry deletion only runs on an entry that is already removed, so the
//! removal has already stopped its work. Deleting a combined album also deletes
//! the placeholder entries that were merged into it.
//!
//! Combining albums writes a combine record that names the exact files of every
//! entry involved, and startup checks that each still exists unchanged. A
//! deletion therefore retires every combine record that names a deleted file:
//! the record keeps only the surviving members, and its undo data is replaced
//! by a note that an entry was deleted, so that combine can no longer be undone.
//!
//! Every deletion writes its plan to `state/item-purges/<item>.json` before
//! touching anything. Step one retires combine records, moves each folder into
//! `archive/.purge/<item>`, deletes the catalog and private rows in one
//! transaction and then the moved folders. Startup repeats step one for any
//! saved plan before it checks or rebuilds the catalog. When an entry stays,
//! step two saves its new revision (without the deleted discs' history or copy
//! records) once the store is open. Until then a rebuild treats the plan's
//! evidence as already gone, because the entry's last revision still lists it.
//!
//! Shared data stays: works, editions, snapshots, list entries (they show as a
//! removed entry), and analysis results that another dump's records point at.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

const JOURNALS: &str = "item-purges";
const TRASH: &str = ".purge";
const RETIRED: &str = "An entry in this combine was permanently deleted, so it can't be undone.";
const LEGACY: &str = "These files are stored in a way Collector does not recognise, so they were left alone. Nothing was deleted.";

/// Everything one deletion removes. Archive paths are relative to the archive root.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ItemPurgePlan {
    /// The entry the deletion was asked for.
    item_id: ItemId,
    collection_id: CollectionId,
    /// Entries deleted entirely; empty when only some originals go.
    items: Vec<String>,
    folders: Vec<String>,
    /// Combine records to retire.
    groups: Vec<String>,
    captures: Vec<String>,
    assets: Vec<String>,
    evidence: Vec<String>,
    recipes: Vec<String>,
    processing: Vec<String>,
    processing_groups: Vec<String>,
    imports: Vec<String>,
    jobs: Vec<String>,
    /// The surviving entry's new revision, when only some originals go.
    edit: Option<KeptEntryEdit>,
    #[serde(default)]
    files_done: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct KeptEntryEdit {
    captures: Vec<CaptureId>,
    specimens: Vec<SpecimenId>,
}

/// What a deletion covers.
struct Scope {
    /// Entries deleted entirely.
    items: Vec<Item>,
    captures: BTreeSet<String>,
    specimens: BTreeSet<String>,
    components: BTreeSet<String>,
    /// The entry that stays, when only some originals go.
    kept: Option<ItemId>,
}

pub(crate) fn install(db: &Connection) -> Result<()> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS private.purged_items(item_id TEXT PRIMARY KEY, collection_id TEXT NOT NULL, purged_at INTEGER NOT NULL)",
    )
    .map_err(storage)
}

/// A permanently deleted entry's id must never be published again, for
/// example by a replayed command that would otherwise create it afresh.
pub(crate) fn was_purged(db: &Connection, id: ItemId) -> Result<bool> {
    db.query_row(
        "SELECT EXISTS(SELECT 1 FROM private.purged_items WHERE item_id=?)",
        [id.to_string()],
        |row| row.get(0),
    )
    .map_err(storage)
}

fn journals(state: &Path) -> Result<Vec<(PathBuf, ItemPurgePlan)>> {
    let folder = state.join(JOURNALS);
    if !folder.try_exists().map_err(storage)? {
        return Ok(vec![]);
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(&folder).map_err(storage)? {
        let path = entry.map_err(storage)?.path();
        if path.extension().is_some_and(|value| value == "json") {
            paths.push(path);
        }
    }
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let plan =
                serde_json::from_slice(&fs::read(&path).map_err(storage)?).map_err(storage)?;
            Ok((path, plan))
        })
        .collect()
}

/// Evidence that a saved plan is deleting. A rebuild treats it as gone.
pub(crate) fn pending_evidence(state: &Path) -> Result<BTreeSet<String>> {
    Ok(journals(state)?
        .into_iter()
        .flat_map(|(_, plan)| plan.evidence)
        .collect())
}

/// Finish step one of deletions a crash interrupted. Runs at startup before
/// the catalog is checked, so a rebuild never sees half of a deletion.
pub(crate) fn finish_interrupted(db: &mut Connection, archive: &Path, state: &Path) -> Result<()> {
    for (_, plan) in journals(state)? {
        if !plan.files_done {
            tracing::info!(item = %plan.item_id, "finishing an interrupted permanent deletion");
            delete_files(db, archive, state, &plan)?;
        }
    }
    Ok(())
}

impl Store {
    /// Permanently delete a removed entry and the data it owns, including the
    /// placeholders of entries merged into it. Returns the bytes freed. The
    /// caller has checked the actor may write the collection.
    pub(crate) fn purge_removed_item(&self, id: ItemId, expected: Revision) -> Result<u64> {
        let _writer = lock(&self.writer)?;
        let _publication_gate = lock(&self.job_publication_gate)?;
        self.archive_guard.check()?;
        self.finish_saved_plan(id)?;
        let item = Catalog::item(self, id)?;
        if item.revision != expected {
            return Err(Error::Conflict);
        }
        if !item.deleted {
            return Err(Error::Invalid(
                "Remove the entry before deleting it permanently.".into(),
            ));
        }
        if item.lifecycle.merged_into.is_some() {
            return Err(Error::Invalid(
                "This entry was merged into another album. Delete its files from that album."
                    .into(),
            ));
        }
        let mut items = vec![item.clone()];
        let merged = strings(
            &*lock(&self.db)?,
            "SELECT id FROM items WHERE deleted=1 AND json_extract(record,'$.lifecycle.merged_into')=?1 ORDER BY id",
            &id.to_string(),
        )?;
        for source in merged {
            items.push(Catalog::item(self, source.parse().map_err(storage)?)?);
        }
        let mut specimens = BTreeSet::new();
        let mut components = BTreeSet::new();
        for entry in &items {
            for specimen in &entry.specimens {
                specimens.insert(specimen.id.to_string());
                components.extend(specimen.components.iter().map(|c| c.id.to_string()));
            }
        }
        let captures = {
            let db = lock(&self.db)?;
            let mut all = BTreeSet::new();
            for entry in &items {
                all.extend(strings(
                    &db,
                    "SELECT id FROM captures WHERE item_id=?1",
                    &entry.id.to_string(),
                )?);
            }
            all
        };
        let plan = self.purge_plan(
            id,
            item.collection_id,
            Scope {
                items,
                captures,
                specimens,
                components,
                kept: None,
            },
        )?;
        self.run_plan(plan)
    }

    /// Permanently delete chosen originals of an entry that stays, with their
    /// saved formats, covers and check results. `remove_copies` also deletes
    /// the physical copy records those originals came from, when no remaining
    /// original belongs to them. Returns the bytes freed.
    pub(crate) fn purge_item_files(
        &self,
        id: ItemId,
        expected: Revision,
        captures: &[CaptureId],
        remove_copies: bool,
    ) -> Result<u64> {
        let _writer = lock(&self.writer)?;
        let _publication_gate = lock(&self.job_publication_gate)?;
        self.archive_guard.check()?;
        self.finish_saved_plan(id)?;
        let item = Catalog::item(self, id)?;
        if item.revision != expected {
            return Err(Error::Conflict);
        }
        if item.lifecycle.merged_into.is_some() {
            return Err(Error::Invalid(
                "This entry was merged into another album. Delete its files from that album."
                    .into(),
            ));
        }
        let chosen = captures.iter().copied().collect::<BTreeSet<_>>();
        if chosen.is_empty() {
            return Err(Error::Invalid("Choose the files to delete.".into()));
        }
        let owned = strings(
            &*lock(&self.db)?,
            "SELECT id FROM captures WHERE item_id=?1",
            &id.to_string(),
        )?;
        let mut specimens = BTreeSet::new();
        let mut components = BTreeSet::new();
        for capture in &chosen {
            if !owned.contains(&capture.to_string()) {
                return Err(Error::NotFound);
            }
            if !remove_copies {
                continue;
            }
            let Some(specimen) = Catalog::capture(self, *capture)?.association.specimen_id else {
                continue;
            };
            // A copy stays while another of its originals stays.
            let shared = owned
                .iter()
                .filter(|other| !chosen.iter().any(|c| c.to_string() == **other))
                .map(|other| Catalog::capture(self, other.parse().map_err(storage)?))
                .collect::<Result<Vec<_>>>()?
                .iter()
                .any(|other| other.association.specimen_id == Some(specimen));
            if shared {
                continue;
            }
            let copy = item
                .specimens
                .iter()
                .find(|value| value.id == specimen)
                .ok_or(Error::Conflict)?;
            if copy.loans.iter().any(|loan| loan.returned_at.is_none()) {
                return Err(Error::Invalid(
                    "A copy these files came from is on loan. Return it before deleting its record."
                        .into(),
                ));
            }
            specimens.insert(specimen.to_string());
            components.extend(copy.components.iter().map(|c| c.id.to_string()));
        }
        let plan = self.purge_plan(
            id,
            item.collection_id,
            Scope {
                items: vec![],
                captures: chosen.iter().map(ToString::to_string).collect(),
                specimens,
                components,
                kept: Some(id),
            },
        )?;
        self.run_plan(plan)
    }

    fn run_plan(&self, plan: ItemPurgePlan) -> Result<u64> {
        write_journal(&self.state, &plan)?;
        let freed = {
            let mut db = lock(&self.db)?;
            delete_files(&mut db, &self.archive, &self.state, &plan)?
        };
        self.finish_kept_entry(&plan)?;
        Ok(freed)
    }

    /// Step two: save the surviving entry without the deleted originals'
    /// history and copy records, then forget the plan. The caller holds the
    /// archive writer.
    fn finish_kept_entry(&self, plan: &ItemPurgePlan) -> Result<()> {
        if let Some(edit) = &plan.edit {
            let mut item = Catalog::item(self, plan.item_id)?;
            item.specimens
                .retain(|specimen| !edit.specimens.contains(&specimen.id));
            item.lifecycle
                .historical_captures
                .retain(|capture| !edit.captures.contains(capture));
            let expected = item.revision;
            self.publish_under_writer(item, expected, vec![])?;
        }
        remove_journal(&self.state, plan.item_id)
    }

    /// Step two of deletions a crash interrupted, once the store is open. A
    /// plan that can't finish now waits: rebuilds already treat its evidence
    /// as gone, and the next start or deletion of that entry tries again.
    pub(crate) fn finish_interrupted_kept_entries(&self) -> Result<()> {
        let _writer = lock(&self.writer)?;
        for (_, plan) in journals(&self.state)? {
            if plan.files_done
                && let Err(error) = self.finish_kept_entry(&plan)
            {
                tracing::warn!(item = %plan.item_id, %error, "could not yet save an entry after deleting some of its files");
            }
        }
        Ok(())
    }

    /// Finish an earlier deletion of this entry that stopped part way, so a
    /// new plan never replaces one with work left. The caller holds the
    /// archive writer.
    fn finish_saved_plan(&self, id: ItemId) -> Result<()> {
        let path = journal_path(&self.state, id);
        if !path.try_exists().map_err(storage)? {
            return Ok(());
        }
        let mut plan: ItemPurgePlan =
            serde_json::from_slice(&fs::read(&path).map_err(storage)?).map_err(storage)?;
        if !plan.files_done {
            let mut db = lock(&self.db)?;
            delete_files(&mut db, &self.archive, &self.state, &plan)?;
            plan.files_done = true;
        }
        self.finish_kept_entry(&plan)
    }

    fn purge_plan(
        &self,
        item_id: ItemId,
        collection_id: CollectionId,
        scope: Scope,
    ) -> Result<ItemPurgePlan> {
        let db = lock(&self.db)?;
        let collection = collection_path(&db, &self.archive, collection_id)?;
        let relative_folder = |path: &Path| relative(&self.archive, path);
        let items = scope
            .items
            .iter()
            .map(|item| item.id.to_string())
            .collect::<BTreeSet<_>>();
        let mut folders = Vec::new();
        for item in &scope.items {
            let id = item.id.to_string();
            let path: String = db
                .query_row("SELECT path FROM items WHERE id=?", [&id], |row| row.get(0))
                .map_err(storage)?;
            folders.push(path);
            // An entry first saved under its title can have later revisions in
            // a folder named by its id alone; both belong to it.
            for entry in fs::read_dir(collection.join("items")).map_err(storage)? {
                let entry = entry.map_err(storage)?;
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name == id || name.ends_with(&format!("--{id}")) {
                    folders.push(relative_folder(&entry.path())?);
                }
            }
            let authority = collection.join("metadata_authorities").join(&id);
            if authority.try_exists().map_err(storage)? {
                folders.push(relative_folder(&authority)?);
            }
        }

        let item_folders = folders.clone();
        // An archive upgraded from an older layout keeps one marker per entry
        // revision, and startup checks every marker's entry still exists. They
        // go first, so a crash part way never leaves a marker without its entry.
        let mut markers = Vec::new();
        let migrations = self.archive.join(".archive-migrations");
        if migrations.try_exists().map_err(storage)? {
            for step in fs::read_dir(&migrations).map_err(storage)? {
                let records = step.map_err(storage)?.path().join("records");
                if !records.is_dir() {
                    continue;
                }
                for marker in fs::read_dir(&records).map_err(storage)? {
                    let marker = marker.map_err(storage)?.path();
                    let name = marker
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned());
                    if name.is_some_and(|name| {
                        name.ends_with(".done")
                            && items.iter().any(|id| name.starts_with(&format!("{id}-")))
                    }) {
                        markers.push(relative_folder(&marker)?);
                    }
                }
            }
        }
        markers.sort();
        folders.splice(0..0, markers);
        let captures = scope.captures.iter().cloned().collect::<Vec<_>>();
        let mut representations = BTreeSet::new();
        let mut capture_representations = BTreeMap::new();
        for capture in &captures {
            let capture = Catalog::capture(self, capture.parse().map_err(storage)?)?;
            representations.extend(capture.representations.iter().map(|r| r.id.to_string()));
            capture_representations.insert(
                capture.id.to_string(),
                capture
                    .representations
                    .iter()
                    .map(|r| r.id.to_string())
                    .collect::<Vec<_>>(),
            );
            folders.push(relative_folder(
                &collection.join("captures").join(capture.id.to_string()),
            )?);
        }
        // Older imports kept their original files in `files/<representation>`
        // inside the folder of the entry they were first saved with. Those go
        // with that entry's folder, or on their own when that entry stays.
        let legacy = {
            let mut statement = db
                .prepare(
                    "SELECT capture_id,item_id,relative_path FROM legacy_payload_locators
                     WHERE item_id IN (SELECT value FROM json_each(?1)) OR capture_id IN (SELECT value FROM json_each(?2))",
                )
                .map_err(storage)?;
            statement
                .query_map(params![json(&items)?, json(&captures)?], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(storage)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(storage)?
        };
        for (capture, root, relative_path) in legacy {
            let Some(representations) = capture_representations.get(&capture) else {
                // Another entry's files are stored in a folder this removes.
                let owner = Catalog::capture(self, capture.parse().map_err(storage)?)?
                    .association
                    .item_id;
                let title: Option<String> = match owner {
                    Some(owner) => db
                        .query_row(
                            "SELECT json_extract(record,'$.title') FROM items WHERE id=?",
                            [owner.to_string()],
                            |row| row.get(0),
                        )
                        .optional()
                        .map_err(storage)?,
                    None => None,
                };
                return Err(Error::Invalid(format!(
                    "Files of “{}” were first saved with this entry and are still stored in its folder. Delete those files from “{0}” first, or keep this entry removed.",
                    title.unwrap_or_else(|| "another entry".into())
                )));
            };
            if relative_path != "files" {
                return Err(Error::Invalid(LEGACY.into()));
            }
            if items.contains(&root) {
                continue;
            }
            let path: String = db
                .query_row("SELECT path FROM items WHERE id=?", [&root], |row| {
                    row.get(0)
                })
                .map_err(storage)?;
            for representation in representations {
                let folder = Path::new(&path).join("files").join(representation);
                if self.archive.join(&folder).try_exists().map_err(storage)? {
                    folders.push(folder.to_string_lossy().into_owned());
                }
            }
        }

        let mut assets = Vec::new();
        let mut statement = db
            .prepare("SELECT record FROM assets WHERE collection_id=? ORDER BY id")
            .map_err(storage)?;
        for record in statement
            .query_map([collection_id.to_string()], |row| row.get::<_, String>(0))
            .map_err(storage)?
        {
            let record: archive_v3::AssetRecordV3 =
                serde_json::from_str(&record.map_err(storage)?).map_err(storage)?;
            let owned = match &record.asset.subject {
                AssetSubject::Item { item_id } => items.contains(&item_id.to_string()),
                AssetSubject::Specimen { specimen_id } => {
                    scope.specimens.contains(&specimen_id.to_string())
                }
                AssetSubject::Component { component_id } => {
                    scope.components.contains(&component_id.to_string())
                }
                AssetSubject::Capture { capture_id } => {
                    scope.captures.contains(&capture_id.to_string())
                }
                AssetSubject::Work { .. } | AssetSubject::Edition { .. } => false,
            };
            if !owned {
                if record
                    .legacy_payloads
                    .iter()
                    .any(|payload| under(&payload.relative_path, &item_folders))
                {
                    return Err(Error::Invalid(
                        "A cover or image of another entry is still stored in this entry's folder, so it can only stay removed.".into(),
                    ));
                }
                continue;
            }
            assets.push(record.asset.id.to_string());
            folders.push(relative_folder(
                &collection.join("assets").join(record.asset.id.to_string()),
            )?);
            // Older covers keep their bytes in a folder named by the asset,
            // inside the entry's folder; it goes too.
            for payload in &record.legacy_payloads {
                let parent = Path::new(&payload.relative_path)
                    .parent()
                    .ok_or_else(|| Error::Invalid(LEGACY.into()))?;
                let id = record.asset.id.to_string();
                let named = parent.file_name().is_some_and(|name| name == id.as_str())
                    || (parent.file_name().is_some_and(|name| name == "payload")
                        && parent
                            .parent()
                            .and_then(Path::file_name)
                            .is_some_and(|name| name == id.as_str()));
                if !named {
                    return Err(Error::Invalid(LEGACY.into()));
                }
                folders.push(parent.to_string_lossy().into_owned());
            }
        }
        drop(statement);

        // Evidence about a deleted entry, or about only deleted files.
        let mut evidence = Vec::new();
        let mut statement = db
            .prepare("SELECT record FROM evidence WHERE collection_id=? ORDER BY id")
            .map_err(storage)?;
        for record in statement
            .query_map([collection_id.to_string()], |row| row.get::<_, String>(0))
            .map_err(storage)?
        {
            let record: archive_v3::EvidenceRecordV3 =
                serde_json::from_str(&record.map_err(storage)?).map_err(storage)?;
            let about_deleted_files = !record.representation_ids.is_empty()
                && record
                    .representation_ids
                    .iter()
                    .all(|r| representations.contains(&r.to_string()));
            let owned = match record.item_id {
                Some(owner) if items.contains(&owner.to_string()) => true,
                Some(owner) => Some(owner) == scope.kept && about_deleted_files,
                None => about_deleted_files,
            };
            if owned {
                evidence.push(record.id.to_string());
                folders.push(relative_folder(
                    &collection.join("evidence").join(record.id.to_string()),
                )?);
            }
        }
        drop(statement);

        let reps = json(&representations)?;
        let recipes = strings(
            &db,
            "SELECT id FROM conversion_recipes WHERE id IN (SELECT value FROM json_each(?1)) ORDER BY id",
            &reps,
        )?;
        for recipe in &recipes {
            folders.push(relative_folder(
                &collection.join("conversion_recipes").join(recipe),
            )?);
        }

        let imports = {
            let mut statement = db
                .prepare(
                    "SELECT id FROM private.imports WHERE item_id IN (SELECT value FROM json_each(?1))
                       OR capture_id IN (SELECT value FROM json_each(?2))
                     UNION SELECT import_id FROM private.guided_ingest_intents
                       WHERE target_item_id IN (SELECT value FROM json_each(?1)) ORDER BY 1",
                )
                .map_err(storage)?;
            statement
                .query_map(params![json(&items)?, json(&captures)?], |row| {
                    row.get::<_, String>(0)
                })
                .map_err(storage)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(storage)?
        };
        let unfinished: bool = db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM private.imports WHERE id IN (SELECT value FROM json_each(?1)) AND phase NOT IN ('published','cancelled'))",
                [json(&imports)?],
                |row| row.get(0),
            )
            .map_err(storage)?;

        // Jobs about what is deleted. A scan shared with another import stays.
        let jobs = {
            let mut statement = db
                .prepare(
                    "SELECT id,state FROM private.jobs j WHERE (j.item_id IN (SELECT value FROM json_each(?1))
                       OR json_extract(j.request,'$.import_id') IN (SELECT value FROM json_each(?2))
                       OR json_extract(j.request,'$.capture_id') IN (SELECT value FROM json_each(?3))
                       OR json_extract(j.request,'$.representation_id') IN (SELECT value FROM json_each(?4)))
                     AND NOT EXISTS(SELECT 1 FROM private.scan_requests r WHERE r.job_id=j.id
                       AND r.owner NOT IN (SELECT value FROM json_each(?1))
                       AND r.owner NOT IN (SELECT value FROM json_each(?2)))
                     ORDER BY id",
                )
                .map_err(storage)?;
            statement
                .query_map(
                    params![json(&items)?, json(&imports)?, json(&captures)?, reps],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .map_err(storage)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(storage)?
        };
        // Work on the surviving entry may be reading these files.
        let kept_busy: bool = match scope.kept {
            Some(kept) => db
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM private.jobs WHERE item_id=? AND state IN ('queued','running','cancel_requested'))",
                    [kept.to_string()],
                    |row| row.get(0),
                )
                .map_err(storage)?,
            None => false,
        };
        if unfinished {
            return Err(Error::Invalid(
                "An import for these files is still open. Finish or cancel it in Imports first."
                    .into(),
            ));
        }
        if kept_busy
            || jobs.iter().any(|(_, state)| {
                matches!(state.as_str(), "queued" | "running" | "cancel_requested")
            })
        {
            return Err(Error::Conflict);
        }
        let jobs = jobs.into_iter().map(|(id, _)| id).collect::<Vec<_>>();

        // Analysis results of the deleted files. One result can be shared by
        // equivalent dumps; a shared group stays whole, so no other dump loses
        // the result it points at.
        let candidates = {
            let mut statement = db
                .prepare(
                    "SELECT id FROM processing_observations WHERE collection_id=?1 AND
                     ((source_scope='representation' AND source_id IN (SELECT value FROM json_each(?2)))
                      OR (source_scope='import' AND source_id IN (SELECT value FROM json_each(?3))))",
                )
                .map_err(storage)?;
            statement
                .query_map(
                    params![collection_id.to_string(), reps, json(&imports)?],
                    |row| row.get::<_, String>(0),
                )
                .map_err(storage)?
                .collect::<std::result::Result<BTreeSet<_>, _>>()
                .map_err(storage)?
        };
        let mut processing = Vec::new();
        let mut processing_groups = BTreeSet::new();
        for candidate in &candidates {
            let group: Option<String> = db
                .query_row(
                    "SELECT result_id FROM processing_members WHERE id=?",
                    [candidate],
                    |row| row.get(0),
                )
                .optional()
                .map_err(storage)?;
            if let Some(group) = &group {
                let members = strings(
                    &db,
                    "SELECT id FROM processing_members WHERE result_id=?1",
                    group,
                )?;
                if !members.iter().all(|member| candidates.contains(member)) {
                    continue;
                }
                processing_groups.insert(group.clone());
            }
            processing.push(candidate.clone());
            for kind in ["processing_observations", "processing_occurrences"] {
                let folder = collection.join(kind).join(candidate);
                if folder.try_exists().map_err(storage)? {
                    folders.push(relative_folder(&folder)?);
                }
            }
        }

        let mut seen = BTreeSet::new();
        folders.retain(|folder| seen.insert(folder.clone()));
        for folder in &folders {
            publication::validate_relative_path(folder)?;
        }
        let groups = combine_records_naming(&self.archive, &collection, &folders)?;
        let edit = match scope.kept {
            Some(_) => Some(KeptEntryEdit {
                captures: scope
                    .captures
                    .iter()
                    .map(|id| id.parse().map_err(storage))
                    .collect::<Result<Vec<_>>>()?,
                specimens: scope
                    .specimens
                    .iter()
                    .map(|id| id.parse().map_err(storage))
                    .collect::<Result<Vec<_>>>()?,
            }),
            None => None,
        };
        Ok(ItemPurgePlan {
            item_id,
            collection_id,
            items: items.into_iter().collect(),
            folders,
            groups,
            captures,
            assets,
            evidence,
            recipes,
            processing,
            processing_groups: processing_groups.into_iter().collect(),
            imports,
            jobs,
            edit,
            files_done: false,
        })
    }
}

fn under(path: &str, folders: &[String]) -> bool {
    folders
        .iter()
        .any(|folder| path.starts_with(&format!("{folder}/")))
}

/// Combine (and combine-undo) records that name a file in these folders.
fn combine_records_naming(
    archive: &Path,
    collection: &Path,
    folders: &[String],
) -> Result<Vec<String>> {
    let records = collection.join("publication-groups");
    if !records.try_exists().map_err(storage)? {
        return Ok(vec![]);
    }
    let mut found = Vec::new();
    for entry in fs::read_dir(&records).map_err(storage)? {
        let path = entry.map_err(storage)?.path();
        if path.extension().is_none_or(|value| value != "json") {
            continue;
        }
        let manifest: group_publication::GroupManifest =
            serde_json::from_slice(&fs::read(&path).map_err(storage)?).map_err(storage)?;
        if manifest
            .members
            .iter()
            .any(|member| under(&member.path, folders))
        {
            found.push(relative(archive, &path)?);
        }
    }
    found.sort();
    Ok(found)
}

/// Keep only the surviving members of a combine record and replace its undo
/// data. A record with no survivors is deleted. Safe to repeat.
fn retire_combine_record(archive: &Path, record: &str, folders: &[String]) -> Result<()> {
    publication::validate_relative_path(record)?;
    let path = archive.join(record);
    if !path.try_exists().map_err(storage)? {
        return Ok(());
    }
    let mut manifest: group_publication::GroupManifest =
        serde_json::from_slice(&fs::read(&path).map_err(storage)?).map_err(storage)?;
    manifest
        .members
        .retain(|member| !under(&member.path, folders));
    let parent = path.parent().ok_or(Error::Conflict)?;
    if manifest.members.is_empty() {
        fs::remove_file(&path).map_err(storage)?;
        return sync_directory(parent);
    }
    manifest.decision = RawJson::from_value(serde_json::json!({ "retired": RETIRED }));
    let temporary = path.with_extension("json.partial");
    {
        let mut file = File::create(&temporary).map_err(storage)?;
        file.write_all(&serde_json::to_vec_pretty(&manifest).map_err(storage)?)
            .map_err(storage)?;
        file.sync_all().map_err(storage)?;
    }
    fs::rename(&temporary, &path).map_err(storage)?;
    sync_directory(parent)
}

/// The note a retired combine record carries instead of its undo data.
pub(crate) fn retired_combine(decision: &RawJson) -> Option<String> {
    decision
        .parse::<serde_json::Value>()
        .ok()?
        .get("retired")?
        .as_str()
        .map(str::to_owned)
}

fn json<T: Serialize>(values: &T) -> Result<String> {
    serde_json::to_string(values).map_err(storage)
}

fn strings(db: &Connection, sql: &str, value: &str) -> Result<Vec<String>> {
    let mut statement = db.prepare(sql).map_err(storage)?;
    statement
        .query_map([value], |row| row.get::<_, String>(0))
        .map_err(storage)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(storage)
}

fn journal_path(state: &Path, item: ItemId) -> PathBuf {
    state.join(JOURNALS).join(format!("{item}.json"))
}

fn write_journal(state: &Path, plan: &ItemPurgePlan) -> Result<()> {
    let folder = state.join(JOURNALS);
    fs::create_dir_all(&folder).map_err(storage)?;
    let path = journal_path(state, plan.item_id);
    let temporary = path.with_extension("json.partial");
    {
        let mut file = File::create(&temporary).map_err(storage)?;
        file.write_all(&serde_json::to_vec_pretty(plan).map_err(storage)?)
            .map_err(storage)?;
        file.sync_all().map_err(storage)?;
    }
    fs::rename(&temporary, &path).map_err(storage)?;
    sync_directory(&folder)
}

fn remove_journal(state: &Path, item: ItemId) -> Result<()> {
    let journal = journal_path(state, item);
    if journal.try_exists().map_err(storage)? {
        fs::remove_file(&journal).map_err(storage)?;
        sync_directory(&state.join(JOURNALS))?;
    }
    Ok(())
}

/// Step one: retire combine records, move, unlist, then delete. Every part
/// can run again after a crash.
fn delete_files(
    db: &mut Connection,
    archive: &Path,
    state: &Path,
    plan: &ItemPurgePlan,
) -> Result<u64> {
    for record in &plan.groups {
        retire_combine_record(archive, record, &plan.folders)?;
    }
    let trash = archive.join(TRASH).join(plan.item_id.to_string());
    fs::create_dir_all(&trash).map_err(storage)?;
    for (index, folder) in plan.folders.iter().enumerate() {
        publication::validate_relative_path(folder)?;
        let source = archive.join(folder);
        let target = trash.join(index.to_string());
        if source.try_exists().map_err(storage)? && !target.try_exists().map_err(storage)? {
            fs::rename(&source, &target).map_err(storage)?;
            if let Some(parent) = source.parent() {
                sync_directory(parent)?;
            }
        }
    }
    sync_directory(&trash)?;
    unlist(db, archive, plan)?;
    let freed = derivatives::directory_bytes(&trash)?;
    fs::remove_dir_all(&trash).map_err(storage)?;
    // Leave no empty folder behind for backups and exports to copy.
    let trash_root = archive.join(TRASH);
    if fs::read_dir(&trash_root).map_err(storage)?.next().is_none() {
        fs::remove_dir(&trash_root).map_err(storage)?;
        sync_directory(archive)?;
    } else {
        sync_directory(&trash_root)?;
    }
    for import in &plan.imports {
        let scratch = state.join("imports").join(import);
        if scratch.try_exists().map_err(storage)? {
            fs::remove_dir_all(&scratch).map_err(storage)?;
        }
    }
    if plan.edit.is_some() {
        // Step two still has to save the surviving entry.
        write_journal(
            state,
            &ItemPurgePlan {
                files_done: true,
                ..plan.clone()
            },
        )?;
    } else {
        remove_journal(state, plan.item_id)?;
    }
    Ok(freed)
}

/// Delete the catalog and private rows in one transaction, then mark the
/// catalog checkpoint current so the next start can reuse it.
fn unlist(db: &mut Connection, archive: &Path, plan: &ItemPurgePlan) -> Result<()> {
    let values = [
        (":items", json(&plan.items)?),
        (":captures", json(&plan.captures)?),
        (":assets", json(&plan.assets)?),
        (":evidence", json(&plan.evidence)?),
        (":recipes", json(&plan.recipes)?),
        (":processing", json(&plan.processing)?),
        (":groups", json(&plan.processing_groups)?),
        (":imports", json(&plan.imports)?),
        (":jobs", json(&plan.jobs)?),
        (":roots", {
            let mut roots = plan.items.clone();
            roots.extend(plan.captures.iter().cloned());
            roots.extend(plan.assets.iter().cloned());
            json(&roots)?
        }),
    ];
    let tx = db.transaction().map_err(storage)?;
    let run = |sql: &str| -> Result<()> {
        // Bind only the names this statement uses; SQLite rejects the rest.
        let bound = values
            .iter()
            .filter(|(name, _)| sql.contains(name))
            .map(|(name, value)| (*name, value as &dyn rusqlite::ToSql))
            .collect::<Vec<_>>();
        tx.execute(sql, bound.as_slice()).map_err(storage)?;
        Ok(())
    };
    const ITEMS: &str = "(SELECT value FROM json_each(:items))";
    // Catalog rows. Deleting an item fires triggers that write fresh library
    // tokens for it, so its library rows go after it.
    for sql in [
        format!("DELETE FROM items WHERE id IN {ITEMS}"),
        format!("DELETE FROM search WHERE id IN {ITEMS}"),
        format!("DELETE FROM browse_items WHERE id IN {ITEMS}"),
        format!("DELETE FROM components WHERE item_id IN {ITEMS}"),
        "DELETE FROM captures WHERE id IN (SELECT value FROM json_each(:captures))".into(),
        "DELETE FROM legacy_payload_locators WHERE capture_id IN (SELECT value FROM json_each(:captures))".into(),
        "DELETE FROM assets WHERE id IN (SELECT value FROM json_each(:assets))".into(),
        "DELETE FROM evidence WHERE id IN (SELECT value FROM json_each(:evidence))".into(),
        "DELETE FROM conversion_recipes WHERE id IN (SELECT value FROM json_each(:recipes))".into(),
        "DELETE FROM processing_observations WHERE id IN (SELECT value FROM json_each(:processing))".into(),
        "DELETE FROM processing_members WHERE id IN (SELECT value FROM json_each(:processing))".into(),
        "DELETE FROM processing_groups WHERE id IN (SELECT value FROM json_each(:groups))".into(),
        format!("DELETE FROM metadata_authorities WHERE id IN {ITEMS}"),
        format!("DELETE FROM metadata_refreshes WHERE item_id IN {ITEMS}"),
        format!("DELETE FROM metadata_candidates WHERE item_id IN {ITEMS}"),
        format!("DELETE FROM metadata_health WHERE item_id IN {ITEMS}"),
        format!("DELETE FROM metadata_language_display WHERE item_id IN {ITEMS}"),
        format!("DELETE FROM album_group_members WHERE item_id IN {ITEMS}"),
        format!("DELETE FROM music_search WHERE item_id IN {ITEMS}"),
        format!("DELETE FROM library_entries WHERE item_id IN {ITEMS}"),
        format!("DELETE FROM library_entry_attempts WHERE item_id IN {ITEMS}"),
        format!("DELETE FROM library_entry_basics WHERE item_id IN {ITEMS}"),
        format!("DELETE FROM library_track_entries WHERE item_id IN {ITEMS}"),
        format!("DELETE FROM library_entry_versions WHERE item_id IN {ITEMS}"),
        "DELETE FROM entity_owners WHERE root_id IN (SELECT value FROM json_each(:roots))".into(),
        "DELETE FROM entity_references WHERE root_id IN (SELECT value FROM json_each(:roots))".into(),
        // Private rows: the imports, their work and history.
        format!("DELETE FROM private.guided_ingest_intents WHERE target_item_id IN {ITEMS} OR import_id IN (SELECT value FROM json_each(:imports))"),
        "DELETE FROM private.imports WHERE id IN (SELECT value FROM json_each(:imports))".into(),
        "DELETE FROM private.item_import_generations WHERE import_id IN (SELECT value FROM json_each(:imports))".into(),
        "DELETE FROM private.import_cleanup WHERE import_id IN (SELECT value FROM json_each(:imports))".into(),
        "DELETE FROM private.inbox_cleanup WHERE import_id IN (SELECT value FROM json_each(:imports))".into(),
        "DELETE FROM private.intake_members WHERE import_id IN (SELECT value FROM json_each(:imports))".into(),
        "DELETE FROM private.provisional_processing_observations WHERE import_id IN (SELECT value FROM json_each(:imports)) OR job_id IN (SELECT value FROM json_each(:jobs))".into(),
        "DELETE FROM private.scan_requests WHERE job_id IN (SELECT value FROM json_each(:jobs)) OR owner IN (SELECT value FROM json_each(:imports))".into(),
        "DELETE FROM private.scan_admissions WHERE job_id IN (SELECT value FROM json_each(:jobs))".into(),
        "DELETE FROM private.preparation_members WHERE batch_id IN (SELECT value FROM json_each(:jobs)) OR job_id IN (SELECT value FROM json_each(:jobs))".into(),
        "DELETE FROM private.derivative_direct_requests WHERE job_id IN (SELECT value FROM json_each(:jobs))".into(),
        "DELETE FROM private.processing_jobs WHERE job_id IN (SELECT value FROM json_each(:jobs))".into(),
        "DELETE FROM private.import_publication_stages WHERE job_id IN (SELECT value FROM json_each(:jobs))".into(),
        "DELETE FROM private.item_job_generations WHERE job_id IN (SELECT value FROM json_each(:jobs))".into(),
        "DELETE FROM private.digital_checkpoints WHERE job_id IN (SELECT value FROM json_each(:jobs))".into(),
        "DELETE FROM private.acquisition_links WHERE owner_job IN (SELECT value FROM json_each(:jobs))".into(),
        "DELETE FROM private.acquisition_waits WHERE owner_job IN (SELECT value FROM json_each(:jobs))".into(),
        format!("DELETE FROM private.snapshot_bindings WHERE item_id IN {ITEMS} OR job_id IN (SELECT value FROM json_each(:jobs))"),
        "DELETE FROM private.work_history WHERE target IN (SELECT 'import:'||value FROM json_each(:imports) UNION SELECT 'job:'||value FROM json_each(:jobs))".into(),
        "DELETE FROM private.jobs WHERE id IN (SELECT value FROM json_each(:jobs))".into(),
        format!("DELETE FROM private.activity WHERE item_id IN {ITEMS}"),
        format!("DELETE FROM private.bulk_library_members WHERE item_id IN {ITEMS}"),
        format!("DELETE FROM private.game_scan_members WHERE item_id IN {ITEMS}"),
        format!("DELETE FROM private.library_dirty WHERE item_id IN {ITEMS}"),
        "DELETE FROM private.library_dirty_captures WHERE capture_id IN (SELECT value FROM json_each(:captures))".into(),
    ] {
        run(&sql)?;
    }
    if plan.edit.is_some() {
        // The surviving entry's track list and summaries no longer include
        // the deleted files.
        music_search::refresh(&tx, plan.item_id)?;
        tx.execute(
            "INSERT OR IGNORE INTO private.library_dirty(item_id) VALUES(?)",
            [plan.item_id.to_string()],
        )
        .map_err(storage)?;
    }
    for folder in &plan.folders {
        tx.execute(
            "DELETE FROM archive_index_files WHERE path=?2 OR path LIKE ?1||'/%' ESCAPE '\\'",
            [
                folder
                    .replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_"),
                folder.clone(),
            ],
        )
        .map_err(storage)?;
    }
    for record in &plan.groups {
        let path = archive.join(record);
        if path.try_exists().map_err(storage)? {
            tx.execute(
                "UPDATE archive_index_files SET stamp=? WHERE path=?",
                params![index_checkpoint::stamp(&path)?, record],
            )
            .map_err(storage)?;
        } else {
            tx.execute("DELETE FROM archive_index_files WHERE path=?", [record])
                .map_err(storage)?;
        }
    }
    for item in &plan.items {
        tx.execute(
            "INSERT OR IGNORE INTO private.purged_items VALUES(?,?,?)",
            params![item, plan.collection_id.to_string(), Utc::now().timestamp()],
        )
        .map_err(storage)?;
    }
    index_checkpoint::seal(
        &tx,
        &archive.canonicalize().map_err(storage)?.to_string_lossy(),
    )?;
    tx.commit().map_err(storage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use collector_app::{CollectionService, CommandContext, CreateCollectionCommand};

    fn context() -> CommandContext {
        CommandContext::new(TransactionId::new().to_string()).unwrap()
    }

    #[test]
    fn startup_finishes_a_deletion_a_crash_interrupted() {
        let root = tempfile::tempdir().unwrap();
        let (archive, state) = (root.path().join("archive"), root.path().join("state"));
        let (id, kept) = {
            let store = Arc::new(Store::open(&archive, &state).unwrap());
            store
                .create_user("owner", "a long test password", true)
                .unwrap();
            let session = store
                .login("owner", "a long test password", "setup")
                .unwrap()
                .1;
            let service = CollectionService::new(
                store.clone(),
                Arc::new(collector_modules::Registry::builtins()),
            );
            let collection = service
                .create_collection(
                    &session,
                    CreateCollectionCommand {
                        name: "Music".into(),
                        module: ModuleId("org.junk.music".into()),
                    },
                    &context(),
                )
                .unwrap();
            let entry = |title: &str| {
                let item = Item {
                    purchase_orders: vec![],
                    lifecycle: Default::default(),
                    id: ItemId::new(),
                    collection_id: collection.id,
                    revision: Revision(0),
                    title: title.into(),
                    subtitle: String::new(),
                    identity_link: ItemIdentityLink::None,
                    domain: ModuleData {
                        module: ModuleId("org.junk.music".into()),
                        schema_version: 1,
                        payload: RawJson::from_value(serde_json::json!({})),
                    },
                    identity: IdentityState::Unidentified,
                    translations: vec![],
                    identifiers: vec![],
                    specimens: vec![],
                    wishlist: true,
                    captures: vec![],
                    assets: vec![],
                    metadata: MetadataState::default(),
                    deleted: false,
                    updated_at: Utc::now(),
                };
                Catalog::save(store.as_ref(), item, Revision(0)).unwrap()
            };
            let doomed = entry("Doomed");
            let kept = entry("Kept");
            let removed = store
                .change_item_removed(doomed.id, doomed.revision, true)
                .unwrap();
            // Crash after the plan was saved and the first folder moved.
            let plan = store
                .purge_plan(
                    removed.id,
                    removed.collection_id,
                    Scope {
                        items: vec![removed.clone()],
                        captures: BTreeSet::new(),
                        specimens: BTreeSet::new(),
                        components: BTreeSet::new(),
                        kept: None,
                    },
                )
                .unwrap();
            write_journal(&state, &plan).unwrap();
            let trash = archive.join(TRASH).join(doomed.id.to_string());
            fs::create_dir_all(&trash).unwrap();
            let (index, folder) = plan
                .folders
                .iter()
                .enumerate()
                .find(|(_, folder)| archive.join(folder).exists())
                .unwrap();
            fs::rename(archive.join(folder), trash.join(index.to_string())).unwrap();
            (doomed.id, kept.id)
        };
        let store = Store::open(&archive, &state).unwrap();
        assert!(matches!(Catalog::item(&store, id), Err(Error::NotFound)));
        assert!(Catalog::item(&store, kept).is_ok());
        assert!(!archive.join(TRASH).join(id.to_string()).exists());
        assert!(!journal_path(&state, id).exists());
        store.reindex().unwrap();
        assert!(matches!(Catalog::item(&store, id), Err(Error::NotFound)));
    }
}
