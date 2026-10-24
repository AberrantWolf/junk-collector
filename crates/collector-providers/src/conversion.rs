use std::{
    ffi::OsStr,
    io::{Read, SeekFrom, Write},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

use collector_app::{
    AudioConversionPort, ConversionScratch, ConversionValidation, EncoderAvailability,
    ProcessingExecutionContext,
};
use collector_core::{ByteLength, ConversionRecipe, ConversionSettings, Digest, Error, Result};
use sha2::{Digest as _, Sha256};

const IO_FRAMES: usize = 16 * 1024;
const MAX_DIAGNOSTIC_BYTES: usize = 64 * 1024;
const MAX_OUTPUT_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const VERSION_TIMEOUT: Duration = Duration::from_secs(5);
const CONVERSION_TIMEOUT: Duration = Duration::from_secs(6 * 60 * 60);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncoderKind {
    Flac,
    Opus,
}

impl EncoderKind {
    fn executable_name(self) -> &'static str {
        match self {
            Self::Flac => "flac",
            Self::Opus => "opusenc",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConfiguredEncoder {
    kind: EncoderKind,
    path: PathBuf,
    version: String,
}

impl ConfiguredEncoder {
    /// Probe an administrator-configured executable directly. The reported tool
    /// version report (including reported codec-library versions) is recipe identity; changing the tool/library report changes
    /// the recipe instead of silently reusing cached output.
    pub fn probe(kind: EncoderKind, path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let (status, stdout, stderr) = run_small_process(&path, ["--version"], VERSION_TIMEOUT)?;
        if !status.success() {
            return Err(Error::Unavailable(format!(
                "{} version probe failed: {}",
                kind.executable_name(),
                safe_diagnostic(&stderr)
            )));
        }
        let text = if stdout.is_empty() { &stderr } else { &stdout };
        let version_report = String::from_utf8_lossy(text);
        let line = version_report
            .lines()
            .find(|line| !line.trim().is_empty())
            .map(str::trim)
            .unwrap_or_default();
        if line.is_empty()
            || line.len() > 255
            || line.chars().any(char::is_control)
            || !line.to_ascii_lowercase().contains(kind.executable_name())
        {
            return Err(Error::Unavailable(format!(
                "{} returned an invalid version report",
                kind.executable_name()
            )));
        }
        let _version = line
            .split_whitespace()
            .find(|field| field.as_bytes().first().is_some_and(u8::is_ascii_digit))
            .map(|field| {
                field.trim_matches(|character: char| {
                    !character.is_ascii_alphanumeric() && character != '.' && character != '-'
                })
            })
            .filter(|field| !field.is_empty())
            .ok_or_else(|| {
                Error::Unavailable(format!(
                    "{} returned no usable version identifier",
                    kind.executable_name()
                ))
            })?;
        let identity = version_report
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" | ");
        if identity.len() > 255 || identity.chars().any(char::is_control) {
            return Err(Error::Unavailable(
                "encoder identity report exceeds recipe bounds".into(),
            ));
        }
        Ok(Self {
            kind,
            path,
            version: identity,
        })
    }

    pub fn version(&self) -> &str {
        &self.version
    }
}

#[derive(Clone, Debug, Default)]
pub struct ProcessAudioConverter {
    flac: Option<ConfiguredEncoder>,
    opus: Option<ConfiguredEncoder>,
}

impl ProcessAudioConverter {
    pub fn new(flac: Option<ConfiguredEncoder>, opus: Option<ConfiguredEncoder>) -> Result<Self> {
        if flac
            .as_ref()
            .is_some_and(|tool| tool.kind != EncoderKind::Flac)
            || opus
                .as_ref()
                .is_some_and(|tool| tool.kind != EncoderKind::Opus)
        {
            return Err(Error::Invalid(
                "encoder configured for the wrong codec".into(),
            ));
        }
        Ok(Self { flac, opus })
    }

    fn tool(&self, settings: &ConversionSettings) -> Option<&ConfiguredEncoder> {
        match settings {
            ConversionSettings::Flac { .. } => self.flac.as_ref(),
            ConversionSettings::Opus { .. } => self.opus.as_ref(),
        }
    }
}

impl AudioConversionPort for ProcessAudioConverter {
    fn configured_availability(
        &self,
        profile: &collector_core::ConversionProfile,
    ) -> EncoderAvailability {
        let tool = match profile {
            collector_core::ConversionProfile::Flac => self.flac.as_ref(),
            collector_core::ConversionProfile::Opus { .. } => self.opus.as_ref(),
        };
        match tool {
            Some(tool) => EncoderAvailability::Available {
                encoder: tool.kind.executable_name().into(),
                version: tool.version.clone(),
            },
            None => EncoderAvailability::Unavailable {
                reason: match profile {
                    collector_core::ConversionProfile::Flac => {
                        "FLAC encoder is not configured. Add encoders.flac to collector.toml and restart."
                    }
                    collector_core::ConversionProfile::Opus { .. } => {
                        "Opus encoder is not configured. Add encoders.opus to collector.toml and restart."
                    }
                }
                .into(),
            },
        }
    }

    fn availability(&self, recipe: &ConversionRecipe) -> EncoderAvailability {
        if let Err(error) = recipe.validate() {
            return EncoderAvailability::Unavailable {
                reason: error.to_string(),
            };
        }
        match self.tool(&recipe.settings) {
            Some(tool) => match ConfiguredEncoder::probe(tool.kind, &tool.path) {
                Ok(current) if current.version == recipe.encoder_version => {
                    EncoderAvailability::Available {
                        encoder: recipe.encoder.clone(),
                        version: current.version,
                    }
                }
                Ok(_) => EncoderAvailability::Unavailable {
                    reason: "configured encoder version does not match the immutable recipe".into(),
                },
                Err(error) => EncoderAvailability::Unavailable {
                    reason: error.to_string(),
                },
            },
            None => EncoderAvailability::Unavailable {
                reason: format!("configured {} tool is unavailable", recipe.encoder),
            },
        }
    }

    fn convert(
        &self,
        recipe: &ConversionRecipe,
        source: &mut (dyn junk_libs_disc::CdPcmReader + Send),
        scratch: &mut dyn ConversionScratch,
        execution: &mut dyn ProcessingExecutionContext,
    ) -> Result<ConversionValidation> {
        checkpoint(execution)?;
        recipe.validate()?;
        let tool = self.tool(&recipe.settings).ok_or_else(|| {
            Error::Unavailable(format!("configured {} tool is unavailable", recipe.encoder))
        })?;
        let current_tool = ConfiguredEncoder::probe(tool.kind, &tool.path)?;
        checkpoint(execution)?;
        if current_tool.version != recipe.encoder_version {
            return Err(Error::Unavailable(
                "configured encoder version does not match the immutable recipe".into(),
            ));
        }
        if scratch.seek(SeekFrom::End(0)).map_err(conversion_io)? != 0 {
            return Err(Error::Invalid(
                "conversion scratch must be a new empty handle".into(),
            ));
        }
        scratch.seek(SeekFrom::Start(0)).map_err(conversion_io)?;

        let result = (|| -> Result<ConversionValidation> {
            let source_frames = checked_source_frames(recipe, source.total_frames())?;
            let input_bytes = source_frames
                .checked_mul(4)
                .ok_or_else(|| Error::Invalid("conversion input length overflow".into()))?;
            let output_limit = input_bytes
                .checked_mul(2)
                .and_then(|bytes| bytes.checked_add(1024 * 1024))
                .unwrap_or(MAX_OUTPUT_BYTES)
                .min(MAX_OUTPUT_BYTES);
            let arguments = encoder_arguments(&recipe.settings, input_bytes);
            let mut command = isolated_command(&current_tool.path);
            let mut child = command
                .args(&arguments)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|error| {
                    Error::Unavailable(format!("failed to start {}: {error}", recipe.encoder))
                })?;
            let stdin = child.stdin.take().expect("piped stdin");
            let stdout = child.stdout.take().expect("piped stdout");
            let stderr = child.stderr.take().expect("piped stderr");
            let started = Instant::now();
            let stopped = AtomicBool::new(false);
            let worker_failed = AtomicBool::new(false);

            let (status, write_result, copy_result, diagnostics, cancelled) =
                thread::scope(|scope| {
                    let write = scope.spawn(|| {
                        let result = write_selected_pcm(source, &recipe.tracks, stdin, &stopped);
                        if result.is_err() {
                            worker_failed.store(true, Ordering::SeqCst);
                        }
                        result
                    });
                    let copy = scope.spawn(|| {
                        let result = copy_bounded(stdout, &mut *scratch, output_limit);
                        if result.is_err() {
                            worker_failed.store(true, Ordering::SeqCst);
                        }
                        result
                    });
                    let diagnostic = scope.spawn(|| {
                        let result = read_bounded(stderr, MAX_DIAGNOSTIC_BYTES);
                        if result.is_err() {
                            worker_failed.store(true, Ordering::SeqCst);
                        }
                        result
                    });
                    let (status, cancelled) =
                        wait_for_conversion(&mut child, execution, started, &worker_failed);
                    stopped.store(true, Ordering::SeqCst);
                    terminate(&mut child);
                    (
                        status,
                        write.join().expect("PCM writer did not panic"),
                        copy.join().expect("output reader did not panic"),
                        diagnostic.join().expect("diagnostic reader did not panic"),
                        cancelled,
                    )
                });
            checkpoint(execution)?;
            let status = status?;
            if cancelled {
                return Err(Error::JobCancelled);
            }
            let diagnostics = diagnostics.map_err(conversion_io)?;
            if !status.success() {
                return Err(Error::Unavailable(format!(
                    "{} conversion failed: {}",
                    recipe.encoder,
                    safe_diagnostic(&diagnostics)
                )));
            }
            write_result.map_err(conversion_io)?;
            let bytes = copy_result.map_err(conversion_io)?;
            scratch.flush().map_err(conversion_io)?;

            let (decoded_frames, lossless_pcm_equal, delay, padding) = match recipe.settings {
                ConversionSettings::Flac { .. } => {
                    scratch.seek(SeekFrom::Start(0)).map_err(conversion_io)?;
                    let tracks = recipe
                        .tracks
                        .iter()
                        .map(|track| (track.start.0, track.length.0))
                        .collect::<Vec<_>>();
                    let validation = junk_libs_audio::validation::validate_flac(
                        &tracks,
                        source,
                        scratch,
                        source_frames,
                        &mut || checkpoint(execution).map_err(std::io::Error::other),
                    );
                    checkpoint(execution)?;
                    validation.map_err(conversion_io)?;
                    (source_frames, true, 0, 0)
                }
                ConversionSettings::Opus { .. } => {
                    scratch.seek(SeekFrom::Start(0)).map_err(conversion_io)?;
                    let inspection = junk_libs_audio::validation::inspect_opus(
                        scratch,
                        source_frames,
                        &mut || checkpoint(execution).map_err(std::io::Error::other),
                    );
                    checkpoint(execution)?;
                    let opus = inspection.map_err(conversion_io)?;
                    (opus.decoded_frames, false, opus.pre_skip, opus.end_padding)
                }
            };
            let digest = digest_scratch(scratch, bytes, execution)?;
            Ok(ConversionValidation {
                media_type: match recipe.settings {
                    ConversionSettings::Flac { .. } => "audio/flac",
                    ConversionSettings::Opus { .. } => "audio/ogg; codecs=opus",
                }
                .into(),
                bytes: ByteLength(bytes),
                digest,
                decoded_format: recipe.output_format.clone(),
                decoded_frames,
                lossless_pcm_equal,
                encoder_delay_frames: delay,
                end_padding_frames: padding,
            })
        })();
        if result.is_err() {
            scratch.discard_partial().map_err(conversion_io)?;
        }
        result
    }
}

fn encoder_arguments(settings: &ConversionSettings, input_bytes: u64) -> Vec<String> {
    match settings {
        ConversionSettings::Flac { level } => vec![
            "--stdout".into(),
            "--silent".into(),
            "--force-raw-format".into(),
            "--endian=little".into(),
            "--sign=signed".into(),
            "--channels=2".into(),
            "--bps=16".into(),
            "--sample-rate=44100".into(),
            format!("--compression-level-{level}"),
            format!("--input-size={input_bytes}"),
            "-".into(),
        ],
        ConversionSettings::Opus {
            bitrate_kbps,
            frame_milliseconds,
        } => vec![
            "--quiet".into(),
            "--raw".into(),
            "--raw-endianness".into(),
            "0".into(),
            "--vbr".into(),
            "--comp".into(),
            "10".into(),
            "--expect-loss".into(),
            "0".into(),
            "--raw-bits".into(),
            "16".into(),
            "--raw-rate".into(),
            "44100".into(),
            "--raw-chan".into(),
            "2".into(),
            "--bitrate".into(),
            bitrate_kbps.to_string(),
            "--framesize".into(),
            frame_milliseconds.to_string(),
            "--music".into(),
            "-".into(),
            "-".into(),
        ],
    }
}

fn checked_source_frames(recipe: &ConversionRecipe, available: u64) -> Result<u64> {
    let mut total = 0_u64;
    for track in &recipe.tracks {
        let end = track
            .start
            .0
            .checked_add(track.length.0)
            .ok_or_else(|| Error::Invalid("conversion track range overflow".into()))?;
        if end > available {
            return Err(Error::Invalid(
                "conversion track exceeds the authorized PCM".into(),
            ));
        }
        total = total
            .checked_add(track.length.0)
            .ok_or_else(|| Error::Invalid("conversion length overflow".into()))?;
    }
    Ok(total)
}

fn write_selected_pcm(
    source: &mut (dyn junk_libs_disc::CdPcmReader + Send),
    tracks: &[collector_core::ConversionTrack],
    mut output: impl Write,
    stopped: &AtomicBool,
) -> std::io::Result<()> {
    let mut frames = vec![[0_i16; 2]; IO_FRAMES];
    let mut bytes = vec![0_u8; IO_FRAMES * 4];
    for track in tracks {
        source
            .seek_frame(track.start.0)
            .map_err(std::io::Error::other)?;
        let mut remaining = track.length.0;
        while remaining != 0 {
            if stopped.load(Ordering::SeqCst) {
                return Err(std::io::Error::other("conversion stopped"));
            }
            let wanted = usize::try_from(remaining.min(IO_FRAMES as u64)).unwrap();
            let count = source
                .read_frames(&mut frames[..wanted])
                .map_err(std::io::Error::other)?;
            if count > wanted {
                return Err(std::io::Error::other(
                    "PCM reader exceeded requested frames",
                ));
            }
            if count == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "authorized PCM ended inside a recipe track",
                ));
            }
            for (frame, raw) in frames[..count]
                .iter()
                .zip(bytes.as_chunks_mut::<4>().0.iter_mut())
            {
                raw[..2].copy_from_slice(&frame[0].to_le_bytes());
                raw[2..].copy_from_slice(&frame[1].to_le_bytes());
            }
            output.write_all(&bytes[..count * 4])?;
            remaining -= count as u64;
        }
    }
    Ok(())
}

