//! Application-owned, resumable track/lookup pipeline; workers and providers own mechanisms.
use crate::*;
use collector_modules::music::*;
use junk_libs_music::{LocalTrack, Release, fingerprint::AudioFingerprint, provider};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DigitalMatchingProgress {
    pub tracks: usize,
    pub fingerprints: usize,
    pub lookups: usize,
    pub recordings: usize,
    pub recordings_total: usize,
    pub releases: usize,
    pub releases_total: usize,
    pub artwork_pending: bool,
}
#[derive(Default, Serialize, Deserialize)]
struct Checkpoint {
    fingerprints: BTreeMap<ObservationId, Vec<DigitalFileFingerprint>>,
    local: BTreeMap<String, LocalTrack>,
    recording_pages: BTreeSet<String>,
    recording_targets: BTreeSet<String>,
    release_hits: BTreeMap<String, usize>,
    releases: BTreeMap<String, Release>,
    skipped_releases: BTreeSet<String>,
    snapshots: BTreeMap<String, SnapshotId>,
    covers: BTreeSet<String>,
    artwork: Vec<MetadataArtworkOption>,
    artwork_error: Option<String>,
    truncated: bool,
    progress: DigitalMatchingProgress,
    result: Option<serde_json::Value>,
}
impl CollectionService {
    pub(crate) fn digital_fingerprint_cache_key(
        &self,
        frozen: &FrozenProcessingJob,
        inventory: &PackageInventory,
    ) -> Result<Option<String>> {
        let selection: DigitalFingerprintSelection = frozen.selection.parse()?;
        let [file] = selection.files.as_slice() else {
            return Ok(None);
        };
        let Some(frames) = selection.frames.get(file) else {
            return Ok(None);
        };
        let expected = frozen
            .source_fingerprint
            .files
            .iter()
            .find(|f| f.id == *file)
            .ok_or(Error::Conflict)?;
        let name = &inventory
            .files
            .iter()
            .find(|f| f.id == *file)
            .ok_or(Error::Conflict)?
            .display_name;
        Ok(Some(
            Digest::sha256(
                &serde_json::to_vec(&(
                    "digital-prefix-v1",
                    frozen.collection_id,
                    &expected.sha256,
                    expected.bytes,
                    frames,
                    name.as_str()
                        .rsplit('.')
                        .next()
                        .unwrap_or("")
                        .to_ascii_lowercase(),
                    junk_libs_music::fingerprint::IMPLEMENTATION,
                ))
                .map_err(|e| Error::Storage(e.to_string()))?,
            )
            .hex,
        ))
    }
    pub(crate) fn rebind_cached_fingerprint(
        &self,
        frozen: &FrozenProcessingJob,
        inventory: &PackageInventory,
        cached: RawJson,
    ) -> ModuleResult<collector_modules::OperationOutput> {
        let selection: DigitalFingerprintSelection = frozen
            .selection
            .parse()
            .map_err(|e| ModuleError::Invalid(e.to_string()))?;
        let [file] = selection.files.as_slice() else {
            return Err(ModuleError::Invalid("fingerprint cache selection".into()));
        };
        let audio: AudioFingerprint = cached
            .parse()
            .map_err(|e| ModuleError::Invalid(e.to_string()))?;
        audio
            .validate()
            .map_err(|e| ModuleError::Invalid(e.to_string()))?;
        if selection.frames.get(file) != Some(&audio.total_frames()) {
            return Err(ModuleError::Invalid("cached duration changed".into()));
        }
        let file_index = inventory
            .files
            .iter()
            .position(|f| f.id == *file)
            .ok_or(ModuleError::AccessDenied)?;
        Ok(collector_modules::OperationOutput {
            value: RawJson::from_value(serde_json::json!(DigitalFingerprintOutput {
                tracks: vec![DigitalFileFingerprint {
                    file_id: *file,
                    file_index,
                    audio
                }]
            })),
        })
    }
    fn digital_child(
        &self,
        claim: &JobClaim,
        stage: &str,
        kind: &str,
        frozen: FrozenJobInput,
    ) -> Result<JobRecord> {
        let id = JobId(deterministic_inventory_uuid(
            claim.user_id,
            "digital.step",
            &format!("{}:{stage}", claim.id),
        ));
        let id=self.jobs.enqueue_job(JobSpec {id,user_id:claim.user_id,collection_id:claim.collection_id,item_id:claim.item_id,kind:kind.into(),phase:"queued".into(),total_units:0,request:serde_json::json!({"digital_parent":claim.id,"digital_parent_claim":claim.claim_token,"stage":stage}),frozen_input:Some(frozen)})?;
        self.jobs.job_record(id, claim.user_id)
    }
    fn digital_provider_child(
        &self,
        claim: &JobClaim,
        f: &FrozenDigitalMatching,
        stage: &str,
        adapter: (&str, &str),
        mut query: serde_json::Value,
        maximum: u64,
    ) -> Result<JobRecord> {
        if f.refresh && matches!(adapter.0, ACOUSTID_PROVIDER_ID | MUSICBRAINZ_PROVIDER_ID) {
            query["refresh"] = serde_json::Value::Bool(true);
        }
        self.digital_child(
            claim,
            stage,
            "catalog_acquisition",
            FrozenJobInput::Acquisition(FrozenAcquisitionJob {
                request: ProviderRequest {
                    collection_id: f.sources[0].collection_id,
                    provider: ModuleId(adapter.0.into()),
                    operation: OperationId::new(adapter.1)?,
                    source_fingerprint: None,
                    query: RawJson::from_value(query),
                    maximum_response_bytes: maximum,
                },
                collection_binding: f.sources[0].collection_binding.clone(),
                item_id: Some(f.item_id),
                snapshot_id: SnapshotId(deterministic_inventory_uuid(
                    claim.user_id,
                    "digital.response",
                    &format!("{}:{stage}", claim.id),
                )),
                created_at: f.sources[0].created_at,
            }),
        )
    }
    fn digital_child_snapshot(
        &self,
        child: &JobRecord,
        collection: CollectionId,
    ) -> Result<SnapshotRecord> {
        let value = child.result.as_ref().ok_or(Error::Conflict)?;
        let value = value.get("completion").unwrap_or(value);
        let id: SnapshotId =
            serde_json::from_value(value.get("snapshot_id").cloned().ok_or(Error::Conflict)?)
                .map_err(|_| Error::Conflict)?;
        self.processing.snapshot(SnapshotQuery {
            collection_id: collection,
            snapshot_id: id,
        })
    }
    pub(crate) fn run_digital_matching(&self, claim: &JobClaim) -> Result<serde_json::Value> {
        let Some(FrozenJobInput::DigitalMatching(f)) = &claim.frozen_input else {
            return Err(Error::Conflict);
        };
        f.validate()?;
        self.current_digital_matching(claim, f)?;
        let mut state: Checkpoint = self
            .jobs
            .digital_checkpoint(claim.id, claim.user_id)?
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| Error::Storage(e.to_string()))?
            .unwrap_or_default();
        let mut waiting = Vec::new();
        // Children run on the global processing budget, so albums cannot multiply CPU threads.
        for source in &f.sources {
            if state.fingerprints.contains_key(&source.observation_id) {
                continue;
            }
            let child = self.digital_child(
                claim,
                &format!("fingerprint:{}", source.observation_id),
                "processing_operation",
                FrozenJobInput::Processing(source.clone()),
            )?;
            if child.state != JobState::Completed {
                pending_child(&child, &mut waiting)?;
                continue;
            }
            let o = self
                .processing
                .processing_record(source.observation_id)?
                .observation;
            if o.source_fingerprint != source.source_fingerprint
                || o.operation.as_str() != DIGITAL_FINGERPRINT_OPERATION
                || o.support != OperationSupport::Supported
                || o.operation_version != source.operation_version
            {
                return Err(Error::Conflict);
            }
            let out: DigitalFingerprintOutput = o.typed_result.payload.parse()?;
            let selection: DigitalFingerprintSelection = source.selection.parse()?;
            let expected: BTreeSet<_> = selection.files.iter().copied().collect();
            let actual: BTreeSet<_> = out.tracks.iter().map(|t| t.file_id).collect();
            if expected != actual || actual.len() != out.tracks.len() {
                return Err(Error::Conflict);
            }
            for t in &out.tracks {
                let track = f
                    .tracks
                    .iter()
                    .find(|v| {
                        v.file == t.file_id
                            && PackageScope::Representation(v.source) == source.source
                    })
                    .ok_or(Error::Conflict)?;
                if track.frames != t.audio.total_frames()
                    || track.sample_rate != t.audio.sample_rate
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
            }
            state.fingerprints.insert(source.observation_id, out.tracks);
        }
        // Completed tracks immediately feed AcoustID, independently of unfinished fingerprints.
        for track in &f.tracks {
            let key = format!("{}:{}", track.source, track.file);
            if state.local.contains_key(&key) {
                continue;
            }
            let audio = state
                .fingerprints
                .values()
                .flatten()
                .find(|t| t.file_id == track.file)
                .map(|t| &t.audio);
            let Some(audio) = audio else {
                continue;
            };
            let recordings = if audio.usable() {
                let child=self.digital_provider_child(claim,f,&format!("track:{}",track.file),(ACOUSTID_PROVIDER_ID,ACOUSTID_ACQUIRE_OPERATION),serde_json::json!({"duration":audio.duration_ms.div_ceil(1000),"fingerprint":audio.fingerprint}),1024*1024)?;
                if child.state != JobState::Completed {
                    pending_child(&child, &mut waiting)?;
                    continue;
                }
                let snapshot = self.digital_child_snapshot(&child, f.sources[0].collection_id)?;
                provider::parse_acoustid(&self.matching_bytes(claim, &snapshot, 1024 * 1024)?)
                    .map_err(|e| Error::Unavailable(e.to_string()))?
            } else {
                vec![]
            };
            state.local.insert(
                key.clone(),
                LocalTrack {
                    key,
                    duration_ms: audio.duration_ms,
                    disc: track.disc,
                    position: track.position,
                    recordings,
                },
            );
        }
        let recordings: BTreeSet<_> = state
            .local
            .values()
            .flat_map(|t| {
                t.recordings
                    .iter()
                    .filter(|r| r.score >= 0.5)
                    .map(|r| r.recording_id.clone())
            })
            .collect();
        state.truncated |= recordings.len() > 32;
        for recording in &recordings {
            if state.recording_targets.len() >= 32 {
                break;
            }
            state.recording_targets.insert(recording.clone());
        }
        // MusicBrainz starts while other AcoustID lookups and local calculations are running.
        for recording in state.recording_targets.clone() {
            if state.recording_pages.contains(&recording) {
                continue;
            }
            let child=self.digital_provider_child(claim,f,&format!("recording:{recording}"),(MUSICBRAINZ_PROVIDER_ID,MUSICBRAINZ_ACQUIRE_OPERATION),serde_json::json!({"kind":"recording_releases","recording_id":recording,"offset":0,"refresh":false}),8*1024*1024)?;
            if child.state != JobState::Completed {
                pending_child(&child, &mut waiting)?;
                continue;
            }
            let snapshot = self.digital_child_snapshot(&child, f.sources[0].collection_id)?;
            let page = provider::parse_releases(
                &self.matching_bytes(claim, &snapshot, 8 * 1024 * 1024)?,
                false,
            )
            .map_err(|e| Error::Unavailable(e.to_string()))?;
            state.truncated |= page.total.is_some_and(|n| n as usize > page.releases.len());
            for r in page.releases {
                *state.release_hits.entry(r.id).or_default() += 1;
            }
            state.recording_pages.insert(recording.clone());
        }
        let discovery_done = state.local.len() == f.tracks.len()
            && state
                .recording_targets
                .iter()
                .all(|id| state.recording_pages.contains(id));
        // Rank release demand after all recording responses; avoid speculative expansion.
        let mut release_ids: Vec<_> = state
            .release_hits
            .iter()
            .map(|(id, hits)| (id.clone(), *hits))
            .collect();
        release_ids.sort_by_key(|(id, hits)| (std::cmp::Reverse(*hits), id.clone()));
        state.truncated |= release_ids.len() > f.release_limit;
        let requested: Vec<_> = release_ids
            .into_iter()
            .take(f.release_limit)
            .map(|(id, _)| id)
            .collect();
        if discovery_done {
            for id in &requested {
                if state.releases.contains_key(id) || state.skipped_releases.contains(id) {
                    continue;
                }
                let child = self.digital_provider_child(
                    claim,
                    f,
                    &format!("release:{id}"),
                    (MUSICBRAINZ_PROVIDER_ID, MUSICBRAINZ_ACQUIRE_OPERATION),
                    serde_json::json!({"kind":"release_id","release_id":id,"refresh":false}),
                    8 * 1024 * 1024,
                )?;
                if child.state != JobState::Completed {
                    pending_child(&child, &mut waiting)?;
                    continue;
                }
                let snapshot = self.digital_child_snapshot(&child, f.sources[0].collection_id)?;
                let page = provider::parse_releases(
                    &self.matching_bytes(claim, &snapshot, 8 * 1024 * 1024)?,
                    true,
                )
                .map_err(|e| Error::Unavailable(e.to_string()))?;
                if page.releases.len() != 1
                    || page.releases[0].id != *id
                    || page.releases[0]
                        .media
                        .iter()
                        .map(|m| m.tracks.len())
                        .sum::<usize>()
                        > junk_libs_music::MAX_TRACKS
                {
                    return Err(Error::Conflict);
                }
                let cost = serde_json::to_vec(&page.releases[0])
                    .map_err(|e| Error::Storage(e.to_string()))?
                    .len();
                let used = serde_json::to_vec(&state.releases)
                    .map_err(|e| Error::Storage(e.to_string()))?
                    .len();
                if cost > 512 * 1024 || used + cost > 4 * 1024 * 1024 {
                    state.truncated = true;
                    state.skipped_releases.insert(id.clone());
                    continue;
                }
                state.snapshots.insert(id.clone(), snapshot.id);
                state
                    .releases
                    .insert(id.clone(), page.releases.into_iter().next().unwrap());
            }
        }
        let releases_done = discovery_done
            && requested
                .iter()
                .all(|id| state.releases.contains_key(id) || state.skipped_releases.contains(id));
        if discovery_done && (!state.releases.is_empty() || releases_done) {
            let matches = junk_libs_music::matching::rank(
                &state.local.values().cloned().collect::<Vec<_>>(),
                &state.releases.values().cloned().collect::<Vec<_>>(),
                &mut || {
                    self.current_digital_matching(claim, f)
                        .map(|_| ())
                        .map_err(|_| junk_libs_music::Error::Cancelled)
                },
            )
            .map_err(|e| Error::Unavailable(e.to_string()))?;
            if releases_done
                && self
                    .streaming_providers
                    .contains_key(COVER_ART_ARCHIVE_PROVIDER_ID)
                && state.artwork_error.is_none()
            {
                for candidate in matches.iter().take(8) {
                    let id = &candidate.release.id;
                    if state.covers.contains(id) {
                        continue;
                    }
                    let child = self.digital_provider_child(
                        claim,
                        f,
                        &format!("cover:{id}"),
                        (
                            COVER_ART_ARCHIVE_PROVIDER_ID,
                            COVER_ART_ARCHIVE_ACQUIRE_OPERATION,
                        ),
                        serde_json::to_value(CoverArtArchiveQuery::Listing {
                            release_id: id.clone(),
                        })
                        .map_err(|e| Error::Invalid(e.to_string()))?,
                        8 * 1024 * 1024,
                    )?;
                    if matches!(child.state, JobState::Failed | JobState::Cancelled) {
                        state.artwork_error =
                            Some(child.error.unwrap_or_else(|| "Cover lookup failed".into()));
                        break;
                    }
                    if child.state != JobState::Completed {
                        waiting.push(child.id);
                        continue;
                    }
                    let snapshot =
                        self.digital_child_snapshot(&child, f.sources[0].collection_id)?;
                    let observation: CoverArtArchiveSnapshotObservation =
                        snapshot.parsed_observation.parse()?;
                    if observation.http_status != 404 {
                        let bytes = self.matching_bytes(claim, &snapshot, 8 * 1024 * 1024)?;
                        let parser = self
                            .catalog_parsers
                            .get(COVER_ART_ARCHIVE_PROVIDER_ID)
                            .ok_or(Error::Conflict)?;
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
                                state.artwork.push(MetadataArtworkOption {
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
                    state.covers.insert(id.clone());
                }
            }
            state.result = Some(crate::digital_matching::bounded_digital_result(
                DigitalMatchingResult {
                    can_expand: f.release_limit < 32 && state.release_hits.len() > f.release_limit,
                    artwork: state.artwork.clone(),
                    artwork_error: state.artwork_error.clone(),
                    matches,
                    truncated: state.truncated,
                    snapshots: state.snapshots.clone(),
                    tracks: f.tracks.clone(),
                    fingerprints: f.sources.iter().map(|s| s.observation_id).collect(),
                    inventory_revision: f.inventory_revision,
                },
                &mut || self.current_digital_matching(claim, f).map(|_| ()),
            )?);
        }
        state.progress = DigitalMatchingProgress {
            tracks: f.tracks.len(),
            fingerprints: state.fingerprints.values().map(Vec::len).sum(),
            lookups: state.local.len(),
            recordings: state.recording_pages.len(),
            recordings_total: state.recording_targets.len(),
            releases: state.releases.len(),
            releases_total: if discovery_done { requested.len() } else { 0 },
            artwork_pending: releases_done && !waiting.is_empty(),
        };
        if waiting.is_empty() {
            self.current_digital_matching(claim, f)?;
            self.jobs.wait_digital(
                claim,
                serde_json::to_value(&state).map_err(|e| Error::Storage(e.to_string()))?,
                &[],
            )?;
            return state.result.ok_or(Error::Conflict);
        }
        self.jobs.wait_digital(
            claim,
            serde_json::to_value(state).map_err(|e| Error::Storage(e.to_string()))?,
            &waiting,
        )?;
        Err(Error::CommandInProgress)
    }
}
fn pending_child(child: &JobRecord, waiting: &mut Vec<JobId>) -> Result<()> {
    match child.state {
        JobState::Failed => Err(Error::Unavailable(
            child
                .error
                .clone()
                .unwrap_or_else(|| "Album identification failed".into()),
        )),
        JobState::Cancelled | JobState::CancelRequested => Err(Error::JobCancelled),
        _ => {
            waiting.push(child.id);
            Ok(())
        }
    }
}
