mod systems;
pub use systems::*;
mod identification;
use collector_core::PackageFileName;
pub use identification::*;
use std::{collections::BTreeMap, io::Read};

use collector_core::{
    ByteLength, Digest, Error, HashAlgorithm, MatchStrength, MetadataCandidateIdentity,
    MetadataCandidateObservation, MetadataText, ModuleData, ModuleId, PackageFileId,
    PackageInventory, RawJson,
};
use junk_libs_game_formats::{CopierHeader, InesVersion, QualifiedHashes};
use serde::{Deserialize, Serialize};

use crate::{
    Availability, Capability, CapabilityId, CollectionModule, DomainFieldDescriptor,
    DomainFieldKind, ModuleDescriptor, ModuleError, ModulePresentation, ModuleResult,
    OperationEntity, OperationOutput, OperationRequest, OperationScope, PackageAccess, PatchValue,
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
    #[serde(default)]
    pub automatic_platform: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CatalogCandidate {
    pub provider: ModuleId,
    pub provider_record_id: String,
    pub label: String,
    pub matched_units: Vec<CatalogMatchedUnit>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CatalogMatchedUnit {
    pub source_file_id: PackageFileId,
    pub track_number: Option<u8>,
    pub digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "match", rename_all = "snake_case")]
pub enum CatalogMatchResult {
    Inconclusive {
        reason: String,
    },
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
    HeaderlessInes,
    InesPayload,
    N64BigEndian,
    SnesPayload { copier_header: bool },
    PcePayload { copier_header: bool },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "profile", rename_all = "snake_case", deny_unknown_fields)]
enum GameHashSelection {
    Raw,
    InesPayload,
    N64BigEndian,
    SnesPayload { copier_header: bool },
    PcePayload { copier_header: bool },
}

impl From<GameHashSelection> for GameHashProfile {
    fn from(value: GameHashSelection) -> Self {
        match value {
            GameHashSelection::Raw => Self::Raw,
            GameHashSelection::InesPayload => Self::InesPayload,
            GameHashSelection::N64BigEndian => Self::N64BigEndian,
            GameHashSelection::SnesPayload { copier_header } => Self::SnesPayload { copier_header },
            GameHashSelection::PcePayload { copier_header } => Self::PcePayload { copier_header },
        }
    }
}

