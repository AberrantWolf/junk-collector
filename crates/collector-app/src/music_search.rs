//! Rebuildable song-search policy. No media bytes are opened here.
use crate::*;
use unicode_normalization::UnicodeNormalization;
pub fn normalized_music_search(value: &str) -> String {
    value.trim().nfkc().flat_map(char::to_lowercase).collect()
}
pub use collector_api_types::{MusicSearchHit, MusicSearchQuery, MusicSearchRecord};

pub fn music_search_artist_override(
    item: &Item,
    record: &MetadataAuthorityRecord,
) -> Result<Option<String>> {
    let authority = metadata_authority_for_inventory(item, record)?;
    authority
        .domain_overrides
        .as_ref()
        .filter(|v| v.module.0 == "org.junk.music" && v.schema_version == 1)
        .map(|v| {
            v.payload
                .parse::<collector_modules::music::MusicMetadataOverrides>()
        })
        .transpose()
        .map(|v| v.and_then(|v| v.artist))
}

pub fn music_search_labels(
    item: &Item,
    display: Option<&ResolvedMetadataDisplay>,
    artist_override: Option<&str>,
) -> (String, String) {
    let data = if item.domain.schema_version == 1 {
        item.domain
            .payload
            .parse::<collector_modules::music::MusicItemData>()
            .ok()
    } else {
        None
    };
    let album = display.map_or_else(|| item.title.clone(), |v| v.title.value.clone());
    let artist = artist_override
        .map(str::to_owned)
        .or_else(|| data.and_then(|v| v.artist))
        .or_else(|| {
            display
                .and_then(|v| v.subtitle.as_ref())
                .map(|v| v.value.clone())
        })
        .unwrap_or_else(|| item.subtitle.clone());
    (album, artist)
}

pub fn manual_music_track_title(
    tracks: &[collector_modules::music::Track],
    position: u16,
) -> Option<String> {
    tracks
        .iter()
        .find(|track| track.position == position)
        .map(|track| track.title.trim())
        .filter(|title| {
            !title.is_empty()
                && title.len() <= 512
                && !title.chars().any(char::is_control)
                && *title != format!("Track {position:02}")
        })
        .map(str::to_owned)
}

