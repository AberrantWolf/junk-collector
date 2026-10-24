//! Indexed, payload-free browse queries. Policy is described by the typed query
//! and application-owned sort fields; this adapter only projects and selects.
use super::*;
use collector_core::{BrowseFilter, BrowseSort};

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    scope: String,
    key: String,
    title: String,
    id: String,
}

pub(super) fn project(db: &Connection, item: &Item) -> Result<()> {
    let (artist, platform) = collector_app::browse_sort_fields(item);
    let missing_last = |value: String| {
        if value.trim().is_empty() {
            "\u{10ffff}".into()
        } else {
            value.to_lowercase()
        }
    };
    db.execute("INSERT INTO browse_items VALUES (?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET title_key=excluded.title_key,artist_key=excluded.artist_key,platform_key=excluded.platform_key", params![item.id.to_string(),item.collection_id.to_string(),item.title.to_lowercase(),missing_last(artist),missing_last(platform),recent_key(item.updated_at)]).map_err(storage)?;
    library_care::project_preparation(db, item.id)?;
    library_care::project_health(db, item)
}
fn recent_key(time: chrono::DateTime<chrono::Utc>) -> String {
    // Fixed width gives newest-first lexicographic ordering without offsets.
    format!(
        "{:020}",
        i128::from(i64::MAX) - i128::from(time.timestamp_millis())
    )
}
pub(super) fn set_added(
    db: &Connection,
    id: ItemId,
    time: chrono::DateTime<chrono::Utc>,
) -> Result<()> {
    db.execute(
        "UPDATE browse_items SET recent_key=? WHERE id=?",
        params![recent_key(time), id.to_string()],
    )
    .map_err(storage)?;
    Ok(())
}
/// Rebuild creation order from the earliest retained, digest-checked revision,
/// never filesystem mtime or the latest edit. Reads metadata only at reindex.
pub(super) fn first_record_time(
    path: &Path,
    collection: CollectionId,
) -> Result<chrono::DateTime<chrono::Utc>> {
    let mut revisions = directories(&path.join("revisions"))?;
    revisions.sort();
    let first = revisions
        .first()
        .ok_or_else(|| Error::Storage("item has no revisions".into()))?;
    if first.join("manifest.v3.toml").exists() {
        return Ok(migration_v3::read_item(first, collection)?.0.updated_at);
    }
    let suffix = if first.join("manifest.v2.toml").exists() {
        ".v2"
    } else {
        ""
    };
    let manifest: Manifest = toml::from_str(
        &fs::read_to_string(first.join(format!("manifest{suffix}.toml"))).map_err(storage)?,
    )
    .map_err(storage)?;
    let bytes = fs::read(first.join(format!("record{suffix}.json"))).map_err(storage)?;
    if Digest::sha256(&bytes).hex != manifest.record_sha256 || manifest.collection_id != collection
    {
        return Err(Error::Storage(
            "earliest item revision digest/collection mismatch".into(),
        ));
    }
    let record: RevisionRecord = serde_json::from_slice(&bytes).map_err(storage)?;
    if record.item.id != manifest.item_id || record.item.revision != manifest.revision {
        return Err(Error::Storage(
            "earliest item revision identity mismatch".into(),
        ));
    }
    Ok(record.item.updated_at)
}

