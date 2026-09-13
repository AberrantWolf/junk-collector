use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{AssetId, Error, LanguageTag, ModuleData, ModuleId, RawJson, Result};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MetadataCandidateIdentity {
    pub provider: ModuleId,
    pub provider_record_id: String,
    pub provider_record_version: Option<String>,
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
        if let Some(candidate) = &self.selected_candidate {
            candidate.provider.validate()?;
            if candidate.provider_record_id.is_empty()
                || candidate.provider_record_id.len() > 1024
                || candidate.provider_record_id.chars().any(char::is_control)
                || candidate
                    .provider_record_version
                    .as_ref()
                    .is_some_and(|version| {
                        version.is_empty()
                            || version.len() > 1024
                            || version.chars().any(char::is_control)
                    })
            {
                return Err(Error::Invalid("invalid metadata candidate identity".into()));
            }
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
