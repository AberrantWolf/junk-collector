//! Mechanisms for private library maintenance state and indexed queries.
use super::*;
use collector_app::{
    LibraryCarePort, MetadataCareQuery, MetadataPreferences, UpdateMetadataPreferences,
};

pub(crate) fn install(db: &Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS metadata_health(item_id TEXT PRIMARY KEY,collection_id TEXT NOT NULL,title_key TEXT NOT NULL,missing_name INTEGER NOT NULL,missing_artwork INTEGER NOT NULL,album_ambiguity INTEGER NOT NULL,incomplete_provider_information INTEGER NOT NULL,no_match INTEGER NOT NULL,manual_conflict INTEGER NOT NULL,needs_attention INTEGER NOT NULL);
    CREATE TABLE IF NOT EXISTS album_group_members(item_id TEXT PRIMARY KEY,collection_id TEXT NOT NULL,album_id TEXT NOT NULL);
    CREATE INDEX IF NOT EXISTS album_group_matches ON album_group_members(collection_id,album_id,item_id);
    CREATE TABLE IF NOT EXISTS preparation_readiness(item_id TEXT PRIMARY KEY,collection_id TEXT NOT NULL,total_tracks INTEGER NOT NULL,ready_tracks INTEGER NOT NULL,source_count INTEGER NOT NULL,needs_attention INTEGER NOT NULL);
    CREATE INDEX IF NOT EXISTS preparation_readiness_attention ON preparation_readiness(needs_attention,collection_id,item_id);
    CREATE TABLE IF NOT EXISTS metadata_language_display(item_id TEXT NOT NULL,language TEXT NOT NULL,title TEXT NOT NULL,subtitle TEXT NOT NULL,title_key TEXT NOT NULL,PRIMARY KEY(item_id,language));
    CREATE INDEX IF NOT EXISTS metadata_language_order ON metadata_language_display(language,title_key,item_id);
    CREATE INDEX IF NOT EXISTS metadata_health_attention ON metadata_health(needs_attention,collection_id,title_key,item_id);
    CREATE INDEX IF NOT EXISTS metadata_health_collection ON metadata_health(collection_id,title_key,item_id);").map_err(storage)?;
    db.execute_batch("CREATE TABLE IF NOT EXISTS private.metadata_preferences(user_id TEXT PRIMARY KEY REFERENCES users(id),revision INTEGER NOT NULL,language TEXT NOT NULL);").map_err(storage)
}
impl LibraryCarePort for Store {
    fn metadata_resolution_jobs(
        &self,
        user: UserId,
        item: ItemId,
    ) -> Result<Vec<collector_app::JobRecord>> {
        let ids = {
            let db = lock(&self.db)?;
            let mut stmt=db.prepare("SELECT id FROM private.jobs WHERE user_id=? AND item_id=? AND kind='metadata_resolution' ORDER BY CASE WHEN state IN ('queued','running','cancel_requested') THEN 0 ELSE 1 END,created_at DESC,id DESC LIMIT 25").map_err(storage)?;
            stmt.query_map(params![user.to_string(), item.to_string()], |row| {
                row.get::<_, String>(0)
            })
            .map_err(storage)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(storage)?
        };
        ids.into_iter()
            .map(|id| self.job_record(id.parse().map_err(storage)?, user))
            .collect()
    }
    fn metadata_care_items(&self, user: UserId, query: &MetadataCareQuery) -> Result<Page<Item>> {
        let limit = query.limit.unwrap_or(50);
        if !(1..=200).contains(&limit) || query.search.len() > 256 {
            return Err(Error::Invalid(
                "metadata query must use a 1–200 entry page and at most 256 search characters"
                    .into(),
            ));
        }
        let language = self
            .metadata_preferences(user)?
            .language
            .as_str()
            .split('-')
            .next()
            .unwrap_or("en")
            .to_ascii_lowercase();
        let scope = Digest::sha256(
            &serde_json::to_vec(&(
                user,
                &language,
                query.collection_id,
                &query.search,
                query.reason,
                query.needs_attention,
            ))
            .map_err(storage)?,
        )
        .hex;
        let cursor = if let Some(raw) = &query.cursor {
            if raw.len() > 32768 {
                return Err(Error::Invalid("metadata cursor is too large".into()));
            }
            let (saved, title, id): (String, String, String) = serde_json::from_slice(
                &hex::decode(raw).map_err(|_| Error::Invalid("invalid metadata cursor".into()))?,
            )
            .map_err(|_| Error::Invalid("invalid metadata cursor".into()))?;
            if saved != scope || id.parse::<ItemId>().is_err() {
                return Err(Error::Invalid(
                    "metadata cursor belongs to different results".into(),
                ));
            }
            Some((title, id))
        } else {
            None
        };
        let failure = "EXISTS(SELECT 1 FROM private.jobs j WHERE j.item_id=i.id AND j.user_id=?1 AND j.kind='metadata_resolution' AND j.state='failed' AND NOT EXISTS(SELECT 1 FROM private.jobs later WHERE later.item_id=j.item_id AND later.user_id=j.user_id AND later.kind=j.kind AND (later.created_at,later.id)>(j.created_at,j.id)))";
        let grouping = "EXISTS(SELECT 1 FROM album_group_members own JOIN album_group_members peer ON peer.collection_id=own.collection_id AND peer.album_id=own.album_id AND peer.item_id<>own.item_id JOIN items sibling ON sibling.id=peer.item_id AND sibling.deleted=0 WHERE own.item_id=i.id)";
        let reason = match query.reason {
            Some(collector_app::MetadataReviewReason::GroupingProposal) => grouping,
            Some(collector_app::MetadataReviewReason::MissingName) => "h.missing_name=1",
            Some(collector_app::MetadataReviewReason::MissingArtwork) => "h.missing_artwork=1",
            Some(collector_app::MetadataReviewReason::AlbumAmbiguity) => "h.album_ambiguity=1",
            Some(collector_app::MetadataReviewReason::IncompleteProviderInformation) => {
                "h.incomplete_provider_information=1"
            }
            Some(collector_app::MetadataReviewReason::NoMatch) => "h.no_match=1",
            Some(collector_app::MetadataReviewReason::ManualConflict) => "h.manual_conflict=1",
            Some(collector_app::MetadataReviewReason::LookupFailed) => failure,
            None => "1",
        };
        let db = lock(&self.db)?;
        let sql = format!(
            "SELECT i.record,COALESCE(d.title_key,h.title_key) FROM metadata_health h JOIN items i ON i.id=h.item_id LEFT JOIN metadata_language_display d ON d.item_id=i.id AND d.language=?9 JOIN private.grants g ON g.collection_id=h.collection_id AND g.user_id=?1 AND g.role>=1 WHERE i.deleted=0 AND (?2 IS NULL OR h.collection_id=?2) AND (?3=0 OR h.needs_attention=1 OR {failure} OR {grouping}) AND ({reason}) AND (?4='' OR instr(lower(COALESCE(d.title,i.title)||' '||COALESCE(d.subtitle,i.subtitle)),lower(?4))>0) AND (?5=0 OR (COALESCE(d.title_key,h.title_key),i.id)>(?6,?7)) ORDER BY COALESCE(d.title_key,h.title_key),i.id LIMIT ?8"
        );
        let mut stmt = db.prepare(&sql).map_err(storage)?;
        let mut rows = stmt
            .query(params![
                user.to_string(),
                query.collection_id.map(|id| id.to_string()),
                query.needs_attention,
                query.search,
                cursor.is_some(),
                cursor.as_ref().map(|value| value.0.as_str()).unwrap_or(""),
                cursor.as_ref().map(|value| value.1.as_str()).unwrap_or(""),
                u32::from(limit) + 1,
                language
            ])
            .map_err(storage)?;
        let mut items = Vec::new();
        let mut last = None;
        let mut more = false;
        while let Some(row) = rows.next().map_err(storage)? {
            if items.len() == usize::from(limit) {
                more = true;
                break;
            }
            let item: Item = serde_json::from_str(&row.get::<_, String>(0).map_err(storage)?)
                .map_err(storage)?;
            last = Some((
                scope.clone(),
                row.get::<_, String>(1).map_err(storage)?,
                item.id.to_string(),
            ));
            items.push(item);
        }
        let next_cursor = if more {
            last.map(|value| serde_json::to_vec(&value).map(hex::encode).map_err(storage))
                .transpose()?
        } else {
            None
        };
        Ok(Page { items, next_cursor })
    }

