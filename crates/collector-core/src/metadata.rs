use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    AssetId, Error, LanguageTag, MatchStrength, ModuleData, ModuleId, ObservationId, RawJson,
    Result,
};

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, utoipa::ToSchema,
)]
#[serde(deny_unknown_fields)]
pub struct MetadataCandidateIdentity {
    pub provider: ModuleId,
    pub provider_record_id: String,
    pub provider_record_version: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MetadataText {
    pub value: String,
    pub language: Option<LanguageTag>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MetadataCandidateObservation {
    pub identity: MetadataCandidateIdentity,
    pub title: MetadataText,
    pub subtitle: Option<MetadataText>,
    pub artwork: Option<AssetId>,
    /// A provider's complete identity assertion is ranked before all partial
    /// identifier or layout evidence. It is descriptive identity evidence, not
    /// integrity or catalog verification.
    pub complete_identity: bool,
    pub strength: MatchStrength,
    /// Lower values are preferred. Policy supplies this value; array order never
    /// participates in resolution.
    pub provider_priority: u16,
    #[schema(value_type = Object)]
    pub observation: RawJson,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MetadataRefreshOutcome {
    Candidates,
    NoMatch,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MetadataRefreshObservation {
    pub id: ObservationId,
    pub outcome: MetadataRefreshOutcome,
    pub candidates: Vec<MetadataCandidateObservation>,
    #[schema(value_type = Object)]
    pub raw_observation: RawJson,
    pub observed_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum MetadataValueSource {
    Manual,
    Provisional,
    Candidate { identity: MetadataCandidateIdentity },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolvedMetadataText {
    pub value: String,
    pub language: Option<LanguageTag>,
    pub source: MetadataValueSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MetadataResolutionKind {
    Manual,
    Selected,
    Automatic,
    Tentative,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolvedMetadataDisplay {
    #[serde(default)]
    pub domain_fields: std::collections::BTreeMap<String, ResolvedMetadataText>,
    pub title: ResolvedMetadataText,
    pub subtitle: Option<ResolvedMetadataText>,
    pub artwork: Option<AssetId>,
    pub candidate: Option<MetadataCandidateIdentity>,
    pub resolution: MetadataResolutionKind,
}

impl MetadataCandidateObservation {
    pub fn validate(&self) -> Result<()> {
        validate_candidate_identity(&self.identity)?;
        validate_text(&self.title.value, false)?;
        if let Some(subtitle) = &self.subtitle {
            validate_text(&subtitle.value, false)?;
        }
        if self.observation.get().len() > 64 * 1024 {
            return Err(Error::Invalid(
                "metadata candidate observation is oversized".into(),
            ));
        }
        Ok(())
    }
}

impl MetadataRefreshObservation {
    pub fn validate(&self) -> Result<()> {
        if self.candidates.len() > 128 || self.raw_observation.get().len() > 8 * 1024 * 1024 {
            return Err(Error::Invalid("metadata refresh is oversized".into()));
        }
        if matches!(self.outcome, MetadataRefreshOutcome::Candidates) == self.candidates.is_empty()
        {
            return Err(Error::Invalid(
                "metadata refresh outcome disagrees with its candidates".into(),
            ));
        }
        let mut identities = BTreeSet::new();
        for candidate in &self.candidates {
            candidate.validate()?;
            if !identities.insert(candidate.identity.clone()) {
                return Err(Error::Invalid(
                    "metadata refresh contains a duplicate candidate".into(),
                ));
            }
        }
        Ok(())
    }
}

impl ResolvedMetadataDisplay {
    pub fn validate(&self) -> Result<()> {
        if self.domain_fields.len() > 32 {
            return Err(Error::Invalid("too many resolved domain fields".into()));
        }
        for (key, value) in &self.domain_fields {
            validate_text(key, false)?;
            validate_text(&value.value, false)?;
            validate_value_source(&value.source)?;
        }
        validate_text(&self.title.value, false)?;
        validate_value_source(&self.title.source)?;
        if let Some(subtitle) = &self.subtitle {
            validate_text(&subtitle.value, false)?;
            validate_value_source(&subtitle.source)?;
        }
        if let Some(candidate) = &self.candidate {
            validate_candidate_identity(candidate)?;
        }
        Ok(())
    }
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, utoipa::ToSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum MetadataField {
    Title,
    Subtitle,
}

/// Common fields are text, not arbitrary JSON. Artwork has exactly one authority
/// slot on MetadataAuthority; unknown legacy values remain in its raw envelope.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MetadataFieldOverride {
    pub field: MetadataField,
    pub value: String,
    pub language: Option<LanguageTag>,
}

#[derive(Clone, Debug, Default, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MetadataAuthority {
    /// `None` means automatic candidate selection.
    pub selected_candidate: Option<MetadataCandidateIdentity>,
    /// Last accepted automatic decision, distinct from explicit user selection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automatic_candidate: Option<MetadataCandidateIdentity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automatic_language: Option<LanguageTag>,
    pub field_overrides: Vec<MetadataFieldOverride>,
    /// Unrecognized historical field authority, retained verbatim and never applied
    /// as common or domain authority without an explicit owner-aware conversion.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schema(value_type = Vec<serde_json::Value>)]
    pub preserved_field_overrides: Vec<RawJson>,
    pub artwork_choice: Option<AssetId>,
    /// Typed domain authority owned by the collection module. Core preserves the
    /// envelope; registry dispatch validates its owner and schema semantics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain_overrides: Option<ModuleData>,
    /// Legacy/provider-specific authority keys remain exact and portable.
    #[schema(value_type = Object)]
    pub retained_unknown_fields: BTreeMap<String, RawJson>,
}

impl<'de> Deserialize<'de> for MetadataAuthority {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            selected_candidate: Option<MetadataCandidateIdentity>,
            #[serde(default)]
            automatic_candidate: Option<MetadataCandidateIdentity>,
            #[serde(default)]
            automatic_language: Option<LanguageTag>,
            field_overrides: Vec<RawJson>,
            #[serde(default)]
            preserved_field_overrides: Vec<RawJson>,
            artwork_choice: Option<AssetId>,
            #[serde(default)]
            domain_overrides: Option<ModuleData>,
            retained_unknown_fields: BTreeMap<String, RawJson>,
        }
        #[derive(Deserialize)]
        struct Field {
            field: String,
        }
        let wire = Wire::deserialize(deserializer)?;
        let mut result = Self {
            selected_candidate: wire.selected_candidate,
            automatic_candidate: wire.automatic_candidate,
            automatic_language: wire.automatic_language,
            field_overrides: Vec::new(),
            preserved_field_overrides: wire.preserved_field_overrides,
            artwork_choice: wire.artwork_choice,
            domain_overrides: wire.domain_overrides,
            retained_unknown_fields: wire.retained_unknown_fields,
        };
        for raw in wire.field_overrides {
            let field: Field = raw.parse().map_err(serde::de::Error::custom)?;
            match field.field.as_str() {
                "title" | "subtitle" => result
                    .field_overrides
                    .push(raw.parse().map_err(serde::de::Error::custom)?),
                _ => result.preserved_field_overrides.push(raw),
            }
        }
        result.validate().map_err(serde::de::Error::custom)?;
        Ok(result)
    }
}

impl MetadataAuthority {
    pub fn validate(&self) -> Result<()> {
        for candidate in self
            .selected_candidate
            .iter()
            .chain(self.automatic_candidate.iter())
        {
            validate_candidate_identity(candidate)?;
        }
        if self.preserved_field_overrides.len() > 128
            || self
                .preserved_field_overrides
                .iter()
                .any(|value| value.get().len() > 64 * 1024)
        {
            return Err(Error::Invalid(
                "preserved field overrides are oversized".into(),
            ));
        }
        let mut fields = BTreeSet::new();
        if self.field_overrides.len() > 2
            || self.field_overrides.iter().any(|entry| {
                !fields.insert(entry.field)
                    || (entry.field == MetadataField::Title && entry.value.trim().is_empty())
                    || entry.value.len() > 4096
                    || entry.value.chars().any(char::is_control)
            })
        {
            return Err(Error::Invalid(
                "duplicate or invalid metadata override".into(),
            ));
        }
        if let Some(overrides) = &self.domain_overrides {
            overrides.module.validate()?;
            if overrides.schema_version == 0 || overrides.payload.get().len() > 1024 * 1024 {
                return Err(Error::Invalid(
                    "domain metadata overrides are invalid or oversized".into(),
                ));
            }
        }
        if self.retained_unknown_fields.len() > 128
            || self.retained_unknown_fields.iter().any(|(key, value)| {
                key.is_empty() || key.len() > 255 || value.get().len() > 1024 * 1024
            })
        {
            return Err(Error::Invalid(
                "unknown metadata authority is oversized".into(),
            ));
        }
        Ok(())
    }
}

fn validate_candidate_identity(candidate: &MetadataCandidateIdentity) -> Result<()> {
    candidate.provider.validate()?;
    if candidate.provider_record_id.is_empty()
        || candidate.provider_record_id.len() > 1024
        || candidate.provider_record_id.chars().any(char::is_control)
        || candidate
            .provider_record_version
            .as_ref()
            .is_some_and(|version| {
                version.is_empty() || version.len() > 1024 || version.chars().any(char::is_control)
            })
    {
        return Err(Error::Invalid("invalid metadata candidate identity".into()));
    }
    Ok(())
}

fn validate_value_source(source: &MetadataValueSource) -> Result<()> {
    match source {
        MetadataValueSource::Manual | MetadataValueSource::Provisional => Ok(()),
        MetadataValueSource::Candidate { identity } => validate_candidate_identity(identity),
    }
}

fn validate_text(value: &str, empty_allowed: bool) -> Result<()> {
    if (!empty_allowed && value.trim().is_empty())
        || value.len() > 4096
        || value.chars().any(char::is_control)
    {
        return Err(Error::Invalid("invalid metadata display text".into()));
    }
    Ok(())
}
