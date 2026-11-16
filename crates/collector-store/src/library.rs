//! Rebuildable compact rows with transactionally invalidated publication tokens.
use super::*;
use collector_app::{LibraryEntry, LibraryEntryBuild, LibraryEntrySource};

pub(crate) fn install(connection: &Connection) -> Result<()> {
    connection.execute_batch("CREATE TABLE IF NOT EXISTS library_track_entries(item_id TEXT NOT NULL,namespace TEXT NOT NULL,language TEXT NOT NULL,role INTEGER NOT NULL,token TEXT NOT NULL,record TEXT NOT NULL,PRIMARY KEY(item_id,namespace,language,role));").map_err(storage)?;
    // A stopped upgrade must not retain a basic row with half-built filter flags.
    let tx = connection.unchecked_transaction().map_err(storage)?;
    let db = &*tx;
    db.execute_batch("CREATE TABLE IF NOT EXISTS library_entry_versions(item_id TEXT PRIMARY KEY,token TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS library_entries(item_id TEXT NOT NULL,namespace TEXT NOT NULL,language TEXT NOT NULL,role INTEGER NOT NULL,token TEXT NOT NULL,record TEXT NOT NULL,PRIMARY KEY(item_id,namespace,language,role));
        INSERT OR IGNORE INTO library_entry_versions SELECT id,hex(randomblob(16)) FROM items;")
        .map_err(storage)?;
    db.execute_batch("CREATE TABLE IF NOT EXISTS library_entry_basics(item_id TEXT PRIMARY KEY,record TEXT NOT NULL,files_attached INTEGER NOT NULL,playable INTEGER NOT NULL,listening INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS library_entry_attempts(item_id TEXT NOT NULL,namespace TEXT NOT NULL,language TEXT NOT NULL,role INTEGER NOT NULL,token TEXT NOT NULL,retry_at INTEGER NOT NULL,PRIMARY KEY(item_id,namespace,language,role));").map_err(storage)?;
    // Additive upgrade backfill only: ordinary reads never open these payloads.
    let mut statement = db
        .prepare(
            "SELECT record FROM items WHERE id NOT IN (SELECT item_id FROM library_entry_basics)",
        )
        .map_err(storage)?;
    let mut rows = statement.query([]).map_err(storage)?;
    while let Some(row) = rows.next().map_err(storage)? {
        let item: Item =
            serde_json::from_str(&row.get::<_, String>(0).map_err(storage)?).map_err(storage)?;
        project_basic(db, &item)?;
    }
    drop(rows);
    drop(statement);
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
    db.execute_batch("CREATE TABLE IF NOT EXISTS library_clock(token TEXT NOT NULL); INSERT INTO library_clock SELECT hex(randomblob(16)) WHERE NOT EXISTS(SELECT 1 FROM library_clock);
        CREATE TABLE IF NOT EXISTS private.library_clock(token TEXT NOT NULL); INSERT INTO private.library_clock SELECT hex(randomblob(16)) WHERE NOT EXISTS(SELECT 1 FROM private.library_clock);") .map_err(storage)?;
    // Derivative Jobs are keyed by Capture and survive reassociation. Resolve
    // the current owning Item when the library worker drains the invalidations.
    db.execute_batch("CREATE TABLE IF NOT EXISTS private.library_dirty(item_id TEXT PRIMARY KEY);
        CREATE TABLE IF NOT EXISTS private.library_dirty_captures(capture_id TEXT PRIMARY KEY);
        CREATE TRIGGER IF NOT EXISTS private.library_derivative_admitted AFTER INSERT ON jobs WHEN NEW.kind='music_derivative' BEGIN
            INSERT OR IGNORE INTO library_dirty_captures SELECT json_extract(NEW.request,'$.capture_id') WHERE json_extract(NEW.request,'$.capture_id') IS NOT NULL;
        END;
        CREATE TRIGGER IF NOT EXISTS private.library_derivative_state AFTER UPDATE OF state ON jobs WHEN NEW.kind='music_derivative' AND NEW.state<>OLD.state BEGIN
            INSERT OR IGNORE INTO library_dirty_captures SELECT json_extract(NEW.request,'$.capture_id') WHERE json_extract(NEW.request,'$.capture_id') IS NOT NULL;
        END;").map_err(storage)?;
    for (schema, tables) in [
        (
            "main",
            &[
                "library_entry_versions",
                "library_entries",
                "browse_items",
                "metadata_language_display",
            ][..],
        ),
        (
            "private",
            &[
                "grants",
                "metadata_preferences",
                "users",
                "library_dirty",
                "library_dirty_captures",
            ][..],
        ),
    ] {
        for table in tables {
            for operation in ["INSERT", "UPDATE", "DELETE"] {
                db.execute_batch(&format!("CREATE TRIGGER IF NOT EXISTS {schema}.catalog_{table}_{operation} AFTER {operation} ON {table} BEGIN UPDATE library_clock SET token=hex(randomblob(16)); END;")).map_err(storage)?;
            }
        }
    }
    install_cursor_clocks(&tx)?;
    db.execute("DELETE FROM library_entries WHERE NOT EXISTS(SELECT 1 FROM library_track_entries t WHERE t.item_id=library_entries.item_id AND t.namespace=library_entries.namespace AND t.language=library_entries.language AND t.role=library_entries.role AND t.token=library_entries.token)", []).map_err(storage)?;
    tx.commit().map_err(storage)
}

fn install_cursor_clocks(db: &Connection) -> Result<()> {
    for (schema, clock, tables) in [
        (
            "main",
            "library_cursor_clock",
            &[
                "browse_items",
                "library_entry_basics",
                "metadata_language_display",
            ][..],
        ),
        (
            "private",
            "library_cursor_clock",
            &["grants", "metadata_preferences", "users"][..],
        ),
        (
            "private",
            "library_work_clock",
            &["jobs", "imports", "guided_ingest_intents"][..],
        ),
    ] {
        db.execute_batch(&format!(
            "CREATE TABLE IF NOT EXISTS {schema}.{clock}(token TEXT NOT NULL);
             INSERT INTO {schema}.{clock} SELECT hex(randomblob(16))
               WHERE NOT EXISTS(SELECT 1 FROM {schema}.{clock});"
        ))
        .map_err(storage)?;
        for table in tables {
            for operation in ["INSERT", "UPDATE", "DELETE"] {
                let when = match (*table, operation) {
                    ("jobs", "UPDATE") => {
                        "WHEN OLD.state IS NOT NEW.state OR OLD.item_id IS NOT NEW.item_id OR OLD.request IS NOT NEW.request"
                    }
                    ("imports", "UPDATE") => {
                        "WHEN OLD.phase IS NOT NEW.phase OR OLD.item_id IS NOT NEW.item_id"
                    }
                    ("guided_ingest_intents", "UPDATE") => {
                        "WHEN OLD.target_item_id IS NOT NEW.target_item_id OR OLD.settled IS NOT NEW.settled OR OLD.cancelled IS NOT NEW.cancelled OR OLD.enrichment_error IS NOT NEW.enrichment_error OR OLD.preparation_error IS NOT NEW.preparation_error"
                    }
                    _ => "",
                };
                db.execute_batch(&format!(
                    "CREATE TRIGGER IF NOT EXISTS {schema}.{clock}_{table}_{operation}
                     AFTER {operation} ON {table} {when}
                     BEGIN UPDATE {clock} SET token=hex(randomblob(16)); END;"
                ))
                .map_err(storage)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn cursor_revision(db: &Connection, filter: BrowseFilter) -> Result<String> {
    let sql = match filter {
        BrowseFilter::AlbumPlaybackNeedsAttention => {
            "SELECT a.token||b.token||w.token||p.token||q.token FROM main.library_cursor_clock a,private.library_cursor_clock b,private.library_work_clock w,main.library_clock p,private.library_clock q"
        }
        BrowseFilter::NeedsAttention => {
            "SELECT a.token||b.token||w.token FROM main.library_cursor_clock a,private.library_cursor_clock b,private.library_work_clock w"
        }
        _ => {
            "SELECT a.token||b.token FROM main.library_cursor_clock a,private.library_cursor_clock b"
        }
    };
    db.query_row(sql, [], |r| r.get(0)).map_err(storage)
}

pub(crate) fn sources(page: Page<browse::BrowseRow>) -> Result<Page<LibraryEntrySource>> {
    let items = page
        .items
        .into_iter()
        .map(|row| {
            Ok(LibraryEntrySource {
                id: row.id,
                collection_id: row.collection_id,
                role: row.role,
                generation: row.generation,
                pending: serde_json::from_str(&row.basic).map_err(storage)?,
                retrying: row.retrying,
                cursor: row.cursor,
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
        db.execute("DELETE FROM library_entry_attempts WHERE item_id=? AND namespace=? AND language=? AND role=? AND token=?",params![entry.id.to_string(),namespace,language,role as u8,generation]).map_err(storage)?;
        prune(db, entry.id, namespace)?;
    }
    Ok(changed == 1)
}

pub(crate) fn project_basic(db: &Connection, item: &Item) -> Result<()> {
    let entry = LibraryEntry::pending(item);
    db.execute(
        "INSERT OR REPLACE INTO library_entry_basics VALUES(?,?,0,0,0)",
        params![
            item.id.to_string(),
            serde_json::to_string(&entry).map_err(storage)?
        ],
    )
    .map_err(storage)?;
    project_filters(db, item.id)
}

pub(crate) fn project_filters(db: &Connection, id: ItemId) -> Result<()> {
    let sql = format!(
        "UPDATE library_entry_basics SET (files_attached,playable,listening)=(SELECT {},{},{} FROM items i WHERE i.id=?1) WHERE item_id=?1",
        browse::FILES_ATTACHED,
        browse::PLAYABLE,
        browse::LISTENING
    );
    db.execute(&sql, [id.to_string()]).map_err(storage)?;
    Ok(())
}

// Presentation demand comes from actual enabled accounts/grants, not browser visits.
const DEMAND: &str = "SELECT g.collection_id,g.role,COALESCE(p.language,'en') AS language,MIN(u.id) AS user_id FROM private.grants g JOIN private.users u ON u.id=g.user_id AND u.enabled=1 LEFT JOIN private.metadata_preferences p ON p.user_id=u.id GROUP BY g.collection_id,g.role,COALESCE(p.language,'en')";

pub(crate) fn next_build(
    db: &Connection,
    namespace: &str,
    now: i64,
) -> Result<Option<LibraryEntryBuild>> {
    let tx = db.unchecked_transaction().map_err(storage)?;
    tx.execute("UPDATE library_entry_versions SET token=hex(randomblob(16)) WHERE item_id IN (SELECT item_id FROM private.library_dirty UNION SELECT c.item_id FROM private.library_dirty_captures d JOIN captures c ON c.id=d.capture_id)",[]).map_err(storage)?;
    tx.execute_batch(
        "DELETE FROM private.library_dirty; DELETE FROM private.library_dirty_captures;",
    )
    .map_err(storage)?;
    tx.commit().map_err(storage)?;
    let sql = format!(
        "WITH demand AS ({DEMAND}) SELECT i.id,i.collection_id,d.role,d.language,d.user_id,v.token,u.administrator FROM items i JOIN library_entry_versions v ON v.item_id=i.id JOIN demand d ON d.collection_id=i.collection_id JOIN private.users u ON u.id=d.user_id LEFT JOIN library_entries c ON c.item_id=i.id AND c.namespace=?1 AND c.language=d.language AND c.role=d.role AND c.token=v.token LEFT JOIN library_entry_attempts a ON a.item_id=i.id AND a.namespace=?1 AND a.language=d.language AND a.role=d.role WHERE c.item_id IS NULL AND (i.deleted=0 OR d.role>=1) AND (a.token IS NULL OR a.token<>v.token OR a.retry_at<=?2) ORDER BY CASE WHEN a.token=v.token THEN a.retry_at ELSE 0 END,i.id,d.role,d.language LIMIT 1"
    );
    let selected = db
        .query_row(&sql, params![namespace, now], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, bool>(6)?,
            ))
        })
        .optional()
        .map_err(storage)?;
    selected
        .map(
            |(id, collection, role, language, user, generation, administrator)| {
                Ok(LibraryEntryBuild {
                    id: id.parse().map_err(storage)?,
                    collection_id: collection.parse().map_err(storage)?,
                    role: role_from_database(role)?,
                    language,
                    generation,
                    actor: Principal {
                        user_id: user.parse().map_err(storage)?,
                        administrator,
                        device_id: None,
                    },
                })
            },
        )
        .transpose()
}

pub(crate) fn defer(
    db: &Connection,
    namespace: &str,
    build: &LibraryEntryBuild,
    now: i64,
) -> Result<()> {
    db.execute("INSERT OR REPLACE INTO library_entry_attempts SELECT ?1,?2,?3,?4,token,?6 FROM library_entry_versions WHERE item_id=?1 AND token=?5",params![build.id.to_string(),namespace,build.language,build.role as u8,build.generation,now+5]).map_err(storage)?;
    prune(db, build.id, namespace)
}

fn prune(db: &Connection, id: ItemId, namespace: &str) -> Result<()> {
    for table in [
        "library_entries",
        "library_entry_attempts",
        "library_track_entries",
    ] {
        let sql = format!(
            "DELETE FROM {table} WHERE item_id=?1 AND (namespace<>?2 OR NOT EXISTS(SELECT 1 FROM items i JOIN private.grants g ON g.collection_id=i.collection_id JOIN private.users u ON u.id=g.user_id AND u.enabled=1 LEFT JOIN private.metadata_preferences p ON p.user_id=u.id WHERE i.id={table}.item_id AND g.role={table}.role AND COALESCE(p.language,'en')={table}.language AND (i.deleted=0 OR g.role>=1)))"
        );
        db.execute(&sql, params![id.to_string(), namespace])
            .map_err(storage)?;
    }
    Ok(())
}

pub(crate) fn revision(db: &Connection) -> Result<String> {
    db.query_row(
        "SELECT a.token || b.token FROM library_clock a, private.library_clock b",
        [],
        |row| row.get(0),
    )
    .map_err(storage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use collector_app::{CollectionService, CommandContext, CreateCollectionCommand};
    use rusqlite::hooks::{AuthAction, AuthContext, Authorization};

    fn fixture() -> (
        tempfile::TempDir,
        Arc<Store>,
        Principal,
        CollectionService,
        Item,
    ) {
        fixture_with_clock(Arc::new(SystemClock::default()))
    }

    fn fixture_with_clock(
        clock: Arc<dyn Clock>,
    ) -> (
        tempfile::TempDir,
        Arc<Store>,
        Principal,
        CollectionService,
        Item,
    ) {
        let root = tempfile::tempdir().unwrap();
        let store = Arc::new(
            Store::open_with_clock(
                root.path().join("archive"),
                root.path().join("state"),
                clock,
            )
            .unwrap(),
        );
        store
            .create_user("owner", "a long test password", true)
            .unwrap();
        let authenticated = store
            .login("owner", "a long test password", "fixture")
            .unwrap()
            .1;
        let actor = authenticated.principal.clone();
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
            purchase_orders: vec![],
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
        (root, store, actor, service, saved)
    }

    #[test]
    fn cache_language_is_frozen_before_resolving_metadata() {
        use collector_app::{MetadataAuthorityRecord, ProcessingPort, UpdateMetadataPreferences};
        use std::sync::atomic::{AtomicBool, Ordering};
        let (root, store, actor, service, saved) = fixture();
        let candidates = [("en", "English album"), ("ja", "日本語のアルバム")]
            .into_iter()
            .map(|(language, title)| MetadataCandidateObservation {
                identity: MetadataCandidateIdentity {
                    provider: ModuleId("org.example.library".into()),
                    provider_record_id: "album".into(),
                    provider_record_version: Some(language.into()),
                },
                title: MetadataText {
                    value: title.into(),
                    language: Some(LanguageTag::new(language).unwrap()),
                },
                subtitle: None,
                artwork: None,
                complete_identity: true,
                strength: MatchStrength::Identifier,
                provider_priority: 1,
                observation: RawJson::from_value(serde_json::json!({})),
            })
            .collect::<Vec<_>>();
        let authority = MetadataAuthority {
            automatic_candidate: Some(candidates[0].identity.clone()),
            automatic_language: Some(LanguageTag::new("en").unwrap()),
            ..Default::default()
        };
        let display =
            collector_app::resolve_metadata_display(&saved, &authority, &candidates, "en").unwrap();
        store
            .publish_metadata_authority(
                TransactionId::new(),
                MetadataAuthorityRecord {
                    resolution_policy: 4,
                    current_candidates: Some(
                        candidates.iter().map(|c| c.identity.clone()).collect(),
                    ),
                    inventory: Some((&saved).into()),
                    collection_id: saved.collection_id,
                    item_id: saved.id,
                    revision: Revision(1),
                    authority,
                    observations: vec![MetadataRefreshObservation {
                        id: ObservationId::new(),
                        outcome: MetadataRefreshOutcome::Candidates,
                        candidates,
                        raw_observation: RawJson::from_value(serde_json::json!({})),
                        observed_at: Utc::now(),
                    }],
                    display: Some(display),
                    updated_at: Utc::now(),
                },
                None,
            )
            .unwrap();
        service
            .update_metadata_preferences(
                &actor,
                UpdateMetadataPreferences {
                    expected_revision: Revision(0),
                    language: LanguageTag::new("ja").unwrap(),
                },
                &CommandContext::new("cache-japanese").unwrap(),
            )
            .unwrap();
        // Another Japanese reader keeps this variant wanted after the owner's
        // preference changes between work selection and metadata calculation.
        let japanese_user = store
            .create_user("japanese-reader", "another long password", false)
            .unwrap();
        collector_app::LibraryCarePort::save_metadata_preferences(
            &*store,
            japanese_user,
            &UpdateMetadataPreferences {
                expected_revision: Revision(0),
                language: LanguageTag::new("ja").unwrap(),
            },
        )
        .unwrap();
        let switched = Arc::new(AtomicBool::new(false));
        let flag = switched.clone();
        let owner = actor.user_id.to_string();
        let joining_user = japanese_user.to_string();
        let joining_collection = saved.collection_id.to_string();
        store.readers.for_each(|db| {
            let flag = flag.clone();
            let private = Connection::open(root.path().join("state/private.sqlite")).unwrap();
            let owner = owner.clone();
            let joining_user = joining_user.clone();
            let joining_collection = joining_collection.clone();
            db.authorizer(Some(move |context: AuthContext<'_>| {
                if matches!(
                    context.action,
                    AuthAction::Read {
                        table_name: "items",
                        column_name: "record"
                    }
                ) && !flag.swap(true, Ordering::SeqCst)
                {
                    private.execute("INSERT INTO grants(collection_id,user_id,role,revision) VALUES(?,?,2,1)",params![joining_collection,joining_user]).unwrap();
                    private.execute("UPDATE metadata_preferences SET language='en' WHERE user_id=?",[&owner]).unwrap();
                }
                Authorization::Allow
            })).unwrap();
        });
        assert!(service.rebuild_next_library_entry().unwrap());
        assert!(switched.load(Ordering::SeqCst));
        allow_details(&store);
        drain(&service);
        let query = BrowseQuery {
            collection_id: Some(saved.collection_id),
            ..Default::default()
        };
        let japanese = Principal {
            user_id: japanese_user,
            administrator: false,
            device_id: None,
        };
        assert_eq!(
            service
                .library_entries(&japanese, query.clone())
                .unwrap()
                .items[0]
                .title,
            "日本語のアルバム"
        );
        assert_eq!(
            service
                .library_entries(&actor, query.clone())
                .unwrap()
                .items[0]
                .title,
            "English album"
        );
        for (revision, language, title) in
            [(1, "en", "English album"), (2, "ja", "日本語のアルバム")]
        {
            service
                .update_metadata_preferences(
                    &actor,
                    UpdateMetadataPreferences {
                        expected_revision: Revision(revision),
                        language: LanguageTag::new(language).unwrap(),
                    },
                    &CommandContext::new(format!("cache-language-{language}")).unwrap(),
                )
                .unwrap();
            drain(&service);
            assert_eq!(
                service
                    .library_entries(&actor, query.clone())
                    .unwrap()
                    .items[0]
                    .title,
                title
            );
        }
    }

    fn drain(service: &CollectionService) {
        for _ in 0..1000 {
            if !service.rebuild_next_library_entry().unwrap() {
                return;
            }
        }
        panic!("summary rebuild did not settle");
    }

    fn deny_details(store: &Store) {
        let deny = |context: AuthContext<'_>| match context.action {
            AuthAction::Read {
                table_name,
                column_name,
            } if (table_name == "items" && column_name == "record")
                || matches!(table_name, "processing_observations" | "conversion_recipes")
                || table_name == "captures" && !matches!(column_name, "id" | "item_id") =>
            {
                Authorization::Deny
            }
            _ => Authorization::Allow,
        };
        lock(&store.db).unwrap().authorizer(Some(deny)).unwrap();
        store
            .readers
            .for_each(|db| db.authorizer(Some(deny)).unwrap());
    }
    fn allow_details(store: &Store) {
        lock(&store.db)
            .unwrap()
            .authorizer(None::<fn(AuthContext<'_>) -> Authorization>)
            .unwrap();
        store.readers.for_each(|db| {
            db.authorizer(None::<fn(AuthContext<'_>) -> Authorization>)
                .unwrap()
        });
    }

    #[test]
    fn preparation_list_uses_only_compact_rows_even_when_pending() {
        use collector_app::{LibrarySummaryState as State, MetadataCareQuery};
        let (_root, store, actor, service, saved) = fixture();
        let profile = ConversionProfile::Opus { bitrate_kbps: 128 };
        let mut query = MetadataCareQuery {
            collection_id: Some(saved.collection_id),
            search: "recorded".into(),
            reason: None,
            needs_attention: true,
            cursor: None,
            limit: Some(1),
        };
        deny_details(&store);
        assert!(
            service
                .music_album_preparation_plan(&actor, saved.id, &profile)
                .is_err()
        );
        let cold = service
            .preparation_care(&actor, query.clone(), &profile)
            .unwrap();
        assert_eq!(cold.summary_state, State::Pending);
        assert!(cold.items.is_empty());
        allow_details(&store);
        drain(&service);
        deny_details(&store);
        let warm = service
            .preparation_care(&actor, query.clone(), &profile)
            .unwrap();
        assert_eq!(warm.summary_state, State::Ready);
        assert_eq!(warm.items[0].item.id, saved.id);
        assert!(warm.items[0].readiness.as_ref().unwrap().needs_attention);
        assert_ne!(warm.catalog_revision, cold.catalog_revision);
        query.search = "elsewhere".into();
        let excluded = service.preparation_care(&actor, query, &profile).unwrap();
        assert!(excluded.items.is_empty());
        assert_eq!(excluded.summary_state, State::Ready);
    }

    #[test]
    fn start_preparation_freezes_more_than_one_page_without_reading_album_details() {
        use collector_app::{
            BulkLibraryState, LibrarySelection, MetadataCareQuery, StartLibraryPreparation,
        };

        let (root, store, actor, service, saved) = fixture();
        for index in 0..64 {
            let mut album = saved.clone();
            album.id = ItemId::new();
            album.title = format!("Recorded album {index:02}");
            Catalog::save(&*store, album, Revision(0)).unwrap();
        }
        let mut other = saved.clone();
        other.id = ItemId::new();
        other.title = "Outside the search".into();
        Catalog::save(&*store, other, Revision(0)).unwrap();
        let viewer = store
            .create_user("batch-viewer", "a long viewer password", false)
            .unwrap();
        Catalog::grant(&*store, saved.collection_id, viewer, Role::Viewer).unwrap();
        drain(&service);
        deny_details(&store);
        assert!(matches!(
            service.start_library_preparation(
                &Principal {
                    user_id: viewer,
                    administrator: false,
                    device_id: None
                },
                StartLibraryPreparation {
                    profile: ConversionProfile::Opus { bitrate_kbps: 128 },
                    selection: LibrarySelection::Explicit {
                        items: vec![saved.id]
                    },
                    sources: Default::default(),
                },
                &CommandContext::new("viewer-cannot-prepare").unwrap(),
            ),
            Err(Error::Forbidden)
        ));
        let receipt = CommandContext::new("prepare-all-with-compact-rows").unwrap();
        let request = StartLibraryPreparation {
            profile: ConversionProfile::Opus { bitrate_kbps: 128 },
            selection: LibrarySelection::AllMatching {
                query: MetadataCareQuery {
                    collection_id: Some(saved.collection_id),
                    search: "recorded".into(),
                    reason: None,
                    needs_attention: true,
                    cursor: None,
                    limit: Some(50),
                },
                exclusions: vec![saved.id],
            },
            sources: Default::default(),
        };
        let batch = service
            .start_library_preparation(&actor, request.clone(), &receipt)
            .unwrap();
        assert_eq!(batch.state, BulkLibraryState::Accepted);
        assert_eq!(batch.selected, 64);
        assert_eq!(batch.pending, 64);
        assert_eq!(batch.tracks, 0);
        assert_eq!(
            service
                .start_library_preparation(&actor, request, &receipt)
                .unwrap()
                .id,
            batch.id
        );
        let members = service
            .bulk_library_members(&actor, batch.id, None, 100)
            .unwrap()
            .items;
        assert_eq!(members.len(), 64);
        assert!(
            members
                .iter()
                .all(|member| member.item_id != saved.id && member.job_id.is_none())
        );
        allow_details(&store);
        drop(service);
        drop(store);
        let reopened = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
        let claim = reopened
            .claim_job("resumed-preparation", 30)
            .unwrap()
            .unwrap();
        assert_eq!(claim.id, batch.id);
        assert_eq!(claim.user_id, actor.user_id);

        // Ready members can appear just before the worker commits completion.
        reopened
            .complete_job(claim.id, &claim.claim_token, serde_json::json!({}))
            .unwrap();
        collector_app::BulkLibraryPort::reconcile_bulk_library(&reopened, actor.user_id, batch.id)
            .unwrap();
        assert_eq!(
            reopened
                .claim_job("wake-coordinator", 30)
                .unwrap()
                .unwrap()
                .id,
            batch.id
        );
    }

    #[test]
    fn cold_warm_and_invalidated_library_never_read_item_or_track_payloads() {
        use collector_app::LibrarySummaryState as State;
        let (root, store, actor, service, saved) = fixture();
        let query = BrowseQuery {
            collection_id: Some(saved.collection_id),
            ..Default::default()
        };
        deny_details(&store);
        assert!(
            service.item_presented(&actor, saved.id).is_err(),
            "detail trap inactive"
        );
        let cold = service.library_entries(&actor, query.clone()).unwrap();
        assert_eq!(cold.summary_state, State::Pending);
        assert_eq!(cold.items.len(), 1);
        assert!(
            cold.items[0].artwork.is_none()
                && cold.items[0].accuraterip.is_none()
                && !cold.items[0].can_edit
        );
        for filter in [
            BrowseFilter::All,
            BrowseFilter::Wishlist,
            BrowseFilter::FilesAttached,
            BrowseFilter::Playable,
            BrowseFilter::NeedsAttention,
            BrowseFilter::AlbumPlaybackNeedsAttention,
            BrowseFilter::Removed,
        ] {
            service
                .library_entries(
                    &actor,
                    BrowseQuery {
                        filter,
                        ..query.clone()
                    },
                )
                .unwrap();
        }
        service
            .library_entries(
                &actor,
                BrowseQuery {
                    listening_only: true,
                    ..query.clone()
                },
            )
            .unwrap();
        let attention = service
            .library_entries(
                &actor,
                BrowseQuery {
                    filter: BrowseFilter::AlbumPlaybackNeedsAttention,
                    ..query.clone()
                },
            )
            .unwrap();
        assert!(attention.items.is_empty());
        assert_eq!(
            attention.summary_state,
            State::Pending,
            "empty filtered pages lost pending work"
        );
        allow_details(&store);
        drain(&service);
        deny_details(&store);
        let warm = service.library_entries(&actor, query.clone()).unwrap();
        assert_eq!(warm.summary_state, State::Ready);
        lock(&store.db)
            .unwrap()
            .execute(
                "UPDATE library_entry_versions SET token=hex(randomblob(16)) WHERE item_id=?",
                [saved.id.to_string()],
            )
            .unwrap();
        let invalidated = service.library_entries(&actor, query.clone()).unwrap();
        assert_eq!(invalidated.summary_state, State::Pending);
        assert!(
            invalidated.items[0].album_playback.is_none(),
            "stale readiness was exposed"
        );
        allow_details(&store);
        // Dirty generation survives restart and is repaired without a browse.
        let preserved = serde_json::to_value(Catalog::item(&*store, saved.id).unwrap()).unwrap();
        drop(service);
        drop(store);
        let db = Connection::open(root.path().join("state/catalog.sqlite")).unwrap();
        let triggers = db
            .prepare(
                "SELECT name FROM sqlite_master WHERE type='trigger' AND name LIKE 'library_%'",
            )
            .unwrap()
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        for name in triggers {
            db.execute_batch(&format!("DROP TRIGGER \"{}\"", name.replace('"', "\"\"")))
                .unwrap();
        }
        db.execute_batch("DROP TABLE library_entry_basics; DROP TABLE library_entry_attempts; DROP TABLE library_entries; DROP TABLE library_entry_versions;").unwrap();
        drop(db);
        let store =
            Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
        assert!(
            store.catalog_reused,
            "additive install replayed the archive"
        );
        assert_eq!(
            serde_json::to_value(Catalog::item(&*store, saved.id).unwrap()).unwrap(),
            preserved
        );
        let service = CollectionService::new(
            store.clone(),
            Arc::new(collector_modules::Registry::builtins()),
        );
        assert!(
            service.rebuild_next_library_entry().unwrap(),
            "startup work required a Library request"
        );
        assert!(!service.rebuild_next_library_entry().unwrap());
        deny_details(&store);
        let rebuilt = service.library_entries(&actor, query).unwrap();
        assert_ne!(
            rebuilt.catalog_revision, warm.catalog_revision,
            "rebuilt projections must reject earlier cursors"
        );
        assert_eq!(
            serde_json::to_value(rebuilt.items).unwrap(),
            serde_json::to_value(warm.items).unwrap()
        );
    }
    #[test]
    fn stateless_counts_byte_pages_and_track_headers_use_only_projected_metadata() {
        let (_root, store, actor, service, saved) = fixture();
        for index in 0..8 {
            let mut item = saved.clone();
            item.id = ItemId::new();
            item.revision = Revision(0);
            item.title = format!("Album {index} {}", "音".repeat(300));
            Catalog::save(&*store, item, Revision(0)).unwrap();
        }
        let query = BrowseQuery {
            collection_id: Some(saved.collection_id),
            max_bytes: Some(8192),
            ..Default::default()
        };
        let summary = service.library_summary(&actor, query.clone()).unwrap();
        assert_eq!(summary.total_items, 9);
        assert!(summary.total_is_exact);
        let pending = service.library_track_summary(&actor, saved.id).unwrap();
        assert!(pending.pending);
        assert_eq!(pending.total_items, None);
        while service.rebuild_next_library_entry().unwrap() {}
        let summary = service.library_summary(&actor, query.clone()).unwrap();
        deny_details(&store);
        let before_reads = lock(&store.db).unwrap().total_changes();
        let header = service.library_track_summary(&actor, saved.id).unwrap();
        assert!(!header.pending);
        assert_eq!(header.total_items, Some(0));
        let mut cursor = None;
        let mut ids = std::collections::BTreeSet::new();
        let mut pages = 0;
        loop {
            let page = service
                .library_entries(
                    &actor,
                    BrowseQuery {
                        cursor,
                        revision: Some(summary.catalog_revision.clone()),
                        ..query.clone()
                    },
                )
                .unwrap();
            assert!(serde_json::to_vec(&page).unwrap().len() <= 8192);
            for row in page.items {
                assert!(ids.insert(row.id));
            }
            pages += 1;
            cursor = page.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        assert!(pages > 1);
        assert_eq!(ids.len(), 9);
        assert_eq!(
            lock(&store.db).unwrap().total_changes(),
            before_reads,
            "count/page/header reads must not maintain client refresh state"
        );
        let denied = Principal {
            device_id: None,
            user_id: UserId::new(),
            administrator: false,
        };
        assert!(matches!(
            service.library_summary(&denied, query.clone()),
            Err(Error::Forbidden)
        ));
        assert!(matches!(
            service.library_track_summary(&denied, saved.id),
            Err(Error::Forbidden)
        ));
        allow_details(&store);
        let mut changed = saved;
        changed.title = "Changed title".into();
        Catalog::save(&*store, changed.clone(), changed.revision).unwrap();
        assert!(matches!(
            service.library_entries(
                &actor,
                BrowseQuery {
                    revision: Some(summary.catalog_revision),
                    ..query
                }
            ),
            Err(Error::Conflict)
        ));
        assert!(
            service
                .library_track_summary(&actor, changed.id)
                .unwrap()
                .pending
        );
    }

    struct AdvancingClock(std::sync::atomic::AtomicI64);
    impl Clock for AdvancingClock {
        fn now(&self) -> chrono::DateTime<Utc> {
            chrono::DateTime::from_timestamp(self.0.load(std::sync::atomic::Ordering::SeqCst), 0)
                .unwrap()
        }
        fn monotonic_millis(&self) -> u64 {
            self.0.load(std::sync::atomic::Ordering::SeqCst) as u64 * 1000
        }
    }

    #[test]
    fn failed_summary_retries_survive_restart_without_starving_peers_or_new_changes() {
        use collector_app::LibrarySummaryState as State;
        let clock = Arc::new(AdvancingClock(std::sync::atomic::AtomicI64::new(
            1_790_000_000,
        )));
        let (root, store, actor, service, first) = fixture_with_clock(clock.clone());
        deny_details(&store);
        assert!(service.rebuild_next_library_entry().is_err());
        allow_details(&store);
        let mut other = first.clone();
        other.id = ItemId::new();
        other.title = "Unaffected peer".into();
        let peer = Catalog::save(&*store, other, Revision(0)).unwrap();
        assert!(
            service.rebuild_next_library_entry().unwrap(),
            "failed row starved peer"
        );
        assert!(
            !service.rebuild_next_library_entry().unwrap(),
            "failed row spun before retry time"
        );
        let query = BrowseQuery {
            collection_id: Some(first.collection_id),
            ..Default::default()
        };
        let rows = service.library_entries(&actor, query.clone()).unwrap();
        assert_eq!(
            rows.items
                .iter()
                .find(|r| r.id == first.id)
                .unwrap()
                .summary_state,
            State::Retrying
        );
        assert_eq!(
            rows.items
                .iter()
                .find(|r| r.id == peer.id)
                .unwrap()
                .summary_state,
            State::Ready
        );
        drop(service);
        drop(store);
        let store = Arc::new(
            Store::open_with_clock(
                root.path().join("archive"),
                root.path().join("state"),
                clock.clone(),
            )
            .unwrap(),
        );
        let service = CollectionService::new(
            store.clone(),
            Arc::new(collector_modules::Registry::builtins()),
        );
        assert!(
            !service.rebuild_next_library_entry().unwrap(),
            "restart erased backoff"
        );
        clock.0.fetch_add(6, std::sync::atomic::Ordering::SeqCst);
        assert!(service.rebuild_next_library_entry().unwrap());
        assert_eq!(
            service
                .library_entries(&actor, query.clone())
                .unwrap()
                .summary_state,
            State::Ready
        );
        let mut changed = Catalog::item(&*store, first.id).unwrap();
        let revision = changed.revision;
        changed.title = "First change".into();
        let mut changed = Catalog::save(&*store, changed, revision).unwrap();
        deny_details(&store);
        assert!(service.rebuild_next_library_entry().is_err());
        allow_details(&store);
        let revision = changed.revision;
        changed.title = "Newer change".into();
        Catalog::save(&*store, changed, revision).unwrap();
        assert!(
            service.rebuild_next_library_entry().unwrap(),
            "new generation inherited old failure delay"
        );
        let rows = service.library_entries(&actor, query).unwrap();
        assert_eq!(rows.summary_state, State::Ready);
        assert_eq!(
            rows.items.iter().find(|r| r.id == first.id).unwrap().title,
            "Newer change"
        );
    }

    #[test]
    fn background_build_covers_all_current_variants_without_eviction_churn() {
        use collector_app::{LibraryCarePort, UpdateMetadataPreferences};
        let (_root, store, actor, service, saved) = fixture();
        let mut users = Vec::new();
        for n in 0..13 {
            let user = UserId::new();
            lock(&store.db).unwrap().execute("INSERT INTO private.users(id,username,password_hash,administrator) SELECT ?1,?2,password_hash,0 FROM private.users WHERE id=?3",params![user.to_string(),format!("variant{n}"),actor.user_id.to_string()]).unwrap();
            store
                .grant(saved.collection_id, user, Role::Viewer)
                .unwrap();
            store
                .save_metadata_preferences(
                    user,
                    &UpdateMetadataPreferences {
                        expected_revision: Revision(0),
                        language: LanguageTag::new(format!("en-x-v{n}")).unwrap(),
                    },
                )
                .unwrap();
            users.push(user);
        }
        drain(&service);
        assert!(!service.rebuild_next_library_entry().unwrap());
        assert_eq!(
            lock(&store.db)
                .unwrap()
                .query_row(
                    "SELECT COUNT(*) FROM library_entries WHERE item_id=?",
                    [saved.id.to_string()],
                    |r| r.get::<_, usize>(0)
                )
                .unwrap(),
            14
        );
        store
            .save_metadata_preferences(
                users[0],
                &UpdateMetadataPreferences {
                    expected_revision: Revision(1),
                    language: LanguageTag::new("ja").unwrap(),
                },
            )
            .unwrap();
        assert!(service.rebuild_next_library_entry().unwrap());
        assert!(!service.rebuild_next_library_entry().unwrap());
        assert_eq!(
            lock(&store.db)
                .unwrap()
                .query_row(
                    "SELECT COUNT(*) FROM library_entries WHERE item_id=?",
                    [saved.id.to_string()],
                    |r| r.get::<_, usize>(0)
                )
                .unwrap(),
            14,
            "obsolete preference variant retained"
        );
    }
    #[test]
    fn failed_additive_install_rolls_back_partial_basic_rows() {
        let (_root, store, _actor, _service, saved) = fixture();
        let mut db = lock(&store.db).unwrap();
        db.execute_batch("DROP TABLE library_entry_basics; DROP TABLE library_entry_attempts;")
            .unwrap();
        // Fail the filter calculation after the basic record has been inserted.
        db.authorizer(Some(|context: AuthContext<'_>| match context.action {
            AuthAction::Read {
                table_name: "captures",
                ..
            } => Authorization::Deny,
            _ => Authorization::Allow,
        }))
        .unwrap();
        assert!(install(&db).is_err());
        db.authorizer(None::<fn(AuthContext<'_>) -> Authorization>)
            .unwrap();
        assert!(
            !db.query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='library_entry_basics')",
                [],
                |r| r.get::<_, bool>(0)
            )
            .unwrap()
        );
        assert!(
            db.query_row(
                "SELECT EXISTS(SELECT 1 FROM items WHERE id=?)",
                [saved.id.to_string()],
                |r| r.get::<_, bool>(0)
            )
            .unwrap()
        );
        install(&db).unwrap();
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM library_entry_basics", [], |r| r
                .get::<_, usize>(0))
                .unwrap(),
            1
        );
    }
    #[test]
    fn metadata_headers_and_rows_avoid_graph_reads_and_follow_inventory_changes() {
        let (_root, store, actor, service, saved) = fixture();
        let old = service
            .metadata_read_revision(&actor, saved.id)
            .unwrap()
            .revision;
        let mut changed = saved.clone();
        changed.title = "Changed inventory title".into();
        let changed = Catalog::save(&*store, changed, saved.revision).unwrap();
        assert_ne!(
            old,
            service
                .metadata_read_revision(&actor, saved.id)
                .unwrap()
                .revision
        );
        deny_details(&store);
        let header = service.metadata_read_revision(&actor, saved.id);
        let rows = service.metadata_care(
            &actor,
            collector_app::MetadataCareQuery {
                collection_id: Some(saved.collection_id),
                search: String::new(),
                reason: None,
                needs_attention: false,
                cursor: None,
                limit: Some(50),
            },
        );
        allow_details(&store);
        assert!(header.is_ok());
        assert_eq!(rows.unwrap().items[0].title, changed.title);
        assert_eq!(
            service
                .metadata_view(&actor, saved.id)
                .unwrap()
                .display
                .title
                .value,
            changed.title
        );
    }
    #[test]
    fn interactive_reader_contention_is_typed_and_recovers_when_a_reader_returns() {
        let (_root, store, actor, service, saved) = fixture();
        let mut readers = (0..read_pool::CONNECTIONS)
            .map(|_| store.read_db().unwrap())
            .collect::<Vec<_>>();
        assert!(matches!(
            service.metadata_read_revision(&actor, saved.id),
            Err(Error::CatalogBusy)
        ));
        drop(readers.pop());
        assert!(service.metadata_read_revision(&actor, saved.id).is_ok());
    }

    #[test]
    fn library_read_rejects_a_role_downgraded_during_its_snapshot() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let (root, store, _owner, service, saved) = fixture();
        let user = store
            .create_user("curator", "a long curator password", false)
            .unwrap();
        Catalog::grant(&*store, saved.collection_id, user, Role::Curator).unwrap();
        let actor = Principal {
            user_id: user,
            administrator: false,
            device_id: None,
        };
        let query = BrowseQuery {
            collection_id: Some(saved.collection_id),
            ..Default::default()
        };
        let changed = Arc::new(AtomicBool::new(false));
        store.readers.for_each(|db| {
            let changed = changed.clone();
            let private = Connection::open(root.path().join("state/private.sqlite")).unwrap();
            let collection = saved.collection_id;
            db.authorizer(Some(move |context: AuthContext<'_>| {
                if matches!(context.action, AuthAction::Read { table_name: "browse_items", .. })
                    && !changed.swap(true, Ordering::SeqCst)
                {
                    private.execute(
                        "UPDATE grants SET role=0,revision=revision+1 WHERE collection_id=? AND user_id=?",
                        params![collection.to_string(), user.to_string()],
                    ).unwrap();
                }
                Authorization::Allow
            })).unwrap();
        });
        assert!(matches!(
            service.library_entries(&actor, query.clone()),
            Err(Error::Conflict)
        ));
        assert!(changed.load(Ordering::SeqCst));
        allow_details(&store);
        let page = service.library_entries(&actor, query).unwrap();
        assert_eq!(page.items[0].id, saved.id);
        assert!(!page.items[0].can_edit);
    }

    #[test]
    fn visible_rows_and_cursors_survive_summary_writes_but_fence_membership_changes() {
        let (_root, store, actor, service, first) = fixture();
        let second = Catalog::save(
            &*store,
            Item {
                id: ItemId::new(),
                revision: Revision(0),
                title: "Second album".into(),
                ..first.clone()
            },
            Revision(0),
        )
        .unwrap();
        let query = BrowseQuery {
            collection_id: Some(first.collection_id),
            limit: Some(1),
            ..Default::default()
        };
        let old = service.library_entries(&actor, query.clone()).unwrap();
        assert_eq!(old.items[0].id, first.id);
        assert_eq!(
            old.items[0].summary_state,
            collector_app::LibrarySummaryState::Pending
        );
        drain(&service);
        let updated = service
            .refresh_library_rows(
                &actor,
                query.clone(),
                vec![first.id],
                &old.membership_revision,
            )
            .unwrap();
        assert_eq!(
            updated.items[0].summary_state,
            collector_app::LibrarySummaryState::Ready
        );
        assert_ne!(updated.catalog_revision, old.catalog_revision);
        let next = service
            .library_entries(
                &actor,
                BrowseQuery {
                    cursor: old.next_cursor.clone(),
                    ..query.clone()
                },
            )
            .unwrap();
        assert_eq!(next.items[0].id, second.id);
        Catalog::set_item_removed(&*store, second.id, second.revision, true).unwrap();
        assert!(matches!(
            service.refresh_library_rows(
                &actor,
                query.clone(),
                vec![first.id],
                &old.membership_revision
            ),
            Err(Error::Conflict)
        ));
        assert!(matches!(
            service.library_entries(
                &actor,
                BrowseQuery {
                    cursor: old.next_cursor,
                    ..query
                }
            ),
            Err(Error::Conflict)
        ));
    }
}
