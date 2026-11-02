//! Collection identity mapping over shared fingerprint mechanisms.
use super::*;
pub const DIGITAL_FINGERPRINT_OPERATION: &str = "org.junk.music.digital-fingerprint";
pub const ACOUSTID_PROVIDER_ID: &str = "org.junk.providers.acoustid";
pub const ACOUSTID_ACQUIRE_OPERATION: &str = "org.junk.providers.acoustid.acquire";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DigitalFingerprintSelection {
    pub files: Vec<PackageFileId>,
    #[serde(default)]
    pub frames: std::collections::BTreeMap<PackageFileId, u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DigitalFileFingerprint {
    pub file_id: PackageFileId,
    pub file_index: usize,
    pub audio: junk_libs_music::fingerprint::AudioFingerprint,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DigitalFingerprintOutput {
    pub tracks: Vec<DigitalFileFingerprint>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcoustIdQuery {
    #[serde(default)]
    pub refresh: bool,
    pub duration: u64,
    pub fingerprint: String,
}
impl AcoustIdQuery {
    pub fn validate(&self) -> collector_core::Result<()> {
        if self.duration == 0
            || self.duration > 86400
            || self.fingerprint.len() > 65536
            || self.fingerprint.is_empty()
            || !self
                .fingerprint
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
        {
            return Err(collector_core::Error::Invalid(
                "invalid AcoustID lookup".into(),
            ));
        }
        Ok(())
    }
}
pub fn operate(
    request: OperationRequest,
    host: &dyn crate::ModuleHost,
) -> ModuleResult<OperationOutput> {
    let access = operation_access(&request)?;
    let inventory = host.package_inventory(access)?;
    let selection: DigitalFingerprintSelection = request
        .input
        .parse()
        .map_err(|e| ModuleError::Invalid(e.to_string()))?;
    let files: std::collections::BTreeSet<_> = selection.files.iter().copied().collect();
    if files.is_empty()
        || files.len() > junk_libs_music::MAX_TRACKS
        || files.len() != selection.files.len()
    {
        return Err(ModuleError::Invalid(
            "choose 1..=256 distinct audio files".into(),
        ));
    }
    let mut tracks = Vec::new();
    for (index, file) in inventory
        .files
        .iter()
        .enumerate()
        .filter(|(_, f)| files.contains(&f.id))
    {
        host.checkpoint()?;
        if !files::is_audio_name(file.display_name.as_str()) {
            return Err(ModuleError::Invalid(
                "fingerprints require supported audio files".into(),
            ));
        }
        let reader = host.package_file(access, file.id)?;
        let mut checkpoint = || {
            host.checkpoint()
                .map_err(|_| junk_libs_music::Error::Cancelled)
        };
        let audio = if file
            .display_name
            .as_str()
            .to_ascii_lowercase()
            .ends_with(".wav")
        {
            junk_libs_pcm::fingerprint::compute(reader, &mut checkpoint)
        } else {
            let source = junk_libs_music_audio_source(reader, file)?;
            if let Some(&frames) = selection.frames.get(&file.id) {
                junk_libs_music::fingerprint::compute_window(source, frames, &mut checkpoint)
            } else {
                junk_libs_music::fingerprint::compute(source, &mut checkpoint)
            }
        }
        .map_err(|e| ModuleError::FailedOperation(e.to_string()))?;
        tracks.push(DigitalFileFingerprint {
            file_id: file.id,
            file_index: index,
            audio,
        });
    }
    if tracks.len() != files.len() {
        return Err(ModuleError::Invalid("audio file missing".into()));
    }
    host.checkpoint()?;
    Ok(OperationOutput {
        value: RawJson::from_value(
            serde_json::to_value(DigitalFingerprintOutput { tracks })
                .map_err(|e| ModuleError::Invalid(e.to_string()))?,
        ),
    })
}
fn junk_libs_music_audio_source(
    reader: Box<dyn crate::RepresentationReader>,
    file: &collector_core::PackageInventoryFile,
) -> ModuleResult<junk_libs_music::fingerprint::ByteSource> {
    Ok(junk_libs_music::fingerprint::ByteSource::new(
        reader,
        junk_libs_music::fingerprint::MediaHint::new(
            Some(file.bytes.0),
            file.display_name.as_str().rsplit('.').next(),
            None,
        ),
    ))
}
