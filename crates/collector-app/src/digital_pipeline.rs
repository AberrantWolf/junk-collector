//! Application-owned, resumable track/lookup pipeline; workers and providers own mechanisms.
use crate::*;
use collector_modules::music::*;
use junk_libs_music::{
    LocalTrack, Release, fingerprint::AudioFingerprint, matching::AlbumMatch, provider,
};
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
    #[serde(default)]
    pub editions_pending: bool,
}
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Checkpoint {
    fingerprints: BTreeMap<ObservationId, Vec<DigitalFileFingerprint>>,
    local: BTreeMap<String, LocalTrack>,
    recording_pages: BTreeSet<String>,
    recording_targets: BTreeSet<String>,
    release_hits: BTreeMap<String, usize>,
    releases: BTreeMap<String, Release>,
    skipped_releases: BTreeSet<String>,
    release_targets: BTreeSet<String>,
    release_priority: BTreeMap<String, (bool, usize)>,
    releases_at_recording: usize,
    edition_group: Option<String>,
    edition_releases: BTreeSet<String>,
    edition_next: u32,
    edition_pages: usize,
    edition_done: bool,
    edition_error: Option<String>,
    detail_limited: bool,
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
        let cancellation = crate::processing_cancellation::ProcessingCancellation::new(
            self.jobs.clone(),
            claim.id,
            claim.claim_token.clone(),
        );
        let mut checkpoint = || {
            if cancellation.requested()? {
                Err(Error::JobCancelled)
            } else {
                Ok(())
            }
        };
        // Search decisions use canonical inputs, never the bounded UI projection.
        let mut matches: Vec<AlbumMatch> = Vec::new();
        let mut ranked_inputs = None;
        let mut releases_done = false;
        // Admit one useful discovery/detail request at a time. Fingerprinting and
        // recognition remain concurrent, but album membership searches do not fan out.
        loop {
            checkpoint()?;
            let mut recording_pending = false;
            for recording in state.recording_targets.clone() {
                if state.recording_pages.contains(&recording) {
                    continue;
                }
                let child = self.digital_provider_child(claim, f, &format!("recording:{recording}"),
                    (MUSICBRAINZ_PROVIDER_ID, MUSICBRAINZ_ACQUIRE_OPERATION),
                    serde_json::json!({"kind":"recording_releases","recording_id":recording,"offset":0,"refresh":false}), 8*1024*1024)?;
                if child.state != JobState::Completed {
                    pending_child(&child, &mut waiting)?;
                    recording_pending = true;
                    continue;
                }
                let snapshot = self.digital_child_snapshot(&child, f.sources[0].collection_id)?;
                let page = provider::parse_releases(
                    &self.matching_bytes(claim, &snapshot, 8 * 1024 * 1024)?,
                    false,
                )
                .map_err(|e| Error::Unavailable(e.to_string()))?;
                state.truncated |= page.total.is_some_and(|n| n as usize > page.releases.len());
                remember_release_hints(&mut state, &page.releases, f.tracks.len())?;
                state.recording_pages.insert(recording);
            }
            let all_tracks = state.local.len() == f.tracks.len();
            if all_tracks && ranked_inputs != Some((state.local.len(), state.releases.len())) {
                matches = rank_candidates(&state, &mut checkpoint)?;
                ranked_inputs = Some((state.local.len(), state.releases.len()));
            }
            let perfect = all_tracks && matches.iter().any(complete_album_match);
            if perfect && state.edition_group.is_none() {
                state.edition_group = matches
                    .iter()
                    .find(|m| complete_album_match(m))
                    .and_then(|m| m.release.release_group_id.clone());
                if state.edition_group.is_none() {
                    state.edition_done = true;
                }
            }
            if recording_pending {
                break;
            }
            let used = state.releases.len() + state.skipped_releases.len();
            let known_editions_pending = state
                .edition_releases
                .iter()
                .any(|id| !state.releases.contains_key(id) && !state.skipped_releases.contains(id));
            if perfect && !state.edition_done && !known_editions_pending {
                if used >= f.release_limit || state.edition_pages >= 4 {
                    state.detail_limited |= used >= f.release_limit;
                    state.truncated = true;
                    state.edition_done = true;
                } else {
                    let group = state.edition_group.as_ref().ok_or(Error::Conflict)?.clone();
                    let offset = state.edition_next;
                    let child = self.digital_provider_child(claim, f, &format!("editions:{group}:{offset}"),
                        (MUSICBRAINZ_PROVIDER_ID, MUSICBRAINZ_ACQUIRE_OPERATION),
                        serde_json::json!({"kind":"release_group_releases","release_group_id":group,"offset":offset,"refresh":false}), 8*1024*1024)?;
                    if child.state == JobState::Failed {
                        state.edition_error = Some(
                            child
                                .error
                                .unwrap_or_else(|| "Edition lookup failed".into()),
                        );
                        state.edition_done = true;
                    } else if child.state != JobState::Completed {
                        pending_child(&child, &mut waiting)?;
                        break;
                    } else {
                        let snapshot =
                            self.digital_child_snapshot(&child, f.sources[0].collection_id)?;
                        let page = provider::parse_releases(
                            &self.matching_bytes(claim, &snapshot, 8 * 1024 * 1024)?,
                            false,
                        )
                        .map_err(|e| Error::Unavailable(e.to_string()))?;
                        if page.offset != offset
                            || page
                                .releases
                                .iter()
                                .any(|r| r.release_group_id.as_deref() != Some(&group))
                        {
                            return Err(Error::Conflict);
                        }
                        state.edition_next = offset + page.releases.len() as u32;
                        state.edition_pages += 1;
                        state.edition_done = page.releases.is_empty()
                            || page.total.is_none_or(|total| state.edition_next >= total);
                        state
                            .edition_releases
                            .extend(page.releases.iter().map(|r| r.id.clone()));
                        remember_release_hints(&mut state, &page.releases, f.tracks.len())?;
                    }
                }
            }
            // A failed/partial candidate directs the next search to an unexplained
            // track. Do not broaden discovery while its remaining recognition is pending.
            let next_recording = if !perfect && state.recording_targets.len() < 32 {
                next_recording_unbounded(&state, &matches, &f.tracks)
            } else {
                None
            };
            let candidate = state
                .release_hits
                .keys()
                .filter(|id| {
                    !state.releases.contains_key(*id) && !state.skipped_releases.contains(*id)
                })
                .filter(|id| !perfect || state.edition_releases.contains(*id))
                .min_by_key(|id| {
                    (
                        std::cmp::Reverse(state.release_hits[*id]),
                        state
                            .release_priority
                            .get(*id)
                            .copied()
                            .unwrap_or((true, usize::MAX)),
                        (*id).clone(),
                    )
                })
                .cloned();
            if used >= f.release_limit {
                state.detail_limited |= candidate.is_some() || next_recording.is_some();
                state.truncated |= state.detail_limited;
                releases_done = all_tracks;
                break;
            }
            if let Some(recording) = next_recording
                && may_discover_recording(&state, all_tracks, candidate.is_some())
            {
                state.releases_at_recording = state.releases.len();
                state.recording_targets.insert(recording);
                continue;
            }
            if let Some(id) = candidate {
                // Fetch the first candidate while recognition of later files runs;
                // subsequent expansion waits until we can actually assess that candidate.
                if !all_tracks && !state.releases.is_empty() {
                    break;
                }
                state.release_targets.insert(id.clone());
                let child = self.digital_provider_child(
                    claim,
                    f,
                    &format!("release:{id}"),
                    (MUSICBRAINZ_PROVIDER_ID, MUSICBRAINZ_ACQUIRE_OPERATION),
                    serde_json::json!({"kind":"release_id","release_id":id,"refresh":false}),
                    8 * 1024 * 1024,
                )?;
                if perfect && child.state == JobState::Failed {
                    // Edition enrichment is optional once a full candidate is available.
                    // Do not hide it because a different edition could not be downloaded.
                    state.edition_error = Some(
                        child
                            .error
                            .unwrap_or_else(|| "Edition lookup failed".into()),
                    );
                    state.edition_done = true;
                    state.skipped_releases.extend(
                        state
                            .edition_releases
                            .iter()
                            .filter(|id| !state.releases.contains_key(*id))
                            .cloned(),
                    );
                    continue;
                }
                if child.state != JobState::Completed {
                    pending_child(&child, &mut waiting)?;
                    break;
                }
                let snapshot = self.digital_child_snapshot(&child, f.sources[0].collection_id)?;
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
                    .map(|medium| medium.tracks.len())
                    .sum::<usize>()
                    > junk_libs_music::MAX_TRACKS
                {
                    // A valid larger edition cannot be compared within this workflow's bound.
                    state.truncated = true;
                    state.skipped_releases.insert(id);
                    continue;
                }
                let cost = serde_json::to_vec(&page.releases[0])
                    .map_err(|e| Error::Storage(e.to_string()))?
                    .len();
                let used_bytes = serde_json::to_vec(&state.releases)
                    .map_err(|e| Error::Storage(e.to_string()))?
                    .len();
                if cost > 512 * 1024 || used_bytes + cost > 4 * 1024 * 1024 {
                    state.truncated = true;
                    state.skipped_releases.insert(id);
                } else {
                    state.snapshots.insert(id.clone(), snapshot.id);
                    state
                        .releases
                        .insert(id, page.releases.into_iter().next().ok_or(Error::Conflict)?);
                }
                continue;
            }
            if perfect && !state.edition_done {
                continue;
            }
            if all_tracks {
                state.truncated |= !perfect
                    && state.recording_targets.len() >= 32
                    && next_recording_unbounded(&state, &matches, &f.tracks).is_some();
                releases_done = true;
            }
            break;
        }
        if state.local.len() == f.tracks.len() {
            if !matches.is_empty()
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
                    can_expand: f.release_limit < 32 && state.detail_limited,
                    edition_error: state.edition_error.clone(),
                    artwork: state.artwork.clone(),
                    artwork_error: state.artwork_error.clone(),
                    matches,
                    truncated: state.truncated,
                    snapshots: state.snapshots.clone(),
                    tracks: f.tracks.clone(),
                    fingerprints: f.sources.iter().map(|s| s.observation_id).collect(),
                    inventory_revision: f.inventory_revision,
                },
                &mut checkpoint,
            )?);
        }
        state.progress = DigitalMatchingProgress {
            tracks: f.tracks.len(),
            fingerprints: state.fingerprints.values().map(Vec::len).sum(),
            lookups: state.local.len(),
            recordings: state.recording_pages.len(),
            recordings_total: state.recording_targets.len(),
            releases: state.releases.len(),
            releases_total: state.release_targets.len(),
            editions_pending: state.edition_group.is_some() && !releases_done,
            artwork_pending: releases_done && !waiting.is_empty(),
        };
        self.current_digital_matching(claim, f)?;
        if waiting.is_empty() {
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

fn complete_album_match(candidate: &AlbumMatch) -> bool {
    candidate.complete_tracklist
        && !candidate.ambiguous_mapping
        && candidate.duration_conflicts == 0
        && candidate.mappings.len() == candidate.local_tracks
        && candidate.release_tracks == candidate.local_tracks
        && candidate.mappings.iter().all(|mapping| {
            mapping.score >= 0.95 && mapping.duration_delta_ms.is_some_and(|delta| delta <= 2000)
        })
}
fn remember_release_hints(
    state: &mut Checkpoint,
    releases: &[Release],
    tracks: usize,
) -> Result<()> {
    let mut seen = BTreeSet::new();
    for release in releases {
        if !seen.insert(&release.id) {
            return Err(Error::Conflict);
        }
        *state.release_hits.entry(release.id.clone()).or_default() += 1;
        let count: usize = release.media.iter().map(|m| m.track_count as usize).sum();
        let digital = !release.media.is_empty()
            && release
                .media
                .iter()
                .all(|m| m.format.as_deref() == Some("Digital Media"));
        state
            .release_priority
            .insert(release.id.clone(), (!digital, count.abs_diff(tracks)));
    }
    Ok(())
}
fn next_recording_unbounded(
    state: &Checkpoint,
    matches: &[AlbumMatch],
    tracks: &[FrozenMatchTrack],
) -> Option<String> {
    let best = matches.first();
    let mut choices = Vec::new();
    for (order, track) in tracks.iter().enumerate() {
        let key = format!("{}:{}", track.source, track.file);
        let Some(local) = state.local.get(&key) else {
            continue;
        };
        let explained = best.is_some_and(|m| {
            m.mappings.iter().any(|mapping| {
                mapping.key == key && mapping.duration_delta_ms.is_none_or(|delta| delta <= 2000)
            })
        });
        for recording in &local.recordings {
            if recording.score >= 0.5 && !state.recording_targets.contains(&recording.recording_id)
            {
                choices.push((explained, recording.score, order, &recording.recording_id));
            }
        }
    }
    choices.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| b.1.total_cmp(&a.1))
            .then(a.2.cmp(&b.2))
            .then(a.3.cmp(b.3))
    });
    choices.first().map(|value| value.3.clone())
}

