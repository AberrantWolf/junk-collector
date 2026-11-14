//! Recorded file-source resolution. Never opens or hashes audio during browsing.
use crate::*;
use collector_modules::music::{MusicImportAnalysisOutput, files::FileTrackManifest};
/// Encoding is an album-copy choice within each explicit disc directory.
/// Defaults use only recorded observations, and never override curator selections.
fn file_album_selections(
    manifest: &FileTrackManifest,
    files: &[StoredFile],
    role: Role,
) -> Vec<collector_api_types::MusicAlbumSelection> {
    use collector_api_types::MusicAlbumSelection;
    use collector_modules::music::files::path_disc_number;
    let rank = collector_modules::music::files::codec_priority;
    let mut preferred = std::collections::BTreeMap::new();
    for track in &manifest.tracks {
        if let Some(file) = files.get(track.file_index) {
            let choice = rank(&track.audio.codec);
            preferred
                .entry(path_disc_number(&file.name).unwrap_or(1))
                .and_modify(|value: &mut u8| *value = (*value).min(choice))
                .or_insert(choice);
        }
    }
    let mut tag_counts = std::collections::BTreeMap::new();
    let mut stem_counts = std::collections::BTreeMap::new();
    for track in &manifest.tracks {
        let Some(file) = files.get(track.file_index) else {
            continue;
        };
        if file.visibility == Visibility::Curators && role == Role::Viewer {
            continue;
        }
        let scope = (
            path_disc_number(&file.name).unwrap_or(1),
            track.audio.codec.as_str(),
        );
        if let Some(tags) = file_track_tag_key(track) {
            *tag_counts.entry((scope, tags)).or_insert(0usize) += 1;
        }
        if let Some((stem, _)) = file.name.rsplit_once('.') {
            *stem_counts.entry((scope, stem)).or_insert(0usize) += 1;
        }
    }
    manifest
        .tracks
        .iter()
        .map(|track| {
            let Some(file) = files.get(track.file_index) else {
                return MusicAlbumSelection::NeedsReview;
            };
            let disc = path_disc_number(&file.name).unwrap_or(1);
            if preferred.get(&disc) != Some(&rank(&track.audio.codec)) {
                return MusicAlbumSelection::Excluded;
            }
            let scope = (disc, track.audio.codec.as_str());
            let duplicate_tags = file_track_tag_key(track).is_some_and(|tags| {
                tag_counts
                    .get(&(scope, tags))
                    .is_some_and(|count| *count > 1)
            });
            let duplicate_stem = file.name.rsplit_once('.').is_some_and(|(stem, _)| {
                stem_counts
                    .get(&(scope, stem))
                    .is_some_and(|count| *count > 1)
            });
            if duplicate_tags || duplicate_stem {
                MusicAlbumSelection::NeedsReview
            } else {
                MusicAlbumSelection::Included
            }
        })
        .collect()
}

fn file_track_tag_key(
    track: &collector_modules::music::files::FileTrack,
) -> Option<(&str, &str, Option<&str>)> {
    Some((
        track.audio.tag("title")?,
        track.audio.tag("track_number")?,
        track.audio.tag("disc_number"),
    ))
}

fn recorded_file_disc(name: &str, tag: Option<&str>) -> Option<u16> {
    collector_modules::music::files::path_disc_number(name).or_else(|| {
        tag.and_then(|value| value.split('/').next()?.parse().ok())
            .filter(|position| *position > 0)
    })
}