pub fn music_search_projection(
    item: &Item,
    display: Option<&ResolvedMetadataDisplay>,
    candidates: &[MetadataCandidateObservation],
    artist_override: Option<&str>,
) -> Vec<MusicSearchRecord> {
    if item.deleted || item.domain.module.0 != "org.junk.music" || item.domain.schema_version != 1 {
        return Vec::new();
    }
    let Ok(data) = item
        .domain
        .payload
        .parse::<collector_modules::music::MusicItemData>()
    else {
        return Vec::new();
    };
    let (album, artist) = music_search_labels(item, display, artist_override);
    let make = |disc, position, title: String, frames: Option<u64>| MusicSearchRecord {
        source_representation_id: None,
        duration_ms: frames
            .filter(|frames| *frames > 0)
            .and_then(|frames| u64::try_from(u128::from(frames) * 1000 / 44100).ok()),
        item_id: item.id,
        title,
        artist: artist.clone(),
        album: album.clone(),
        disc_position: Some(disc),
        track_position: position,
    };
    let mut tracks = Vec::new();
    if let Some(display) = display.filter(|v| v.resolution != MetadataResolutionKind::Tentative)
        && let Some(candidate) = candidates
            .iter()
            .find(|v| Some(&v.identity) == display.candidate.as_ref())
        && candidate.identity.provider.0 == collector_modules::music::MUSICBRAINZ_PROVIDER_ID
        && let Ok(release) = candidate
            .observation
            .parse::<collector_modules::music::MusicBrainzReleaseObservation>()
    {
        for medium in &release.media {
            for track in &medium.tracks {
                let title = if release.media.len() == 1 {
                    manual_music_track_title(&data.tracks, track.position)
                        .unwrap_or_else(|| track.title.clone())
                } else {
                    track.title.clone()
                };
                tracks.push(make(medium.position, track.position, title, None));
            }
        }
    }
    if tracks.is_empty() {
        tracks.extend(
            data.tracks
                .into_iter()
                .map(|v| make(1, v.position, v.title, Some(v.length.0))),
        );
    }
    tracks
}
impl CollectionService {
    pub fn music_search(
        &self,
        actor: &Principal,
        collection: CollectionId,
        query: MusicSearchQuery,
    ) -> Result<Page<MusicSearchHit>> {
        self.authorize(actor, collection, Action::Browse)?;
        let records = self.reader.music_search(collection, actor.user_id, query)?;
        let mut items = std::collections::BTreeMap::new();
        let mut hits = Vec::new();
        for mut song in records.items {
            if let std::collections::btree_map::Entry::Vacant(entry) = items.entry(song.item_id) {
                let item = self.reader.item(song.item_id)?;
                if item.collection_id != collection {
                    return Err(Error::Forbidden);
                }
                let artwork = self.item_presented(actor, item.id)?.primary_artwork;
                let rows = self.music_track_availability_recorded(actor.user_id, &item)?;
                entry.insert((artwork, rows));
            }
            let (artwork, rows) = &items[&song.item_id];
            let sources = rows
                .iter()
                .filter(|row| {
                    row.primary_playback()
                        && song
                            .source_representation_id
                            .is_none_or(|source| row.source_representation_id == source)
                        && row.track_position == song.track_position
                        && song
                            .disc_position
                            .is_none_or(|disc| row.disc_position == Some(disc))
                })
                .cloned()
                .collect::<Vec<_>>();
            if song.duration_ms.is_none() {
                song.duration_ms = sources.first().and_then(|row| {
                    u64::try_from(
                        u128::from(row.frame_count.0) * 1000 / u128::from(row.sample_rate()),
                    )
                    .ok()
                });
            }
            hits.push(MusicSearchHit {
                song,
                artwork: *artwork,
                sources,
            });
        }
        Ok(Page {
            items: hits,
            next_cursor: records.next_cursor,
        })
    }
}

impl CollectionService {
    pub fn album_playback_snapshot(
        &self,
        actor: &Principal,
        item_id: ItemId,
        source: Option<RepresentationId>,
    ) -> Result<collector_api_types::AlbumPlaybackSnapshot> {
        let item = self.reader.item(item_id)?;
        self.authorize(actor, item.collection_id, Action::Browse)?;
        let targets = self.list_music_targets(actor, &item, source, None)?;
        let rows = self.music_track_availability_recorded(actor.user_id, &item)?;
        let view = self.item_presented(actor, item_id)?;
        let mut tracks = Vec::new();
        for target in targets {
            let ListTarget::MusicTrack {
                source_representation_id,
                track_position,
                ..
            } = target
            else {
                return Err(Error::Invalid("album playback requires tracks".into()));
            };
            let row = rows
                .iter()
                .find(|v| {
                    v.source_representation_id == source_representation_id
                        && v.track_position == track_position
                        && v.primary_playback()
                })
                .ok_or(Error::Conflict)?;
            tracks.push(row.clone());
        }
        Ok(collector_api_types::AlbumPlaybackSnapshot {
            item_id,
            collection_id: item.collection_id,
            title: view.effective_metadata.title.value,
            artist: view
                .effective_metadata
                .subtitle
                .map(|v| v.value)
                .unwrap_or(view.subtitle),
            tracks,
        })
    }
}

/// Unidentified recorded audio remains searchable without inventing a disc assignment.
pub fn recorded_song_positions(observations: &[ProcessingObservation]) -> Result<Vec<u16>> {
    let mut positions = std::collections::BTreeSet::new();
    for observation in observations {
        if let Some(collector_modules::music::CdProcessingResult::Layout { tracks, .. }) =
            crate::metadata::recorded_music_layout(observation)?
        {
            positions.extend(
                tracks
                    .into_iter()
                    .filter(|v| v.kind == CdTrackKind::Audio)
                    .map(|v| u16::from(v.position)),
            );
        }
    }
    Ok(positions.into_iter().collect())
}