fn may_discover_recording(state: &Checkpoint, all_tracks: bool, has_candidate: bool) -> bool {
    state.recording_targets.is_empty()
        || all_tracks && (!has_candidate || state.releases.len() > state.releases_at_recording)
}

fn rank_candidates(
    state: &Checkpoint,
    checkpoint: &mut dyn FnMut() -> Result<()>,
) -> Result<Vec<AlbumMatch>> {
    let mut matches = junk_libs_music::matching::rank(
        &state.local.values().cloned().collect::<Vec<_>>(),
        &state.releases.values().cloned().collect::<Vec<_>>(),
        &mut || checkpoint().map_err(|_| junk_libs_music::Error::Cancelled),
    )
    .map_err(|error| match error {
        junk_libs_music::Error::Cancelled => Error::JobCancelled,
        other => Error::Unavailable(other.to_string()),
    })?;
    // Retain the strongest candidate first when the presentation budget truncates editions.
    matches.sort_by_key(|candidate| !complete_album_match(candidate));
    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use junk_libs_music::{Medium, RecordingMatch, Track};
    fn id(n: usize) -> String {
        format!("00000000-0000-4000-8000-{n:012}")
    }
    #[test]
    fn resume_ranks_canonical_releases_even_when_projection_drops_the_strong_match() {
        let source = RepresentationId::new();
        let frozen: Vec<_> = (1..=256)
            .map(|i| FrozenMatchTrack {
                name: format!("{i:03}.flac"),
                source,
                file: PackageFileId::new(),
                source_position: i,
                frames: 576000,
                sample_rate: 48000,
                disc: Some(1),
                position: Some(i),
            })
            .collect();
        let local: Vec<_> = frozen
            .iter()
            .enumerate()
            .map(|(i, t)| LocalTrack {
                key: format!("{}:{}", t.source, t.file),
                duration_ms: 12000,
                disc: Some(1),
                position: Some((i + 1) as u16),
                recordings: vec![
                    RecordingMatch {
                        recording_id: id(i + 1),
                        score: 0.8,
                    },
                    RecordingMatch {
                        recording_id: id(1000 + i + 1),
                        score: 1.0,
                    },
                ],
            })
            .collect();
        let releases: Vec<_> = (0..4)
            .map(|n| Release {
                id: id(10000 + n),
                title: format!("Edition {n}"),
                artist: "Artist".into(),
                artist_ids: vec![],
                release_group_id: Some(id(80)),
                disambiguation: None,
                language: None,
                script: None,
                date: None,
                country: None,
                barcode: None,
                media: vec![Medium {
                    position: Some(1),
                    format: Some("Digital Media".into()),
                    track_count: 256,
                    disc_ids: vec![],
                    tracks: (0..256)
                        .map(|i| Track {
                            position: (i + 1) as u16,
                            title: format!("{} {i}", "t".repeat(650)),
                            artist: String::new(),
                            recording_id: Some(id(if n == 3 || i <= n {
                                1000 + i + 1
                            } else {
                                i + 1
                            })),
                            duration_ms: Some(12000),
                        })
                        .collect(),
                }],
            })
            .collect();
        assert!(
            releases
                .iter()
                .all(|r| serde_json::to_vec(r).unwrap().len() < 512 * 1024)
        );

        let raw_matches =
            junk_libs_music::matching::rank(&local, &releases, &mut || Ok(())).unwrap();
        let projected = bounded_digital_result(
            DigitalMatchingResult {
                edition_error: None,
                can_expand: false,
                artwork: vec![],
                artwork_error: None,
                matches: raw_matches,
                truncated: false,
                snapshots: BTreeMap::new(),
                tracks: frozen,
                fingerprints: vec![],
                inventory_revision: Revision(1),
            },
            &mut || Ok(()),
        )
        .unwrap();
        let shown: DigitalMatchingResult = serde_json::from_value(projected.clone()).unwrap();
        assert!(shown.truncated && !shown.matches.iter().any(complete_album_match));
        let state = Checkpoint {
            local: local
                .into_iter()
                .map(|track| (track.key.clone(), track))
                .collect(),
            releases: releases
                .into_iter()
                .map(|release| (release.id.clone(), release))
                .collect(),
            result: Some(projected),
            ..Default::default()
        };
        let resumed: Checkpoint =
            serde_json::from_value(serde_json::to_value(state).unwrap()).unwrap();
        let ranked = rank_candidates(&resumed, &mut || Ok(())).unwrap();
        assert!(complete_album_match(&ranked[0]));
        assert_eq!(ranked[0].release.id, id(10003));
    }
}