    fn album_group_proposals(
        &self,
        user: UserId,
        item: ItemId,
        after: Option<ItemId>,
        page_limit: u16,
    ) -> Result<Page<Item>> {
        if !(1..=200).contains(&page_limit) {
            return Err(Error::Invalid("Use a 1–200 proposal page.".into()));
        }
        let db = lock(&self.db)?;
        let mut stmt=db.prepare("SELECT i.record FROM album_group_members own JOIN album_group_members peer ON peer.collection_id=own.collection_id AND peer.album_id=own.album_id AND peer.item_id<>own.item_id JOIN items i ON i.id=peer.item_id JOIN private.grants g ON g.user_id=?1 AND g.collection_id=i.collection_id AND g.role>=1 WHERE own.item_id=?2 AND i.deleted=0 AND (?3 IS NULL OR i.id>?3) ORDER BY i.id LIMIT ?4").map_err(storage)?;
        let mut items = stmt
            .query_map(
                params![
                    user.to_string(),
                    item.to_string(),
                    after.map(|v| v.to_string()),
                    u32::from(page_limit) + 1
                ],
                |r| r.get::<_, String>(0),
            )
            .map_err(storage)?
            .map(|raw| serde_json::from_str::<Item>(&raw.map_err(storage)?).map_err(storage))
            .collect::<Result<Vec<_>>>()?;
        let next_cursor = if items.len() > usize::from(page_limit) {
            items.pop();
            items.last().map(|item| item.id.to_string())
        } else {
            None
        };
        Ok(Page { items, next_cursor })
    }
    fn preparation_care_items(
        &self,
        user: UserId,
        query: &MetadataCareQuery,
    ) -> Result<Page<Item>> {
        let page_limit = query.limit.unwrap_or(50);
        if !(1..=200).contains(&page_limit) || query.search.len() > 256 {
            return Err(Error::Invalid(
                "Use a 1–200 entry page and a search of at most 256 characters.".into(),
            ));
        }
        let language = self
            .metadata_preferences(user)?
            .language
            .as_str()
            .split('-')
            .next()
            .unwrap_or("en")
            .to_ascii_lowercase();
        let scope = Digest::sha256(
            &serde_json::to_vec(&(
                user,
                query.collection_id,
                &query.search,
                query.needs_attention,
                "preparation",
                &language,
            ))
            .map_err(storage)?,
        )
        .hex;
        let cursor = if let Some(raw) = &query.cursor {
            if raw.len() > 32768 {
                return Err(Error::Invalid("preparation cursor too large".into()));
            }
            let (saved, title, id): (String, String, String) =
                serde_json::from_slice(&hex::decode(raw).map_err(storage)?).map_err(storage)?;
            if saved != scope {
                return Err(Error::Invalid(
                    "This cursor belongs to different preparation results.".into(),
                ));
            }
            Some((title, id))
        } else {
            None
        };
        let db = lock(&self.db)?;
        let mut stmt=db.prepare("SELECT i.record,COALESCE(d.title_key,b.title_key) FROM preparation_readiness r JOIN items i ON i.id=r.item_id JOIN browse_items b ON b.id=i.id LEFT JOIN metadata_language_display d ON d.item_id=i.id AND d.language=?9 JOIN private.grants g ON g.collection_id=i.collection_id AND g.user_id=?1 AND g.role>=1 JOIN private.users u ON u.id=g.user_id AND u.enabled=1 WHERE i.deleted=0 AND (?2 IS NULL OR i.collection_id=?2) AND (?3=0 OR r.needs_attention=1) AND (?4='' OR instr(lower(COALESCE(d.title,i.title)||' '||COALESCE(d.subtitle,i.subtitle)),lower(?4))>0) AND (?5=0 OR (COALESCE(d.title_key,b.title_key),i.id)>(?6,?7)) ORDER BY COALESCE(d.title_key,b.title_key),i.id LIMIT ?8").map_err(storage)?;
        let rows = stmt
            .query_map(
                params![
                    user.to_string(),
                    query.collection_id.map(|v| v.to_string()),
                    query.needs_attention,
                    query.search,
                    cursor.is_some(),
                    cursor.as_ref().map(|v| v.0.as_str()).unwrap_or(""),
                    cursor.as_ref().map(|v| v.1.as_str()).unwrap_or(""),
                    u32::from(page_limit) + 1,
                    language
                ],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .map_err(storage)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(storage)?;
        let more = rows.len() > usize::from(page_limit);
        let mut items = Vec::new();
        let mut last = None;
        for (raw, title) in rows.into_iter().take(usize::from(page_limit)) {
            let item: Item = serde_json::from_str(&raw).map_err(storage)?;
            last = Some((scope.clone(), title, item.id.to_string()));
            items.push(item);
        }
        let next_cursor = if more {
            last.map(|v| serde_json::to_vec(&v).map(hex::encode).map_err(storage))
                .transpose()?
        } else {
            None
        };
        Ok(Page { items, next_cursor })
    }
    fn metadata_preferences(&self, user: UserId) -> Result<MetadataPreferences> {
        let db = lock(&self.db)?;
        let exists: bool = db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM private.users WHERE id=? AND enabled=1)",
                [user.to_string()],
                |row| row.get(0),
            )
            .map_err(storage)?;
        if !exists {
            return Err(Error::Unauthenticated);
        }
        let value: Option<(u64, String)> = db
            .query_row(
                "SELECT revision,language FROM private.metadata_preferences WHERE user_id=?",
                [user.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(storage)?;
        let (revision, language) = value.unwrap_or((0, "en".into()));
        Ok(MetadataPreferences {
            revision: Revision(revision),
            language: LanguageTag::new(language)?,
        })
    }
    fn save_metadata_preferences(
        &self,
        user: UserId,
        command: &UpdateMetadataPreferences,
    ) -> Result<MetadataPreferences> {
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        let revision: u64 = tx
            .query_row(
                "SELECT revision FROM private.metadata_preferences WHERE user_id=?",
                [user.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?
            .unwrap_or(0);
        if Revision(revision) != command.expected_revision {
            return Err(Error::Conflict);
        }
        let result = MetadataPreferences {
            revision: Revision(revision.checked_add(1).ok_or(Error::Conflict)?),
            language: LanguageTag::new(command.language.as_str())?,
        };
        tx.execute("INSERT INTO private.metadata_preferences VALUES(?,?,?) ON CONFLICT(user_id) DO UPDATE SET revision=excluded.revision,language=excluded.language",params![user.to_string(),result.revision.0,result.language.as_str()]).map_err(storage)?;
        self.record_command(&tx, &result)?;
        tx.commit().map_err(storage)?;
        Ok(result)
    }
}

pub(crate) fn project_health(db: &Connection, item: &Item) -> Result<()> {
    let authority: Option<String> = db
        .query_row(
            "SELECT record FROM metadata_authorities WHERE id=?",
            [item.id.to_string()],
            |row| row.get(0),
        )
        .optional()
        .map_err(storage)?;
    let record = authority
        .map(|raw| {
            serde_json::from_str::<collector_app::MetadataAuthorityRecord>(&raw).map_err(storage)
        })
        .transpose()?;
    db.execute(
        "DELETE FROM album_group_members WHERE item_id=?",
        [item.id.to_string()],
    )
    .map_err(storage)?;
    let mut missing_name = item.title.trim().is_empty();
    let mut missing_artwork = !item
        .assets
        .iter()
        .any(|asset| asset.kind == AssetKind::Cover);
    let mut ambiguity = false;
    let mut incomplete = false;
    let mut no_match = false;
    let mut conflict = false;
    db.execute(
        "DELETE FROM metadata_language_display WHERE item_id=?",
        [item.id.to_string()],
    )
    .map_err(storage)?;
    if let Some(record) = record {
        let authority = collector_app::metadata_authority_for_inventory(item, &record)?;
        let mut statement = db.prepare("SELECT record FROM metadata_candidates WHERE item_id=? ORDER BY candidate_key LIMIT 4097").map_err(storage)?;
        let history = statement
            .query_map([item.id.to_string()], |row| row.get::<_, String>(0))
            .map_err(storage)?
            .map(|row| {
                serde_json::from_str::<MetadataCandidateObservation>(&row.map_err(storage)?)
                    .map_err(storage)
            })
            .collect::<Result<Vec<_>>>()?;
        let candidates = history
            .iter()
            .filter(|value| {
                record
                    .current_candidates
                    .as_ref()
                    .is_none_or(|current| current.contains(&value.identity))
            })
            .cloned()
            .collect::<Vec<_>>();
        if !item.deleted
            && let Some(album) = collector_app::accepted_album_group_key(item, &record, &history)?
        {
            db.execute(
                "INSERT INTO album_group_members VALUES(?,?,?)",
                params![item.id.to_string(), item.collection_id.to_string(), album],
            )
            .map_err(storage)?;
        }
        let display = collector_app::resolve_record_metadata(item, &record, &history)?;
        let mut languages = std::collections::BTreeSet::from(["en".to_string()]);
        for language in history
            .iter()
            .filter_map(|candidate| candidate.title.language.as_ref())
        {
            languages.insert(
                language
                    .as_str()
                    .split('-')
                    .next()
                    .unwrap_or("en")
                    .to_ascii_lowercase(),
            );
        }
        if let Some(language) = &authority.automatic_language {
            languages.insert(
                language
                    .as_str()
                    .split('-')
                    .next()
                    .unwrap_or("en")
                    .to_ascii_lowercase(),
            );
        }
        for language in languages {
            let mut localized = record.clone();
            localized.authority.automatic_language = Some(LanguageTag::new(&language)?);
            let display = collector_app::resolve_record_metadata(item, &localized, &history)?;
            db.execute(
                "INSERT INTO metadata_language_display VALUES(?,?,?,?,?)",
                params![
                    item.id.to_string(),
                    language,
                    display.title.value,
                    display
                        .subtitle
                        .map(|value| value.value)
                        .unwrap_or_default(),
                    display.title.value.to_lowercase()
                ],
            )
            .map_err(storage)?;
        }

        let (proposed, uncertain) = collector_app::automatic_metadata_candidate(
            &candidates,
            authority
                .automatic_language
                .as_ref()
                .map_or("en", LanguageTag::as_str),
        );
        ambiguity = uncertain == Some(collector_app::MetadataReviewReason::AlbumAmbiguity);
        incomplete =
            uncertain == Some(collector_app::MetadataReviewReason::IncompleteProviderInformation);
        missing_name = display.candidate.is_none()
            && !authority
                .field_overrides
                .iter()
                .any(|field| field.field == MetadataField::Title);
        missing_artwork = missing_artwork && display.artwork.is_none();
        no_match = record
            .observations
            .last()
            .is_some_and(|value| value.outcome == MetadataRefreshOutcome::NoMatch);
        conflict = proposed.is_some_and(|candidate| {
            authority
                .field_overrides
                .iter()
                .any(|field| match field.field {
                    MetadataField::Title => field.value != candidate.title.value,
                    MetadataField::Subtitle => {
                        Some(field.value.as_str())
                            != candidate
                                .subtitle
                                .as_ref()
                                .map(|value| value.value.as_str())
                    }
                })
        });
    }
    db.execute("INSERT INTO metadata_health VALUES(?,?,?,?,?,?,?,?,?,?) ON CONFLICT(item_id) DO UPDATE SET collection_id=excluded.collection_id,title_key=excluded.title_key,missing_name=excluded.missing_name,missing_artwork=excluded.missing_artwork,album_ambiguity=excluded.album_ambiguity,incomplete_provider_information=excluded.incomplete_provider_information,no_match=excluded.no_match,manual_conflict=excluded.manual_conflict,needs_attention=excluded.needs_attention",params![item.id.to_string(),item.collection_id.to_string(),item.title.to_lowercase(),missing_name,missing_artwork,ambiguity,incomplete,no_match,conflict,missing_name||missing_artwork||ambiguity||incomplete||no_match||conflict]).map_err(storage)?;
    Ok(())
}

/// Rebuildable summary from immutable observations and recipes, without opening media.
pub(crate) fn project_preparation(db: &Connection, item_id: ItemId) -> Result<()> {
    let raw: Option<String> = db
        .query_row(
            "SELECT record FROM items WHERE id=?",
            [item_id.to_string()],
            |r| r.get(0),
        )
        .optional()
        .map_err(storage)?;
    let Some(raw) = raw else {
        return Ok(());
    };
    let item: Item = serde_json::from_str(&raw).map_err(storage)?;
    if item.domain.module.0 != "org.junk.music" || item.deleted {
        db.execute(
            "DELETE FROM preparation_readiness WHERE item_id=?",
            [item_id.to_string()],
        )
        .map_err(storage)?;
        return Ok(());
    }
    let mut stmt = db
        .prepare("SELECT record FROM captures WHERE item_id=? ORDER BY id")
        .map_err(storage)?;
    let captures = stmt
        .query_map([item_id.to_string()], |r| r.get::<_, String>(0))
        .map_err(storage)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(storage)?;
    drop(stmt);
    let mut discs = std::collections::BTreeMap::<String, (u32, u32)>::new();
    let mut sources = 0u32;
    for raw in captures {
        let capture: Capture = serde_json::from_str(&raw).map_err(storage)?;
        if item.lifecycle.historical_captures.contains(&capture.id) {
            continue;
        }
        for source in capture
            .representations
            .iter()
            .filter(|r| r.role == RepresentationRole::PreservationMaster)
        {
            sources += 1;
            let mut stmt=db.prepare("SELECT record FROM processing_observations WHERE source_scope='representation' AND source_id=? AND operation IN ('org.junk.music.cd-structure','org.junk.music.import-analysis') ORDER BY created_at DESC,id DESC").map_err(storage)?;
            let mut rows = stmt.query([source.id.to_string()]).map_err(storage)?;
            let mut layout = None;
            while let Some(row) = rows.next().map_err(storage)? {
                let record: collector_app::PortableProcessingRecord =
                    serde_json::from_str(&row.get::<_, String>(0).map_err(storage)?)
                        .map_err(storage)?;
                let fingerprint = &record.observation.source_fingerprint;
                if fingerprint.files.len() != source.files.len()
                    || fingerprint
                        .files
                        .iter()
                        .zip(&source.files)
                        .any(|(f, s)| f.sha256 != s.digest || f.bytes != s.bytes)
                {
                    continue;
                }
                if let Some((key, total)) =
                    collector_app::preparation_layout_projection(&record.observation, source)?
                {
                    layout = Some((key, total, fingerprint.clone()));
                    break;
                }
            }
            drop(rows);
            drop(stmt);
            let Some((key, total, fingerprint)) = layout else {
                continue;
            };
            let mut ready = std::collections::BTreeSet::new();
            for representation in capture.representations.iter().filter(|r| {
                r.source == Some(source.id) && r.profile == Some(ConversionProfile::Flac)
            }) {
                let recipe: Option<String> = db
                    .query_row(
                        "SELECT record FROM conversion_recipes WHERE id=?",
                        [representation.id.to_string()],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(storage)?;
                if let Some(raw) = recipe {
                    let recipe: collector_core::ConversionRecipe =
                        serde_json::from_str(&raw).map_err(storage)?;
                    if recipe.source_fingerprint == fingerprint {
                        for track in recipe.tracks {
                            ready.insert(track.position);
                        }
                    }
                }
            }
            let counts = discs.entry(key).or_default();
            counts.0 = counts.0.max(u32::from(total));
            counts.1 = counts.1.max(ready.len() as u32);
        }
    }
    let (total, ready) = discs
        .values()
        .fold((0u32, 0u32), |(t, r), (dt, dr)| (t + dt, r + dr));
    db.execute("INSERT INTO preparation_readiness VALUES(?,?,?,?,?,?) ON CONFLICT(item_id) DO UPDATE SET collection_id=excluded.collection_id,total_tracks=excluded.total_tracks,ready_tracks=excluded.ready_tracks,source_count=excluded.source_count,needs_attention=excluded.needs_attention",params![item_id.to_string(),item.collection_id.to_string(),total,ready,sources,total==0||ready<total]).map_err(storage)?;
    Ok(())
}