fn wait_for_conversion(
    child: &mut Child,
    execution: &mut dyn ProcessingExecutionContext,
    started: Instant,
    worker_failed: &AtomicBool,
) -> (Result<ExitStatus>, bool) {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return (Ok(status), false),
            Ok(None) => {}
            Err(error) => return (Err(conversion_io(error)), false),
        }
        let cancelled = match execution.cancellation_requested() {
            Ok(cancelled) => cancelled,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return (Err(error), false);
            }
        };
        if worker_failed.load(Ordering::SeqCst) {
            terminate(child);
            return (
                Err(Error::Unavailable("encoder I/O worker failed".into())),
                false,
            );
        }
        if cancelled || started.elapsed() > CONVERSION_TIMEOUT {
            let _ = child.kill();
            let waited = child.wait().map_err(conversion_io);
            return (
                if cancelled {
                    waited
                } else {
                    Err(Error::Unavailable("conversion timed out".into()))
                },
                cancelled,
            );
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn digest_scratch(
    scratch: &mut dyn ConversionScratch,
    expected: u64,
    execution: &mut dyn ProcessingExecutionContext,
) -> Result<Digest> {
    scratch.seek(SeekFrom::Start(0)).map_err(conversion_io)?;
    let mut hasher = Sha256::new();
    let mut bytes = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        checkpoint(execution)?;
        let count = scratch.read(&mut buffer).map_err(conversion_io)?;
        if count == 0 {
            break;
        }
        bytes = bytes
            .checked_add(count as u64)
            .ok_or_else(|| Error::Invalid("conversion output length overflow".into()))?;
        if bytes > expected {
            return Err(Error::Invalid("conversion scratch grew".into()));
        }
        hasher.update(&buffer[..count]);
    }
    if bytes != expected {
        return Err(Error::Invalid("conversion scratch length changed".into()));
    }
    Ok(Digest {
        algorithm: collector_core::HashAlgorithm::Sha256,
        hex: hex::encode(hasher.finalize()),
    })
}

