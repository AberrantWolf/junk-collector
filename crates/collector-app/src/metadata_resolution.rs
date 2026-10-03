//! One durable resolution computation for import, repair and explicit lookups.
//! Provider I/O and archive publication use the existing bounded processing ports.
use crate::*;

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolveMetadataCommand {
    pub lookup: AcquireMetadataCommand,
    #[serde(default)]
    pub acquire_artwork: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenMetadataResolution {
    pub acquisition: FrozenAcquisitionJob,
    pub item_id: ItemId,
    pub inventory_revision: Revision,
    pub generation: u64,
    pub language: LanguageTag,
    pub sources: Vec<SourceFingerprint>,
    pub acquire_artwork: bool,
    pub artwork_candidate: Option<MetadataCandidateIdentity>,
    pub authority_revision: Revision,
}
impl FrozenMetadataResolution {
    pub fn validate(&self) -> Result<()> {
        FrozenJobInput::Acquisition(self.acquisition.clone()).validate()?;
        LanguageTag::new(self.language.as_str())?;
        if self.acquisition.item_id != Some(self.item_id) || self.sources.len() > 1024 {
            return Err(Error::Invalid("invalid metadata resolution inputs".into()));
        }
        for source in &self.sources {
            source.validate()?;
        }
        if let Some(candidate) = &self.artwork_candidate {
            MetadataAuthority {
                selected_candidate: Some(candidate.clone()),
                ..Default::default()
            }
            .validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MetadataResolutionResult {
    pub item_id: ItemId,
    pub snapshot_id: SnapshotId,
    pub outcome: Option<MetadataRefreshOutcome>,
    pub artwork_options: Vec<MetadataArtworkOption>,
    pub artwork_error: Option<String>,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct MetadataResolutionView {
    pub job: JobView,
    pub result: Option<MetadataResolutionResult>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenExistingMetadata {
    pub job_id: JobId,
    pub item_id: ItemId,
    pub collection_id: CollectionId,
    pub inventory_revision: Revision,
    pub generation: u64,
    pub snapshot_id: SnapshotId,
    pub source_observation_id: ObservationId,
    pub source: SourceFingerprint,
    pub binding: CollectionBinding,
    pub language: LanguageTag,
}
impl FrozenExistingMetadata {
    pub fn validate(&self) -> Result<()> {
        self.source.validate()?;
        self.binding.validate()?;
        LanguageTag::new(self.language.as_str())?;
        Ok(())
    }
}

impl CollectionService {
    pub fn metadata_resolutions(
        &self,
        actor: &Principal,
        item_id: ItemId,
    ) -> Result<Vec<MetadataResolutionView>> {
        let item = self.reader.item(item_id)?;
        self.authorize(actor, item.collection_id, Action::Curate)?;
        self.care
            .metadata_resolution_jobs(actor.user_id, item_id)?
            .into_iter()
            .map(|job| {
                let result = job
                    .result
                    .as_ref()
                    .map(|value| value.get("completion").unwrap_or(value))
                    .filter(|value| value.get("snapshot_id").is_some())
                    .map(|value| {
                        serde_json::from_value(value.clone())
                            .map_err(|error| Error::Storage(error.to_string()))
                    })
                    .transpose()?;
                Ok(MetadataResolutionView {
                    job: job.into(),
                    result,
                })
            })
            .collect()
    }

    pub fn resolve_metadata(
        &self,
        actor: &Principal,
        item_id: ItemId,
        command: ResolveMetadataCommand,
        context: &CommandContext,
    ) -> Result<JobId> {
        let item = self.reader.item(item_id)?;
        self.authorize(actor, item.collection_id, Action::Curate)?;
        // Receipt inputs are the submitted command. Mutable configuration and
        // preferences are resolved only inside the effect on its first execution.
        self.execute_command(
            actor,
            context,
            "metadata.resolve",
            &(item_id, &command),
            |effects| {
                let item = self.reader.item(item_id)?;
                let eligibility = self.content_write_eligibility(actor, item.collection_id)?;
                self.validate_owned_domain(&eligibility, &item.domain)?;
                if item.deleted {
                    return Err(Error::NotFound);
                }
                let job = deterministic_job_id(actor.user_id, "metadata.resolve", context.key());
                let frozen = self.freeze_metadata_resolution(
                    actor.user_id,
                    &item,
                    command.clone(),
                    SnapshotId(job.0),
                )?;
                effects.enqueue_job(JobSpec {
                    id: job,
                    user_id: actor.user_id,
                    collection_id: Some(item.collection_id),
                    item_id: Some(item_id),
                    kind: "metadata_resolution".into(),
                    phase: "queued".into(),
                    total_units: 36 * 1024 * 1024,
                    request: serde_json::json!({"item_id":item_id,"metadata_resolution":true}),
                    frozen_input: Some(FrozenJobInput::MetadataResolution(frozen)),
                })
            },
        )
    }

    pub(crate) fn freeze_metadata_resolution(
        &self,
        user: UserId,
        item: &Item,
        command: ResolveMetadataCommand,
        snapshot_id: SnapshotId,
    ) -> Result<FrozenMetadataResolution> {
        use collector_modules::music::*;
        let eligibility = self.content_write_eligibility_for_user(user, item.collection_id)?;
        self.validate_owned_domain(&eligibility, &item.domain)?;
        if eligibility.owner.0 != "org.junk.music" {
            return Err(Error::Invalid(
                "this lookup requires a music collection".into(),
            ));
        }
        let (provider, operation, query, bound, artwork_candidate) = match command.lookup {
            AcquireMetadataCommand::MusicBrainzRelease {
                release_id,
                refresh,
            } => (
                MUSICBRAINZ_PROVIDER_ID,
                MUSICBRAINZ_ACQUIRE_OPERATION,
                serde_json::to_value(MusicBrainzQuery::ReleaseId {
                    release_id,
                    refresh,
                }),
                8 * 1024 * 1024,
                None,
            ),
            AcquireMetadataCommand::MusicBrainzDisc { disc_id, refresh } => (
                MUSICBRAINZ_PROVIDER_ID,
                MUSICBRAINZ_ACQUIRE_OPERATION,
                serde_json::to_value(MusicBrainzQuery::DiscId { disc_id, refresh }),
                8 * 1024 * 1024,
                None,
            ),
            AcquireMetadataCommand::MusicBrainzSearch { query, refresh } => (
                MUSICBRAINZ_PROVIDER_ID,
                MUSICBRAINZ_ACQUIRE_OPERATION,
                serde_json::to_value(MusicBrainzQuery::Search { query, refresh }),
                8 * 1024 * 1024,
                None,
            ),
            AcquireMetadataCommand::CoverArtListing { candidate } => {
                self.ensure_resolution_candidate(item, &candidate)?;
                (
                    COVER_ART_ARCHIVE_PROVIDER_ID,
                    COVER_ART_ARCHIVE_ACQUIRE_OPERATION,
                    serde_json::to_value(CoverArtArchiveQuery::Listing {
                        release_id: candidate.provider_record_id.clone(),
                    }),
                    8 * 1024 * 1024,
                    Some(candidate),
                )
            }
            AcquireMetadataCommand::CoverArtImage {
                candidate,
                image_id,
            } => {
                self.ensure_resolution_candidate(item, &candidate)?;
                (
                    COVER_ART_ARCHIVE_PROVIDER_ID,
                    COVER_ART_ARCHIVE_ACQUIRE_OPERATION,
                    serde_json::to_value(CoverArtArchiveQuery::Image {
                        release_id: candidate.provider_record_id.clone(),
                        image_id,
                    }),
                    20 * 1024 * 1024,
                    Some(candidate),
                )
            }
        };
        if !self.streaming_providers.contains_key(provider)
            || !self.catalog_parsers.contains_key(provider)
        {
            return Err(Error::Unavailable("the metadata provider is not configured; manual information and originals remain available".into()));
        }
        let mut sources = Vec::new();
        for capture in &item.captures {
            for representation in &capture.representations {
                sources.push(self.processing.recorded_source(
                    item.collection_id,
                    PackageScope::Representation(representation.id),
                )?);
            }
        }
        let frozen = FrozenMetadataResolution {
            acquisition: FrozenAcquisitionJob {
                request: ProviderRequest {
                    collection_id: item.collection_id,
                    provider: ModuleId(provider.into()),
                    operation: OperationId::new(operation)?,
                    source_fingerprint: None,
                    query: RawJson::from_value(
                        query.map_err(|error| Error::Invalid(error.to_string()))?,
                    ),
                    maximum_response_bytes: bound,
                },
                collection_binding: eligibility.binding,
                item_id: Some(item.id),
                snapshot_id,
                created_at: self.authentication.now(),
            },
            item_id: item.id,
            inventory_revision: item.revision,
            generation: item.lifecycle.generation,
            language: LanguageTag::new(self.metadata_language(user)?)?,
            sources,
            acquire_artwork: command.acquire_artwork,
            artwork_candidate,
            authority_revision: self
                .processing
                .metadata_authority(item.collection_id, item.id)?
                .map_or(Revision(0), |record| record.revision),
        };
        if provider == MUSICBRAINZ_PROVIDER_ID {
            frozen
                .acquisition
                .request
                .query
                .parse::<MusicBrainzQuery>()?
                .validate()?;
        } else {
            frozen
                .acquisition
                .request
                .query
                .parse::<CoverArtArchiveQuery>()?
                .validate()?;
        }
        frozen.validate()?;
        Ok(frozen)
    }

    fn ensure_resolution_candidate(
        &self,
        item: &Item,
        candidate: &MetadataCandidateIdentity,
    ) -> Result<()> {
        if candidate.provider.0 != collector_modules::music::MUSICBRAINZ_PROVIDER_ID
            || !self
                .processing
                .metadata_candidates(item.collection_id, item.id)?
                .iter()
                .any(|value| &value.identity == candidate)
        {
            return Err(Error::Invalid(
                "artwork must identify an observed album release".into(),
            ));
        }
        Ok(())
    }

    pub(crate) fn resolution_item(
        &self,
        claim: &JobClaim,
        frozen: &FrozenMetadataResolution,
    ) -> Result<Item> {
        if self
            .jobs
            .cancellation_requested(claim.id, &claim.claim_token)?
        {
            return Err(Error::JobCancelled);
        }
        let item = self.reader.item(frozen.item_id)?;
        let eligibility =
            self.content_write_eligibility_for_user(claim.user_id, item.collection_id)?;
        self.validate_frozen_binding(&eligibility, &frozen.acquisition.collection_binding)?;
        if item.deleted
            || item.revision != frozen.inventory_revision
            || item.lifecycle.generation != frozen.generation
        {
            return Err(Error::Conflict);
        }
        for source in &frozen.sources {
            if self
                .processing
                .recorded_source(item.collection_id, source.source)?
                != *source
            {
                return Err(Error::Conflict);
            }
        }
        Ok(item)
    }

    pub(crate) fn run_metadata_resolution(&self, claim: &JobClaim) -> Result<serde_json::Value> {
        use collector_modules::music::*;
        let Some(FrozenJobInput::MetadataResolution(frozen)) = &claim.frozen_input else {
            return Err(Error::Storage(
                "metadata resolution has no frozen input".into(),
            ));
        };
        frozen.validate()?;
        let item = self.resolution_item(claim, frozen)?;
        let snapshot_id = self.acquire_snapshot_for_job(claim, &frozen.acquisition)?;
        let mut result = MetadataResolutionResult {
            item_id: item.id,
            snapshot_id,
            outcome: None,
            artwork_options: vec![],
            artwork_error: None,
        };
        if frozen.acquisition.request.provider.0 == MUSICBRAINZ_PROVIDER_ID {
            let record =
                self.refresh_resolution_metadata(claim, frozen, snapshot_id, vec![], "text")?;
            result.outcome = record.observations.last().map(|value| value.outcome);
            if frozen.acquire_artwork
                && result.outcome == Some(MetadataRefreshOutcome::Candidates)
                && record.authority.artwork_choice.is_none()
                && let Some(candidate) = record.display.and_then(|value| value.candidate)
            {
                // Text is committed before optional artwork. An artwork failure
                // is a partial outcome, never a failed text resolution.
                match self.resolve_automatic_artwork(claim, frozen, snapshot_id, candidate) {
                    Ok(()) => {}
                    Err(Error::JobCancelled) => return Err(Error::JobCancelled),
                    Err(error) => result.artwork_error = Some(error.to_string()),
                }
            }
        } else {
            let query: CoverArtArchiveQuery = frozen.acquisition.request.query.parse()?;
            match query {
                CoverArtArchiveQuery::Listing { .. } => {
                    result.artwork_options =
                        self.resolution_artwork_options(claim, frozen, snapshot_id)?
                }
                CoverArtArchiveQuery::Image { .. } => {
                    let asset = self.publish_resolution_artwork(claim, frozen, snapshot_id)?;
                    self.select_resolution_artwork(claim, frozen, asset)?;
                }
            }
        }
        serde_json::to_value(result).map_err(|error| Error::Storage(error.to_string()))
    }

    fn resolution_artwork_options(
        &self,
        claim: &JobClaim,
        frozen: &FrozenMetadataResolution,
        snapshot_id: SnapshotId,
    ) -> Result<Vec<MetadataArtworkOption>> {
        self.resolution_item(claim, frozen)?;
        let snapshot = self.processing.snapshot(SnapshotQuery {
            collection_id: frozen.acquisition.request.collection_id,
            snapshot_id,
        })?;
        let parser = self
            .catalog_parsers
            .get(&snapshot.provider.0)
            .ok_or_else(|| Error::Unavailable("artwork parser is unavailable".into()))?;
        let bytes = crate::metadata::read_exact_snapshot_bytes_controlled(
            self.processing.as_ref(),
            &snapshot,
            8 * 1024 * 1024,
            &mut || {
                self.jobs
                    .cancellation_requested(claim.id, &claim.claim_token)
            },
        )?;
        let parsed =
            parser.parse_snapshot(&snapshot, &mut std::io::Cursor::new(bytes), &mut || {
                self.jobs
                    .cancellation_requested(claim.id, &claim.claim_token)
            })?;
        parsed
            .entries
            .into_iter()
            .map(|raw| {
                let option: collector_modules::music::CoverArtObservation = raw.parse()?;
                option.validate()?;
                Ok(MetadataArtworkOption {
                    snapshot_id,
                    release_id: option.release_id,
                    image_id: option.image_id,
                    front: option.front,
                    back: option.back,
                    approved: option.approved,
                    types: option.types,
                })
            })
            .collect()
    }

    fn artwork_acquisition(
        &self,
        claim: &JobClaim,
        frozen: &FrozenMetadataResolution,
        query: collector_modules::music::CoverArtArchiveQuery,
        stage: &str,
    ) -> Result<SnapshotId> {
        self.resolution_item(claim, frozen)?;
        let mut acquisition = frozen.acquisition.clone();
        acquisition.request.provider =
            ModuleId(collector_modules::music::COVER_ART_ARCHIVE_PROVIDER_ID.into());
        acquisition.request.operation =
            OperationId::new(collector_modules::music::COVER_ART_ARCHIVE_ACQUIRE_OPERATION)?;
        acquisition.request.query = RawJson::from_value(
            serde_json::to_value(&query).map_err(|error| Error::Invalid(error.to_string()))?,
        );
        acquisition.request.maximum_response_bytes = if stage == "image" {
            20 * 1024 * 1024
        } else {
            8 * 1024 * 1024
        };
        acquisition.snapshot_id = SnapshotId(deterministic_inventory_uuid(
            claim.user_id,
            stage,
            &claim.id.to_string(),
        ));
        self.acquire_snapshot_for_job(claim, &acquisition)
    }

    fn resolve_automatic_artwork(
        &self,
        claim: &JobClaim,
        frozen: &FrozenMetadataResolution,
        text_snapshot: SnapshotId,
        candidate: MetadataCandidateIdentity,
    ) -> Result<()> {
        use collector_modules::music::CoverArtArchiveQuery;
        let listing = self.artwork_acquisition(
            claim,
            frozen,
            CoverArtArchiveQuery::Listing {
                release_id: candidate.provider_record_id.clone(),
            },
            "listing",
        )?;
        let mut options = self.resolution_artwork_options(claim, frozen, listing)?;
        options.sort_by_key(|value| (!value.front, !value.approved, value.image_id.clone()));
        let Some(option) = options.into_iter().find(|value| value.front) else {
            return Ok(());
        };
        if option.release_id != candidate.provider_record_id {
            return Err(Error::Invalid(
                "artwork belongs to a different album release".into(),
            ));
        }
        let image = self.artwork_acquisition(
            claim,
            frozen,
            CoverArtArchiveQuery::Image {
                release_id: option.release_id,
                image_id: option.image_id,
            },
            "image",
        )?;
        let asset = self.publish_resolution_artwork(claim, frozen, image)?;
        self.refresh_resolution_metadata(
            claim,
            frozen,
            text_snapshot,
            vec![ProviderArtworkLink {
                provider_record_id: candidate.provider_record_id,
                asset_id: asset,
            }],
            "artwork",
        )?;
        Ok(())
    }

    fn publish_resolution_artwork(
        &self,
        claim: &JobClaim,
        frozen: &FrozenMetadataResolution,
        snapshot_id: SnapshotId,
    ) -> Result<AssetId> {
        let item = self.resolution_item(claim, frozen)?;
        let asset = AssetId(deterministic_inventory_uuid(
            claim.user_id,
            "resolution-cover",
            &snapshot_id.to_string(),
        ));
        match self.content.asset_state(asset) {
            Ok(state) if !state.deleted => return Ok(asset),
            Err(Error::NotFound) => {}
            Ok(_) => return Err(Error::Conflict),
            Err(error) => return Err(error),
        }
        let snapshot = self.processing.snapshot(SnapshotQuery {
            collection_id: item.collection_id,
            snapshot_id,
        })?;
        let observation: collector_modules::music::CoverArtArchiveSnapshotObservation =
            snapshot.parsed_observation.parse()?;
        let collector_modules::music::CoverArtArchiveQuery::Image {
            release_id,
            image_id,
        } = observation.query
        else {
            return Err(Error::Invalid("artwork snapshot is not an image".into()));
        };
        let bytes = crate::metadata::read_exact_snapshot_bytes_controlled(
            self.processing.as_ref(),
            &snapshot,
            20 * 1024 * 1024,
            &mut || {
                self.jobs
                    .cancellation_requested(claim.id, &claim.claim_token)
            },
        )?;
        self.content.create_asset(
            asset,
            CreateAssetCommand {
                collection_id: item.collection_id,
                subject: AssetSubject::Item { item_id: item.id },
                kind: AssetKind::Cover,
                visibility: Visibility::Shared,
                alt_text: format!(
                    "Cover Art Archive image {image_id} for MusicBrainz release {release_id}"
                ),
            },
            &bytes,
            AssetWriteContext::Item(ItemEffectContext {
                item_id: item.id,
                generation: frozen.generation,
                source: ItemEffectSource::Job {
                    id: claim.id,
                    claim: claim.claim_token.parse().map_err(|_| Error::Conflict)?,
                },
            }),
        )?;
        Ok(asset)
    }
}
