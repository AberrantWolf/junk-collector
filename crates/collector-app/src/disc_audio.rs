//! Listening policy over immutable module observations; no payloads are opened here.
use crate::*;
use collector_modules::disc_audio::{DiscAudioManifest, TrackMetadataOverride};

impl CollectionService {
    pub(crate) fn recorded_game_audio(
        &self,
        collection: CollectionId,
        source: RepresentationId,
    ) -> Result<Option<DiscAudioManifest>> {
        let scope = PackageScope::Representation(source);
        let fingerprint = self.processing.recorded_source(collection, scope)?;
        for operation in [
            collector_modules::disc_audio::GAME_CD_AUDIO_OPERATION,
            "org.junk.games.identify",
            "org.junk.games.import-analysis",
        ] {
            let mut after = None;
            loop {
                let page = self.processing.observations(&ProcessingQuery {
                    collection_id: collection,
                    source: scope,
                    source_fingerprint: fingerprint.clone(),
                    operation: Some(OperationId::new(operation)?),
                    after,
                    limit: 200,
                })?;
                for observation in &page.items {
                    if observation.source_fingerprint != fingerprint
                        || observation.typed_result.module.0 != "org.junk.games"
                        || observation.typed_result.schema_version != 1
                        || observation.operation_version != 1
                    {
                        continue;
                    }
                    if let Some(audio) = collector_modules::games::recorded_disc_audio(
                        operation,
                        &observation.typed_result.payload,
                    )? {
                        return Ok(Some(audio));
                    }
                }
                if page.next_cursor.is_none() {
                    break;
                }
                after = page.items.last().map(|value| value.id);
            }
        }
        Ok(None)
    }
    pub(crate) fn listening_track_overrides(
        &self,
        item: &Item,
    ) -> Result<Vec<TrackMetadataOverride>> {
        let Some(record) = self
            .processing
            .metadata_authority(item.collection_id, item.id)?
        else {
            return Ok(vec![]);
        };
        let authority = metadata_authority_for_inventory(item, &record)?;
        let Some(value) = authority
            .domain_overrides
            .filter(|v| v.module == item.domain.module)
        else {
            return Ok(vec![]);
        };
        match value.module.0.as_str() {
            "org.junk.music" => Ok(value
                .payload
                .parse::<collector_modules::music::MusicMetadataOverrides>()?
                .track_overrides),
            "org.junk.games" => Ok(value
                .payload
                .parse::<collector_modules::games::GameMetadataOverrides>()?
                .track_overrides),
            _ => Ok(vec![]),
        }
    }
}

/// Source-bound, rebuildable song projection from committed geometry only.
pub fn game_audio_search_projection(
    item: &Item,
    observations: &[ProcessingObservation],
    album: &str,
    artist: &str,
    authority: Option<&MetadataAuthorityRecord>,
) -> Result<Vec<MusicSearchRecord>> {
    if item.deleted || item.domain.module.0 != "org.junk.games" {
        return Ok(vec![]);
    }
    let overrides = authority
        .map(|record| metadata_authority_for_inventory(item, record))
        .transpose()?
        .and_then(|v| v.domain_overrides)
        .filter(|v| v.module == item.domain.module && v.schema_version == 1)
        .map(|v| {
            v.payload
                .parse::<collector_modules::games::GameMetadataOverrides>()
        })
        .transpose()?
        .map(|v| v.track_overrides)
        .unwrap_or_default();
    let mut songs = std::collections::BTreeMap::new();
    for observation in observations {
        let PackageScope::Representation(source) = observation.source_fingerprint.source else {
            continue;
        };
        if observation.typed_result.module.0 != "org.junk.games"
            || observation.typed_result.schema_version != 1
            || observation.operation_version != 1
        {
            continue;
        }
        let Some(manifest) = collector_modules::games::recorded_disc_audio(
            observation.operation.as_str(),
            &observation.typed_result.payload,
        )?
        else {
            continue;
        };
        for track in manifest.tracks {
            let edit = overrides.iter().find(|v| {
                v.source_representation_id == source && v.track_position == track.position
            });
            songs
                .entry((source, track.position))
                .or_insert(MusicSearchRecord {
                    source_representation_id: Some(source),
                    duration_ms: u64::try_from(u128::from(track.frame_count) * 1000 / 44_100).ok(),
                    item_id: item.id,
                    title: edit
                        .and_then(|v| v.title.clone())
                        .unwrap_or_else(|| format!("Track {:02}", track.position)),
                    artist: artist.into(),
                    album: album.into(),
                    disc_position: edit.and_then(|v| v.disc_position),
                    track_position: track.position,
                });
        }
    }
    Ok(songs.into_values().collect())
}