impl CollectionService {
    /// A failed optional identification may already have been followed by
    /// preservation. Retry against those exact archived bytes through the same
    /// retained-analysis implementation, without reopening or republishing intake.
    pub(crate) fn analyze_preserved_import_for_job(
        &self,
        claim: &JobClaim,
        original: &FrozenProcessingJob,
        mut record: ImportRecord,
    ) -> Result<ImportView> {
        let capture = self.reader.capture(record.view.capture_id)?;
        let item = self.reader.item(claim.item_id.ok_or(Error::Conflict)?)?;
        if capture.collection_id != original.collection_id
            || capture.association.item_id != Some(item.id)
            || item.deleted
            || !capture
                .representations
                .iter()
                .any(|source| source.id == record.view.representation_id)
        {
            return Err(Error::Conflict);
        }
        let source = PackageScope::Representation(record.view.representation_id);
        let committed = self
            .processing
            .recorded_source(original.collection_id, source)?;
        let rebinding = ImportSourceRebinding {
            provisional_fingerprint: original.source_fingerprint.clone(),
            committed_fingerprint: committed.clone(),
            files: original
                .source_fingerprint
                .files
                .iter()
                .zip(&committed.files)
                .map(|(before, after)| SourceFileRebinding {
                    import_file_id: before.id,
                    representation_file_id: after.id,
                    bytes: before.bytes,
                    sha256: before.sha256.clone(),
                })
                .collect(),
        };
        rebinding.validate()?;
        let mut retained = original.clone();
        retained.source = source;
        retained.source_fingerprint = committed;
        let mut selection: serde_json::Value = retained.selection.parse()?;
        selection["item_generation"] = serde_json::json!(item.lifecycle.generation);
        retained.selection = RawJson::from_value(selection);
        self.run_frozen_processing(claim, retained)?;
        let observation = self.processing.processing_record(original.observation_id)?;
        if let Some(intent) = self
            .provisional_imports
            .guided_ingest_intent(record.view.id)?
        {
            self.apply_import_album_selection(
                &intent,
                &record,
                &item,
                ItemEffectSource::Job {
                    id: claim.id,
                    claim: claim.claim_token.parse().map_err(|_| Error::Conflict)?,
                },
            )?;
        }
        record.view.limitations = observation.observation.limitations;
        record.view.analysis_job_id = Some(claim.id);
        let revision = record.view.revision;
        Ok(self.provisional_imports.save_import(record, revision)?.view)
    }
    pub(crate) fn recorded_file_tracks(
        &self,
        collection: CollectionId,
        source: RepresentationId,
    ) -> Result<Option<FileTrackManifest>> {
        let scope = PackageScope::Representation(source);
        let fingerprint = self.processing.recorded_source(collection, scope)?;
        let mut after = None;
        loop {
            let page = self.processing.observations(&ProcessingQuery {
                collection_id: collection,
                source: scope,
                source_fingerprint: fingerprint.clone(),
                operation: Some(OperationId::new("org.junk.music.import-analysis")?),
                after,
                limit: 200,
            })?;
            for observation in &page.items {
                if observation.source_fingerprint != fingerprint {
                    continue;
                }
                let Ok(output) = observation
                    .typed_result
                    .payload
                    .parse::<MusicImportAnalysisOutput>()
                else {
                    continue;
                };
                let Some(mut manifest) = output.file_tracks else {
                    continue;
                };
                manifest.validate()?;
                let record = self.processing.processing_record(observation.id)?;
                for track in &mut manifest.tracks {
                    if let Some(rebinding) = &record.import_rebinding {
                        rebinding.validate()?;
                        let mapping = rebinding
                            .files
                            .get(track.file_index)
                            .ok_or(Error::Conflict)?;
                        if mapping.import_file_id != track.file_id {
                            return Err(Error::Conflict);
                        }
                        track.file_id = mapping.representation_file_id;
                    }
                    if fingerprint
                        .files
                        .get(track.file_index)
                        .is_none_or(|f| f.id != track.file_id)
                    {
                        return Err(Error::Conflict);
                    }
                }
                return Ok(Some(manifest));
            }
            if page.next_cursor.is_none() {
                return Ok(None);
            }
            after = page.items.last().map(|o| o.id);
        }
    }
    /// Missing unselected alternates do not make a complete playback selection partial.
    pub(crate) fn missing_file_album_tracks(
        &self,
        item: &Item,
        source_id: RepresentationId,
        manifest: &FileTrackManifest,
        available: &std::collections::BTreeSet<u16>,
    ) -> Result<bool> {
        if manifest
            .tracks
            .iter()
            .all(|track| available.contains(&track.position))
        {
            return Ok(false);
        }
        let source = item
            .captures
            .iter()
            .flat_map(|capture| &capture.representations)
            .find(|source| source.id == source_id)
            .ok_or(Error::Conflict)?;
        let authority = self
            .processing
            .metadata_authority(item.collection_id, item.id)?;
        let reviewed = file_track_overrides(item, authority.as_ref())?
            .into_iter()
            .filter(|edit| edit.source_representation_id == source_id)
            .filter_map(|edit| {
                edit.album_selected
                    .map(|selected| (edit.track_position, selected))
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let selections = file_album_selections(manifest, &source.files, Role::Curator);
        Ok(manifest
            .tracks
            .iter()
            .zip(selections)
            .any(|(track, selection)| {
                !available.contains(&track.position)
                    && reviewed
                        .get(&track.position)
                        .copied()
                        .unwrap_or(selection != collector_api_types::MusicAlbumSelection::Excluded)
            }))
    }

    pub(crate) fn file_track_rows(
        &self,
        item: &Item,
        capture: &Capture,
        source: &Representation,
        manifest: FileTrackManifest,
        user: UserId,
        use_playback_numbers: bool,
    ) -> Result<Vec<MusicTrackAvailability>> {
        let role = self.reader.role(user, item.collection_id)?;
        let manual = item
            .domain
            .payload
            .parse::<collector_modules::music::MusicItemData>()
            .ok();
        let authority = self
            .processing
            .metadata_authority(item.collection_id, item.id)?;
        let edits = file_track_overrides(item, authority.as_ref())?
            .into_iter()
            .filter(|edit| edit.source_representation_id == source.id)
            .map(|edit| (edit.track_position, edit))
            .collect::<std::collections::BTreeMap<_, _>>();
        let active = self.jobs.active_derivative_jobs(capture.id)?;
        let fingerprint = self
            .processing
            .recorded_source(item.collection_id, PackageScope::Representation(source.id))?;
        let can_write = self
            .content_write_eligibility_for_job(user, Some(item.collection_id), Some(item.id))
            .is_ok();
        let mut retained = Vec::new();
        for representation in capture
            .representations
            .iter()
            .filter(|r| r.source == Some(source.id) && r.profile.is_some())
        {
            match self.processing.recipe(RecipeId(representation.id.0)) {
                Ok(recipe) if recipe.source_fingerprint == fingerprint => {
                    retained.push((representation, recipe))
                }
                Ok(_) | Err(Error::NotFound) => {}
                Err(error) => return Err(error),
            }
        }
        let mut rows = Vec::new();
        let mut disc_counts = std::collections::BTreeMap::<u16, u16>::new();
        let selections = file_album_selections(&manifest, &source.files, role);
        for (track, default_selection) in manifest.tracks.iter().zip(selections) {
            use collector_api_types::MusicAlbumSelection;
            let edit = edits.get(&track.position);
            let file = source.files.get(track.file_index).ok_or(Error::Conflict)?;
            let audio = &track.audio;
            let album_selection = match edit.and_then(|e| e.album_selected) {
                Some(true) => MusicAlbumSelection::Included,
                Some(false) => MusicAlbumSelection::Excluded,
                None => default_selection,
            };
            let disc_position = edit
                .and_then(|edit| edit.disc_position)
                .or_else(|| recorded_file_disc(&file.name, audio.tag("disc_number")));
            // Number the album's tracks in source order. Count private members
            // too so the same shared track keeps its number across roles.
            let playback_number = (use_playback_numbers
                && album_selection != MusicAlbumSelection::Excluded)
                .then(|| {
                    let count = disc_counts.entry(disc_position.unwrap_or(1)).or_default();
                    *count += 1;
                    *count
                });
            // Curation access can play a private original; listener views never
            // receive its tags or media target.
            if file.visibility == Visibility::Curators && role == Role::Viewer {
                continue;
            }
            let title = edit
                .and_then(|e| e.title.clone())
                .or_else(|| {
                    manual
                        .as_ref()
                        .and_then(|d| manual_music_track_title(&d.tracks, track.position))
                })
                .or_else(|| audio.tag("title").map(str::to_owned))
                .unwrap_or_else(|| file.name.rsplit('/').next().unwrap_or("Track").to_owned());
            let wav = audio.codec == "wav";
            let float_wav = wav && audio.bits_per_sample == Some(32);
            let encoder = self.encoder_availability(&ConversionProfile::Flac);
            let can_prepare = wav
                && !float_wav
                && can_write
                && matches!(encoder, EncoderAvailability::Available { .. });
            let coordinate = ConversionTrack {
                position: track.position,
                start: PcmFrame(0),
                length: PcmFrame(audio.decoded_frames.ok_or(Error::Conflict)?),
            };
            let published = retained.iter().find(|(_, recipe)| {
                crate::derivatives::profile_for_recipe(recipe) == ConversionProfile::Flac
                    && recipe.tracks.as_slice() == [coordinate.clone()]
            });
            let state = if wav {
                if let Some((representation, _)) = published {
                    let output = representation.files.first().ok_or(Error::Conflict)?;
                    let inventory = self.processing.package_inventory(
                        item.collection_id,
                        PackageScope::Representation(representation.id),
                    )?;
                    MusicTrackAvailabilityState::Available {
                        representation_id: representation.id,
                        file_id: inventory.files.first().ok_or(Error::Conflict)?.id,
                        file_index: 0,
                        length: output.bytes,
                        media_type: "audio/flac".into(),
                        digest: output.digest.clone(),
                    }
                } else if active.iter().any(|job| {
                    job.input.recipe.source_fingerprint == fingerprint
                        && job.input.recipe.tracks.as_slice() == [coordinate.clone()]
                        && matches!(job.input.recipe.settings, ConversionSettings::Flac { .. })
                }) {
                    MusicTrackAvailabilityState::Preparing
                } else {
                    MusicTrackAvailabilityState::Unavailable {
                        reason: if float_wav {
                            "Choose live FLAC or Opus, or prepare a saved Opus copy of this float WAV."
                                .into()
                        } else {
                            match encoder {
                                EncoderAvailability::Unavailable { reason } => reason,
                                _ => "Prepare FLAC to listen to this WAV original.".into(),
                            }
                        },
                    }
                }
            } else {
                MusicTrackAvailabilityState::Available {
                    representation_id: source.id,
                    file_id: track.file_id,
                    file_index: track.file_index,
                    length: file.bytes,
                    media_type: if audio.codec == "flac" {
                        "audio/flac"
                    } else {
                        "audio/mpeg"
                    }
                    .into(),
                    digest: file.digest.clone(),
                }
            };
            rows.push(MusicTrackAvailability {
                can_stream_original: true,
                album_selection,
                display_position: edit
                    .and_then(|e| e.display_position)
                    .or_else(|| {
                        audio
                            .tag("track_number")
                            .and_then(|value| value.split('/').next()?.parse().ok())
                    })
                    .or(playback_number),
                audio_format: Some(collector_api_types::MusicAudioFormat {
                    codec: if wav && !float_wav {
                        "flac".into()
                    } else {
                        audio.codec.clone()
                    },
                    sample_rate: audio.sample_rate,
                    channels: audio.channels,
                    bits_per_sample: audio.bits_per_sample,
                    delay_frames: audio.delay_frames,
                    padding_frames: audio.padding_frames,
                }),
                capture_id: capture.id,
                source_label: "Digital original".into(),
                disc_position,
                source_representation_id: source.id,
                track_position: track.position,
                start_frame: PcmFrame(0),
                frame_count: PcmFrame(audio.decoded_frames.ok_or(Error::Conflict)?),
                title,
                profile: (wav && !float_wav).then_some(ConversionProfile::Flac),
                can_prepare,
                state,
            });

            let original = rows.last().ok_or(Error::Conflict)?.clone();
            for profile in ConversionProfile::PRESETS.into_iter().skip(1) {
                let geometry = crate::derivatives::file_conversion_frames(audio, &profile);
                let mut row = original.clone();
                row.profile = Some(profile.clone());
                row.audio_format = Some(collector_api_types::MusicAudioFormat {
                    codec: "opus".into(),
                    sample_rate: 48000,
                    channels: 2,
                    bits_per_sample: None,
                    delay_frames: None,
                    padding_frames: None,
                });
                row.can_prepare = false;
                row.state = match geometry {
                    Ok(frames) => {
                        row.frame_count = PcmFrame(frames);
                        let track = ConversionTrack {
                            length: row.frame_count,
                            ..coordinate.clone()
                        };
                        if let Some((representation, _)) = retained.iter().find(|(_, recipe)| {
                            crate::derivatives::profile_for_recipe(recipe) == profile
                                && recipe.tracks.as_slice() == [track.clone()]
                        }) {
                            let output = representation.files.first().ok_or(Error::Conflict)?;
                            let inventory = self.processing.package_inventory(
                                item.collection_id,
                                PackageScope::Representation(representation.id),
                            )?;
                            MusicTrackAvailabilityState::Available {
                                representation_id: representation.id,
                                file_id: inventory.files.first().ok_or(Error::Conflict)?.id,
                                file_index: 0,
                                length: output.bytes,
                                media_type: "audio/ogg; codecs=opus".into(),
                                digest: output.digest.clone(),
                            }
                        } else if active.iter().any(|job| {
                            job.input.recipe.source_fingerprint == fingerprint
                                && job.input.recipe.tracks.as_slice() == [track.clone()]
                                && crate::derivatives::profile_for_recipe(&job.input.recipe)
                                    == profile
                        }) {
                            MusicTrackAvailabilityState::Preparing
                        } else {
                            let reason = match self.encoder_availability(&profile) {
                                EncoderAvailability::Unavailable { reason } => reason,
                                EncoderAvailability::Available { .. } => {
                                    row.can_prepare = can_write;
                                    "This track has not been prepared at this Opus bitrate.".into()
                                }
                            };
                            MusicTrackAvailabilityState::Unavailable { reason }
                        }
                    }
                    Err(error) => MusicTrackAvailabilityState::Unavailable {
                        reason: error.to_string(),
                    },
                };
                rows.push(row);
            }
        }
        Ok(rows)
    }
}

impl CollectionService {
    /// Import reconciliation reuses the ordinary Asset and metadata publication
    /// effects. Originals, not a repeated tag lookup, retain the source bytes.
    pub(crate) fn apply_file_tags(
        &self,
        intent: &GuidedIngestIntent,
        record: &ImportRecord,
        item: &Item,
    ) -> Result<bool> {
        self.apply_file_source_metadata(
            &FileMetadataContext {
                user_id: intent.user_id,
                source: record.view.representation_id,
                capture: record.view.capture_id,
                token: intent.import_id.to_string(),
                owner: ItemEffectSource::GuidedImport(intent.import_id),
                created_at: intent.created_at,
                job: None,
            },
            item,
            None,
        )
    }
    fn apply_file_source_metadata(
        &self,
        context: &FileMetadataContext,
        item: &Item,
        manifest: Option<FileTrackManifest>,
    ) -> Result<bool> {
        let resolved = match manifest {
            Some(manifest) => Some(manifest),
            None => self.recorded_file_tracks(item.collection_id, context.source)?,
        };
        let Some(manifest) = resolved else {
            return Ok(false);
        };
        let observation_id = ObservationId(deterministic_inventory_uuid(
            context.user_id,
            "file-tags",
            &context.token,
        ));
        if self
            .processing
            .metadata_refresh_observed(item.collection_id, item.id, observation_id)?
        {
            return Ok(true);
        }
        let capture = self.reader.capture(context.capture)?;
        if capture.association.item_id != Some(item.id) || item.deleted {
            return Err(Error::Conflict);
        }
        let source = capture
            .representations
            .iter()
            .find(|r| r.id == context.source)
            .ok_or(Error::Conflict)?;
        let public_tracks = manifest
            .tracks
            .iter()
            .filter(|t| {
                source
                    .files
                    .get(t.file_index)
                    .is_some_and(|f| f.visibility == Visibility::Shared)
            })
            .collect::<Vec<_>>();
        let mut artwork = None;
        // Sidecar covers use the same bounded Asset effect as uploaded artwork.
        for (index, file) in source.files.iter().enumerate() {
            let basename = file
                .name
                .rsplit('/')
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            if file.visibility != Visibility::Shared
                || !matches!(
                    basename.as_str(),
                    "cover.jpg"
                        | "cover.jpeg"
                        | "cover.png"
                        | "folder.jpg"
                        | "folder.png"
                        | "front.jpg"
                        | "front.png"
                )
            {
                continue;
            }
            if file.bytes.0 > 20 * 1024 * 1024 {
                return Err(Error::Invalid("cover exceeds image limit".into()));
            }
            let inventory = self
                .processing
                .package_inventory(item.collection_id, PackageScope::Representation(source.id))?;
            let member = inventory.files.get(index).ok_or(Error::Conflict)?;
            let mut reader = FileMetadataReader {
                reader: self.processing.package_file(
                    item.collection_id,
                    inventory.scope,
                    member.id,
                )?,
                imports: self.provisional_imports.clone(),
                jobs: self.jobs.clone(),
                owner: context.owner,
                read_bytes: 0,
            };
            let mut bytes = Vec::new();
            reader
                .by_ref()
                .take(20 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| Error::Unavailable(e.to_string()))?;
            if bytes.len() as u64 != file.bytes.0 || Digest::sha256(&bytes) != file.digest {
                return Err(Error::Conflict);
            }
            artwork = self.publish_file_cover(context, item, source.id, index, &bytes)?;
            break;
        }
        if artwork.is_none()
            && let Some(track) = public_tracks.iter().find(|track| track.embedded_artwork)
        {
            let scope = PackageScope::Representation(source.id);
            let inventory = self
                .processing
                .package_inventory(item.collection_id, scope)?;
            let expected = self.processing.recorded_source(item.collection_id, scope)?;
            let checkpoint =
                || self.check_file_enrichment(context, item.id, item.lifecycle.generation);
            if self.fingerprint_inventory_checked(&inventory, &mut || checkpoint())? != expected {
                return Err(Error::Conflict);
            }
            let member = inventory
                .files
                .get(track.file_index)
                .ok_or(Error::Conflict)?;
            let reader = FileMetadataReader {
                reader: self
                    .processing
                    .package_file(item.collection_id, scope, member.id)?,
                imports: self.provisional_imports.clone(),
                jobs: self.jobs.clone(),
                owner: context.owner,
                read_bytes: 0,
            };
            let info = if track.audio.codec == "wav" {
                let mut checkpoint_error = None;
                let wav = junk_libs_pcm::WavReader::open(reader, &mut || {
                    checkpoint().map_err(|error| {
                        checkpoint_error = Some(error.clone());
                        std::io::Error::other(error)
                    })
                });
                if let Some(error) = checkpoint_error {
                    return Err(error);
                }
                wav.map_err(|e| Error::Unavailable(e.to_string()))?.metadata
            } else {
                let mut checkpoint_error = None;
                let metadata = junk_libs_audio::inspect_file_audio_metadata(
                    junk_libs_audio::ByteSource::new(
                        reader,
                        junk_libs_audio::MediaHint::new(
                            Some(member.bytes.0),
                            member.display_name.as_str().rsplit('.').next(),
                            None,
                        ),
                    ),
                    &mut || {
                        checkpoint().map_err(|error| {
                            checkpoint_error = Some(error);
                            junk_libs_audio::AudioDecodeError::Cancelled
                        })
                    },
                );
                if let Some(error) = checkpoint_error {
                    return Err(error);
                }
                metadata.map_err(|e| Error::Unavailable(e.to_string()))?
            };
            checkpoint()?;
            if self.fingerprint_inventory_checked(&inventory, &mut || checkpoint())? != expected {
                return Err(Error::Conflict);
            }
            if let Some((_, bytes)) = info.pictures.first() {
                artwork =
                    self.publish_file_cover(context, item, source.id, track.file_index, bytes)?;
            }
        }
        let shared_files = source
            .files
            .iter()
            .enumerate()
            .filter(|(_, file)| file.visibility == Visibility::Shared)
            .map(|(index, _)| index)
            .collect();
        let candidate = collector_modules::music::files::metadata_candidate(
            &manifest,
            source.id,
            capture.id,
            &self
                .processing
                .recorded_source(item.collection_id, PackageScope::Representation(source.id))?,
            &shared_files,
            artwork,
            &item.title,
        )?;
        let revision = self
            .processing
            .metadata_authority(item.collection_id, item.id)?
            .map_or(Revision(0), |r| r.revision);
        let command = RefreshMetadataCommand {
            source_import: match context.owner {
                ItemEffectSource::GuidedImport(id) => Some(id),
                _ => None,
            },
            expected_revision: revision,
            observation: MetadataRefreshObservation {
                id: observation_id,
                outcome: MetadataRefreshOutcome::Candidates,
                candidates: vec![candidate],
                raw_observation: RawJson::from_value(
                    serde_json::json!({"source_operation": context.token, "source_representation_id": source.id}),
                ),
                observed_at: context.created_at,
            },
        };
        let actor = Principal {
            user_id: context.user_id,
            administrator: false,
            device_id: None,
        };
        self.check_file_enrichment(context, item.id, item.lifecycle.generation)?;
        if let Some(claim) = &context.job {
            let eligibility = self.content_write_eligibility_for_job(
                context.user_id,
                Some(item.collection_id),
                Some(item.id),
            )?;
            self.refresh_metadata_effect(
                item,
                &eligibility.owner,
                &command,
                &self.metadata_language(context.user_id)?,
                Some(claim),
                self.processing.as_ref(),
            )?;
        } else {
            self.refresh_metadata(
                &actor,
                item.id,
                command,
                &CommandContext::new(format!("file-tags-{}", context.token))?,
            )?;
        }
        Ok(true)
    }
}

/// Store projections consume module-owned file DTOs through application policy.
pub fn file_music_search_projection(
    item: &Item,
    observations: &[ProcessingObservation],
    album: &str,
    artist: &str,
    artist_override: Option<&str>,
    authority: Option<&MetadataAuthorityRecord>,
) -> Result<Vec<MusicSearchRecord>> {
    let manual = item
        .domain
        .payload
        .parse::<collector_modules::music::MusicItemData>()
        .ok();
    let edits = file_track_overrides(item, authority)?
        .into_iter()
        .map(|edit| ((edit.source_representation_id, edit.track_position), edit))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut songs = std::collections::BTreeMap::new();
    for observation in observations {
        let Ok(output) = observation
            .typed_result
            .payload
            .parse::<MusicImportAnalysisOutput>()
        else {
            continue;
        };
        let Some(manifest) = output.file_tracks else {
            continue;
        };
        manifest.validate()?;
        let PackageScope::Representation(source) = observation.source_fingerprint.source else {
            continue;
        };
        let representation = item
            .captures
            .iter()
            .flat_map(|c| &c.representations)
            .find(|r| r.id == source)
            .ok_or(Error::Conflict)?;
        let selections = file_album_selections(&manifest, &representation.files, Role::Viewer);
        for (track, selection) in manifest.tracks.into_iter().zip(selections) {
            let edit = edits.get(&(source, track.position));
            if !edit
                .and_then(|edit| edit.album_selected)
                .unwrap_or(selection != collector_api_types::MusicAlbumSelection::Excluded)
            {
                continue;
            }
            if representation
                .files
                .get(track.file_index)
                .is_none_or(|f| f.visibility != Visibility::Shared)
            {
                continue;
            }
            let audio = track.audio;
            let position = track.position;
            let title = edit
                .and_then(|e| e.title.clone())
                .or_else(|| {
                    manual
                        .as_ref()
                        .and_then(|m| manual_music_track_title(&m.tracks, position))
                })
                .or_else(|| audio.tag("title").map(str::to_owned))
                .unwrap_or_else(|| format!("Track {position:02}"));
            let disc = edit.and_then(|edit| edit.disc_position).or_else(|| {
                recorded_file_disc(
                    &representation.files[track.file_index].name,
                    audio.tag("disc_number"),
                )
            });
            songs.entry((disc, position)).or_insert(MusicSearchRecord {
                source_representation_id: None,
                item_id: item.id,
                title,
                artist: artist_override
                    .map(str::to_owned)
                    .or_else(|| audio.tag("artist").map(str::to_owned))
                    .unwrap_or_else(|| artist.to_owned()),
                album: album.to_owned(),
                disc_position: disc,
                track_position: position,
                duration_ms: audio.decoded_frames.and_then(|frames| {
                    u64::try_from(u128::from(frames) * 1000 / u128::from(audio.sample_rate)).ok()
                }),
            });
        }
    }
    Ok(songs.into_values().collect())
}

struct FileMetadataContext {
    user_id: UserId,
    source: RepresentationId,
    capture: CaptureId,
    token: String,
    owner: ItemEffectSource,
    created_at: chrono::DateTime<Utc>,
    job: Option<JobClaim>,
}
fn check_metadata_owner(
    imports: &dyn ProvisionalImportPort,
    jobs: &dyn JobPort,
    owner: ItemEffectSource,
) -> Result<()> {
    let cancelled = match owner {
        ItemEffectSource::GuidedImport(id) => imports
            .guided_ingest_intent(id)?
            .is_none_or(|i| i.cancelled),
        ItemEffectSource::Job { id, claim } => {
            jobs.cancellation_requested(id, &claim.to_string())?
        }
        _ => {
            return Err(Error::Invalid(
                "File metadata requires durable request ownership".into(),
            ));
        }
    };
    if cancelled {
        return Err(Error::JobCancelled);
    }
    Ok(())
}
impl CollectionService {
    fn check_file_enrichment(
        &self,
        context: &FileMetadataContext,
        item: ItemId,
        generation: u64,
    ) -> Result<()> {
        check_metadata_owner(
            self.provisional_imports.as_ref(),
            self.jobs.as_ref(),
            context.owner,
        )?;
        let current = self.reader.item(item)?;
        if current.deleted || current.lifecycle.generation != generation {
            return Err(Error::JobCancelled);
        }
        self.content_write_eligibility_for_job(
            context.user_id,
            Some(current.collection_id),
            Some(item),
        )?;
        Ok(())
    }
    fn publish_file_cover(
        &self,
        owner: &FileMetadataContext,
        item: &Item,
        source: RepresentationId,
        file_index: usize,
        bytes: &[u8],
    ) -> Result<Option<AssetId>> {
        validate_asset_upload(&AssetKind::Cover, "Embedded cover", bytes)?;
        self.check_file_enrichment(owner, item.id, item.lifecycle.generation)?;
        let context = CommandContext::new(format!("file-cover-{}", owner.token))?;
        let digest = Digest::sha256(bytes);
        let matches = self
            .content
            .asset_states_for_item(item.id)?
            .into_iter()
            .filter(|a| {
                a.asset.subject == AssetSubject::Item { item_id: item.id }
                    && a.asset.kind == AssetKind::Cover
                    && a.asset.file.digest == digest
            })
            .collect::<Vec<_>>();
        let existing = matches.iter().find(|a| !a.deleted);
        if existing.is_none() && !matches.is_empty() {
            return Ok(None);
        }
        let id = if let Some(asset) = existing {
            asset.asset.id
        } else {
            use sha2::{Digest as _, Sha256};
            let mut hash = Sha256::new();
            hash.update(b"collector-file-cover-v1");
            hash.update(item.id.0.as_bytes());
            hash.update(digest.hex.as_bytes());
            let mut id = [0u8; 16];
            id.copy_from_slice(&hash.finalize()[..16]);
            id[6] = (id[6] & 15) | 0x50;
            id[8] = (id[8] & 63) | 0x80;
            AssetId(uuid::Uuid::from_bytes(id))
        };
        let actor = Principal {
            user_id: owner.user_id,
            administrator: false,
            device_id: None,
        };
        let asset = self.execute_command(
            &actor,
            &context,
            "file.cover",
            &(item.id, source, file_index, Digest::sha256(bytes)),
            |effects| {
                effects.materialize_provider_artwork(
                    id,
                    CreateAssetCommand {
                        collection_id: item.collection_id,
                        subject: AssetSubject::Item { item_id: item.id },
                        kind: AssetKind::Cover,
                        visibility: Visibility::Shared,
                        alt_text: "Cover from digital original package".into(),
                    },
                    bytes,
                    ItemEffectContext {
                        item_id: item.id,
                        generation: item.lifecycle.generation,
                        source: owner.owner,
                    },
                    None,
                )
            },
        )?;
        Ok(Some(asset.asset.id))
    }
    pub(crate) fn enrich_retained_file_analysis(
        &self,
        claim: &JobClaim,
        frozen: &FrozenProcessingJob,
    ) -> Result<()> {
        if frozen.operation.as_str() != "org.junk.music.import-analysis" {
            return Ok(());
        }
        let PackageScope::Representation(source) = frozen.source else {
            return Err(Error::Conflict);
        };
        let observed = self.processing.processing_record(frozen.observation_id)?;
        let analysis: MusicImportAnalysisOutput =
            observed.observation.typed_result.payload.parse()?;
        let Some(manifest) = analysis.file_tracks else {
            return Ok(());
        };
        let item = self.reader.item(claim.item_id.ok_or(Error::NotFound)?)?;
        let input: serde_json::Value = frozen.selection.parse()?;
        if input.get("item_generation").and_then(|n| n.as_u64()) != Some(item.lifecycle.generation)
        {
            return Err(Error::Conflict);
        }
        let capture = item
            .captures
            .iter()
            .find(|c| {
                c.association.item_id == Some(item.id)
                    && c.representations.iter().any(|r| r.id == source)
            })
            .ok_or(Error::Conflict)?;
        self.apply_file_source_metadata(
            &FileMetadataContext {
                user_id: claim.user_id,
                source,
                capture: capture.id,
                token: frozen.observation_id.to_string(),
                owner: ItemEffectSource::Job {
                    id: claim.id,
                    claim: claim.claim_token.parse().map_err(|_| Error::Conflict)?,
                },
                created_at: frozen.created_at,
                job: Some(claim.clone()),
            },
            &item,
            Some(manifest),
        )?;
        Ok(())
    }
}
struct FileMetadataReader {
    reader: Box<dyn MediaReader>,
    imports: Arc<dyn ProvisionalImportPort>,
    jobs: Arc<dyn JobPort>,
    owner: ItemEffectSource,
    read_bytes: u64,
}
impl Read for FileMetadataReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        check_metadata_owner(self.imports.as_ref(), self.jobs.as_ref(), self.owner)
            .map_err(std::io::Error::other)?;
        if self.read_bytes >= 64 * 1024 * 1024 {
            return Err(std::io::Error::other(
                "metadata probing read limit exceeded",
            ));
        }
        let len = buffer
            .len()
            .min((64 * 1024 * 1024 - self.read_bytes) as usize);
        let count = self.reader.read(&mut buffer[..len])?;
        self.read_bytes += count as u64;
        Ok(count)
    }
}
impl Seek for FileMetadataReader {
    fn seek(&mut self, p: std::io::SeekFrom) -> std::io::Result<u64> {
        self.reader.seek(p)
    }
}

