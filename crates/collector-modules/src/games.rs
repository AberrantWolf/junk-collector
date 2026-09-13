use std::collections::BTreeMap;

use collector_core::{
    ByteLength, Digest, HashAlgorithm, ModuleData, ModuleId, PackageFileId, PackageInventory,
    RawJson,
};
use junk_libs_game_formats::{CopierHeader, InesVersion, QualifiedHashes};
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
#[serde(deny_unknown_fields)]
pub struct GameTrackDigest {
    pub track_number: u8,
    pub source_file_id: PackageFileId,
    pub source_byte_offset: u64,
    pub bytes: ByteLength,
    pub synthetic_pregap_frames: u32,
    pub synthetic_postgap_frames: u32,
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
    TrackHashes {
        tracks: Vec<GameTrackDigest>,
    },
    Structure {
        format: String,
        fields: BTreeMap<String, GameStructureValue>,
    },
    CatalogMatch {
        catalog: CatalogMatchResult,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameHashProfile {
    Raw,
    InesPayload,
    N64BigEndian,
    SnesPayload { copier_header: bool },
    PcePayload { copier_header: bool },
}

fn module_hash_error(error: junk_libs_game_formats::AnalysisError) -> ModuleError {
    match error {
        junk_libs_game_formats::AnalysisError::UnsupportedVariant(message) => {
            ModuleError::UnavailableCapability(message)
        }
        error => ModuleError::FailedOperation(error.to_string()),
    }
}

fn digest_result(file_id: PackageFileId, hashes: QualifiedHashes) -> GameProcessingResult {
    GameProcessingResult::Hashes {
        files: vec![GameFileDigest {
            file_id,
            bytes: ByteLength(hashes.bytes),
            domain: NormalizedByteDomain {
                name: hashes.domain.name.into(),
                version: hashes.domain.version,
            },
            digests: vec![
                Digest::new(HashAlgorithm::Sha256, hashes.sha256).expect("shared SHA-256 is valid"),
                Digest::new(HashAlgorithm::Sha1, hashes.sha1).expect("shared SHA-1 is valid"),
                Digest::new(HashAlgorithm::Md5, hashes.md5).expect("shared MD5 is valid"),
                Digest::new(HashAlgorithm::Crc32, hashes.crc32).expect("shared CRC32 is valid"),
            ],
        }],
    }
}

/// Run one qualified reader-only normalization without owning filesystem or
/// catalog policy. The caller retains the raw master and provides its durable ID.
pub fn hash_game_file(
    file_id: PackageFileId,
    reader: &mut dyn crate::RepresentationReader,
    profile: GameHashProfile,
) -> ModuleResult<GameProcessingResult> {
    let hashes = match profile {
        GameHashProfile::Raw => junk_libs_game_formats::hash_raw(reader),
        GameHashProfile::InesPayload => {
            junk_libs_game_formats::hash_ines_payload(reader).map(|(_, hashes)| hashes)
        }
        GameHashProfile::N64BigEndian => {
            junk_libs_game_formats::hash_n64(reader).map(|(_, hashes)| hashes)
        }
        GameHashProfile::SnesPayload { copier_header } => {
            junk_libs_game_formats::hash_snes_payload(
                reader,
                if copier_header {
                    CopierHeader::Present512
                } else {
                    CopierHeader::Absent
                },
            )
        }
        GameHashProfile::PcePayload { copier_header } => junk_libs_game_formats::hash_pce_payload(
            reader,
            if copier_header {
                CopierHeader::Present512
            } else {
                CopierHeader::Absent
            },
        ),
    }
    .map_err(module_hash_error)?;
    Ok(digest_result(file_id, hashes))
}

/// Read iNES header structure as a separate observation from normalized hashes
/// and any later catalog match.
pub fn inspect_ines_structure(
    reader: &mut dyn crate::RepresentationReader,
) -> ModuleResult<GameProcessingResult> {
    let layout = junk_libs_game_formats::inspect_ines(reader).map_err(module_hash_error)?;
    Ok(GameProcessingResult::Structure {
        format: match layout.version {
            InesVersion::INes => "ines",
            InesVersion::Nes2 => "nes2",
        }
        .into(),
        fields: BTreeMap::from([
            (
                "mapper".into(),
                GameStructureValue::Unsigned(layout.mapper.into()),
            ),
            (
                "submapper".into(),
                GameStructureValue::Unsigned(layout.submapper.into()),
            ),
            (
                "prg_rom_bytes".into(),
                GameStructureValue::Bytes(ByteLength(layout.prg_rom_bytes)),
            ),
            (
                "chr_rom_bytes".into(),
                GameStructureValue::Bytes(ByteLength(layout.chr_rom_bytes)),
            ),
        ]),
    })
}

/// Hash every ordered raw-2352 track described by a CUE sheet. Package-relative
/// references resolve within the frozen authorized inventory; no adapter path is
/// accepted or returned.
pub fn hash_cue_package(
    cue_text: &str,
    cue_file_id: PackageFileId,
    inventory: &PackageInventory,
    mut open_file: impl FnMut(PackageFileId) -> ModuleResult<Box<dyn crate::RepresentationReader>>,
) -> ModuleResult<GameProcessingResult> {
    let resolve = |name: &str| {
        inventory
            .resolve_relative(cue_file_id, name)
            .map_err(|error| {
                junk_libs_game_formats::AnalysisError::InvalidFormat(error.to_string())
            })
    };
    let tracks = junk_libs_game_formats::hash_cue_raw_2352(
        cue_text,
        |name| resolve(name).map(|file| file.bytes.0),
        |name| {
            let file = resolve(name)?;
            open_file(file.id)
                .map_err(|error| junk_libs_game_formats::AnalysisError::Other(error.to_string()))
        },
    )
    .map_err(module_hash_error)?;
    Ok(GameProcessingResult::TrackHashes {
        tracks: tracks
            .into_iter()
            .map(|track| {
                let file = inventory
                    .resolve_relative(cue_file_id, &track.source_name)
                    .expect("shared CUE source was resolved above");
                GameTrackDigest {
                    track_number: track.track_number,
                    source_file_id: file.id,
                    source_byte_offset: track.source_byte_offset,
                    bytes: ByteLength(track.hashes.bytes),
                    synthetic_pregap_frames: track.synthetic_pregap_frames,
                    synthetic_postgap_frames: track.synthetic_postgap_frames,
                    domain: NormalizedByteDomain {
                        name: track.hashes.domain.name.into(),
                        version: track.hashes.domain.version,
                    },
                    digests: vec![
                        Digest::new(HashAlgorithm::Sha256, track.hashes.sha256)
                            .expect("shared SHA-256 is valid"),
                        Digest::new(HashAlgorithm::Sha1, track.hashes.sha1)
                            .expect("shared SHA-1 is valid"),
                        Digest::new(HashAlgorithm::Md5, track.hashes.md5)
                            .expect("shared MD5 is valid"),
                        Digest::new(HashAlgorithm::Crc32, track.hashes.crc32)
                            .expect("shared CRC32 is valid"),
                    ],
                }
            })
            .collect(),
    })
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
                        || (file.bytes.0 == 0 && file.domain.name != "org.junk.game.raw-bytes")
                        || !matches!(
                            (file.domain.name.as_str(), file.domain.version),
                            ("org.junk.game.raw-bytes", 1)
                                | ("org.junk.game.nes.ines-payload", 1)
                                | ("org.junk.game.n64.z64-big-endian", 1)
                                | ("org.junk.game.snes.headerless-payload", 1)
                                | ("org.junk.game.pce.hucard-payload", 1)
                        )
                        || !valid_qualified_digests(&file.digests)
                })
            {
                return Err(ModuleError::Invalid("invalid game hash result".into()));
            }
        }
        GameProcessingResult::TrackHashes { tracks } => {
            let mut previous = 0;
            let mut source_ends = BTreeMap::new();
            if tracks.is_empty()
                || tracks.len() > 99
                || tracks.iter().any(|track| {
                    let invalid = track.track_number <= previous
                        || track.track_number > 99
                        || track.bytes.0 == 0
                        || !track.bytes.0.is_multiple_of(2352)
                        || !track.source_byte_offset.is_multiple_of(2352)
                        || track.domain.name != "org.junk.game.cd.raw-2352-track"
                        || track.domain.version != 1
                        || !valid_qualified_digests(&track.digests);
                    let end = track.source_byte_offset.checked_add(track.bytes.0);
                    let overlap = source_ends.get(&track.source_file_id).is_some_and(|previous_end| track.source_byte_offset < *previous_end);
                    if let Some(end) = end { source_ends.insert(track.source_file_id, end); }
                    previous = track.track_number;
                    invalid || end.is_none() || overlap
                })
            {
                return Err(ModuleError::Invalid(
                    "invalid ordered game track hash result".into(),
                ));
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

fn valid_qualified_digests(digests: &[Digest]) -> bool {
    if digests.len() != 4 || digests.iter().any(|digest| digest.validate().is_err()) {
        return false;
    }
    [
        HashAlgorithm::Sha256,
        HashAlgorithm::Sha1,
        HashAlgorithm::Md5,
        HashAlgorithm::Crc32,
    ]
    .into_iter()
    .all(|algorithm| {
        digests
            .iter()
            .filter(|digest| digest.algorithm == algorithm)
            .count()
            == 1
    })
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