pub(super) fn browse(db: &Connection, user: UserId, query: BrowseQuery) -> Result<Page<Item>> {
    let limit = query.limit.unwrap_or(50).clamp(1, 200) as usize;
    let search = query.search.as_deref().unwrap_or("").trim();
    if search.len() > 256 {
        return Err(Error::Invalid("search is too long".into()));
    }
    let scope = Digest::sha256(
        &serde_json::to_vec(&(
            user,
            query.collection_id,
            search,
            query.sort,
            query.filter,
            query.listening_only,
        ))
        .map_err(storage)?,
    )
    .hex;
    let cursor = match query.cursor {
        Some(raw) if raw.len() <= 32768 => {
            let bytes =
                hex::decode(raw).map_err(|_| Error::Invalid("invalid browse cursor".into()))?;
            let value: Cursor = serde_json::from_slice(&bytes)
                .map_err(|_| Error::Invalid("invalid browse cursor".into()))?;
            if value.scope != scope || value.id.parse::<ItemId>().is_err() {
                return Err(Error::Invalid(
                    "browse cursor belongs to different results".into(),
                ));
            }
            Some(value)
        }
        Some(_) => return Err(Error::Invalid("browse cursor is too large".into())),
        None => None,
    };
    let key = match query.sort {
        BrowseSort::Title => "b.title_key",
        BrowseSort::RecentlyAdded => "b.recent_key",
        BrowseSort::Artist => "b.artist_key",
        BrowseSort::Platform => "b.platform_key",
    };
    // These filters select committed records, never inspect source payloads.
    // Attention is deliberately requester-scoped: it cannot reveal private jobs.
    let filter = match query.filter {
        BrowseFilter::All => "1",
        BrowseFilter::Wishlist => "json_extract(i.record,'$.wishlist')=1",
        BrowseFilter::FilesAttached => {
            "EXISTS(SELECT 1 FROM captures c, json_each(c.record,'$.representations') r, json_each(r.value,'$.files') f WHERE c.item_id=i.id)"
        }
        BrowseFilter::Playable => {
            "EXISTS(SELECT 1 FROM captures c, json_each(c.record,'$.representations') r, json_each(r.value,'$.files') f WHERE c.item_id=i.id AND ((json_extract(r.value,'$.role')='preservation_master' AND (lower(json_extract(f.value,'$.name')) GLOB '*.flac' OR lower(json_extract(f.value,'$.name')) GLOB '*.mp3' OR lower(json_extract(f.value,'$.name')) GLOB '*.wav' OR lower(json_extract(f.value,'$.name')) GLOB '*.opus' OR lower(json_extract(f.value,'$.name')) GLOB '*.ogg')) OR (json_extract(r.value,'$.source') IS NOT NULL AND json_extract(r.value,'$.profile.codec') IN ('flac','opus') AND EXISTS(SELECT 1 FROM conversion_recipes cr WHERE cr.id=json_extract(r.value,'$.id') AND cr.collection_id=i.collection_id AND json_array_length(cr.record,'$.tracks')=1))))"
        }
        BrowseFilter::NeedsAttention => {
            "(EXISTS(SELECT 1 FROM private.jobs j WHERE j.item_id=i.id AND j.user_id=?1 AND j.state='failed') OR EXISTS(SELECT 1 FROM private.imports p WHERE p.item_id=i.id AND p.user_id=?1 AND p.phase='awaiting_review'))"
        }
        // The application evaluates the shared album resolver over these candidates.
        BrowseFilter::AlbumPlaybackNeedsAttention => {
            "json_extract(i.record,'$.domain.module')='org.junk.music'"
        }
        BrowseFilter::Removed => "1",
    };
    let terms = search
        .split_whitespace()
        .map(|word| format!("\"{}\"*", word.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" AND ");
    let fts = if terms.is_empty() {
        ""
    } else {
        " AND i.id IN (SELECT id FROM search WHERE search MATCH ?7)"
    };
    let listening = if query.listening_only {
        "(json_extract(i.record,'$.domain.module')='org.junk.music' OR EXISTS(SELECT 1 FROM music_search m WHERE m.item_id=i.id) OR EXISTS(SELECT 1 FROM captures c, json_each(c.record,'$.representations') r WHERE c.item_id=i.id AND EXISTS(SELECT 1 FROM conversion_recipes recipe WHERE recipe.id=json_extract(r.value,'$.id'))))"
    } else {
        "1"
    };
    let sql = format!(
        "SELECT i.record,{key},b.title_key FROM browse_items b JOIN items i ON i.id=b.id JOIN private.grants g ON g.collection_id=i.collection_id AND g.user_id=?1 WHERE i.deleted={} AND {} AND (?2 IS NULL OR i.collection_id=?2) AND ({filter}) AND ({listening}) AND (?8=0 OR ({key},b.title_key,i.id)>(?3,?4,?5)){fts} ORDER BY {key},b.title_key,i.id LIMIT ?6",
        i64::from(query.filter == BrowseFilter::Removed),
        if query.filter == BrowseFilter::Removed {
            "g.role>=1"
        } else {
            "1"
        }
    );
    let mut stmt = db.prepare(&sql).map_err(storage)?;
    let mut rows = stmt
        .query(params![
            user.to_string(),
            query.collection_id.map(|v| v.to_string()),
            cursor.as_ref().map(|v| v.key.as_str()).unwrap_or(""),
            cursor.as_ref().map(|v| v.title.as_str()).unwrap_or(""),
            cursor.as_ref().map(|v| v.id.as_str()).unwrap_or(""),
            (limit + 1) as i64,
            terms,
            cursor.is_some()
        ])
        .map_err(storage)?;
    let mut items = Vec::new();
    let mut next = None;
    let mut has_more = false;
    while let Some(row) = rows.next().map_err(storage)? {
        if items.len() == limit {
            has_more = true;
            break;
        }
        let item: Item =
            serde_json::from_str(&row.get::<_, String>(0).map_err(storage)?).map_err(storage)?;
        next = Some(Cursor {
            scope: scope.clone(),
            key: row.get(1).map_err(storage)?,
            title: row.get(2).map_err(storage)?,
            id: item.id.to_string(),
        });
        items.push(item);
    }
    // The loop consumes the lookahead row only when the page is full.
    let next_cursor = if has_more {
        next.map(|v| serde_json::to_vec(&v).map(hex::encode).map_err(storage))
            .transpose()?
    } else {
        None
    };
    drop(rows);
    drop(stmt);
    for item in &mut items {
        hydrate_captures(db, item)?;
    }
    Ok(Page { items, next_cursor })
}
