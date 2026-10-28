//! Rebuildable compact rows with transactionally invalidated publication tokens.
use super::*;
use collector_app::{LibraryEntry, LibraryEntrySource};

const VARIANTS_PER_ITEM: i64 = 12;

pub(crate) fn install(db: &Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS library_entry_versions(item_id TEXT PRIMARY KEY,token TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS library_entries(item_id TEXT NOT NULL,namespace TEXT NOT NULL,language TEXT NOT NULL,role INTEGER NOT NULL,token TEXT NOT NULL,record TEXT NOT NULL,PRIMARY KEY(item_id,namespace,language,role));
        INSERT OR IGNORE INTO library_entry_versions SELECT id,hex(randomblob(16)) FROM items;")
        .map_err(storage)?;
    for (table, item) in [
        ("items", "SELECT {r}.id AS item_id"),
        (
            "captures",
            "SELECT {r}.item_id WHERE {r}.item_id IS NOT NULL",
        ),
        ("metadata_authorities", "SELECT {r}.id AS item_id"),
        ("metadata_candidates", "SELECT {r}.item_id"),
        ("metadata_refreshes", "SELECT {r}.item_id"),
        (
            "processing_observations",
            "SELECT c.item_id FROM entity_owners o JOIN captures c ON c.id=o.root_id WHERE {r}.source_scope='representation' AND o.kind='representation' AND o.id={r}.source_id AND o.collection_id={r}.collection_id AND o.root_kind='capture' AND c.item_id IS NOT NULL",
        ),
        (
            "conversion_recipes",
            "SELECT c.item_id FROM entity_owners o JOIN captures c ON c.id=o.root_id WHERE o.kind='representation' AND o.id={r}.id AND o.collection_id={r}.collection_id AND o.root_kind='capture' AND c.item_id IS NOT NULL",
        ),
        // Shared Work/Edition artwork and collection bindings can affect many
        // rows. These infrequent changes invalidate that collection's rows.
        (
            "assets",
            "SELECT id AS item_id FROM items WHERE collection_id={r}.collection_id",
        ),
        (
            "works",
            "SELECT id AS item_id FROM items WHERE collection_id={r}.collection_id",
        ),
        (
            "editions",
            "SELECT id AS item_id FROM items WHERE collection_id={r}.collection_id",
        ),
        (
            "collections",
            "SELECT id AS item_id FROM items WHERE collection_id={r}.id",
        ),
    ] {
        for (operation, refs) in [
            ("INSERT", &["NEW"][..]),
            ("UPDATE", &["OLD", "NEW"][..]),
            ("DELETE", &["OLD"][..]),
        ] {
            let mut body = String::new();
            for reference in refs {
                let selection = item.replace("{r}", reference);
                body.push_str(&format!(
                    "INSERT INTO library_entry_versions(item_id,token) SELECT item_id,hex(randomblob(16)) FROM ({selection}) WHERE 1 ON CONFLICT(item_id) DO UPDATE SET token=excluded.token;"
                ));
            }
            db.execute_batch(&format!("CREATE TRIGGER IF NOT EXISTS library_{table}_{operation} AFTER {operation} ON {table} BEGIN {body} END;")).map_err(storage)?;
        }
    }
    Ok(())
}

pub(crate) fn sources(
    db: &Connection,
    user: UserId,
    query: BrowseQuery,
    namespace: &str,
    language: &str,
) -> Result<Page<LibraryEntrySource>> {
    let page = browse::rows(db, user, query, Some((namespace, language)))?;
    let items = page
        .items
        .into_iter()
        .map(|row| {
            Ok(LibraryEntrySource {
                id: row.id,
                collection_id: row.collection_id,
                role: row.role,
                generation: row.generation,
                cached: row
                    .record
                    .map(|raw| serde_json::from_str(&raw).map_err(storage))
                    .transpose()?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Page {
        items,
        next_cursor: page.next_cursor,
    })
}

pub(crate) fn cache(
    db: &Connection,
    namespace: &str,
    language: &str,
    role: Role,
    generation: &str,
    entry: &LibraryEntry,
) -> Result<bool> {
    let raw = serde_json::to_string(entry).map_err(storage)?;
    // Keep at most the current policy's row for each role/language. Cache writes
    // do not touch archive checkpoints, job state or source generations.
    db.execute(
        "DELETE FROM library_entries WHERE item_id=? AND namespace<>?",
        params![entry.id.to_string(), namespace],
    )
    .map_err(storage)?;
    let changed = db
        .execute(
            "INSERT OR REPLACE INTO library_entries(item_id,namespace,language,role,token,record)
        SELECT ?1,?2,?3,?4,token,?6 FROM library_entry_versions WHERE item_id=?1 AND token=?5",
            params![
                entry.id.to_string(),
                namespace,
                language,
                role as u8,
                generation,
                raw
            ],
        )
        .map_err(storage)?;
    if changed == 1 {
        // Language preferences are user-controlled. Bound disposable variants,
        // retaining the most recently built rows without changing source data.
        db.execute(
            "DELETE FROM library_entries WHERE item_id=?1 AND rowid NOT IN (
            SELECT rowid FROM library_entries WHERE item_id=?1 ORDER BY rowid DESC LIMIT ?2)",
            params![entry.id.to_string(), VARIANTS_PER_ITEM],
        )
        .map_err(storage)?;
    }
    Ok(changed == 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use collector_app::{CollectionService, CommandContext, CreateCollectionCommand};
    use rusqlite::hooks::{AuthAction, AuthContext, Authorization};

    #[test]
    fn warm_library_needs_no_item_payload_capture_observation_or_recipe_reads() {
        let root = tempfile::tempdir().unwrap();
        let store =
            Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
        store
            .create_user("owner", "a long test password", true)
            .unwrap();
        let authenticated = store
            .login("owner", "a long test password", "fixture")
            .unwrap()
            .1;
        let actor = &authenticated.principal;
        let service = CollectionService::new(
            store.clone(),
            Arc::new(collector_modules::Registry::builtins()),
        );
        let collection = service
            .create_collection(
                &authenticated,
                CreateCollectionCommand {
                    name: "Cached library".into(),
                    module: ModuleId("org.junk.music".into()),
                },
                &CommandContext::new("cache-collection").unwrap(),
            )
            .unwrap();
        let item = Item {
            lifecycle: Default::default(),
            id: ItemId::new(),
            collection_id: collection.id,
            revision: Revision(0),
            title: "Recorded album".into(),
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
            wishlist: false,
            captures: vec![],
            assets: vec![],
            metadata: Default::default(),
            deleted: false,
            updated_at: Utc::now(),
        };
        let saved = Catalog::save(&*store, item, Revision(0)).unwrap();
        let query = BrowseQuery {
            collection_id: Some(collection.id),
            ..Default::default()
        };
        let cold = service.library_entries(actor, query.clone()).unwrap();
        // Turn the resource contract into a database-enforced failure, rather
        // than asserting a particular helper name or a wall-clock threshold.
        lock(&store.db)
            .unwrap()
            .authorizer(Some(|context: AuthContext<'_>| match context.action {
                AuthAction::Read {
                    table_name,
                    column_name,
                } if (table_name == "items" && column_name == "record")
                    || matches!(
                        table_name,
                        "captures" | "processing_observations" | "conversion_recipes"
                    ) =>
                {
                    Authorization::Deny
                }
                _ => Authorization::Allow,
            }));
        assert!(
            service.item_presented(actor, saved.id).is_err(),
            "detail-read trap is not active"
        );
        let warm = service.library_entries(actor, query.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(&warm).unwrap(),
            serde_json::to_value(&cold).unwrap()
        );
        {
            let db = lock(&store.db).unwrap();
            let (namespace, token): (String, String) = db
                .query_row(
                    "SELECT namespace,token FROM library_entries WHERE item_id=?",
                    [saved.id.to_string()],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();
            for number in 0..VARIANTS_PER_ITEM + 2 {
                assert!(
                    cache(
                        &db,
                        &namespace,
                        &format!("en-x-cache{number}"),
                        Role::Owner,
                        &token,
                        &cold.items[0]
                    )
                    .unwrap()
                );
            }
            let count: i64 = db
                .query_row(
                    "SELECT COUNT(*) FROM library_entries WHERE item_id=?",
                    [saved.id.to_string()],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(count <= VARIANTS_PER_ITEM);
            db.authorizer(None::<fn(AuthContext<'_>) -> Authorization>);
        }
        // Eviction discards only a derived view: the ordinary resolver rebuilds
        // the original language without losing or changing the stored Item.
        assert_eq!(
            serde_json::to_value(service.library_entries(actor, query.clone()).unwrap()).unwrap(),
            serde_json::to_value(&cold).unwrap()
        );
        let preserved = serde_json::to_value(Catalog::item(&*store, saved.id).unwrap()).unwrap();
        drop(service);
        drop(store);
        // Model an existing installation with the current catalog format but
        // none of the new disposable cache objects. Its index must stay reusable.
        let db = Connection::open(root.path().join("state/catalog.sqlite")).unwrap();
        let triggers = db
            .prepare(
                "SELECT name FROM sqlite_master WHERE type='trigger' AND name LIKE 'library_%'",
            )
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        for trigger in triggers {
            db.execute_batch(&format!(
                "DROP TRIGGER \"{}\"",
                trigger.replace('"', "\"\"")
            ))
            .unwrap();
        }
        db.execute_batch("DROP TABLE library_entries; DROP TABLE library_entry_versions;")
            .unwrap();
        drop(db);
        let store =
            Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
        assert!(
            store.catalog_reused,
            "additive cache install forced full catalog replay"
        );
        assert_eq!(
            serde_json::to_value(Catalog::item(&*store, saved.id).unwrap()).unwrap(),
            preserved
        );
        let service =
            CollectionService::new(store, Arc::new(collector_modules::Registry::builtins()));
        assert_eq!(
            serde_json::to_value(service.library_entries(actor, query).unwrap()).unwrap(),
            serde_json::to_value(cold).unwrap()
        );
    }
}
