//! Portable list revisions use the ordinary journal/rename commit and checkpoint.
use super::*;
use collector_app::{CollectionListPort, CollectionListSummary, ListSourceRevision};

pub(super) fn install(db: &Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS collection_lists(id TEXT PRIMARY KEY, collection_id TEXT NOT NULL, revision INTEGER NOT NULL, name TEXT NOT NULL, kind TEXT NOT NULL, entry_count INTEGER NOT NULL, deleted INTEGER NOT NULL, record TEXT NOT NULL); CREATE INDEX IF NOT EXISTS collection_lists_collection ON collection_lists(collection_id,deleted,id);") .map_err(storage)
}
pub(super) fn project(db: &Connection, list: &CollectionList) -> Result<()> {
    list.validate()?;
    let changed=db.execute("INSERT INTO collection_lists VALUES(?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,name=excluded.name,entry_count=excluded.entry_count,deleted=excluded.deleted,record=excluded.record WHERE collection_lists.collection_id=excluded.collection_id AND collection_lists.kind=excluded.kind",
        params![list.id.to_string(),list.collection_id.to_string(),list.revision.0,list.name,serde_json::to_string(&list.kind).map_err(storage)?,list.entries.len(),list.deleted,serde_json::to_string(list).map_err(storage)?]).map_err(storage)?;
    if changed != 1 {
        return Err(Error::Invalid(
            "list identity reused across collections or kinds".into(),
        ));
    }
    Ok(())
}
impl TypedArchiveEntity for CollectionList {
    const KIND: &'static str = "list";
    fn archive_id(&self) -> String {
        self.id.to_string()
    }
    fn archive_collection(&self) -> CollectionId {
        self.collection_id
    }
    fn archive_revision(&self) -> Revision {
        self.revision
    }
}
impl CollectionListPort for Store {
    fn collection_lists(
        &self,
        collection: CollectionId,
        search: &str,
        cursor: Option<CollectionListId>,
        limit: u16,
    ) -> Result<Page<CollectionListSummary>> {
        let db = lock(&self.db)?;
        let mut statement = db.prepare("SELECT id,revision,name,kind,entry_count FROM collection_lists WHERE collection_id=? AND deleted=0 AND id>? AND instr(lower(name),lower(?))>0 ORDER BY id LIMIT ?").map_err(storage)?;
        let mut rows = statement
            .query_map(
                params![
                    collection.to_string(),
                    cursor.map(|id| id.to_string()).unwrap_or_default(),
                    search,
                    limit + 1
                ],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, u64>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, u32>(4)?,
                    ))
                },
            )
            .map_err(storage)?
            .map(|row| {
                let (id, revision, name, kind, entry_count) = row.map_err(storage)?;
                Ok(CollectionListSummary {
                    id: id.parse().map_err(storage)?,
                    collection_id: collection,
                    revision: Revision(revision),
                    name,
                    kind: serde_json::from_str(&kind).map_err(storage)?,
                    entry_count,
                    can_edit: false,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let next_cursor =
            (rows.len() > limit as usize).then(|| rows[limit as usize - 1].id.to_string());
        rows.truncate(limit as usize);
        Ok(Page {
            items: rows,
            next_cursor,
        })
    }
    fn collection_list(&self, id: CollectionListId) -> Result<CollectionList> {
        projected_record(&*lock(&self.db)?, "collection_lists", &id.to_string())
    }
    fn publish_collection_list(
        &self,
        user: UserId,
        list: CollectionList,
        expected: Revision,
        sources: Vec<ListSourceRevision>,
    ) -> Result<CollectionListSummary> {
        list.validate()?;
        let _writer = lock(&self.writer)?;
        // Retain the database mutex through the commit: grant changes cannot
        // slip between the authorization check and the archive rename.
        let mut db = lock(&self.db)?;
        let path = {
            let role: Option<u8> = db
                .query_row(
                    "SELECT g.role FROM private.grants g JOIN private.users u ON u.id=g.user_id WHERE g.user_id=? AND g.collection_id=? AND u.enabled=1",
                    [user.to_string(), list.collection_id.to_string()],
                    |r| r.get(0),
                )
                .optional()
                .map_err(storage)?;
            if !role
                .map(|r| role_from_database(r.into()))
                .transpose()?
                .is_some_and(|r| r.allows(Action::Curate))
            {
                return Err(Error::Forbidden);
            }
            if projected_revision(
                &db,
                "collection_lists",
                &list.id.to_string(),
                list.collection_id,
            )? != expected.0
                || expected.0.checked_add(1) != Some(list.revision.0)
            {
                return Err(Error::Conflict);
            }
            if expected.0 > 0 {
                let previous: CollectionList =
                    projected_record(&db, "collection_lists", &list.id.to_string())?;
                if previous.kind != list.kind || previous.deleted {
                    return Err(Error::Conflict);
                }
            }
            for source in &sources {
                if projected_revision(
                    &db,
                    "items",
                    &source.item_id.to_string(),
                    list.collection_id,
                )? != source.item_revision.0
                {
                    return Err(Error::Conflict);
                }
                let mut item: Item = projected_record(&db, "items", &source.item_id.to_string())?;
                if item.deleted {
                    return Err(Error::Conflict);
                }
                let authority_revision: Option<u64> = db
                    .query_row(
                        "SELECT revision FROM metadata_authorities WHERE id=? AND collection_id=?",
                        [source.item_id.to_string(), list.collection_id.to_string()],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(storage)?;
                if authority_revision.unwrap_or(0) != source.metadata_revision.0 {
                    return Err(Error::Conflict);
                }
                hydrate_captures(&db, &mut item)?;
                let observed = item
                    .captures
                    .iter()
                    .map(|capture| (capture.id, capture.revision))
                    .collect::<std::collections::BTreeMap<_, _>>();
                if observed != source.captures.iter().copied().collect() {
                    return Err(Error::Conflict);
                }
                for (capture, expected) in &source.captures {
                    if projected_revision(
                        &db,
                        "captures",
                        &capture.to_string(),
                        list.collection_id,
                    )? != expected.0
                    {
                        return Err(Error::Conflict);
                    }
                }
            }
            collection_path(&db, &self.archive, list.collection_id)?
        };
        let result = CollectionListSummary {
            id: list.id,
            collection_id: list.collection_id,
            revision: list.revision,
            name: list.name.clone(),
            kind: list.kind,
            entry_count: list.entries.len() as u32,
            can_edit: true,
        };
        let scoped = self.with_command_result(&result)?;
        let transaction = publish_v3_revision(
            &scoped,
            V3RevisionTarget {
                collection: &path,
                directory_kind: "lists",
                entity_id: list.id.to_string(),
                collection_id: list.collection_id,
                expected,
                revision: list.revision,
            },
            list.clone(),
            vec![],
        )?;
        {
            let tx = db.transaction().map_err(storage)?;
            project(&tx, &list)?;
            self.record_command(&tx, &result)?;
            tx.commit().map_err(storage)?;
        }
        drop(db);
        finish_v3_publication(self, transaction)?;
        Ok(result)
    }
}
