//! Recorded file-source resolution. Never opens or hashes audio during browsing.
use crate::*;
use collector_modules::music::{MusicImportAnalysisOutput, files::FileTrackManifest};
impl CollectionService {
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
    pub(crate) fn file_track_rows(
        &self,
        item: &Item,
        capture: &Capture,
        source: &Representation,
        manifest: FileTrackManifest,
        user: UserId,
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
        let edits = file_track_overrides(item, authority.as_ref())?;
        let mut rows = Vec::new();
        for track in &manifest.tracks {
            let edit = edits.iter().find(|edit| {
                edit.source_representation_id == source.id && edit.track_position == track.position
            });
            let file = source.files.get(track.file_index).ok_or(Error::Conflict)?;
            // Curation access can play a private original; listener views never
            // receive its tags or media target.
            if file.visibility == Visibility::Curators && role == Role::Viewer {
                continue;
            }
            let audio = &track.audio;
            let title = edit
                .and_then(|e| e.title.clone())
                .or_else(|| {
                    manual
                        .as_ref()
                        .and_then(|d| manual_music_track_title(&d.tracks, track.position))
                })
                .or_else(|| audio.tag("title").map(str::to_owned))
                .unwrap_or_else(|| file.name.rsplit('/').next().unwrap_or("Track").to_owned());
            let alternate = manifest.tracks.iter().any(|other| {
                other.position != track.position
                    && ((audio.tag("title").is_some()
                        && audio.tag("track_number").is_some()
                        && audio.tag("title") == other.audio.tag("title")
                        && audio.tag("track_number") == other.audio.tag("track_number")
                        && audio.tag("disc_number") == other.audio.tag("disc_number"))
                        || source
                            .files
                            .get(other.file_index)
                            .is_some_and(|other_file| {
                                file.name.rsplit_once('.').map(|(stem, _)| stem)
                                    == other_file.name.rsplit_once('.').map(|(stem, _)| stem)
                            }))
            });
            let album_selection = match edit.and_then(|e| e.album_selected) {
                Some(true) => collector_api_types::MusicAlbumSelection::Included,
                Some(false) => collector_api_types::MusicAlbumSelection::Excluded,
                None if alternate => collector_api_types::MusicAlbumSelection::NeedsReview,
                None => collector_api_types::MusicAlbumSelection::Included,
            };
            rows.push(MusicTrackAvailability {
                album_selection,
                display_position: edit.and_then(|e| e.display_position).or_else(|| {
                    audio
                        .tag("track_number")
                        .and_then(|value| value.split('/').next()?.parse().ok())
                }),
                audio_format: Some(collector_api_types::MusicAudioFormat {
                    codec: audio.codec.clone(),
                    sample_rate: audio.sample_rate,
                    channels: audio.channels,
                    bits_per_sample: audio.bits_per_sample,
                    delay_frames: audio.delay_frames,
                    padding_frames: audio.padding_frames,
                }),
                capture_id: capture.id,
                source_label: "Digital original".into(),
                disc_position: edit.and_then(|e| e.disc_position).or_else(|| {
                    audio
                        .tag("disc_number")
                        .and_then(|v| v.split('/').next()?.parse().ok())
                }),
                source_representation_id: source.id,
                track_position: track.position,
                start_frame: PcmFrame(0),
                frame_count: PcmFrame(audio.decoded_frames.ok_or(Error::Conflict)?),
                title,
                profile: None,
                can_prepare: false,
                state: MusicTrackAvailabilityState::Available {
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
                },
            });
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
        let Some(manifest) =
            self.recorded_file_tracks(item.collection_id, record.view.representation_id)?
        else {
            return Ok(false);
        };
        let observation_id = ObservationId(deterministic_inventory_uuid(
            intent.user_id,
            "file-tags",
            &intent.import_id.to_string(),
        ));
        if self
            .processing
            .metadata_refresh_observed(item.collection_id, item.id, observation_id)?
        {
            return Ok(true);
        }
        let capture = self.reader.capture(record.view.capture_id)?;
        if capture.association.item_id != Some(item.id) || item.deleted || intent.cancelled {
            return Err(Error::Conflict);
        }
        let source = capture
            .representations
            .iter()
            .find(|r| r.id == record.view.representation_id)
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
                import_id: intent.import_id,
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
            artwork = Some(self.publish_file_cover(intent, item, source.id, index, &bytes)?);
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
                || self.check_file_enrichment(intent.import_id, item.id, item.lifecycle.generation);
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
                import_id: intent.import_id,
                read_bytes: 0,
            };
            let decoder = junk_libs_audio::AudioDecoder::open(junk_libs_audio::ByteSource::new(
                reader,
                junk_libs_audio::MediaHint::new(
                    Some(member.bytes.0),
                    member.display_name.as_str().rsplit('.').next(),
                    None,
                ),
            ))
            .map_err(|e| Error::Unavailable(e.to_string()))?;
            checkpoint()?;
            if self.fingerprint_inventory_checked(&inventory, &mut || checkpoint())? != expected {
                return Err(Error::Conflict);
            }
            if let Some((_, bytes)) = decoder.inspection.pictures.first() {
                artwork = Some(self.publish_file_cover(
                    intent,
                    item,
                    source.id,
                    track.file_index,
                    bytes,
                )?);
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
            source_import: Some(intent.import_id),
            expected_revision: revision,
            observation: MetadataRefreshObservation {
                id: observation_id,
                outcome: MetadataRefreshOutcome::Candidates,
                candidates: vec![candidate],
                raw_observation: RawJson::from_value(
                    serde_json::json!({"import_id": intent.import_id, "source_representation_id": source.id}),
                ),
                observed_at: intent.created_at,
            },
        };
        let actor = Principal {
            user_id: intent.user_id,
            administrator: false,
            device_id: None,
        };
        self.refresh_metadata(
            &actor,
            item.id,
            command,
            &CommandContext::new(format!("file-tags-{}", intent.import_id))?,
        )?;
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
    let edits = file_track_overrides(item, authority)?;
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
        for track in manifest.tracks {
            if representation
                .files
                .get(track.file_index)
                .is_none_or(|f| f.visibility != Visibility::Shared)
            {
                continue;
            }
            let audio = track.audio;
            let position = track.position;
            let edit = edits.iter().find(|edit| {
                edit.source_representation_id == source && edit.track_position == position
            });
            let title = edit
                .and_then(|e| e.title.clone())
                .or_else(|| {
                    manual
                        .as_ref()
                        .and_then(|m| manual_music_track_title(&m.tracks, position))
                })
                .or_else(|| audio.tag("title").map(str::to_owned))
                .unwrap_or_else(|| format!("Track {position:02}"));
            let disc = edit.and_then(|e| e.disc_position).or_else(|| {
                audio
                    .tag("disc_number")
                    .and_then(|v| v.split('/').next()?.parse().ok())
            });
            songs.entry((disc, position)).or_insert(MusicSearchRecord {
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

impl CollectionService {
    fn check_file_enrichment(&self, import: ImportId, item: ItemId, generation: u64) -> Result<()> {
        let intent = self
            .provisional_imports
            .guided_ingest_intent(import)?
            .ok_or(Error::NotFound)?;
        let current = self.reader.item(item)?;
        if intent.cancelled || current.deleted || current.lifecycle.generation != generation {
            return Err(Error::JobCancelled);
        }
        self.content_write_eligibility_for_user(intent.user_id, current.collection_id)?;
        Ok(())
    }
    fn publish_file_cover(
        &self,
        intent: &GuidedIngestIntent,
        item: &Item,
        source: RepresentationId,
        file_index: usize,
        bytes: &[u8],
    ) -> Result<AssetId> {
        validate_asset_upload(&AssetKind::Cover, "Embedded cover", bytes)?;
        self.check_file_enrichment(intent.import_id, item.id, item.lifecycle.generation)?;
        let context = CommandContext::new(format!("file-cover-{}", intent.import_id))?;
        let id = deterministic_new_asset_id(intent.user_id, context.key());
        let actor = Principal {
            user_id: intent.user_id,
            administrator: false,
            device_id: None,
        };
        self.execute_command(
            &actor,
            &context,
            "file.cover",
            &(item.id, source, file_index, Digest::sha256(bytes)),
            |effects| {
                effects.create_asset(
                    id,
                    CreateAssetCommand {
                        collection_id: item.collection_id,
                        subject: AssetSubject::Item { item_id: item.id },
                        kind: AssetKind::Cover,
                        visibility: Visibility::Shared,
                        alt_text: "Cover from purchased original package".into(),
                    },
                    bytes,
                    AssetWriteContext::Item(ItemEffectContext {
                        item_id: item.id,
                        generation: item.lifecycle.generation,
                        source: ItemEffectSource::GuidedImport(intent.import_id),
                    }),
                )
            },
        )?;
        Ok(id)
    }
}
struct FileMetadataReader {
    reader: Box<dyn MediaReader>,
    imports: Arc<dyn ProvisionalImportPort>,
    import_id: ImportId,
    read_bytes: u64,
}
impl Read for FileMetadataReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self
            .imports
            .guided_ingest_intent(self.import_id)
            .map_err(std::io::Error::other)?
            .is_none_or(|intent| intent.cancelled)
        {
            return Err(std::io::Error::other("file metadata extraction cancelled"));
        }
        if self.read_bytes >= 64 * 1024 * 1024 {
            return Err(std::io::Error::other(
                "metadata probing read limit exceeded",
            ));
        }
        let length = buffer
            .len()
            .min((64 * 1024 * 1024 - self.read_bytes) as usize);
        let count = self.reader.read(&mut buffer[..length])?;
        self.read_bytes += count as u64;
        Ok(count)
    }
}
impl Seek for FileMetadataReader {
    fn seek(&mut self, position: std::io::SeekFrom) -> std::io::Result<u64> {
        self.reader.seek(position)
    }
}

fn file_track_overrides(
    item: &Item,
    record: Option<&MetadataAuthorityRecord>,
) -> Result<Vec<collector_modules::music::MusicTrackOverride>> {
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

impl CollectionService {
    pub(crate) fn validate_file_track_edits(
        &self,
        item: &Item,
        authority: &MetadataAuthority,
        previous: &[collector_modules::music::MusicTrackOverride],
    ) -> Result<()> {
        let Some(overrides) = &authority.domain_overrides else {
            return Ok(());
        };
        let overrides = overrides
            .payload
            .parse::<collector_modules::music::MusicMetadataOverrides>()?;
        let mut positions = std::collections::BTreeMap::new();
        for edit in overrides
            .track_overrides
            .iter()
            .filter(|edit| !previous.contains(edit))
        {
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
                let manifest = self
                    .recorded_file_tracks(item.collection_id, edit.source_representation_id)?
                    .ok_or_else(|| {
                        Error::Invalid("No supported file track manifest is recorded.".into())
                    })?;
                entry.insert(
                    manifest
                        .tracks
                        .into_iter()
                        .map(|t| t.position)
                        .collect::<std::collections::BTreeSet<_>>(),
                );
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
