use std::{collections::BTreeMap, fmt, str::FromStr};

use crate::{
    ByteLength, CollectionId, Digest, Error, HashAlgorithm, ImportId, ModuleData, ObservationId,
    PackageFileId, RecipeId, RepresentationId, Result, SnapshotId,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const SOURCE_FINGERPRINT_ENCODING_VERSION: u16 = 1;
const SOURCE_FINGERPRINT_MAGIC: &[u8; 4] = b"JCSF";

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, utoipa::ToSchema)]
#[serde(transparent)]
#[schema(as = CapabilityId)]
pub struct OperationId(String);

impl OperationId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 120
            || value.split('.').count() < 3
            || value.split('.').any(str::is_empty)
            || !value.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
            })
        {
            return Err(Error::Invalid(
                "operation must be a namespaced lowercase identifier".into(),
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OperationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for OperationId {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        Self::new(value)
    }
}

impl<'de> Deserialize<'de> for OperationId {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    utoipa::ToSchema,
)]
#[serde(tag = "scope", content = "id", rename_all = "snake_case")]
pub enum PackageScope {
    Import(ImportId),
    Representation(RepresentationId),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, utoipa::ToSchema)]
#[serde(transparent)]
/// A validated package-relative logical name, never an adapter filesystem path.
pub struct PackageFileName(String);

