//! Durable, explicit digital album discovery. Provider policy lives here; shared
//! crates own decoding, protocol interpretation and one-to-one comparison.
use crate::*;
use collector_modules::music::*;
use junk_libs_music::{LocalTrack, matching::AlbumMatch, provider};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FindDigitalAlbums {
    pub expected_revision: Revision,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrozenMatchTrack {
    pub name: String,
    pub source: RepresentationId,
    pub file: PackageFileId,
    pub source_position: u16,
    pub frames: u64,
    pub sample_rate: u32,
    pub disc: Option<u16>,
    pub position: Option<u16>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrozenDigitalMatching {
    pub item_id: ItemId,
    pub inventory_revision: Revision,
    pub generation: u64,
    pub metadata_revision: Revision,
    pub sources: Vec<FrozenProcessingJob>,
    pub tracks: Vec<FrozenMatchTrack>,
}
impl FrozenDigitalMatching {
    pub fn validate(&self) -> Result<()> {
        if self.sources.is_empty()
            || self.sources.len() > 99
            || self.tracks.is_empty()
            || self.tracks.len() > junk_libs_music::MAX_TRACKS
        {
            return Err(Error::Invalid(
                "choose an album with 1..=256 shared FLAC/MP3 tracks".into(),
            ));
        }
        for source in &self.sources {
            source.validate()?;
            if source.operation.as_str() != DIGITAL_FINGERPRINT_OPERATION {
                return Err(Error::Conflict);
            }
        }
        let mut keys = BTreeSet::new();
        for t in &self.tracks {
            if !keys.insert((t.source, t.file))
                || t.source_position == 0
                || !self.sources.iter().any(|s| {
                    s.source == PackageScope::Representation(t.source)
                        && s.source_fingerprint.files.iter().any(|f| f.id == t.file)
                })
            {
                return Err(Error::Conflict);
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DigitalMatchingResult {
    #[serde(default)]
    pub artwork: Vec<MetadataArtworkOption>,
    #[serde(default)]
    pub artwork_error: Option<String>,
    pub matches: Vec<AlbumMatch>,
    pub truncated: bool,
    pub snapshots: BTreeMap<String, SnapshotId>,
    pub tracks: Vec<FrozenMatchTrack>,
    pub fingerprints: Vec<ObservationId>,
    pub inventory_revision: Revision,
}
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct DigitalMatchingView {
    pub job: JobView,
    #[schema(value_type=Object)]
    pub result: Option<serde_json::Value>,
}
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct DigitalMatchingStatus {
    pub available: bool,
    pub jobs: Vec<DigitalMatchingView>,
}
impl CollectionService {
    pub fn digital_album_matches(
        &self,
        actor: &Principal,
        id: ItemId,
    ) -> Result<DigitalMatchingStatus> {
        let item = self.reader.item(id)?;
        self.authorize(actor, item.collection_id, Action::Curate)?;
        let jobs = self
            .care
            .digital_matching_jobs(actor.user_id, id)?
            .into_iter()
            .map(|j| DigitalMatchingView {
                result: j
                    .result
                    .as_ref()
                    .map(|v| v.get("completion").unwrap_or(v).clone()),
                job: j.into(),
            })
            .collect();
        Ok(DigitalMatchingStatus {
            available: self.streaming_providers.contains_key(ACOUSTID_PROVIDER_ID)
                && self
                    .streaming_providers
                    .contains_key(MUSICBRAINZ_PROVIDER_ID),
            jobs,
        })
    }
    pub fn find_digital_albums(
        &self,
        actor: &Principal,
        id: ItemId,
        command: FindDigitalAlbums,
        context: &CommandContext,
    ) -> Result<JobId> {
        let item = self.reader.item(id)?;
        self.content_write_eligibility(actor, item.collection_id)?;
        self.execute_command(
            actor,
            context,
            "music.match-digital",
            &(id, &command),
            |effects| {
                let item = self.reader.item(id)?;
                self.content_write_eligibility(actor, item.collection_id)?;
                if item.deleted
                    || item.domain.module.0 != "org.junk.music"
                    || item.revision != command.expected_revision
                {
                    return Err(Error::Conflict);
                }
                let job_id =
                    deterministic_job_id(actor.user_id, "music.match-digital", context.key());
                let frozen = self.freeze_digital_matching(actor, &item, job_id)?;
                effects.enqueue_job(JobSpec {
                    id: job_id,
                    user_id: actor.user_id,
                    collection_id: Some(item.collection_id),
                    item_id: Some(item.id),
                    kind: "digital_album_matching".into(),
                    phase: "queued".into(),
                    total_units: 0,
                    request: serde_json::json!({"item_id":item.id}),
                    frozen_input: Some(FrozenJobInput::DigitalMatching(frozen)),
                })
            },
        )
    }
    fn freeze_digital_matching(
        &self,
        actor: &Principal,
        item: &Item,
        id: JobId,
    ) -> Result<FrozenDigitalMatching> {
        let eligibility = self.content_write_eligibility(actor, item.collection_id)?;
        if !self.streaming_providers.contains_key(ACOUSTID_PROVIDER_ID)
            || !self
                .streaming_providers
                .contains_key(MUSICBRAINZ_PROVIDER_ID)
        {
            return Err(Error::Unavailable("Configure providers.acoustid_api_key and providers.musicbrainz_user_agent to identify digital albums".into()));
        }
        let mut sources = Vec::new();
        let mut tracks = Vec::new();
        for capture in item
            .captures
            .iter()
            .filter(|c| c.association.item_id == Some(item.id))
        {
            for source in capture
                .representations
                .iter()
                .filter(|r| r.role == RepresentationRole::PreservationMaster)
            {
                let Some(manifest) = self.recorded_file_tracks(item.collection_id, source.id)?
                else {
                    continue;
                };
                let rows =
                    self.file_track_rows(item, capture, source, manifest.clone(), actor.user_id)?;
                let mut files = Vec::new();
                for row in rows {
                    use collector_api_types::MusicAlbumSelection;
                    if row.album_selection == MusicAlbumSelection::NeedsReview {
                        return Err(Error::Unavailable("Choose which alternate audio files belong to the album before matching".into()));
                    }
                    if row.album_selection == MusicAlbumSelection::Excluded {
                        continue;
                    }
                    let t = manifest
                        .tracks
                        .iter()
                        .find(|t| t.position == row.track_position)
                        .ok_or(Error::Conflict)?;
                    if source.files[t.file_index].visibility != Visibility::Shared {
                        return Err(Error::Unavailable("Restricted audio cannot be sent to fingerprint providers; review file visibility first".into()));
                    }
                    files.push(t.file_id);
                    tracks.push(FrozenMatchTrack {
                        name: source.files[t.file_index].name.clone(),
                        source: source.id,
                        file: t.file_id,
                        source_position: t.position,
                        frames: t.audio.decoded_frames.ok_or(Error::Conflict)?,
                        sample_rate: t.audio.sample_rate,
                        disc: row.disc_position,
                        position: row.display_position,
                    });
                }
                if files.is_empty() {
                    continue;
                }
                let fingerprint = self
                    .processing
                    .recorded_source(item.collection_id, PackageScope::Representation(source.id))?;
                let selection = RawJson::from_value(serde_json::json!({"files":files}));
                // Reuse only the same computation inputs; otherwise allocate immutable output identity.
                let mut observation_id = ObservationId(deterministic_inventory_uuid(
                    actor.user_id,
                    "digital.fingerprint",
                    &format!("{id}:{}", source.id),
                ));
                let mut after = None;
                loop {
                    let page = self.processing.observations(&ProcessingQuery {
                        collection_id: item.collection_id,
                        source: PackageScope::Representation(source.id),
                        source_fingerprint: fingerprint.clone(),
                        operation: Some(OperationId::new(DIGITAL_FINGERPRINT_OPERATION)?),
                        after,
                        limit: 200,
                    })?;
                    for o in &page.items {
                        let record = self.processing.processing_record(o.id)?;
                        if record
                            .inputs
                            .as_ref()
                            .is_some_and(|i| i.selection == selection.value().unwrap_or_default())
                            && o.support == OperationSupport::Supported
                        {
                            observation_id = o.id;
                        }
                    }
                    if page.next_cursor.is_none() {
                        break;
                    }
                    after = page.items.last().map(|o| o.id);
                }
                sources.push(FrozenProcessingJob {
                    collection_id: item.collection_id,
                    collection_binding: eligibility.binding.clone(),
                    operation: OperationId::new(DIGITAL_FINGERPRINT_OPERATION)?,
                    operation_version: self
                        .modules
                        .capability_owner(&OperationId::new(DIGITAL_FINGERPRINT_OPERATION)?)?
                        .1,
                    source: PackageScope::Representation(source.id),
                    source_fingerprint: fingerprint,
                    snapshot_id: None,
                    observation_id,
                    selection,
                    created_at: self.authentication.now(),
                });
            }
        }
        let metadata_revision = self
            .processing
            .metadata_authority(item.collection_id, item.id)?
            .map_or(Revision(0), |r| r.revision);
        let frozen = FrozenDigitalMatching {
            item_id: item.id,
            inventory_revision: item.revision,
            generation: item.lifecycle.generation,
            metadata_revision,
            sources,
            tracks,
        };
        frozen.validate()?;
        Ok(frozen)
    }
    fn validate_digital_metadata_revision(
        &self,
        item: &Item,
        f: &FrozenDigitalMatching,
    ) -> Result<()> {
        let current = self
            .processing
            .metadata_authority(item.collection_id, item.id)?
            .map_or(Revision(0), |r| r.revision);
        if current != f.metadata_revision {
            return Err(Error::Conflict);
        }
        Ok(())
    }
    pub(crate) fn validate_reviewed_digital_selection(
        &self,
        item: &Item,
        user: UserId,
        reviewed: &ReviewedDigitalMatch,
    ) -> Result<()> {
        let expected: BTreeSet<_> = reviewed
            .tracks
            .iter()
            .map(|track| (track.source_representation_id, track.track_position))
            .collect();
        let mut current = BTreeSet::new();
        for capture in item
            .captures
            .iter()
            .filter(|capture| capture.association.item_id == Some(item.id))
        {
            for source in capture
                .representations
                .iter()
                .filter(|source| source.role == RepresentationRole::PreservationMaster)
            {
                let Some(manifest) = self.recorded_file_tracks(item.collection_id, source.id)?
                else {
                    continue;
                };
                for row in self.file_track_rows(item, capture, source, manifest, user)? {
                    match row.album_selection {
                        collector_api_types::MusicAlbumSelection::Excluded => {}
                        collector_api_types::MusicAlbumSelection::NeedsReview => {
                            return Err(Error::Conflict);
                        }
                        _ => {
                            current.insert((source.id, row.track_position));
                        }
                    }
                }
            }
        }
        if current != expected {
            return Err(Error::Conflict);
        }
        Ok(())
    }
    pub(crate) fn current_digital_matching(
        &self,
        claim: &JobClaim,
        f: &FrozenDigitalMatching,
    ) -> Result<Item> {
        if self
            .jobs
            .cancellation_requested(claim.id, &claim.claim_token)?
        {
            return Err(Error::JobCancelled);
        }
        let item = self.reader.item(f.item_id)?;
        let eligibility =
            self.content_write_eligibility_for_user(claim.user_id, item.collection_id)?;
        if item.deleted
            || item.revision != f.inventory_revision
            || item.lifecycle.generation != f.generation
        {
            return Err(Error::Conflict);
        }
        self.validate_digital_metadata_revision(&item, f)?;
        for source in &f.sources {
            self.validate_frozen_binding(&eligibility, &source.collection_binding)?;
            if self
                .processing
                .recorded_source(item.collection_id, source.source)?
                != source.source_fingerprint
            {
                return Err(Error::Conflict);
            }
        }
        Ok(item)
    }
    fn digital_acquisition(
        &self,
        claim: &JobClaim,
        f: &FrozenDigitalMatching,
        adapter: (&str, &str),
        query: serde_json::Value,
        stage: &str,
        maximum: u64,
    ) -> Result<SnapshotRecord> {
        let item = self.current_digital_matching(claim, f)?;
        let acquisition = FrozenAcquisitionJob {
            request: ProviderRequest {
                collection_id: item.collection_id,
                provider: ModuleId(adapter.0.into()),
                operation: OperationId::new(adapter.1)?,
                source_fingerprint: None,
                query: RawJson::from_value(query),
                maximum_response_bytes: maximum,
            },
            collection_binding: f.sources[0].collection_binding.clone(),
            item_id: Some(item.id),
            snapshot_id: SnapshotId(deterministic_inventory_uuid(
                claim.user_id,
                "digital.response",
                &format!("{}:{stage}", claim.id),
            )),
            created_at: f.sources[0].created_at,
        };
        let id = self.acquire_snapshot_for_job(claim, &acquisition)?;
        self.current_digital_matching(claim, f)?;
        self.processing.snapshot(SnapshotQuery {
            collection_id: item.collection_id,
            snapshot_id: id,
        })
    }
    fn matching_bytes(
        &self,
        claim: &JobClaim,
        record: &SnapshotRecord,
        maximum: u64,
    ) -> Result<Vec<u8>> {
        crate::metadata::read_exact_snapshot_bytes_controlled(
            self.processing.as_ref(),
            record,
            maximum,
            &mut || {
                self.jobs
                    .cancellation_requested(claim.id, &claim.claim_token)
            },
        )
    }
    pub(crate) fn run_digital_matching(&self, claim: &JobClaim) -> Result<serde_json::Value> {
        let Some(FrozenJobInput::DigitalMatching(f)) = &claim.frozen_input else {
            return Err(Error::Conflict);
        };
        f.validate()?;
        self.current_digital_matching(claim, f)?;
        let mut fingerprints = BTreeMap::new();
        for source in &f.sources {
            self.current_digital_matching(claim, f)?;
            self.run_frozen_processing(claim, source.clone())?;
            let o = self
                .processing
                .processing_record(source.observation_id)?
                .observation;
            if o.source_fingerprint != source.source_fingerprint
                || o.support != OperationSupport::Supported
                || o.operation.as_str() != DIGITAL_FINGERPRINT_OPERATION
            {
                return Err(Error::Conflict);
            }
            let out: DigitalFingerprintOutput = o.typed_result.payload.parse()?;
            let PackageScope::Representation(source_id) = source.source else {
                return Err(Error::Conflict);
            };
            for t in out.tracks {
                let expected = f
                    .tracks
                    .iter()
                    .find(|v| v.file == t.file_id && v.source == source_id)
                    .ok_or(Error::Conflict)?;
                if expected.frames != t.audio.decoded_frames
                    || expected.sample_rate != t.audio.sample_rate
                    || source
                        .source_fingerprint
                        .files
                        .get(t.file_index)
                        .is_none_or(|v| v.id != t.file_id)
                {
                    return Err(Error::Conflict);
                }
                t.audio
                    .validate()
                    .map_err(|e| Error::Invalid(e.to_string()))?;
                fingerprints.insert(t.file_id, t.audio);
            }
        }
        let mut local = Vec::new();
        let mut recordings = BTreeSet::new();
        for t in &f.tracks {
            let audio = fingerprints.get(&t.file).ok_or(Error::Conflict)?;
            if !audio.usable() {
                local.push(LocalTrack {
                    key: format!("{}:{}", t.source, t.file),
                    duration_ms: audio.duration_ms,
                    disc: t.disc,
                    position: t.position,
                    recordings: vec![],
                });
                continue;
            }
            let snapshot=self.digital_acquisition(claim,f,(ACOUSTID_PROVIDER_ID,ACOUSTID_ACQUIRE_OPERATION),serde_json::json!({"duration":audio.duration_ms.div_ceil(1000),"fingerprint":audio.fingerprint}),&format!("track:{}",t.file),1024*1024)?;
            let found =
                provider::parse_acoustid(&self.matching_bytes(claim, &snapshot, 1024 * 1024)?)
                    .map_err(|e| Error::Unavailable(e.to_string()))?;
            for r in &found {
                if r.score >= 0.5 {
                    recordings.insert(r.recording_id.clone());
                }
            }
            local.push(LocalTrack {
                key: format!("{}:{}", t.source, t.file),
                duration_ms: audio.duration_ms,
                disc: t.disc,
                position: t.position,
                recordings: found,
            });
        }
        let mut release_ids = BTreeMap::<String, usize>::new();
        let mut truncated = recordings.len() > 32;
        for recording in recordings.into_iter().take(32) {
            let snapshot=self.digital_acquisition(claim,f,(MUSICBRAINZ_PROVIDER_ID,MUSICBRAINZ_ACQUIRE_OPERATION),serde_json::json!({"kind":"recording_releases","recording_id":recording,"offset":0,"refresh":false}),&format!("recording:{recording}"),8*1024*1024)?;
            let page = provider::parse_releases(
                &self.matching_bytes(claim, &snapshot, 8 * 1024 * 1024)?,
                false,
            )
            .map_err(|e| Error::Unavailable(e.to_string()))?;
            truncated |= page.total.is_some_and(|n| n as usize > page.releases.len());
            for r in page.releases {
                *release_ids.entry(r.id).or_default() += 1;
            }
        }
        truncated |= release_ids.len() > 32;
        let mut snapshots = BTreeMap::new();
        let mut releases = Vec::new();
        let mut release_ids: Vec<_> = release_ids.into_iter().collect();
        release_ids.sort_by_key(|(id, hits)| (std::cmp::Reverse(*hits), id.clone()));
        for (id, _) in release_ids.into_iter().take(32) {
            let snapshot = self.digital_acquisition(
                claim,
                f,
                (MUSICBRAINZ_PROVIDER_ID, MUSICBRAINZ_ACQUIRE_OPERATION),
                serde_json::json!({"kind":"release_id","release_id":id,"refresh":false}),
                &format!("release:{id}"),
                8 * 1024 * 1024,
            )?;
            let page = provider::parse_releases(
                &self.matching_bytes(claim, &snapshot, 8 * 1024 * 1024)?,
                true,
            )
            .map_err(|e| Error::Unavailable(e.to_string()))?;
            if page.releases.len() != 1 || page.releases[0].id != id {
                return Err(Error::Conflict);
            }
            if page.releases[0]
                .media
                .iter()
                .map(|m| m.tracks.len())
                .sum::<usize>()
                > junk_libs_music::MAX_TRACKS
            {
                truncated = true;
                continue;
            }
            snapshots.insert(id, snapshot.id);
            releases.extend(page.releases);
        }
        let matches = junk_libs_music::matching::rank(&local, &releases, &mut || {
            self.current_digital_matching(claim, f)
                .map(|_| ())
                .map_err(|_| junk_libs_music::Error::Cancelled)
        })
        .map_err(|e| Error::Unavailable(e.to_string()))?;
        let mut artwork = Vec::new();
        let mut artwork_error = None;
        if self
            .streaming_providers
            .contains_key(COVER_ART_ARCHIVE_PROVIDER_ID)
        {
            for candidate in matches.iter().take(8) {
                let id = &candidate.release.id;
                let listing = self.digital_acquisition(
                    claim,
                    f,
                    (
                        COVER_ART_ARCHIVE_PROVIDER_ID,
                        COVER_ART_ARCHIVE_ACQUIRE_OPERATION,
                    ),
                    serde_json::to_value(CoverArtArchiveQuery::Listing {
                        release_id: id.clone(),
                    })
                    .map_err(|e| Error::Invalid(e.to_string()))?,
                    &format!("cover:{id}"),
                    8 * 1024 * 1024,
                );
                let snapshot = match listing {
                    Ok(s) => s,
                    Err(e @ (Error::CommandInProgress | Error::JobCancelled)) => return Err(e),
                    Err(e) => {
                        artwork_error = Some(e.to_string());
                        break;
                    }
                };
                let observation: CoverArtArchiveSnapshotObservation =
                    snapshot.parsed_observation.parse()?;
                if observation.http_status == 404 {
                    continue;
                }
                let parser = self
                    .catalog_parsers
                    .get(COVER_ART_ARCHIVE_PROVIDER_ID)
                    .ok_or(Error::Conflict)?;
                let bytes = self.matching_bytes(claim, &snapshot, 8 * 1024 * 1024)?;
                let parsed = parser.parse_snapshot(
                    &snapshot,
                    &mut std::io::Cursor::new(bytes),
                    &mut || {
                        self.jobs
                            .cancellation_requested(claim.id, &claim.claim_token)
                    },
                )?;
                for entry in parsed.entries {
                    let cover: CoverArtObservation = entry.parse()?;
                    if cover.front {
                        artwork.push(MetadataArtworkOption {
                            snapshot_id: snapshot.id,
                            release_id: cover.release_id,
                            image_id: cover.image_id,
                            front: true,
                            back: cover.back,
                            approved: cover.approved,
                            types: cover.types,
                        });
                        break;
                    }
                }
            }
        }
        self.current_digital_matching(claim, f)?;
        bounded_digital_result(
            DigitalMatchingResult {
                artwork,
                artwork_error,
                matches,
                truncated,
                snapshots,
                tracks: f.tracks.clone(),
                fingerprints: f.sources.iter().map(|s| s.observation_id).collect(),
                inventory_revision: f.inventory_revision,
            },
            &mut || self.current_digital_matching(claim, f).map(|_| ()),
        )
    }
}

// Include source labels, mappings and optional artwork in the completion budget.
// Leave room for the Job's completion wrapper within its existing 1-MiB bound.
fn bounded_digital_result(
    mut result: DigitalMatchingResult,
    checkpoint: &mut dyn FnMut() -> Result<()>,
) -> Result<serde_json::Value> {
    checkpoint()?;
    let candidates = std::mem::take(&mut result.matches);
    let mut bytes = serde_json::to_vec(&result)
        .map_err(|error| Error::Storage(error.to_string()))?
        .len();
    if bytes > 900_000 {
        return Err(Error::Invalid(
            "Digital matching source labels exceed the completion budget".into(),
        ));
    }
    for candidate in candidates {
        checkpoint()?;
        let candidate_bytes = serde_json::to_vec(&candidate)
            .map_err(|error| Error::Storage(error.to_string()))?
            .len()
            + 1;
        if bytes + candidate_bytes > 900_000 {
            result.truncated = true;
            continue;
        }
        bytes += candidate_bytes;
        result.matches.push(candidate);
    }
    let ids: BTreeSet<_> = result
        .matches
        .iter()
        .map(|candidate| candidate.release.id.as_str())
        .collect();
    result.snapshots.retain(|id, _| ids.contains(id.as_str()));
    result
        .artwork
        .retain(|cover| ids.contains(cover.release_id.as_str()));
    checkpoint()?;
    serde_json::to_value(result).map_err(|error| Error::Storage(error.to_string()))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReviewedDigitalMatch {
    pub snapshot_id: SnapshotId,
    pub release_id: String,
    pub tracks: Vec<collector_modules::disc_audio::TrackMetadataOverride>,
}
impl CollectionService {
    pub(crate) fn review_digital_match(
        &self,
        user: UserId,
        item: &Item,
        job_id: JobId,
        release_id: &str,
    ) -> Result<ReviewedDigitalMatch> {
        let job = self.jobs.job_record(job_id, user)?;
        if job.kind != "digital_album_matching"
            || job.item_id != Some(item.id)
            || job.state != JobState::Completed
        {
            return Err(Error::Conflict);
        }
        let Some(FrozenJobInput::DigitalMatching(f)) = self.jobs.frozen_job_input(job_id, user)?
        else {
            return Err(Error::Conflict);
        };
        f.validate()?;
        if item.deleted
            || item.revision != f.inventory_revision
            || item.lifecycle.generation != f.generation
        {
            return Err(Error::Conflict);
        }
        self.validate_digital_metadata_revision(item, &f)?;
        for source in &f.sources {
            if self
                .processing
                .recorded_source(item.collection_id, source.source)?
                != source.source_fingerprint
            {
                return Err(Error::Conflict);
            }
        }
        let result: DigitalMatchingResult = serde_json::from_value(
            job.result
                .as_ref()
                .ok_or(Error::Conflict)?
                .get("completion")
                .unwrap_or(job.result.as_ref().ok_or(Error::Conflict)?)
                .clone(),
        )
        .map_err(|e| Error::Storage(e.to_string()))?;
        let choice = result
            .matches
            .iter()
            .find(|m| m.release.id == release_id)
            .ok_or(Error::Conflict)?;
        if !choice.complete_tracklist
            || choice.ambiguous_mapping
            || choice.mappings.len() != f.tracks.len()
            || choice.release_tracks != f.tracks.len()
            || choice.duration_conflicts > 0
        {
            return Err(Error::Invalid("This release needs manual mapping: only complete, unambiguous album mappings can be applied together".into()));
        }
        let snapshot_id = *result.snapshots.get(release_id).ok_or(Error::Conflict)?;
        let record = self.processing.snapshot(SnapshotQuery {
            collection_id: item.collection_id,
            snapshot_id,
        })?;
        let bytes = crate::metadata::read_exact_snapshot_bytes_controlled(
            self.processing.as_ref(),
            &record,
            8 * 1024 * 1024,
            &mut || Ok(false),
        )?;
        let page =
            provider::parse_releases(&bytes, true).map_err(|e| Error::Invalid(e.to_string()))?;
        if page.releases.first() != Some(&choice.release) {
            return Err(Error::Conflict);
        }
        let mut seen = BTreeSet::new();
        let mut slots = BTreeSet::new();
        let mut tracks = Vec::new();
        for map in &choice.mappings {
            let local = f
                .tracks
                .iter()
                .find(|t| map.key == format!("{}:{}", t.source, t.file))
                .ok_or(Error::Conflict)?;
            if !seen.insert((local.source, local.file)) || !slots.insert((map.medium, map.position))
            {
                return Err(Error::Conflict);
            }
            let track = choice
                .release
                .media
                .iter()
                .find(|m| m.position == Some(map.medium))
                .and_then(|m| m.tracks.iter().find(|t| t.position == map.position))
                .ok_or(Error::Conflict)?;
            if track.recording_id.as_deref() != Some(&map.recording_id) {
                return Err(Error::Conflict);
            }
            tracks.push(collector_modules::disc_audio::TrackMetadataOverride {
                source_representation_id: local.source,
                track_position: local.source_position,
                title: Some(track.title.clone()),
                disc_position: Some(map.medium),
                display_position: Some(map.position),
                album_selected: Some(true),
            });
        }
        Ok(ReviewedDigitalMatch {
            snapshot_id,
            release_id: release_id.into(),
            tracks,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completion_budget_counts_source_names_as_well_as_release_tracks() {
        let release_id = "00000000-0000-4000-8000-000000000001";
        let track = junk_libs_music::Track {
            position: 1,
            title: "x".repeat(2000),
            artist: "y".repeat(750),
            recording_id: None,
            duration_ms: Some(12000),
        };
        let release = junk_libs_music::Release {
            id: release_id.into(),
            title: "Release".into(),
            artist: "Artist".into(),
            artist_ids: vec![],
            release_group_id: None,
            disambiguation: None,
            language: None,
            script: None,
            date: None,
            country: None,
            barcode: None,
            media: vec![junk_libs_music::Medium {
                position: Some(1),
                format: None,
                disc_ids: vec![],
                track_count: 256,
                tracks: (1..=256)
                    .map(|position| junk_libs_music::Track {
                        position,
                        ..track.clone()
                    })
                    .collect(),
            }],
        };
        let candidate = AlbumMatch {
            release,
            mappings: vec![],
            local_tracks: 256,
            release_tracks: 256,
            complete_tracklist: true,
            ambiguous_mapping: false,
            duration_conflicts: 0,
            unmatched: vec![],
        };
        let result = DigitalMatchingResult {
            artwork: vec![],
            artwork_error: None,
            matches: vec![candidate],
            truncated: false,
            snapshots: BTreeMap::from([(release_id.into(), SnapshotId::new())]),
            tracks: (0..256)
                .map(|_| FrozenMatchTrack {
                    name: vec!["\"".repeat(128); 7].join("/"),
                    source: RepresentationId::new(),
                    file: PackageFileId::new(),
                    source_position: 1,
                    frames: 529200,
                    sample_rate: 44100,
                    disc: None,
                    position: None,
                })
                .collect(),
            fingerprints: vec![],
            inventory_revision: Revision(1),
        };
        assert!(
            serde_json::to_vec(&result).unwrap().len() > 1024 * 1024,
            "the fixture must exceed the real Job limit"
        );
        let value = bounded_digital_result(result, &mut || Ok(())).unwrap();
        assert!(serde_json::to_vec(&value).unwrap().len() < 1024 * 1024);
        assert_eq!(value["truncated"], true);
        assert!(value["matches"].as_array().unwrap().is_empty());
        assert!(value["snapshots"].as_object().unwrap().is_empty());
    }
}