fn copy_bounded(
    mut input: impl Read,
    mut output: impl Write,
    maximum: u64,
) -> std::io::Result<u64> {
    let mut total = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(count as u64)
            .ok_or_else(|| std::io::Error::other("encoder output length overflow"))?;
        if total > maximum {
            return Err(std::io::Error::other("encoder output exceeded its bound"));
        }
        output.write_all(&buffer[..count])?;
    }
    Ok(total)
}

fn read_bounded(mut input: impl Read, maximum: usize) -> std::io::Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        if output.len().saturating_add(count) > maximum {
            return Err(std::io::Error::other(
                "process diagnostics exceeded their bound",
            ));
        }
        output.extend_from_slice(&buffer[..count]);
    }
    Ok(output)
}

fn run_small_process<I, S>(
    path: &Path,
    arguments: I,
    timeout: Duration,
) -> Result<(ExitStatus, Vec<u8>, Vec<u8>)>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut child = isolated_command(path)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| Error::Unavailable(format!("failed to start tool: {error}")))?;
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let started = Instant::now();
    thread::scope(|scope| {
        let out = scope.spawn(|| read_bounded(stdout, MAX_DIAGNOSTIC_BYTES));
        let err = scope.spawn(|| read_bounded(stderr, MAX_DIAGNOSTIC_BYTES));
        let status = loop {
            if let Some(status) = child.try_wait().map_err(conversion_io)? {
                terminate(&mut child);
                break status;
            }
            if started.elapsed() > timeout {
                terminate(&mut child);
                return Err(Error::Unavailable("tool version probe timed out".into()));
            }
            thread::sleep(Duration::from_millis(10));
        };
        Ok((
            status,
            out.join()
                .expect("version stdout reader did not panic")
                .map_err(conversion_io)?,
            err.join()
                .expect("version stderr reader did not panic")
                .map_err(conversion_io)?,
        ))
    })
}