impl PackageFileName {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 1024
            || value.starts_with('/')
            || value.contains(['\\', ':'])
            || value.chars().any(char::is_control)
            || value
                .split('/')
                .any(|part| part.is_empty() || matches!(part, "." | ".."))
        {
            return Err(Error::Invalid("unsafe package display name".into()));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for PackageFileName {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PackageInventoryFile {
    pub id: PackageFileId,
    pub display_name: PackageFileName,
    pub bytes: ByteLength,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PackageInventory {
    pub collection_id: CollectionId,
    pub scope: PackageScope,
    pub files: Vec<PackageInventoryFile>,
}

impl PackageInventory {
    pub fn new(
        collection_id: CollectionId,
        scope: PackageScope,
        files: Vec<PackageInventoryFile>,
    ) -> Result<Self> {
        let inventory = Self {
            collection_id,
            scope,
            files,
        };
        inventory.validate()?;
        Ok(inventory)
    }

    pub fn validate(&self) -> Result<()> {
        if self.files.is_empty() || self.files.len() > 4096 {
            return Err(Error::Invalid(
                "package inventory must contain between 1 and 4096 files".into(),
            ));
        }
        let mut ids = std::collections::BTreeSet::new();
        let mut names = std::collections::BTreeSet::new();
        if self.files.iter().any(|file| {
            !ids.insert(file.id) || !names.insert(file.display_name.as_str().to_lowercase())
        }) {
            return Err(Error::Invalid(
                "package inventory contains a duplicate file identity or name".into(),
            ));
        }
        for name in &names {
            for (index, _) in name.match_indices('/') {
                if names.contains(&name[..index]) {
                    return Err(Error::Invalid("package file shadows a directory".into()));
                }
            }
        }
        Ok(())
    }

    pub fn file_named(&self, name: &PackageFileName) -> Option<&PackageInventoryFile> {
        self.files.iter().find(|file| &file.display_name == name)
    }

    /// Resolves a package-relative reference without exposing or consulting host paths.
    pub fn resolve_relative(
        &self,
        declaring_file: PackageFileId,
        declared_name: &str,
    ) -> Result<&PackageInventoryFile> {
        if declared_name.is_empty()
            || declared_name.len() > 1024
            || declared_name.starts_with('/')
            || declared_name.contains(['\\', ':'])
            || declared_name.chars().any(char::is_control)
        {
            return Err(Error::Invalid("unsafe package-relative reference".into()));
        }

        let declaring_name = self
            .files
            .iter()
            .find(|file| file.id == declaring_file)
            .ok_or(Error::NotFound)?
            .display_name
            .as_str();
        let mut components: Vec<&str> = declaring_name.split('/').collect();
        components.pop();
        for component in declared_name.split('/') {
            match component {
                "" => return Err(Error::Invalid("empty package path component".into())),
                "." => {}
                ".." => {
                    if components.pop().is_none() {
                        return Err(Error::Invalid(
                            "package-relative reference escapes the package".into(),
                        ));
                    }
                }
                value => components.push(value),
            }
        }
        let resolved = PackageFileName::new(components.join("/"))?;
        self.file_named(&resolved).ok_or(Error::NotFound)
    }
}

impl<'de> Deserialize<'de> for PackageInventory {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            collection_id: CollectionId,
            scope: PackageScope,
            files: Vec<PackageInventoryFile>,
        }
        let wire = Wire::deserialize(deserializer)?;
        Self::new(wire.collection_id, wire.scope, wire.files).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PackageFileFingerprint {
    pub id: PackageFileId,
    pub bytes: ByteLength,
    pub sha256: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(try_from = "SourceFingerprintWire")]
pub struct SourceFingerprint {
    pub encoding_version: u16,
    pub source: PackageScope,
    pub files: Vec<PackageFileFingerprint>,
    pub layout_profile_digest: Option<Digest>,
    pub parameters: BTreeMap<String, String>,
    pub digest: Digest,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceFingerprintWire {
    encoding_version: u16,
    source: PackageScope,
    files: Vec<PackageFileFingerprint>,
    layout_profile_digest: Option<Digest>,
    parameters: BTreeMap<String, String>,
    digest: Digest,
}
impl TryFrom<SourceFingerprintWire> for SourceFingerprint {
    type Error = Error;
    fn try_from(wire: SourceFingerprintWire) -> Result<Self> {
        let value = Self {
            encoding_version: wire.encoding_version,
            source: wire.source,
            files: wire.files,
            layout_profile_digest: wire.layout_profile_digest,
            parameters: wire.parameters,
            digest: wire.digest,
        };
        value.validate()?;
        Ok(value)
    }
}

impl SourceFingerprint {
    pub fn calculate(
        source: PackageScope,
        files: Vec<PackageFileFingerprint>,
        layout_profile_digest: Option<Digest>,
        parameters: BTreeMap<String, String>,
    ) -> Result<Self> {
        let provisional = Self {
            encoding_version: SOURCE_FINGERPRINT_ENCODING_VERSION,
            source,
            files,
            layout_profile_digest,
            parameters,
            digest: Digest::sha256(&[]),
        };
        provisional.validate_inputs()?;
        let bytes = provisional.canonical_bytes()?;
        Ok(Self {
            digest: Digest::sha256(&bytes),
            ..provisional
        })
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_inputs()?;
        let actual = Digest::sha256(&self.canonical_bytes()?);
        if actual != self.digest {
            return Err(Error::Invalid("source fingerprint digest mismatch".into()));
        }
        Ok(())
    }

    /// Compare the ordered frozen file descriptors with an authorized inventory.
    /// The adapter must still hash actual reader bytes; this does not verify them.
    pub fn validate_inventory(&self, inventory: &PackageInventory) -> Result<()> {
        self.validate()?;
        inventory.validate()?;
        if self.source != inventory.scope
            || self.files.len() != inventory.files.len()
            || self
                .files
                .iter()
                .zip(&inventory.files)
                .any(|(frozen, file)| frozen.id != file.id || frozen.bytes != file.bytes)
        {
            return Err(Error::Invalid(
                "fingerprint does not describe this package inventory".into(),
            ));
        }
        Ok(())
    }

    fn validate_inputs(&self) -> Result<()> {
        if self.encoding_version != SOURCE_FINGERPRINT_ENCODING_VERSION {
            return Err(Error::Invalid(
                "unsupported source fingerprint encoding".into(),
            ));
        }
        if self.files.is_empty() || self.files.len() > 4096 {
            return Err(Error::Invalid(
                "source fingerprint must contain between 1 and 4096 files".into(),
            ));
        }
        let mut ids = std::collections::BTreeSet::new();
        for file in &self.files {
            if !ids.insert(file.id) {
                return Err(Error::Invalid(
                    "source fingerprint contains duplicate file identity".into(),
                ));
            }
            require_sha256(&file.sha256)?;
        }
        if let Some(digest) = &self.layout_profile_digest {
            require_sha256(digest)?;
        }
        if self.parameters.len() > 128
            || self.parameters.iter().any(|(key, value)| {
                key.is_empty()
                    || key.len() > 255
                    || value.len() > 4096
                    || key.chars().chain(value.chars()).any(char::is_control)
            })
        {
            return Err(Error::Invalid(
                "source fingerprint parameters are invalid or oversized".into(),
            ));
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        self.validate_inputs()?;
        let mut output = Vec::new();
        output.extend_from_slice(SOURCE_FINGERPRINT_MAGIC);
        output.extend_from_slice(&self.encoding_version.to_be_bytes());
        match self.source {
            PackageScope::Import(id) => {
                output.push(1);
                output.extend_from_slice(id.0.as_bytes());
            }
            PackageScope::Representation(id) => {
                output.push(2);
                output.extend_from_slice(id.0.as_bytes());
            }
        }
        push_count(&mut output, self.files.len())?;
        for file in &self.files {
            output.extend_from_slice(file.id.0.as_bytes());
            output.extend_from_slice(&file.bytes.0.to_be_bytes());
            output.extend_from_slice(&digest_bytes(&file.sha256)?);
        }
        if let Some(digest) = &self.layout_profile_digest {
            output.push(1);
            output.extend_from_slice(&digest_bytes(digest)?);
        } else {
            output.push(0);
        }
        push_count(&mut output, self.parameters.len())?;
        for (key, value) in &self.parameters {
            push_string(&mut output, key)?;
            push_string(&mut output, value)?;
        }
        Ok(output)
    }
}

fn require_sha256(digest: &Digest) -> Result<()> {
    digest.validate()?;
    if digest.algorithm != HashAlgorithm::Sha256 {
        return Err(Error::Invalid("source fingerprints require SHA-256".into()));
    }
    Ok(())
}

fn digest_bytes(digest: &Digest) -> Result<[u8; 32]> {
    require_sha256(digest)?;
    let decoded = hex::decode(&digest.hex).map_err(|_| Error::Invalid("invalid SHA-256".into()))?;
    decoded
        .try_into()
        .map_err(|_| Error::Invalid("invalid SHA-256 length".into()))
}

fn push_count(output: &mut Vec<u8>, count: usize) -> Result<()> {
    let count = u32::try_from(count).map_err(|_| Error::Invalid("count overflow".into()))?;
    output.extend_from_slice(&count.to_be_bytes());
    Ok(())
}

fn push_string(output: &mut Vec<u8>, value: &str) -> Result<()> {
    push_count(output, value.len())?;
    output.extend_from_slice(value.as_bytes());
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "support", rename_all = "snake_case")]
pub enum OperationSupport {
    Supported,
    UnavailableDependency { dependency: String, reason: String },
    UnsupportedLayout { reason: String },
    ExternallyUnqualified { reason: String },
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, utoipa::ToSchema,
)]
#[serde(transparent)]
pub struct CdSector(pub i64);

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, utoipa::ToSchema,
)]
#[serde(transparent)]
pub struct PcmFrame(pub u64);

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, utoipa::ToSchema,
)]
#[serde(transparent)]
pub struct StereoSampleFrameOffset(pub i32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CdTrackKind {
    Audio,
    Data,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CdTrackLayout {
    pub position: u8,
    pub kind: CdTrackKind,
    pub index_00: Option<CdSector>,
    pub index_01: CdSector,
    pub pcm_start: Option<PcmFrame>,
    pub pcm_length: Option<PcmFrame>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ProcessingObservation {
    pub id: ObservationId,
    pub collection_id: CollectionId,
    pub operation: OperationId,
    pub operation_version: u32,
    pub source_fingerprint: SourceFingerprint,
    pub snapshot_id: Option<SnapshotId>,
    pub support: OperationSupport,
    pub raw_observations: Vec<ModuleData>,
    pub typed_result: ModuleData,
    pub limitations: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Provenance for an observation first made against private Import scratch and
/// later committed against the immutable Representation package. Import and
/// Representation fingerprints are deliberately different identities.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceFileRebinding {
    pub import_file_id: PackageFileId,
    pub representation_file_id: PackageFileId,
    pub bytes: ByteLength,
    pub sha256: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ImportSourceRebinding {
    pub provisional_fingerprint: SourceFingerprint,
    pub committed_fingerprint: SourceFingerprint,
    pub files: Vec<SourceFileRebinding>,
}

impl ImportSourceRebinding {
    pub fn validate(&self) -> Result<()> {
        self.provisional_fingerprint.validate()?;
        self.committed_fingerprint.validate()?;
        if !matches!(self.provisional_fingerprint.source, PackageScope::Import(_))
            || !matches!(
                self.committed_fingerprint.source,
                PackageScope::Representation(_)
            )
            || self.files.is_empty()
            || self.files.len() != self.provisional_fingerprint.files.len()
            || self.files.len() != self.committed_fingerprint.files.len()
        {
            return Err(Error::Invalid("invalid Import source rebinding".into()));
        }
        for ((mapping, provisional), committed) in self
            .files
            .iter()
            .zip(&self.provisional_fingerprint.files)
            .zip(&self.committed_fingerprint.files)
        {
            require_sha256(&mapping.sha256)?;
            if mapping.import_file_id != provisional.id
                || mapping.representation_file_id != committed.id
                || mapping.bytes != provisional.bytes
                || mapping.bytes != committed.bytes
                || mapping.sha256 != provisional.sha256
                || mapping.sha256 != committed.sha256
            {
                return Err(Error::Invalid(
                    "Import source rebinding does not preserve ordered bytes".into(),
                ));
            }
        }
        Ok(())
    }
}

impl ProcessingObservation {
    pub fn validate(&self) -> Result<()> {
        if self.operation_version == 0 {
            return Err(Error::Invalid(
                "processing operation version must be non-zero".into(),
            ));
        }
        self.source_fingerprint.validate()?;
        self.typed_result.module.validate()?;
        if self.typed_result.schema_version == 0
            || self.typed_result.payload.get().len() > 1024 * 1024
        {
            return Err(Error::Invalid(
                "typed result is invalid or oversized".into(),
            ));
        }
        let reason = match &self.support {
            OperationSupport::Supported => None,
            OperationSupport::UnavailableDependency { dependency, reason } => {
                if dependency.is_empty()
                    || dependency.len() > 255
                    || dependency.chars().any(char::is_control)
                {
                    return Err(Error::Invalid("invalid dependency limitation".into()));
                }
                Some(reason)
            }
            OperationSupport::UnsupportedLayout { reason }
            | OperationSupport::ExternallyUnqualified { reason } => Some(reason),
        };
        if reason.is_some_and(|value| {
            value.is_empty() || value.len() > 4096 || value.chars().any(char::is_control)
        }) {
            return Err(Error::Invalid("invalid operation support reason".into()));
        }
        let mut raw_bytes = 0_usize;
        for observation in &self.raw_observations {
            raw_bytes = raw_bytes.saturating_add(observation.payload.get().len());
            if observation.schema_version == 0 || raw_bytes > 8 * 1024 * 1024 {
                return Err(Error::Invalid(
                    "raw observations are invalid or oversized".into(),
                ));
            }
            observation.module.validate()?;
        }
        if self.raw_observations.len() > 128
            || self.limitations.len() > 128
            || self.limitations.iter().any(|limitation| {
                limitation.len() > 4096 || limitation.chars().any(char::is_control)
            })
        {
            return Err(Error::Invalid(
                "processing observations or limitations are oversized".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "codec", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConversionSettings {
    Flac {
        level: u8,
    },
    Opus {
        bitrate_kbps: u16,
        frame_milliseconds: u16,
    },
}

impl ConversionSettings {
    fn validate(&self) -> Result<()> {
        match self {
            Self::Flac { level: 5 } => Ok(()),
            Self::Opus {
                bitrate_kbps: 96 | 128 | 192,
                frame_milliseconds: 20,
            } => Ok(()),
            _ => Err(Error::Invalid("unsupported conversion settings".into())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PcmSampleFormat {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub signed: bool,
    pub little_endian: bool,
}

impl PcmSampleFormat {
    pub const CDDA_I16_LE_STEREO: Self = Self {
        sample_rate: 44_100,
        channels: 2,
        bits_per_sample: 16,
        signed: true,
        little_endian: true,
    };

    pub const OPUS_DECODED_I16_STEREO: Self = Self {
        sample_rate: 48_000,
        channels: 2,
        bits_per_sample: 16,
        signed: true,
        little_endian: true,
    };

    pub fn validate(&self) -> Result<()> {
        if self.sample_rate == 0
            || self.channels == 0
            || self.channels > 32
            || !matches!(self.bits_per_sample, 8 | 16 | 24 | 32)
        {
            return Err(Error::Invalid("invalid integer PCM format".into()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConversionTrack {
    pub position: u16,
    pub start: PcmFrame,
    pub length: PcmFrame,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConversionRecipe {
    pub id: RecipeId,
    pub version: u32,
    pub settings: ConversionSettings,
    pub source_fingerprint: SourceFingerprint,
    pub tracks: Vec<ConversionTrack>,
    pub encoder: String,
    pub encoder_version: String,
    pub input_format: PcmSampleFormat,
    pub output_format: PcmSampleFormat,
    pub digest: Digest,
}

impl ConversionRecipe {
    /// Finalize a recipe before persistence. IDs identify records; the digest
    /// identifies conversion contents and deliberately excludes the record ID.
    pub fn seal(mut self) -> Result<Self> {
        self.digest = Digest::sha256(&self.canonical_bytes()?);
        Ok(self)
    }

    pub fn validate(&self) -> Result<()> {
        let expected = Digest::sha256(&self.canonical_bytes()?);
        if self.digest != expected {
            return Err(Error::Invalid("conversion recipe digest mismatch".into()));
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        self.validate_inputs()?;
        // Version 1: fixed JSON tuple/struct field order, integer values only.
        // SourceFingerprint owns its separately versioned encoding and digest.
        serde_json::to_vec(&(
            "JCCR",
            self.version,
            &self.settings,
            &self.source_fingerprint.digest,
            &self.tracks,
            &self.encoder,
            &self.encoder_version,
            &self.input_format,
            &self.output_format,
        ))
        .map_err(|error| Error::Invalid(error.to_string()))
    }

    fn validate_inputs(&self) -> Result<()> {
        if self.version != 1
            || self.encoder.is_empty()
            || self.encoder.len() > 255
            || self.encoder_version.is_empty()
            || self.encoder_version.len() > 255
            || self
                .encoder
                .chars()
                .chain(self.encoder_version.chars())
                .any(char::is_control)
            || self.tracks.is_empty()
            || self.tracks.len() > 4096
        {
            return Err(Error::Invalid(
                "conversion recipe is incomplete or unsupported".into(),
            ));
        }
        self.source_fingerprint.validate()?;
        self.settings.validate()?;
        self.input_format.validate()?;
        self.output_format.validate()?;
        match &self.settings {
            ConversionSettings::Flac { .. }
                if self.encoder != "flac"
                    || self.input_format != PcmSampleFormat::CDDA_I16_LE_STEREO
                    || self.output_format != PcmSampleFormat::CDDA_I16_LE_STEREO =>
            {
                return Err(Error::Invalid(
                    "FLAC recipe must use flac with unchanged CD-DA integer PCM".into(),
                ));
            }
            ConversionSettings::Opus { .. }
                if self.encoder != "opusenc"
                    || self.input_format != PcmSampleFormat::CDDA_I16_LE_STEREO
                    || self.output_format != PcmSampleFormat::OPUS_DECODED_I16_STEREO =>
            {
                return Err(Error::Invalid(
                    "Opus recipe must use opusenc from CD-DA PCM to 48-kHz stereo".into(),
                ));
            }
            _ => {}
        }
        let mut previous_position = 0;
        let mut previous_end = 0;
        for track in &self.tracks {
            if track.position <= previous_position
                || track.length.0 == 0
                || track.start.0 < previous_end
            {
                return Err(Error::Invalid(
                    "conversion tracks must be ordered, nonempty and nonoverlapping".into(),
                ));
            }
            previous_position = track.position;
            previous_end = track
                .start
                .0
                .checked_add(track.length.0)
                .ok_or_else(|| Error::Invalid("conversion track span overflow".into()))?;
        }
        Ok(())
    }
}