#[derive(Serialize, Deserialize)]
struct GameFileOperationInput {
    file_index: usize,
    #[serde(flatten)]
    profile: GameHashSelection,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GameCueOperationInput {
    cue_file_index: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GameInspectOperationInput {
    file_index: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "input", rename_all = "snake_case", deny_unknown_fields)]
enum CatalogAnalysisSelection {
    RawFiles,
    File {
        file_index: usize,
        #[serde(flatten)]
        profile: GameHashSelection,
    },
    CueRaw2352 {
        cue_file_index: usize,
    },
}

#[derive(Serialize, Deserialize)]
struct CatalogVerificationInput {
    snapshot_id: collector_core::SnapshotId,
    system: String,
    #[serde(flatten)]
    analysis: CatalogAnalysisSelection,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportAnalysisInput {
    #[serde(default)]
    snapshot_id: Option<collector_core::SnapshotId>,
    #[serde(default)]
    system: Option<String>,
    #[serde(default)]
    domain: Option<ModuleData>,
    #[serde(default)]
    import_id: Option<collector_core::ImportId>,
    #[serde(default)]
    representation_id: Option<collector_core::RepresentationId>,
    #[serde(default)]
    files: Vec<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GameImportCandidateOutput {
    key: String,
    label: String,
    observation: RawJson,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameImportAnalysisOutput {
    candidates: Vec<GameImportCandidateOutput>,
    limitations: Vec<String>,
    pub observations: Vec<RawJson>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DatSnapshotObservation {
    pub schema_version: u32,
    pub system: String,
    pub catalog: String,
    #[serde(default)]
    pub document_name: String,
    #[serde(default)]
    pub description: String,
    pub version: String,
    pub byte_domain: String,
    pub byte_domain_version: u32,
    pub game_count: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preview_entries: Vec<DatCatalogPreviewEntry>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DatCatalogPreviewEntry {
    pub label: String,
    pub files: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DatCatalogEntryObservation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    pub record_id: String,
    pub label: String,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub serial: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    pub files: Vec<DatCatalogFileObservation>,
    /// Provider files retained from the source row but excluded from matching
    /// because they are outside the configured byte domain. For example, a
    /// Redump CUE checksum is not a raw-2352 track checksum.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excluded_files: Vec<DatCatalogFileObservation>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DatCatalogFileObservation {
    pub name: String,
    pub bytes: ByteLength,
    #[serde(default)]
    pub serial: Option<String>,
    pub crc32: Option<String>,
    pub sha1: Option<String>,
    pub md5: Option<String>,
}

pub const LOCAL_DAT_METADATA_PROVIDER_PRIORITY: u16 = 10;

/// Materialize descriptive DAT rows selected by the qualified D04 match result.
/// Exact catalog bytes remain in their Snapshot; the snapshot digest also enters
/// the candidate version so changed raw input cannot reuse an old identity.
pub fn dat_metadata_candidates(
    provider: ModuleId,
    snapshot_digest: &Digest,
    entries: &[RawJson],
    result: &CatalogMatchResult,
) -> collector_core::Result<Vec<MetadataCandidateObservation>> {
    provider.validate()?;
    snapshot_digest.validate()?;
    validate_catalog_match(result).map_err(|error| Error::Invalid(error.to_string()))?;
    let (matches, complete_identity, strength): (&[CatalogCandidate], bool, MatchStrength) =
        match result {
            CatalogMatchResult::UniqueCompleteMatch { candidate } => (
                std::slice::from_ref(candidate),
                true,
                MatchStrength::CompleteHashes,
            ),
            CatalogMatchResult::AmbiguousCompleteMatches { candidates } => {
                (candidates, true, MatchStrength::CompleteHashes)
            }
            CatalogMatchResult::PartialIdentification { candidates, .. } => {
                (candidates, false, MatchStrength::Identifier)
            }
            CatalogMatchResult::Inconclusive { .. } | CatalogMatchResult::NoMatch => {
                return Ok(Vec::new());
            }
        };
    let rows = entries
        .iter()
        .map(|raw| {
            let row: DatCatalogEntryObservation = raw.parse()?;
            validate_dat_entry(&row).map_err(|error| Error::Invalid(error.to_string()))?;
            Ok((row.record_id.clone(), (row, raw)))
        })
        .collect::<collector_core::Result<BTreeMap<_, _>>>()?;
    let mut candidates = Vec::with_capacity(matches.len());
    for matched in matches {
        if matched.provider != provider {
            return Err(Error::Invalid(
                "DAT metadata candidate belongs to another provider".into(),
            ));
        }
        let (row, raw) = rows
            .get(&matched.provider_record_id)
            .ok_or_else(|| Error::Invalid("matched DAT metadata row is absent".into()))?;
        if matched.label != row.label {
            return Err(Error::Invalid(
                "matched DAT metadata label disagrees with its frozen row".into(),
            ));
        }
        let mut version_input = snapshot_digest.hex.as_bytes().to_vec();
        version_input.push(0);
        version_input.extend_from_slice(raw.get().as_bytes());
        version_input.extend_from_slice(
            &serde_json::to_vec(&(complete_identity, strength))
                .map_err(|error| Error::Invalid(error.to_string()))?,
        );
        let version = Digest::sha256(&version_input);
        let subtitle = [row.region.as_deref(), row.version.as_deref()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" · ");
        candidates.push(MetadataCandidateObservation {
            identity: MetadataCandidateIdentity {
                provider: provider.clone(),
                provider_record_id: row.record_id.clone(),
                provider_record_version: Some(format!("sha256:{}", version.hex)),
            },
            title: MetadataText {
                value: row.label.clone(),
                language: None,
            },
            subtitle: (!subtitle.is_empty()).then_some(MetadataText {
                value: subtitle,
                language: None,
            }),
            artwork: None,
            complete_identity,
            strength,
            provider_priority: LOCAL_DAT_METADATA_PROVIDER_PRIORITY,
            observation: (*raw).clone(),
        });
    }
    candidates.sort_by(|left, right| left.identity.cmp(&right.identity));
    Ok(candidates)
}

#[derive(Clone)]
struct CatalogInputUnit<'a> {
    source_file_id: PackageFileId,
    track_number: Option<u8>,
    bytes: ByteLength,
    domain: &'a NormalizedByteDomain,
    digests: &'a [Digest],
}

/// Match qualified per-file hashes against a provider-parsed DAT snapshot.
/// Completeness requires the declared system and normalized byte domain, the
/// same number of files, and an ordered size/checksum match for every file.
pub fn match_dat_file_catalog<'a>(
    provider: ModuleId,
    expected_system: &str,
    snapshot: &RawJson,
    entries: impl IntoIterator<Item = &'a RawJson>,
    files: &[GameFileDigest],
) -> ModuleResult<GameProcessingResult> {
    let units = files
        .iter()
        .map(|file| CatalogInputUnit {
            source_file_id: file.file_id,
            track_number: None,
            bytes: file.bytes,
            domain: &file.domain,
            digests: &file.digests,
        })
        .collect::<Vec<_>>();
    match_dat_catalog(
        provider,
        expected_system,
        snapshot,
        entries,
        &units,
        &mut || Ok(()),
    )
}

/// Match qualified ordered raw-2352 track hashes. Track order is part of the
/// completeness decision even when every checksum appears elsewhere in a DAT row.
pub fn match_dat_track_catalog<'a>(
    provider: ModuleId,
    expected_system: &str,
    snapshot: &RawJson,
    entries: impl IntoIterator<Item = &'a RawJson>,
    tracks: &[GameTrackDigest],
) -> ModuleResult<GameProcessingResult> {
    let units = tracks
        .iter()
        .map(|track| CatalogInputUnit {
            source_file_id: track.source_file_id,
            track_number: Some(track.track_number),
            bytes: track.bytes,
            domain: &track.domain,
            digests: &track.digests,
        })
        .collect::<Vec<_>>();
    match_dat_catalog(
        provider,
        expected_system,
        snapshot,
        entries,
        &units,
        &mut || Ok(()),
    )
}

fn match_dat_catalog<'a>(
    provider: ModuleId,
    expected_system: &str,
    snapshot: &RawJson,
    entries: impl IntoIterator<Item = &'a RawJson>,
    units: &[CatalogInputUnit<'_>],
    checkpoint: &mut dyn FnMut() -> ModuleResult<()>,
) -> ModuleResult<GameProcessingResult> {
    provider
        .validate()
        .map_err(|error| ModuleError::Invalid(error.to_string()))?;
    let header: DatSnapshotObservation = snapshot.parse().map_err(|error| {
        ModuleError::Invalid(format!("invalid DAT snapshot observation: {error}"))
    })?;
    let safe_header_text = |value: &str, maximum: usize, allow_empty: bool| {
        (allow_empty || !value.trim().is_empty())
            && value.len() <= maximum
            && !value.chars().any(char::is_control)
    };
    if header.schema_version != 1
        || !safe_header_text(&header.system, 120, false)
        || !safe_header_text(&header.catalog, 255, false)
        || !safe_header_text(&header.document_name, 4096, true)
        || !safe_header_text(&header.description, 4096, true)
        || !safe_header_text(&header.version, 4096, false)
        || !safe_header_text(&header.byte_domain, 120, false)
        || header.byte_domain_version == 0
        || header.game_count == 0
        || units.is_empty()
        || units.len() > 4096
    {
        return Err(ModuleError::Invalid(
            "invalid DAT snapshot or match input".into(),
        ));
    }
    if header.system != expected_system {
        return catalog_result(CatalogMatchResult::Inconclusive {
            reason: format!(
                "snapshot system {} does not match requested system {}",
                header.system, expected_system
            ),
        });
    }
    if units.iter().any(|unit| {
        unit.domain.name != header.byte_domain
            || unit.domain.version != header.byte_domain_version
            || unit.digests.iter().any(|digest| digest.validate().is_err())
    }) {
        return catalog_result(CatalogMatchResult::Inconclusive {
            reason: "snapshot and analyzed bytes use different normalization domains".into(),
        });
    }

    let mut complete = Vec::new();
    let mut partial = Vec::new();
    let mut saw_usable_checksums = false;
    let mut unresolved_row = false;
    let mut seen = 0_u64;
    for entry in entries {
        checkpoint()?;
        seen = seen.saturating_add(1);
        if seen > header.game_count {
            return Err(ModuleError::Invalid(
                "DAT index contains more rows than its snapshot declares".into(),
            ));
        }
        let entry: DatCatalogEntryObservation = entry
            .parse()
            .map_err(|error| ModuleError::Invalid(format!("invalid DAT catalog row: {error}")))?;
        validate_dat_entry(&entry)?;
        for (file, included) in entry
            .files
            .iter()
            .map(|file| (file, true))
            .chain(entry.excluded_files.iter().map(|file| (file, false)))
        {
            if dat_file_in_byte_domain(file, &header.byte_domain)? != included {
                return Err(ModuleError::Invalid(
                    "DAT row has inconsistent byte-domain coverage".into(),
                ));
            }
        }
        saw_usable_checksums |= entry.files.iter().any(has_usable_checksum);

        let ordered = entry.files.len() == units.len()
            && entry
                .files
                .iter()
                .zip(units)
                .all(|(file, unit)| match_file(file, unit).is_some());
        // A plausible row with absent checksums cannot be ruled out by a
        // different row's usable but nonmatching checksum.
        unresolved_row |= !ordered
            && entry.files.len() == units.len()
            && entry.files.iter().zip(units).all(|(file, unit)| {
                file.bytes == unit.bytes
                    && [
                        (HashAlgorithm::Sha1, file.sha1.as_deref()),
                        (HashAlgorithm::Md5, file.md5.as_deref()),
                        (HashAlgorithm::Crc32, file.crc32.as_deref()),
                    ]
                    .into_iter()
                    .all(|(algorithm, expected)| {
                        expected.is_none_or(|expected| {
                            unit.digests
                                .iter()
                                .find(|digest| digest.algorithm == algorithm)
                                .is_none_or(|digest| digest.hex == expected)
                        })
                    })
            });
        let mut matched = Vec::new();
        let mut claimed = std::collections::BTreeSet::new();
        for unit in units {
            checkpoint()?;
            let found = entry.files.iter().enumerate().find_map(|(index, file)| {
                (!claimed.contains(&index))
                    .then(|| match_file(file, unit).map(|digest| (index, digest)))
                    .flatten()
            });
            if let Some((index, digest)) = found {
                claimed.insert(index);
                matched.push(CatalogMatchedUnit {
                    source_file_id: unit.source_file_id,
                    track_number: unit.track_number,
                    digest,
                });
            }
        }
        if matched.is_empty() {
            continue;
        }
        let candidate = CatalogCandidate {
            provider: provider.clone(),
            provider_record_id: entry.record_id,
            label: entry.label,
            matched_units: matched,
        };
        if ordered {
            complete.push(candidate);
        } else {
            partial.push(candidate);
        }
        if complete.len() > 128 || partial.len() > 128 {
            return Err(ModuleError::Invalid("too many DAT match candidates".into()));
        }
    }
    if seen != header.game_count {
        return Err(ModuleError::Invalid(
            "DAT index row count does not match its snapshot".into(),
        ));
    }
    let result = match complete.len() {
        1 => CatalogMatchResult::UniqueCompleteMatch {
            candidate: complete.remove(0),
        },
        count if count > 1 => CatalogMatchResult::AmbiguousCompleteMatches {
            candidates: complete,
        },
        _ if !partial.is_empty() => CatalogMatchResult::PartialIdentification {
            matched_files: partial
                .iter()
                .map(|candidate| candidate.matched_units.len())
                .max()
                .unwrap_or(0) as u32,
            total_files: units.len() as u32,
            candidates: partial,
        },
        _ if !saw_usable_checksums || unresolved_row => CatalogMatchResult::Inconclusive {
            reason: "insufficient supported checksums to exclude an unresolved catalog row".into(),
        },
        _ => CatalogMatchResult::NoMatch,
    };
    catalog_result(result)
}

fn catalog_result(catalog: CatalogMatchResult) -> ModuleResult<GameProcessingResult> {
    let result = GameProcessingResult::CatalogMatch { catalog };
    validate_processing_result(&result)?;
    Ok(result)
}

/// Domain policy for catalog coverage. Only CUE metadata may be excluded from
/// raw-2352 track matching; a malformed or unknown payload must fail closed.
pub fn dat_file_in_byte_domain(
    file: &DatCatalogFileObservation,
    domain: &str,
) -> ModuleResult<bool> {
    if domain != "org.junk.game.cd.raw-2352-track" {
        return Ok(true);
    }
    let extension = file.name.rsplit_once('.').map(|(_, extension)| extension);
    if extension.is_some_and(|value| value.eq_ignore_ascii_case("cue")) {
        return Ok(false);
    }
    if !extension.is_some_and(|value| value.eq_ignore_ascii_case("bin"))
        || file.bytes.0 == 0
        || !file.bytes.0.is_multiple_of(2352)
    {
        return Err(ModuleError::Invalid(
            "raw-2352 DAT contains an unsupported or malformed track".into(),
        ));
    }
    Ok(true)
}

fn validate_dat_entry(entry: &DatCatalogEntryObservation) -> ModuleResult<()> {
    let safe = |value: &str, maximum| {
        !value.trim().is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
    };
    if !safe(&entry.record_id, 1024)
        || !safe(&entry.label, 4096)
        || entry.files.is_empty()
        || entry.files.len().saturating_add(entry.excluded_files.len()) > 4096
        || entry.files.iter().chain(&entry.excluded_files).any(|file| {
            !safe(&file.name, 4096)
                || file.serial.as_ref().is_some_and(|value| !safe(value, 4096))
                || file
                    .crc32
                    .as_ref()
                    .is_some_and(|value| !valid_hex(value, 8))
                || file
                    .sha1
                    .as_ref()
                    .is_some_and(|value| !valid_hex(value, 40))
                || file.md5.as_ref().is_some_and(|value| !valid_hex(value, 32))
        })
    {
        return Err(ModuleError::Invalid("invalid DAT catalog entry".into()));
    }
    if [
        entry.region.as_deref(),
        entry.serial.as_deref(),
        entry.version.as_deref(),
        entry.category.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|value| !safe(value, 4096))
    {
        return Err(ModuleError::Invalid("invalid DAT catalog metadata".into()));
    }
    Ok(())
}

fn valid_hex(value: &str, digits: usize) -> bool {
    value.len() == digits
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        && value.bytes().all(|byte| !byte.is_ascii_uppercase())
}

fn has_usable_checksum(file: &DatCatalogFileObservation) -> bool {
    file.sha1.is_some() || file.md5.is_some() || file.crc32.is_some()
}

fn match_file(file: &DatCatalogFileObservation, unit: &CatalogInputUnit<'_>) -> Option<Digest> {
    if file.bytes != unit.bytes {
        return None;
    }
    let declared = [
        (HashAlgorithm::Sha1, file.sha1.as_deref()),
        (HashAlgorithm::Md5, file.md5.as_deref()),
        (HashAlgorithm::Crc32, file.crc32.as_deref()),
    ];
    if !declared.iter().any(|(_, expected)| expected.is_some())
        || declared.iter().any(|(algorithm, expected)| {
            expected.is_some_and(|expected| {
                !unit
                    .digests
                    .iter()
                    .any(|digest| digest.algorithm == *algorithm && digest.hex == expected)
            })
        })
    {
        return None;
    }
    declared.into_iter().find_map(|(algorithm, expected)| {
        expected.and_then(|_| {
            unit.digests
                .iter()
                .find(|digest| digest.algorithm == algorithm)
                .cloned()
        })
    })
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
        GameHashProfile::HeaderlessInes => junk_libs_game_formats::hash_headerless_ines(reader),
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
    fn scan_inputs(
        &self,
        operation: &CapabilityId,
        inventory: &PackageInventory,
        input: &RawJson,
    ) -> ModuleResult<crate::scan_inputs::ScanInputs> {
        let mut plan = crate::scan_inputs::ScanInputs::new(input)?;
        match operation.as_str() {
            "org.junk.games.identify" => {
                let _: Vec<collector_core::SnapshotId> = input
                    .parse::<serde_json::Value>()
                    .map_err(|e| ModuleError::Invalid(e.to_string()))?["snapshot_ids"]
                    .as_array()
                    .ok_or_else(|| ModuleError::Invalid("snapshot_ids required".into()))?
                    .iter()
                    .map(|v| {
                        serde_json::from_value(v.clone())
                            .map_err(|e| ModuleError::Invalid(e.to_string()))
                    })
                    .collect::<ModuleResult<_>>()?;
            }
            "org.junk.games.hash" => {
                plan.file(inventory, "file_index", |_| true)?;
                plan.normalize::<GameFileOperationInput>()?;
            }
            "org.junk.games.inspect-ines" => {
                // The user explicitly chose iNES inspection; filenames are not proof of format.
                plan.file(inventory, "file_index", |_| true)?;
                plan.normalize::<GameInspectOperationInput>()?;
            }
            "org.junk.games.hash-tracks" => {
                plan.file(inventory, "cue_file_index", |n| {
                    n.to_ascii_lowercase().ends_with(".cue")
                })?;
                plan.normalize::<GameCueOperationInput>()?;
            }
            "org.junk.games.catalog-verification" => {
                let value: serde_json::Value = input
                    .parse()
                    .map_err(|e| ModuleError::Invalid(e.to_string()))?;
                match value["input"].as_str() {
                    Some("file") => plan.file(inventory, "file_index", |_| true)?,
                    Some("cue_raw2352") => plan.file(inventory, "cue_file_index", |n| {
                        n.to_ascii_lowercase().ends_with(".cue")
                    })?,
                    _ => {}
                }
                plan.normalize::<CatalogVerificationInput>()?;
                if plan.blockers.is_empty() {
                    let parsed: CatalogVerificationInput = plan
                        .selection
                        .parse()
                        .map_err(|e| ModuleError::Invalid(e.to_string()))?;
                    if parsed.system.trim().is_empty() || parsed.system.len() > 120 {
                        plan.blockers.push("Choose the catalog system.".into());
                    }
                }
            }
            "org.junk.games.import-analysis" => {
                let _: ImportAnalysisInput = input
                    .parse()
                    .map_err(|e| ModuleError::Invalid(e.to_string()))?;
            }
            _ => return Err(ModuleError::UnknownCapability(operation.to_string())),
        }
        let selected: serde_json::Value = plan
            .selection
            .value()
            .map_err(|e| ModuleError::Invalid(e.to_string()))?;
        if selected.get("cue_file_index").is_some() && inventory.files.len() < 2 {
            plan.blockers
                .push("A CUE check requires its referenced data files.".into());
        }
        if (operation.as_str() == "org.junk.games.inspect-ines"
            || selected["profile"] == "ines_payload")
            && let Some(index) = selected["file_index"]
                .as_u64()
                .and_then(|i| usize::try_from(i).ok())
            && inventory
                .files
                .get(index)
                .is_some_and(|file| file.bytes.0 < 16)
        {
            plan.blockers
                .push("An iNES header requires at least 16 bytes.".into());
        }
        if inventory.files.is_empty() {
            plan.blockers.push("The package has no files.".into());
        }
        Ok(plan)
    }

    fn discover_imports(
        &self,
        names: &[PackageFileName],
    ) -> Vec<crate::import_discovery::DiscoveredImportPackage> {
        crate::import_discovery::discover(names, true)
    }
    fn import_readiness(
        &self,
        _names: &[PackageFileName],
        _anchor_file_index: Option<usize>,
    ) -> crate::ImportReadiness {
        // Game analysis consumes the complete inventory; it has no mandatory
        // user-selected anchor. The registered analyzer reports format support.
        crate::ImportReadiness {
            anchors: vec![],
            blockers: vec![],
        }
    }

    fn descriptor(&self) -> ModuleDescriptor {
        let mut descriptor = descriptor(
            "org.junk.games",
            "Video games",
            vec![
                DomainFieldDescriptor {
                    key: "platform".into(),
                    label: "System".into(),
                    required: false,
                    editable: true,
                    kind: DomainFieldKind::Choice {
                        values: standard_game_systems()
                            .into_iter()
                            .map(|s| s.name)
                            .collect(),
                    },
                },
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
        );
        for capability in &mut descriptor.capabilities {
            if matches!(
                capability.id.as_str(),
                "org.junk.games.import-analysis" | "org.junk.games.catalog-verification"
            ) {
                capability.availability = Availability::Available;
            }
        }
        let capability = |suffix: &str, display_name: &str| Capability {
            id: CapabilityId::new(format!("org.junk.games.{suffix}")).unwrap(),
            display_name: display_name.into(),
            availability: Availability::Available,
            scope: OperationScope::CollectionDomain,
        };
        descriptor.capabilities.extend([
            capability("identify", "Identify game automatically"),
            capability("hash", "Hash game data"),
            capability("inspect-ines", "Inspect iNES structure"),
            capability("hash-tracks", "Hash raw 2352-byte tracks"),
        ]);
        descriptor
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
        if let PatchValue::Set(value) = &patch.platform
            && standard_system(value).is_none()
        {
            return Err(ModuleError::Invalid(
                "Choose a recognized standard system".into(),
            ));
        }
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
        validate_texts([&value.platform])?;
        if value
            .platform
            .as_ref()
            .is_some_and(|v| standard_system(v).is_none())
        {
            return Err(ModuleError::Invalid(
                "Choose a recognized standard system".into(),
            ));
        }
        if value.automatic_platform && value.platform.is_some() {
            return Err(ModuleError::Invalid(
                "Automatic system cannot also have a manual override".into(),
            ));
        }
        Ok(())
    }

    fn validate_operation_result(
        &self,
        capability: &CapabilityId,
        result: &RawJson,
    ) -> ModuleResult<()> {
        if capability.as_str() == "org.junk.games.import-analysis" {
            let output = result
                .parse::<GameImportAnalysisOutput>()
                .map_err(|error| ModuleError::Invalid(error.to_string()))?;
            if output.candidates.len() > 128
                || output.limitations.len() > 64
                || !(1..=2).contains(&output.observations.len())
                || output.candidates.iter().any(|candidate| {
                    candidate.key.trim().is_empty()
                        || candidate.key.len() > 1024
                        || candidate.label.trim().is_empty()
                        || candidate.label.len() > 4096
                        || candidate.observation.get().len() > 64 * 1024
                })
                || output.limitations.iter().any(|value| {
                    value.trim().is_empty()
                        || value.len() > 4096
                        || value.chars().any(char::is_control)
                })
            {
                return Err(ModuleError::Invalid(
                    "invalid game Import analysis result".into(),
                ));
            }
            for candidate in &output.candidates {
                let observation = candidate
                    .observation
                    .parse::<CatalogCandidate>()
                    .map_err(|error| ModuleError::Invalid(error.to_string()))?;
                validate_catalog_match(&CatalogMatchResult::UniqueCompleteMatch {
                    candidate: observation.clone(),
                })?;
                if candidate.key
                    != format!(
                        "{}:{}",
                        observation.provider.0, observation.provider_record_id
                    )
                    || !candidate.label.starts_with(&observation.label)
                {
                    return Err(ModuleError::Invalid(
                        "game Import candidate does not match its observation".into(),
                    ));
                }
            }
            for (index, observation) in output.observations.into_iter().enumerate() {
                let observation = observation
                    .parse::<GameProcessingResult>()
                    .map_err(|error| ModuleError::Invalid(error.to_string()))?;
                validate_processing_result(&observation)?;
                if !matches!(
                    (index, &observation),
                    (0, GameProcessingResult::Hashes { .. })
                        | (1, GameProcessingResult::CatalogMatch { .. })
                ) {
                    return Err(ModuleError::Invalid(
                        "game Import analysis contains an unexpected observation".into(),
                    ));
                }
            }
            return Ok(());
        }
        if capability.as_str() == "org.junk.games.identify" {
            let identified: GameIdentification = result
                .parse()
                .map_err(|e| ModuleError::Invalid(e.to_string()))?;
            if identified.analyses.len() > 6
                || identified.limitations.len() > 128
                || identified.catalogs.len() > 128
                || identified.candidates.len() > 4096
                || identified.inspections.len() > 4096
            {
                return Err(ModuleError::Invalid(
                    "Identification exceeds supported bounds".into(),
                ));
            }
            for analysis in &identified.analyses {
                validate_processing_result(analysis)?;
            }
            for catalog in &identified.catalogs {
                validate_catalog_match(&catalog.catalog)?;
            }
            identification::validate_identification_evidence(&identified)?;
            for candidate in &identified.candidates {
                candidate
                    .validate()
                    .map_err(|e| ModuleError::Invalid(e.to_string()))?;
            }
            if matches!(
                identified.status,
                GameDetectionStatus::Malformed | GameDetectionStatus::Unsupported
            ) && (!identified.catalogs.is_empty() || !identified.candidates.is_empty())
            {
                return Err(ModuleError::Invalid(
                    "Invalid input cannot have catalog identity".into(),
                ));
            }
            if identified.inspections.iter().any(|inspection| {
                matches!(
                    inspection.status,
                    GameDetectionStatus::Malformed | GameDetectionStatus::Unsupported
                )
            }) && !matches!(
                identified.status,
                GameDetectionStatus::Malformed | GameDetectionStatus::Unsupported
            ) {
                return Err(ModuleError::Invalid(
                    "Invalid inspection cannot have a successful interpretation".into(),
                ));
            }
            let mut complete = Vec::new();
            for c in identified.candidates.iter().filter(|c| c.complete_identity) {
                if !complete.iter().any(|v| equivalent_game_candidates(c, v)) {
                    complete.push(c.clone());
                }
            }
            if matches!(identified.status, GameDetectionStatus::Recognized) != (complete.len() == 1)
                || matches!(identified.status, GameDetectionStatus::Ambiguous)
                    && complete.len() == 1
            {
                return Err(ModuleError::Invalid(
                    "Identification status disagrees with complete candidates".into(),
                ));
            }
            return Ok(());
        }
        let result = result
            .parse::<GameProcessingResult>()
            .map_err(|error| ModuleError::Invalid(error.to_string()))?;
        validate_processing_result(&result)?;
        let expected_shape = match capability.as_str() {
            "org.junk.games.hash" => matches!(result, GameProcessingResult::Hashes { .. }),
            "org.junk.games.inspect-ines" => {
                matches!(result, GameProcessingResult::Structure { .. })
            }
            "org.junk.games.hash-tracks" => {
                matches!(result, GameProcessingResult::TrackHashes { .. })
            }
            "org.junk.games.catalog-verification" => {
                matches!(result, GameProcessingResult::CatalogMatch { .. })
            }
            _ => false,
        };
        if !expected_shape {
            return Err(ModuleError::Invalid(
                "game operation result does not match its capability".into(),
            ));
        }
        Ok(())
    }

    fn present(&self, data: &ModuleData) -> ModuleResult<ModulePresentation> {
        self.validate_current(data)?;
        Ok(basic_presentation(data, self.descriptor()))
    }

    fn operate(
        &self,
        request: OperationRequest,
        host: &dyn crate::ModuleHost,
    ) -> ModuleResult<OperationOutput> {
        let access = operation_access(&request)?;
        let inventory = host.package_inventory(access)?;
        if request.capability.as_str() == "org.junk.games.identify" {
            let value: serde_json::Value = request
                .input
                .parse()
                .map_err(|e| ModuleError::Invalid(e.to_string()))?;
            let snapshots: Vec<collector_core::SnapshotId> =
                serde_json::from_value(value["snapshot_ids"].clone())
                    .map_err(|e| ModuleError::Invalid(e.to_string()))?;
            let identified = identify_game(host, access, &snapshots)?;
            return Ok(OperationOutput {
                value: RawJson::from_value(
                    serde_json::to_value(identified)
                        .map_err(|e| ModuleError::Invalid(e.to_string()))?,
                ),
            });
        }
        let result = match request.capability.as_str() {
            "org.junk.games.hash" => {
                let input: GameFileOperationInput = request
                    .input
                    .parse()
                    .map_err(|error| ModuleError::Invalid(error.to_string()))?;
                hash_inventory_file(
                    host,
                    access,
                    &inventory,
                    input.file_index,
                    input.profile.into(),
                )?
            }
            "org.junk.games.inspect-ines" => {
                let input: GameInspectOperationInput = request
                    .input
                    .parse()
                    .map_err(|error| ModuleError::Invalid(error.to_string()))?;
                let file = inventory
                    .files
                    .get(input.file_index)
                    .ok_or_else(|| ModuleError::Invalid("file index is out of bounds".into()))?;
                let mut reader = host.package_file(access, file.id)?;
                inspect_ines_structure(reader.as_mut())?
            }
            "org.junk.games.hash-tracks" => {
                let input: GameCueOperationInput = request
                    .input
                    .parse()
                    .map_err(|error| ModuleError::Invalid(error.to_string()))?;
                hash_inventory_cue(host, access, &inventory, input.cue_file_index)?
            }
            "org.junk.games.catalog-verification" => {
                let input: CatalogVerificationInput = request
                    .input
                    .parse()
                    .map_err(|error| ModuleError::Invalid(error.to_string()))?;
                let hashes = analyze_selection(host, access, &inventory, input.analysis)?;
                match_snapshot(host, input.snapshot_id, &input.system, &hashes)?
            }
            "org.junk.games.import-analysis" => {
                let input: ImportAnalysisInput = request
                    .input
                    .parse()
                    .map_err(|error| ModuleError::Invalid(error.to_string()))?;
                let _ = (input.import_id, input.representation_id, input.files);
                let system = match (input.system, input.domain) {
                    (system @ Some(_), _) => system,
                    (None, Some(domain)) => {
                        self.validate_current(&domain)?;
                        domain
                            .payload
                            .parse::<GameItemData>()
                            .map_err(|error| ModuleError::Invalid(error.to_string()))?
                            .platform
                    }
                    (None, None) => None,
                };
                return import_analysis(host, access, &inventory, input.snapshot_id, system);
            }
            _ => {
                return Err(ModuleError::UnknownCapability(
                    request.capability.to_string(),
                ));
            }
        };
        Ok(OperationOutput {
            value: RawJson::from_value(
                serde_json::to_value(result)
                    .map_err(|error| ModuleError::Invalid(error.to_string()))?,
            ),
        })
    }
}

fn operation_access(request: &OperationRequest) -> ModuleResult<PackageAccess> {
    let scope = match request.entity {
        OperationEntity::Import(id) => collector_core::PackageScope::Import(id),
        OperationEntity::Representation(id) => collector_core::PackageScope::Representation(id),
        _ => {
            return Err(ModuleError::Invalid(
                "game processing requires an Import or Representation".into(),
            ));
        }
    };
    Ok(PackageAccess {
        collection_id: request.collection_id,
        scope,
    })
}

fn hash_inventory_file(
    host: &dyn crate::ModuleHost,
    access: PackageAccess,
    inventory: &PackageInventory,
    index: usize,
    profile: GameHashProfile,
) -> ModuleResult<GameProcessingResult> {
    let file = inventory
        .files
        .get(index)
        .ok_or_else(|| ModuleError::Invalid("file index is out of bounds".into()))?;
    let mut reader = host.package_file(access, file.id)?;
    hash_game_file(file.id, reader.as_mut(), profile)
}

fn hash_raw_inventory(
    host: &dyn crate::ModuleHost,
    access: PackageAccess,
    inventory: &PackageInventory,
) -> ModuleResult<GameProcessingResult> {
    let mut files = Vec::with_capacity(inventory.files.len());
    for (index, _) in inventory.files.iter().enumerate() {
        let GameProcessingResult::Hashes { files: mut result } =
            hash_inventory_file(host, access, inventory, index, GameHashProfile::Raw)?
        else {
            unreachable!()
        };
        files.append(&mut result);
    }
    Ok(GameProcessingResult::Hashes { files })
}

fn hash_inventory_cue(
    host: &dyn crate::ModuleHost,
    access: PackageAccess,
    inventory: &PackageInventory,
    cue_index: usize,
) -> ModuleResult<GameProcessingResult> {
    let cue = inventory
        .files
        .get(cue_index)
        .ok_or_else(|| ModuleError::Invalid("CUE file index is out of bounds".into()))?;
    if cue.bytes.0 > 1024 * 1024 {
        return Err(ModuleError::Invalid("CUE sheet is oversized".into()));
    }
    let mut reader = host.package_file(access, cue.id)?;
    let mut text = String::new();
    reader
        .read_to_string(&mut text)
        .map_err(|error| ModuleError::FailedOperation(error.to_string()))?;
    hash_cue_package(&text, cue.id, inventory, |id| host.package_file(access, id))
}

fn analyze_selection(
    host: &dyn crate::ModuleHost,
    access: PackageAccess,
    inventory: &PackageInventory,
    selection: CatalogAnalysisSelection,
) -> ModuleResult<GameProcessingResult> {
    // Catalog interpretation must not let a recognized corrupt container bypass
    // validation merely because an advanced caller explicitly requested Raw.
    if !matches!(selection, CatalogAnalysisSelection::CueRaw2352 { .. }) {
        for file in &inventory.files {
            host.checkpoint()?;
            let mut reader = host.package_file(access, file.id)?;
            let inspected = junk_libs_game_formats::inspect_game_format(reader.as_mut())
                .map_err(module_hash_error)?;
            if matches!(
                inspected.status,
                junk_libs_game_formats::DetectionStatus::Malformed
                    | junk_libs_game_formats::DetectionStatus::Unsupported
            ) {
                return Err(ModuleError::Invalid(inspected.reason));
            }
        }
    }
    match selection {
        CatalogAnalysisSelection::RawFiles => hash_raw_inventory(host, access, inventory),
        CatalogAnalysisSelection::File {
            file_index,
            profile,
        } => hash_inventory_file(host, access, inventory, file_index, profile.into()),
        CatalogAnalysisSelection::CueRaw2352 { cue_file_index } => {
            hash_inventory_cue(host, access, inventory, cue_file_index)
        }
    }
}

fn match_snapshot(
    host: &dyn crate::ModuleHost,
    snapshot_id: collector_core::SnapshotId,
    system: &str,
    hashes: &GameProcessingResult,
) -> ModuleResult<GameProcessingResult> {
    let snapshot = host.catalog_snapshot(snapshot_id)?;
    let units = match hashes {
        GameProcessingResult::Hashes { files } => files
            .iter()
            .map(|file| CatalogInputUnit {
                source_file_id: file.file_id,
                track_number: None,
                bytes: file.bytes,
                domain: &file.domain,
                digests: &file.digests,
            })
            .collect::<Vec<_>>(),
        GameProcessingResult::TrackHashes { tracks } => tracks
            .iter()
            .map(|track| CatalogInputUnit {
                source_file_id: track.source_file_id,
                track_number: Some(track.track_number),
                bytes: track.bytes,
                domain: &track.domain,
                digests: &track.digests,
            })
            .collect(),
        _ => {
            return Err(ModuleError::Invalid(
                "catalog matching requires qualified hashes".into(),
            ));
        }
    };
    match_dat_catalog(
        snapshot.provider,
        system,
        &snapshot.observation,
        snapshot.entries.iter(),
        &units,
        &mut || host.checkpoint(),
    )
}

fn import_analysis(
    host: &dyn crate::ModuleHost,
    access: PackageAccess,
    inventory: &PackageInventory,
    snapshot_id: Option<collector_core::SnapshotId>,
    system: Option<String>,
) -> ModuleResult<OperationOutput> {
    let hashes = analyze_selection(host, access, inventory, CatalogAnalysisSelection::RawFiles)?;
    let mut limitations = Vec::new();
    let mut observations = vec![result_json(&hashes)?];
    let mut candidates = Vec::new();
    match (snapshot_id, system.filter(|value| !value.trim().is_empty())) {
        (Some(snapshot_id), Some(system)) => {
            let matched = match_snapshot(host, snapshot_id, &system, &hashes)?;
            if let GameProcessingResult::CatalogMatch { catalog } = &matched {
                let (values, limitation) = import_candidates(catalog)?;
                candidates = values;
                if let Some(limitation) = limitation {
                    limitations.push(limitation);
                }
            }
            observations.push(result_json(&matched)?);
        }
        (None, _) => limitations.push(
            "No active DAT snapshot was selected; exact hashes were retained for later matching."
                .into(),
        ),
        (Some(_), None) => limitations
            .push("The collection item has no explicit platform/system for DAT matching.".into()),
    }
    let output = GameImportAnalysisOutput {
        candidates,
        limitations,
        observations,
    };
    Ok(OperationOutput {
        value: RawJson::from_value(
            serde_json::to_value(output)
                .map_err(|error| ModuleError::Invalid(error.to_string()))?,
        ),
    })
}

fn result_json(result: &GameProcessingResult) -> ModuleResult<RawJson> {
    Ok(RawJson::from_value(serde_json::to_value(result).map_err(
        |error| ModuleError::Invalid(error.to_string()),
    )?))
}

fn import_candidates(
    catalog: &CatalogMatchResult,
) -> ModuleResult<(Vec<GameImportCandidateOutput>, Option<String>)> {
    let (values, suffix, limitation): (&[CatalogCandidate], &str, Option<String>) = match catalog {
        CatalogMatchResult::UniqueCompleteMatch { candidate } => {
            (std::slice::from_ref(candidate), "exact catalog match", None)
        }
        CatalogMatchResult::AmbiguousCompleteMatches { candidates } => (
            candidates,
            "exact but ambiguous catalog match",
            Some("Multiple complete catalog matches require curator review.".into()),
        ),
        CatalogMatchResult::PartialIdentification { candidates, .. } => (
            candidates,
            "partial identification only",
            Some(
                "Partial catalog matches are tentative identity candidates, not verification."
                    .into(),
            ),
        ),
        CatalogMatchResult::Inconclusive { reason } => {
            return Ok((Vec::new(), Some(reason.clone())));
        }
        CatalogMatchResult::NoMatch => {
            return Ok((
                Vec::new(),
                Some("No catalog row matched the complete analyzed package.".into()),
            ));
        }
    };
    let candidates = values
        .iter()
        .map(|candidate| {
            let checksum_note = if candidate
                .matched_units
                .iter()
                .all(|unit| unit.digest.algorithm == HashAlgorithm::Crc32)
            {
                " (CRC32 only; not cryptographic identity)"
            } else {
                ""
            };
            Ok(GameImportCandidateOutput {
                key: format!("{}:{}", candidate.provider.0, candidate.provider_record_id),
                label: format!("{} — {suffix}{checksum_note}", candidate.label),
                observation: RawJson::from_value(
                    serde_json::to_value(candidate)
                        .map_err(|error| ModuleError::Invalid(error.to_string()))?,
                ),
            })
        })
        .collect::<ModuleResult<Vec<_>>>()?;
    Ok((candidates, limitation))
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
        CatalogMatchResult::Inconclusive { reason } => {
            if reason.trim().is_empty()
                || reason.len() > 4096
                || reason.chars().any(char::is_control)
            {
                return Err(ModuleError::Invalid(
                    "invalid inconclusive catalog result".into(),
                ));
            }
            return Ok(());
        }
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
            if *total_files == 0 || *matched_files == 0 || matched_files > total_files {
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
                || candidate.matched_units.is_empty()
                || candidate
                    .matched_units
                    .iter()
                    .any(|unit| unit.track_number == Some(0) || unit.digest.validate().is_err())
                || candidate
                    .matched_units
                    .iter()
                    .map(|unit| (unit.source_file_id, unit.track_number))
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    != candidate.matched_units.len()
        })
    {
        return Err(ModuleError::Invalid("invalid catalog match result".into()));
    }
    Ok(())
}

#[cfg(test)]
mod cancellation_tests {
    use super::*;

    #[test]
    fn catalog_match_rejects_payloads_hidden_in_excluded_files() {
        let header = RawJson::from_value(serde_json::json!({
            "schema_version":1,"system":"psx","catalog":"fixture","version":"1",
            "byte_domain":"org.junk.game.cd.raw-2352-track","byte_domain_version":1,"game_count":1
        }));
        let domain = NormalizedByteDomain {
            name: "org.junk.game.cd.raw-2352-track".into(),
            version: 1,
        };
        let digests = [Digest::new(HashAlgorithm::Crc32, "11111111").unwrap()];
        let units = [CatalogInputUnit {
            source_file_id: PackageFileId::new(),
            track_number: Some(1),
            bytes: ByteLength(2352),
            domain: &domain,
            digests: &digests,
        }];
        for name in ["missing.bin", "unknown.img"] {
            let row = RawJson::from_value(serde_json::json!({
                "record_id":"one","label":"One",
                "files":[{"name":"one.bin","bytes":2352,"crc32":"11111111"}],
                "excluded_files":[{"name":name,"bytes":2352,"crc32":"22222222"}]
            }));
            assert!(
                match_dat_catalog(
                    ModuleId("org.junk.providers.local-dat".into()),
                    "psx",
                    &header,
                    [&row],
                    &units,
                    &mut || Ok(())
                )
                .is_err()
            );
        }
    }

    #[test]
    fn catalog_scan_stops_at_host_checkpoint() {
        let header = RawJson::from_value(serde_json::json!({
            "schema_version":1,"system":"nes","catalog":"fixture","version":"1",
            "byte_domain":"org.junk.game.raw-bytes","byte_domain_version":1,"game_count":2
        }));
        let row = RawJson::from_value(serde_json::json!({
            "record_id":"one","label":"One","files":[{"name":"one","bytes":1,"crc32":"00000000"}]
        }));
        let domain = NormalizedByteDomain {
            name: "org.junk.game.raw-bytes".into(),
            version: 1,
        };
        let digests = [Digest::new(HashAlgorithm::Crc32, "11111111").unwrap()];
        let units = [CatalogInputUnit {
            source_file_id: PackageFileId::new(),
            track_number: None,
            bytes: ByteLength(1),
            domain: &domain,
            digests: &digests,
        }];
        let mut checks = 0;
        let result = match_dat_catalog(
            ModuleId("org.junk.providers.local-dat".into()),
            "nes",
            &header,
            [&row, &row],
            &units,
            &mut || {
                checks += 1;
                if checks >= 2 {
                    Err(ModuleError::FailedOperation("cancelled".into()))
                } else {
                    Ok(())
                }
            },
        );
        assert!(matches!(result, Err(ModuleError::FailedOperation(_))));
        assert_eq!(checks, 2);
    }
}
