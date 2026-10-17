//! Collection-scoped provider work identity; each consuming Job retains its own authority.
use crate::*;

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProviderResponseStatus {
    Waiting,
    Shared,
    Downloaded,
    Unchanged,
}

pub fn acquisition_key(frozen: &FrozenAcquisitionJob) -> Result<String> {
    frozen.request.validate()?;
    let mut request =
        serde_json::to_value(&frozen.request).map_err(|e| Error::Invalid(e.to_string()))?;
    if frozen.request.provider.0 == "org.junk.providers.musicbrainz"
        && let Some(query) = request.get_mut("query").and_then(|v| v.as_object_mut())
    {
        query.remove("refresh");
    }
    let value = serde_json::json!({"version":1,"request":request});
    Ok(Digest::sha256(&serde_json::to_vec(&value).map_err(|e| Error::Invalid(e.to_string()))?).hex)
}

impl CollectionService {
    pub fn reconcile_provider_acquisitions(&self) -> Result<()> {
        self.jobs.reconcile_acquisitions()
    }
    pub(crate) fn validate_acquisition_owners(
        &self,
        claim: &JobClaim,
        _frozen: &FrozenAcquisitionJob,
    ) -> Result<()> {
        let mut failure = None;
        for owner in self.jobs.acquisition_owners(claim)? {
            if self
                .jobs
                .cancellation_requested(claim.id, &claim.claim_token)?
            {
                return Err(Error::JobCancelled);
            }
            let binding = match self.jobs.frozen_job_input(owner.id, owner.user_id)? {
                Some(FrozenJobInput::Acquisition(input)) => input.collection_binding,
                Some(FrozenJobInput::MetadataResolution(input)) => {
                    input.acquisition.collection_binding
                }
                _ => {
                    failure = Some(Error::Conflict);
                    continue;
                }
            };
            match self.content_write_eligibility_for_job(
                owner.user_id,
                owner.collection_id,
                owner.item_id,
            ) {
                Ok(current) if self.validate_frozen_binding(&current, &binding).is_ok() => {
                    return Ok(());
                }
                Ok(_) => {
                    failure = Some(Error::Conflict);
                }
                Err(error) => {
                    failure = Some(error);
                }
            }
        }
        Err(failure.unwrap_or(Error::JobCancelled))
    }
}

/// Owned response-retention policy, shared by publication and explicit conversion.
/// Other providers require a separate assessment of their identity/consumers.
pub fn reuses_equivalent_provider_responses(provider: &ModuleId) -> bool {
    matches!(
        provider.0.as_str(),
        "org.junk.providers.accuraterip"
            | collector_modules::music::MUSICBRAINZ_PROVIDER_ID
            | collector_modules::music::COVER_ART_ARCHIVE_PROVIDER_ID
    )
}

/// Operational cache bypass is not part of provider response meaning.
pub fn snapshot_response_key(record: &SnapshotRecord) -> Result<String> {
    let mut parsed = record.parsed_observation.value()?;
    if record.provider.0 == "org.junk.providers.musicbrainz"
        && let Some(query) = parsed.get_mut("query").and_then(|v| v.as_object_mut())
    {
        query.remove("refresh");
    }
    Ok(Digest::sha256(
        &serde_json::to_vec(&serde_json::json!([
            record.collection_id,
            record.acquisition_key,
            record.provider,
            record.media_type,
            record.bytes,
            record.digest,
            parsed
        ]))
        .map_err(|e| Error::Invalid(e.to_string()))?,
    )
    .hex)
}