fn checkpoint(execution: &mut dyn ProcessingExecutionContext) -> Result<()> {
    if execution.cancellation_requested()? {
        Err(Error::JobCancelled)
    } else {
        Ok(())
    }
}

fn isolated_command(path: &Path) -> Command {
    let mut command = Command::new(path);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command
}

fn terminate(child: &mut Child) {
    #[cfg(unix)]
    {
        // The child is spawned in its own group; descendants must not retain pipes.
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn safe_diagnostic(bytes: &[u8]) -> String {
    let value = String::from_utf8_lossy(bytes).replace(char::is_control, " ");
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "no bounded diagnostic".into()
    } else {
        trimmed.into()
    }
}

fn conversion_io(error: std::io::Error) -> Error {
    Error::Unavailable(format!("conversion I/O failed: {error}"))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::io::Cursor;

    use collector_app::{AudioConversionPort, EncoderAvailability, FrozenJobInput};
    use collector_core::{
        ConversionTrack, HashAlgorithm, PackageFileFingerprint, PackageFileId, PackageScope,
        PcmFrame, PcmSampleFormat, RecipeId, RepresentationId, SourceFingerprint,
    };

    use super::*;

    struct Frames {
        frames: Vec<[i16; 2]>,
        position: usize,
    }

    impl junk_libs_disc::CdPcmReader for Frames {
        fn total_frames(&self) -> u64 {
            self.frames.len() as u64
        }

        fn seek_frame(
            &mut self,
            frame: u64,
        ) -> std::result::Result<(), junk_libs_core::AnalysisError> {
            if frame > self.frames.len() as u64 {
                return Err(junk_libs_core::AnalysisError::invalid_format(
                    "seek past PCM",
                ));
            }
            self.position = frame as usize;
            Ok(())
        }

        fn read_frames(
            &mut self,
            output: &mut [[i16; 2]],
        ) -> std::result::Result<usize, junk_libs_core::AnalysisError> {
            let count = output.len().min(self.frames.len() - self.position).min(3);
            output[..count].copy_from_slice(&self.frames[self.position..self.position + count]);
            self.position += count;
            Ok(count)
        }
    }

    struct Execution {
        cancel_after: Option<usize>,
        checks: usize,
    }

    impl ProcessingExecutionContext for Execution {
        fn frozen_job(&self) -> &FrozenJobInput {
            panic!("conversion mechanism does not inspect application job payloads")
        }

        fn report_progress(
            &mut self,
            _phase: &str,
            _completed_units: u64,
            _total_units: u64,
        ) -> Result<()> {
            Ok(())
        }

        fn cancellation_requested(&self) -> Result<bool> {
            Ok(self.cancel_after.is_some_and(|limit| self.checks >= limit))
        }
    }

    fn recipe(version: &str, settings: ConversionSettings, frames: u64) -> ConversionRecipe {
        let (encoder, output_format) = match settings {
            ConversionSettings::Flac { .. } => ("flac", PcmSampleFormat::CDDA_I16_LE_STEREO),
            ConversionSettings::Opus { .. } => {
                ("opusenc", PcmSampleFormat::OPUS_DECODED_I16_STEREO)
            }
        };
        ConversionRecipe {
            id: RecipeId::new(),
            version: 1,
            settings,
            source_fingerprint: SourceFingerprint::calculate(
                PackageScope::Representation(RepresentationId::new()),
                vec![PackageFileFingerprint {
                    id: PackageFileId::new(),
                    bytes: ByteLength(frames * 4),
                    sha256: Digest {
                        algorithm: HashAlgorithm::Sha256,
                        hex: "00".repeat(32),
                    },
                }],
                None,
                BTreeMap::new(),
            )
            .unwrap(),
            tracks: vec![ConversionTrack {
                position: 1,
                start: PcmFrame(0),
                length: PcmFrame(frames),
            }],
            encoder: encoder.into(),
            encoder_version: version.into(),
            input_format: PcmSampleFormat::CDDA_I16_LE_STEREO,
            output_format,
            digest: Digest::sha256(b"unsealed"),
        }
        .seal()
        .unwrap()
    }

    #[test]
    fn absent_and_wrong_version_tools_are_explicitly_unavailable() {
        let converter = ProcessAudioConverter::default();
        let base_recipe = recipe("flac 1.5.0", ConversionSettings::Flac { level: 5 }, 4);
        assert!(matches!(
            converter.availability(&base_recipe),
            EncoderAvailability::Unavailable { reason } if reason.contains("unavailable")
        ));
        let mut source = Frames {
            frames: vec![[0, 0]; 4],
            position: 0,
        };
        assert!(matches!(
            converter.convert(
                &base_recipe,
                &mut source,
                &mut Cursor::new(Vec::new()),
                &mut Execution {
                    cancel_after: None,
                    checks: 0
                }
            ),
            Err(Error::Unavailable(_))
        ));
        if let Ok(tool) = ConfiguredEncoder::probe(EncoderKind::Flac, "flac") {
            let converter = ProcessAudioConverter::new(Some(tool), None).unwrap();
            let wrong = recipe("0.0-wrong", ConversionSettings::Flac { level: 5 }, 4);
            assert!(matches!(
                converter.availability(&wrong),
                EncoderAvailability::Unavailable { reason } if reason.contains("version")
            ));
        }
    }

    #[test]
    fn flac_1_5_0_round_trips_independent_integer_pcm_and_exact_ranges() {
        let Ok(tool) = ConfiguredEncoder::probe(EncoderKind::Flac, "flac") else {
            return;
        };
        if tool.version() != "flac 1.5.0" {
            return;
        }
        let converter = ProcessAudioConverter::new(Some(tool), None).unwrap();
        let expected = vec![
            [111, -111],
            [i16::MIN, i16::MAX],
            [-1, 1],
            [0, 0],
            [12_345, -12_345],
            [77, -88],
        ];
        let mut source = Frames {
            frames: expected.clone(),
            position: 0,
        };
        let mut recipe = recipe("flac 1.5.0", ConversionSettings::Flac { level: 5 }, 4);
        recipe.tracks[0].start = PcmFrame(1);
        recipe = recipe.seal().unwrap();
        let mut scratch = Cursor::new(Vec::new());
        let validation = converter
            .convert(
                &recipe,
                &mut source,
                &mut scratch,
                &mut Execution {
                    cancel_after: None,
                    checks: 0,
                },
            )
            .unwrap();
        assert!(validation.lossless_pcm_equal);
        assert_eq!(validation.decoded_frames, 4);
        assert_eq!(
            validation.decoded_format,
            PcmSampleFormat::CDDA_I16_LE_STEREO
        );
        assert_eq!(validation.bytes.0, scratch.get_ref().len() as u64);
        assert_eq!(validation.digest, Digest::sha256(scratch.get_ref()));
        assert_eq!(
            validation.digest.hex,
            "edb1aed9e9366afa9a5734a62cca23b40701a4d698633084ab54b394ce677588"
        );
    }

    #[test]
    fn recipe_range_must_exist_in_the_authorized_reader() {
        let recipe = recipe("flac 1.5.0", ConversionSettings::Flac { level: 5 }, 4);
        let source = Frames {
            frames: vec![[0, 0]; 2],
            position: 0,
        };
        assert!(matches!(
            checked_source_frames(&recipe, source.frames.len() as u64),
            Err(Error::Invalid(_))
        ));
    }

    #[test]
    #[cfg(unix)]
    fn process_failure_and_cancellation_never_report_output_success() {
        let recipe = recipe("flac 1.5.0", ConversionSettings::Flac { level: 5 }, 588);
        // Execute a checked-in fixture rather than a just-written executable:
        // Linux can transiently reject the latter with ETXTBSY on CI filesystems.
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/failing-flac.sh");
        let failing = ConfiguredEncoder::probe(EncoderKind::Flac, path).unwrap();
        let converter = ProcessAudioConverter::new(Some(failing), None).unwrap();
        let mut source = Frames {
            frames: vec![[0, 0]; 588],
            position: 0,
        };
        let mut failed_scratch = Cursor::new(Vec::new());
        assert!(matches!(
            converter.convert(
                &recipe,
                &mut source,
                &mut failed_scratch,
                &mut Execution {
                    cancel_after: None,
                    checks: 0,
                }
            ),
            Err(Error::Unavailable(_))
        ));
        assert!(failed_scratch.get_ref().is_empty());

        let Ok(tool) = ConfiguredEncoder::probe(EncoderKind::Flac, "flac") else {
            return;
        };
        if tool.version() != "flac 1.5.0" {
            return;
        }
        let converter = ProcessAudioConverter::new(Some(tool), None).unwrap();
        let mut source = Frames {
            frames: vec![[1, -1]; 588],
            position: 0,
        };
        let mut cancelled_scratch = Cursor::new(Vec::new());
        assert!(matches!(
            converter.convert(
                &recipe,
                &mut source,
                &mut cancelled_scratch,
                &mut Execution {
                    cancel_after: Some(0),
                    checks: 0,
                }
            ),
            Err(Error::JobCancelled)
        ));
        assert!(cancelled_scratch.get_ref().is_empty());
    }

    #[test]
    #[cfg(unix)]
    fn worker_failure_terminates_a_child_that_keeps_running() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/running-flac.sh");
        let tool = ConfiguredEncoder::probe(EncoderKind::Flac, path).unwrap();
        let converter = ProcessAudioConverter::new(Some(tool), None).unwrap();
        struct BrokenScratch(Cursor<Vec<u8>>);
        impl Read for BrokenScratch {
            fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
                self.0.read(b)
            }
        }
        impl std::io::Seek for BrokenScratch {
            fn seek(&mut self, p: SeekFrom) -> std::io::Result<u64> {
                self.0.seek(p)
            }
        }
        impl Write for BrokenScratch {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("disk full"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        impl ConversionScratch for BrokenScratch {
            fn discard_partial(&mut self) -> std::io::Result<()> {
                self.0.discard_partial()
            }
        }
        let mut scratch = BrokenScratch(Cursor::new(Vec::new()));
        let started = Instant::now();
        let result = converter.convert(
            &recipe("flac 1.5.0", ConversionSettings::Flac { level: 5 }, 4),
            &mut Frames {
                frames: vec![[0, 0]; 4],
                position: 0,
            },
            &mut scratch,
            &mut Execution {
                cancel_after: None,
                checks: 0,
            },
        );
        assert!(result.is_err());
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    #[test]
    fn cancellation_during_independent_validation_clears_output() {
        use std::sync::Arc;
        let Ok(tool) = ConfiguredEncoder::probe(EncoderKind::Flac, "flac") else {
            return;
        };
        let version = tool.version().to_owned();
        let converter = ProcessAudioConverter::new(Some(tool), None).unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        struct Source {
            inner: Frames,
            seeks: usize,
            cancelled: Arc<AtomicBool>,
        }
        impl junk_libs_disc::CdPcmReader for Source {
            fn total_frames(&self) -> u64 {
                self.inner.total_frames()
            }
            fn seek_frame(
                &mut self,
                p: u64,
            ) -> std::result::Result<(), junk_libs_core::AnalysisError> {
                self.seeks += 1;
                if self.seeks == 2 {
                    self.cancelled.store(true, Ordering::SeqCst);
                }
                self.inner.seek_frame(p)
            }
            fn read_frames(
                &mut self,
                b: &mut [[i16; 2]],
            ) -> std::result::Result<usize, junk_libs_core::AnalysisError> {
                self.inner.read_frames(b)
            }
        }
        struct CancelExecution(Arc<AtomicBool>);
        impl ProcessingExecutionContext for CancelExecution {
            fn frozen_job(&self) -> &FrozenJobInput {
                panic!("unused")
            }
            fn report_progress(&mut self, _: &str, _: u64, _: u64) -> Result<()> {
                Ok(())
            }
            fn cancellation_requested(&self) -> Result<bool> {
                Ok(self.0.load(Ordering::SeqCst))
            }
        }
        let mut source = Source {
            inner: Frames {
                frames: vec![[1, -1]; 588],
                position: 0,
            },
            seeks: 0,
            cancelled: cancelled.clone(),
        };
        let mut scratch = Cursor::new(Vec::new());
        let result = converter.convert(
            &recipe(&version, ConversionSettings::Flac { level: 5 }, 588),
            &mut source,
            &mut scratch,
            &mut CancelExecution(cancelled),
        );
        assert!(matches!(result, Err(Error::JobCancelled)));
        assert!(scratch.get_ref().is_empty());
    }
}
