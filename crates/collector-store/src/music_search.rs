//! Metadata-only song projection and bounded literal-substring queries.
use super::*;
use collector_app::{MusicSearchQuery, MusicSearchRecord};
#[derive(Serialize, Deserialize)]
struct Cursor {
    scope: String,
    title: String,
    id: String,
    disc: u16,
    track: u16,
}
pub(super) fn install(db: &Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS music_search(item_id TEXT NOT NULL,collection_id TEXT NOT NULL,disc INTEGER NOT NULL,track INTEGER NOT NULL,title_key TEXT NOT NULL,search_key TEXT NOT NULL,record TEXT NOT NULL,PRIMARY KEY(item_id,disc,track)); CREATE INDEX IF NOT EXISTS music_search_order ON music_search(collection_id,title_key,item_id,disc,track);").map_err(storage)
}
pub(super) fn project(
    db: &Connection,
    item: &Item,
    display: Option<&ResolvedMetadataDisplay>,
    candidates: &[MetadataCandidateObservation],
    artist: Option<&str>,
) -> Result<()> {
    db.execute(
        "DELETE FROM music_search WHERE item_id=?",
        [item.id.to_string()],
    )
    .map_err(storage)?;
    let mut songs = collector_app::music_search_projection(item, display, candidates, artist);
    if item.domain.module.0 == "org.junk.music" && !item.deleted {
        let mut hydrated = item.clone();
        hydrate_captures(db, &mut hydrated)?;
        let mut observations = Vec::new();
        for representation in hydrated
            .captures
            .iter()
            .flat_map(|v| &v.representations)
            .filter(|v| v.role == RepresentationRole::PreservationMaster)
        {
            let fingerprint = processing::representation_fingerprint(representation)?;
            for operation in [
                "org.junk.music.cd-structure",
                "org.junk.music.import-analysis",
            ] {
                let raw:Option<String>=db.query_row("SELECT record FROM processing_observations WHERE collection_id=? AND source_scope='representation' AND source_id=? AND operation=? AND json_extract(record,'$.observation.source_fingerprint.digest.hex')=? ORDER BY created_at DESC,id DESC LIMIT 1",params![item.collection_id.to_string(),representation.id.to_string(),operation,fingerprint.digest.hex],|r|r.get(0)).optional().map_err(storage)?;
                if let Some(raw) = raw {
                    let record: collector_app::PortableProcessingRecord =
                        serde_json::from_str(&raw).map_err(storage)?;
                    if record.observation.source_fingerprint == fingerprint {
                        observations.push(record.observation);
                    }
                }
            }
        }
        let artist_override = artist;
        let (album, artist) = collector_app::music_search_labels(item, display, artist);
        let authority: Option<collector_app::MetadataAuthorityRecord> = db
            .query_row(
                "SELECT record FROM metadata_authorities WHERE id=?",
                [item.id.to_string()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(storage)?
            .map(|raw| serde_json::from_str(&raw).map_err(storage))
            .transpose()?;
        let file_songs = collector_app::file_music_search_projection(
            &hydrated,
            &observations,
            &album,
            &artist,
            artist_override,
            authority.as_ref(),
        )?;
        if !file_songs.is_empty() {
            songs = file_songs;
        }
        if songs.is_empty() {
            for position in collector_app::recorded_song_positions(&observations)? {
                songs.push(MusicSearchRecord {
                    duration_ms: None,
                    item_id: item.id,
                    title: format!("Track {position:02}"),
                    artist: artist.clone(),
                    album: album.clone(),
                    disc_position: None,
                    track_position: position,
                });
            }
        }
    }
    for song in songs {
        let search_key = collector_app::normalized_music_search(&format!(
            "{}\n{}\n{}",
            song.title, song.artist, song.album
        ));
        db.execute(
            "INSERT INTO music_search VALUES(?,?,?,?,?,?,?)",
            params![
                item.id.to_string(),
                item.collection_id.to_string(),
                song.disc_position.unwrap_or(0),
                song.track_position,
                collector_app::normalized_music_search(&song.title),
                search_key,
                serde_json::to_string(&song).map_err(storage)?
            ],
        )
        .map_err(storage)?;
    }
    Ok(())
}
pub(super) fn search(
    db: &Connection,
    collection: CollectionId,
    user: UserId,
    query: MusicSearchQuery,
) -> Result<Page<MusicSearchRecord>> {
    let limit = query.limit.unwrap_or(50);
    let search = collector_app::normalized_music_search(&query.search.unwrap_or_default());
    if !(1..=200).contains(&limit) || search.len() > 256 {
        return Err(Error::Invalid("invalid music search bounds".into()));
    }
    let scope =
        Digest::sha256(&serde_json::to_vec(&(collection, user, &search)).map_err(storage)?).hex;
    let cursor = match query.cursor {
        Some(raw) if raw.len() <= 8192 => {
            let c: Cursor = serde_json::from_slice(
                &hex::decode(raw).map_err(|_| Error::Invalid("invalid music cursor".into()))?,
            )
            .map_err(|_| Error::Invalid("invalid music cursor".into()))?;
            if c.scope != scope {
                return Err(Error::Invalid(
                    "music cursor belongs to different results".into(),
                ));
            }
            Some(c)
        }
        Some(_) => return Err(Error::Invalid("music cursor too large".into())),
        None => None,
    };
    let mut stmt=db.prepare("SELECT m.record FROM music_search m JOIN items i ON i.id=m.item_id JOIN private.grants g ON g.collection_id=m.collection_id AND g.user_id=? WHERE m.collection_id=? AND i.deleted=0 AND instr(m.search_key,?)>0 AND (?=0 OR (m.title_key,m.item_id,m.disc,m.track)>(?,?,?,?)) ORDER BY m.title_key,m.item_id,m.disc,m.track LIMIT ?").map_err(storage)?;
    let raw = stmt
        .query_map(
            params![
                user.to_string(),
                collection.to_string(),
                search,
                cursor.is_some(),
                cursor.as_ref().map_or("", |c| c.title.as_str()),
                cursor.as_ref().map_or("", |c| c.id.as_str()),
                cursor.as_ref().map_or(0, |c| c.disc),
                cursor.as_ref().map_or(0, |c| c.track),
                u32::from(limit) + 1
            ],
            |r| r.get::<_, String>(0),
        )
        .map_err(storage)?;
    let mut items = raw
        .map(|r| serde_json::from_str::<MusicSearchRecord>(&r.map_err(storage)?).map_err(storage))
        .collect::<Result<Vec<_>>>()?;
    let next_cursor = if items.len() > usize::from(limit) {
        items.pop();
        let s = items.last().ok_or(Error::NotFound)?;
        Some(hex::encode(
            serde_json::to_vec(&Cursor {
                scope,
                title: collector_app::normalized_music_search(&s.title),
                id: s.item_id.to_string(),
                disc: s.disc_position.unwrap_or(0),
                track: s.track_position,
            })
            .map_err(storage)?,
        ))
    } else {
        None
    };
    Ok(Page { items, next_cursor })
}

pub(super) fn refresh(db: &Connection, id: ItemId) -> Result<()> {
    let raw: Option<String> = db
        .query_row(
            "SELECT record FROM items WHERE id=?",
            [id.to_string()],
            |r| r.get(0),
        )
        .optional()
        .map_err(storage)?;
    let Some(raw) = raw else {
        return Ok(());
    };
    let item: Item = serde_json::from_str(&raw).map_err(storage)?;
    let metadata: Option<String> = db
        .query_row(
            "SELECT record FROM metadata_authorities WHERE id=?",
            [id.to_string()],
            |r| r.get(0),
        )
        .optional()
        .map_err(storage)?;
    if let Some(raw) = metadata {
        processing::project_effective_metadata_search(
            db,
            &serde_json::from_str(&raw).map_err(storage)?,
        )
    } else {
        project(db, &item, None, &[], None)
    }
}