impl CollectionService {
    pub fn listening_collections(
        &self,
        actor: &Principal,
    ) -> Result<Vec<collector_api_types::ClientCollection>> {
        Ok(self
            .collections(actor)?
            .into_iter()
            .filter(|v| v.supports_listening)
            .map(|v| collector_api_types::ClientCollection {
                id: v.id,
                name: v.name,
                can_browse: v.effective_permissions.can_browse,
                can_play: v.effective_permissions.can_browse,
                can_edit_playlists: v.content_permissions.can_edit,
            })
            .collect())
    }
    pub fn listening_albums(
        &self,
        actor: &Principal,
        mut query: BrowseQuery,
    ) -> Result<Page<collector_api_types::ClientAlbum>> {
        query.listening_only = true;
        let values = self.browse_presented(actor, query)?;
        Ok(Page {
            items: values
                .items
                .into_iter()
                .map(|v| collector_api_types::ClientAlbum {
                    id: v.id,
                    collection_id: v.collection_id,
                    title: v.effective_metadata.title.value,
                    artist: v
                        .effective_metadata
                        .subtitle
                        .map(|s| s.value)
                        .unwrap_or(v.subtitle),
                    artwork: v.primary_artwork,
                })
                .collect(),
            next_cursor: values.next_cursor,
        })
    }
}

impl CollectionService {
    pub(crate) fn validate_listening_track_edits(
        &self,
        item: &Item,
        authority: &MetadataAuthority,
        previous: &[collector_modules::disc_audio::TrackMetadataOverride],
    ) -> Result<()> {
        let Some(overrides) = &authority.domain_overrides else {
            return Ok(());
        };
        let edits = match overrides.module.0.as_str() {
            "org.junk.music" => {
                overrides
                    .payload
                    .parse::<collector_modules::music::MusicMetadataOverrides>()?
                    .track_overrides
            }
            "org.junk.games" => {
                overrides
                    .payload
                    .parse::<collector_modules::games::GameMetadataOverrides>()?
                    .track_overrides
            }
            _ => return Ok(()),
        };
        let mut positions = std::collections::BTreeMap::new();
        for edit in edits.iter().filter(|edit| !previous.contains(edit)) {
            if !item
                .captures
                .iter()
                .flat_map(|c| &c.representations)
                .any(|r| r.id == edit.source_representation_id)
            {
                return Err(Error::Forbidden);
            }
            if let std::collections::btree_map::Entry::Vacant(entry) =
                positions.entry(edit.source_representation_id)
            {
                let tracks: std::collections::BTreeSet<u16> = if let Some(manifest) =
                    self.recorded_file_tracks(item.collection_id, edit.source_representation_id)?
                {
                    manifest.tracks.into_iter().map(|t| t.position).collect()
                } else if let Some(audio) =
                    self.recorded_game_audio(item.collection_id, edit.source_representation_id)?
                {
                    audio.tracks.into_iter().map(|t| t.position).collect()
                } else if let Some(collector_modules::music::CdProcessingResult::Layout {
                    tracks,
                    ..
                }) =
                    self.recorded_music_layout(item.collection_id, edit.source_representation_id)?
                {
                    tracks
                        .into_iter()
                        .filter(|t| t.kind == collector_core::CdTrackKind::Audio)
                        .map(|t| u16::from(t.position))
                        .collect()
                } else {
                    return Err(Error::Invalid(
                        "No supported source track manifest is recorded".into(),
                    ));
                };
                entry.insert(tracks);
            }
            if !positions[&edit.source_representation_id].contains(&edit.track_position) {
                return Err(Error::Invalid(
                    "Track edit does not address a recorded source track.".into(),
                ));
            }
        }
        Ok(())
    }
}
