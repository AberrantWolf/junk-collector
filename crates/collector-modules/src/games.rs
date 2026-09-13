use std::collections::BTreeMap;

use collector_core::{ByteLength, Digest, ModuleData, ModuleId, PackageFileId, RawJson};
use serde::{Deserialize, Serialize};

use crate::{
    CapabilityId, CollectionModule, DomainFieldDescriptor, DomainFieldKind, ModuleDescriptor,
    ModuleError, ModulePresentation, ModuleResult, OperationOutput, OperationRequest, PatchValue,
    apply_patch_value, basic_presentation, descriptor, module_data_from_raw_object,
    raw_domain_object,
};

#[derive(Clone, Debug, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GameItemData {
    pub platform: Option<String>,
    pub region: Option<String>,
    pub publisher: Option<String>,
    pub release_year: Option<u16>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GameItemPatch {
    pub platform: PatchValue<String>,
    pub region: PatchValue<String>,
    pub publisher: PatchValue<String>,
    pub release_year: PatchValue<u16>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GameMetadataOverrides {
    pub platform: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CatalogCandidate {
    pub provider: ModuleId,
    pub provider_record_id: String,
    pub label: String,
    pub matched_files: Vec<PackageFileId>,
    pub matched_digests: Vec<Digest>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "match", rename_all = "snake_case")]
pub enum CatalogMatchResult {
    NoMatch,
    UniqueCompleteMatch {
        candidate: CatalogCandidate,
    },
    AmbiguousCompleteMatches {
        candidates: Vec<CatalogCandidate>,
    },
    PartialIdentification {
        candidates: Vec<CatalogCandidate>,
        matched_files: u32,
        total_files: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NormalizedByteDomain {
    pub name: String,
    pub version: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GameFileDigest {
    pub file_id: PackageFileId,
    pub bytes: ByteLength,
    pub domain: NormalizedByteDomain,
    pub digests: Vec<Digest>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum GameStructureValue {
    Text(String),
    Unsigned(u64),
    Signed(i64),
    Boolean(bool),
    Bytes(ByteLength),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum GameProcessingResult {
    Hashes {
        files: Vec<GameFileDigest>,
    },
    Structure {
        format: String,
        fields: BTreeMap<String, GameStructureValue>,
    },
    CatalogMatch {
        catalog: CatalogMatchResult,
    },
}

pub struct Games;

impl CollectionModule for Games {
    fn descriptor(&self) -> ModuleDescriptor {
        descriptor(
            "org.junk.games",
            "Video games",
            vec![
                text_field("platform", "Platform"),
                text_field("region", "Region"),
                text_field("publisher", "Publisher"),
                DomainFieldDescriptor {
                    key: "release_year".into(),
                    label: "Release year".into(),
                    required: false,
                    editable: true,
                    kind: DomainFieldKind::Integer {
                        minimum: 0,
                        maximum: 9999,
                    },
                },
            ],
        )
    }

    fn validate_current(&self, data: &ModuleData) -> ModuleResult<()> {
        let item: GameItemData = data
            .payload
            .parse()
            .map_err(|error| ModuleError::Invalid(error.to_string()))?;
        validate_texts([&item.platform, &item.region, &item.publisher])?;
        validate_year(item.release_year)
    }

    fn apply_edit(&self, current: &ModuleData, edit: &RawJson) -> ModuleResult<ModuleData> {
        let patch: GameItemPatch = edit
            .parse()
            .map_err(|error| ModuleError::Invalid(error.to_string()))?;
        let mut object = raw_domain_object(current)?;
        apply_patch_value(&mut object, "platform", patch.platform)?;
        apply_patch_value(&mut object, "region", patch.region)?;
        apply_patch_value(&mut object, "publisher", patch.publisher)?;
        apply_patch_value(&mut object, "release_year", patch.release_year)?;
        module_data_from_raw_object(current, object)
    }

    fn validate_metadata_overrides(&self, overrides: &ModuleData) -> ModuleResult<()> {
        let value: GameMetadataOverrides = overrides
            .payload
            .parse()
            .map_err(|error| ModuleError::Invalid(error.to_string()))?;
        validate_texts([&value.platform])
    }

    fn validate_operation_result(
        &self,
        _capability: &CapabilityId,
        result: &RawJson,
    ) -> ModuleResult<()> {
        let result = result
            .parse::<GameProcessingResult>()
            .map_err(|error| ModuleError::Invalid(error.to_string()))?;
        validate_processing_result(&result)
    }

    fn present(&self, data: &ModuleData) -> ModuleResult<ModulePresentation> {
        self.validate_current(data)?;
        Ok(basic_presentation(data, self.descriptor()))
    }

    fn operate(
        &self,
        _request: OperationRequest,
        _host: &dyn crate::ModuleHost,
    ) -> ModuleResult<OperationOutput> {
        Err(ModuleError::UnavailableCapability(
            "game operation handler is not implemented".into(),
        ))
    }
}

fn text_field(key: &str, label: &str) -> DomainFieldDescriptor {
    DomainFieldDescriptor {
        key: key.into(),
        label: label.into(),
        required: false,
        editable: true,
        kind: DomainFieldKind::Text {
            maximum_bytes: 4096,
        },
    }
}

fn validate_texts<const N: usize>(values: [&Option<String>; N]) -> ModuleResult<()> {
    if values
        .into_iter()
        .flatten()
        .any(|value| value.len() > 4096 || value.chars().any(|character| character.is_control()))
    {
        return Err(ModuleError::Invalid(
            "game text is invalid or oversized".into(),
        ));
    }
    Ok(())
}

fn validate_year(value: Option<u16>) -> ModuleResult<()> {
    if value.is_some_and(|year| year > 9999) {
        return Err(ModuleError::Invalid("release year is out of bounds".into()));
    }
    Ok(())
}

fn validate_processing_result(result: &GameProcessingResult) -> ModuleResult<()> {
    match result {
        GameProcessingResult::Hashes { files } => {
            let mut ids = std::collections::BTreeSet::new();
            if files.is_empty()
                || files.len() > 4096
                || files.iter().any(|file| {
                    !ids.insert(file.file_id)
                        || file.domain.name.is_empty()
                        || file.domain.name.len() > 120
                        || file.domain.name.chars().any(char::is_control)
                        || file.domain.version == 0
                        || file.digests.is_empty()
                        || file.digests.len() > 8
                        || file.digests.iter().any(|digest| digest.validate().is_err())
                })
            {
                return Err(ModuleError::Invalid("invalid game hash result".into()));
            }
        }
        GameProcessingResult::Structure { format, fields } => {
            if format.is_empty()
                || format.len() > 120
                || format.chars().any(char::is_control)
                || fields.len() > 128
                || fields.iter().any(|(key, value)| {
                    key.is_empty()
                        || key.len() > 120
                        || key.chars().any(char::is_control)
                        || matches!(value, GameStructureValue::Text(text) if text.len() > 4096 || text.chars().any(char::is_control))
                })
            {
                return Err(ModuleError::Invalid(
                    "invalid game structure result".into(),
                ));
            }
        }
        GameProcessingResult::CatalogMatch { catalog } => validate_catalog_match(catalog)?,
    }
    Ok(())
}

fn validate_catalog_match(result: &CatalogMatchResult) -> ModuleResult<()> {
    let candidates: &[CatalogCandidate] = match result {
        CatalogMatchResult::NoMatch => return Ok(()),
        CatalogMatchResult::UniqueCompleteMatch { candidate } => std::slice::from_ref(candidate),
        CatalogMatchResult::AmbiguousCompleteMatches { candidates } => {
            let identities: std::collections::BTreeSet<_> = candidates
                .iter()
                .map(|candidate| (&candidate.provider.0, &candidate.provider_record_id))
                .collect();
            if identities.len() < 2 || identities.len() != candidates.len() {
                return Err(ModuleError::Invalid(
                    "ambiguous match requires distinct candidates".into(),
                ));
            }
            candidates
        }
        CatalogMatchResult::PartialIdentification {
            candidates,
            matched_files,
            total_files,
        } => {
            if *total_files == 0 || *matched_files == 0 || matched_files >= total_files {
                return Err(ModuleError::Invalid(
                    "invalid partial catalog match counts".into(),
                ));
            }
            candidates
        }
    };
    if candidates.is_empty()
        || candidates.len() > 128
        || candidates.iter().any(|candidate| {
            candidate.provider.validate().is_err()
                || candidate.provider_record_id.is_empty()
                || candidate.provider_record_id.len() > 1024
                || candidate.label.is_empty()
                || candidate.label.len() > 4096
                || candidate
                    .matched_files
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    != candidate.matched_files.len()
                || candidate.matched_files.is_empty()
                || candidate.matched_files.len() != candidate.matched_digests.len()
                || candidate
                    .matched_digests
                    .iter()
                    .any(|digest| digest.validate().is_err())
        })
    {
        return Err(ModuleError::Invalid("invalid catalog match result".into()));
    }
    Ok(())
}