fn file_track_overrides(
    item: &Item,
    record: Option<&MetadataAuthorityRecord>,
) -> Result<Vec<collector_modules::disc_audio::TrackMetadataOverride>> {
    let Some(record) = record else {
        return Ok(vec![]);
    };
    let authority = metadata_authority_for_inventory(item, record)?;
    authority
        .domain_overrides
        .filter(|value| value.module.0 == "org.junk.music")
        .map(|value| {
            value
                .payload
                .parse::<collector_modules::music::MusicMetadataOverrides>()
                .map(|v| v.track_overrides)
        })
        .transpose()
        .map(|value| value.unwrap_or_default())
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateDigitalPurchase {
    pub expected_revision: Revision,
    pub purchase: Option<DigitalPurchase>,
}
impl CollectionService {
    pub fn update_digital_purchase(
        &self,
        actor: &Principal,
        item_id: ItemId,
        capture_id: CaptureId,
        command: UpdateDigitalPurchase,
        context: &CommandContext,
    ) -> Result<PresentedCapture> {
        if let Some(purchase) = &command.purchase {
            purchase.validate()?;
        }
        let item = self.reader.item(item_id)?;
        let role = self.authorize(actor, item.collection_id, Action::Curate)?;
        let capture = self.execute_command(
            actor,
            context,
            "purchase.update",
            &(item_id, capture_id, &command),
            |effects| {
                self.content_write_eligibility(actor, item.collection_id)?;
                if item.deleted || item.domain.module.0 != "org.junk.music" {
                    return Err(Error::Forbidden);
                }
                let mut capture = self.reader.capture(capture_id)?;
                if capture.collection_id != item.collection_id
                    || capture.association.item_id != Some(item_id)
                {
                    return Err(Error::Forbidden);
                }
                let file_audio = capture
                    .representations
                    .iter()
                    .filter(|r| r.role == RepresentationRole::PreservationMaster)
                    .map(|r| {
                        r.files
                            .iter()
                            .map(|f| PackageFileName::new(f.name.clone()))
                            .collect::<Result<Vec<_>>>()
                    })
                    .collect::<Result<Vec<_>>>()?
                    .iter()
                    .any(|names| collector_modules::music::files::is_file_package(names));
                if !file_audio {
                    return Err(Error::Invalid(
                        "Digital purchase details require a file-audio source.".into(),
                    ));
                }
                capture.purchase = command.purchase.clone();
                effects.save_capture(capture, command.expected_revision)
            },
        )?;
        Ok(PresentedCapture::for_role(capture, role))
    }
}

/// Intake classification is a declared source kind, not an audio-validity claim.
pub fn import_preservation_format(
    binding: &CollectionBinding,
    files: &[ImportDeclaredFile],
) -> &'static str {
    if matches!(binding, CollectionBinding::Assigned { module } if module.0 == "org.junk.music")
        && collector_modules::music::files::is_file_package(
            &files
                .iter()
                .map(|f| PackageFileName::new(f.name.clone()))
                .collect::<Result<Vec<_>>>()
                .unwrap_or_default(),
        )
    {
        "original-file-audio-package"
    } else {
        "original-package"
    }
}

#[cfg(test)]
mod tests {
    use super::recorded_file_disc;

    #[test]
    fn disc_folders_take_precedence_over_conflicting_tags() {
        assert_eq!(
            recorded_file_disc("Album/FLAC/CD_2/01.flac", Some("1/2")),
            Some(2)
        );
        assert_eq!(
            recorded_file_disc("Album/Disc 2/FLAC/01.flac", Some("1/2")),
            Some(2)
        );
        assert_eq!(recorded_file_disc("Album/01.flac", Some("3/4")), Some(3));
    }
}
