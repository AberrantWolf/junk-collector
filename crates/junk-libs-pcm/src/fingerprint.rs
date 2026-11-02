//! Chromaprint Test2 over bounded integer WAV PCM, with the shared result contract.
use crate::{IntegerPcmReader, WavReader};
use junk_libs_music::{
    Error, Result,
    fingerprint::{AudioFingerprint, IMPLEMENTATION, MAX_FINGERPRINT_SECONDS},
};
use std::io::{Read, Seek};
pub fn compute<R: Read + Seek + Send>(
    reader: R,
    checkpoint: &mut dyn FnMut() -> Result<()>,
) -> Result<AudioFingerprint> {
    checkpoint()?;
    let opened = WavReader::open(reader, &mut || checkpoint().map_err(std::io::Error::other));
    checkpoint()?;
    let mut reader = opened.map_err(|e| Error::Invalid(e.to_string()))?;
    let format = reader.format();
    let frames = reader.total_frames();
    let cap = frames.min(u64::from(format.sample_rate) * MAX_FINGERPRINT_SECONDS);
    let mut printer = chromaprint::Fingerprinter::new(chromaprint::Algorithm::Test2);
    printer
        .start(format.sample_rate, format.channels)
        .map_err(|e| Error::Invalid(e.to_string()))?;
    let mut bytes = vec![0; 4096 * format.frame_bytes()];
    let mut position = 0u64;
    while position < cap {
        checkpoint()?;
        let wanted = (cap - position).min(4096) as usize;
        let count = reader
            .read_pcm(&mut bytes[..wanted * format.frame_bytes()])
            .map_err(|e| Error::Invalid(e.to_string()))?;
        if count == 0 || count > wanted {
            return Err(Error::Invalid("WAV fingerprint PCM length".into()));
        }
        let take = (cap.saturating_sub(position)).min(count as u64) as usize;
        let samples = bytes[..take * format.frame_bytes()]
            .chunks_exact(usize::from(format.bits / 8))
            .map(|raw| match format.bits {
                16 => i16::from_le_bytes(raw.try_into().unwrap()),
                24 => {
                    let n = i32::from_le_bytes([0, raw[0], raw[1], raw[2]]) >> 8;
                    (f64::from(n) / 256.).round().clamp(-32768., 32767.) as i16
                }
                _ => unreachable!(),
            })
            .collect::<Vec<_>>();
        printer
            .feed(&samples)
            .map_err(|e| Error::Invalid(e.to_string()))?;
        position += count as u64;
    }
    checkpoint()?;
    printer
        .finish()
        .map_err(|e| Error::Invalid(e.to_string()))?;
    let result = AudioFingerprint {
        algorithm: 1,
        implementation: IMPLEMENTATION.into(),
        sample_rate: format.sample_rate,
        channels: format.channels,
        decoded_frames: cap,
        total_frames: Some(frames),
        analyzed_frames: cap,
        duration_ms: frames * 1000 / u64::from(format.sample_rate),
        fingerprint: printer.encode(),
    };
    result.validate()?;
    checkpoint()?;
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wav_fingerprints_have_native_coordinates_and_cancellation() {
        let bytes =
            include_bytes!("../../collector-store/tests/fixtures/digital-audio/03-tagged.wav");
        let result = compute(std::io::Cursor::new(bytes), &mut || Ok(())).unwrap();
        assert_eq!(
            (
                result.sample_rate,
                result.channels,
                result.decoded_frames,
                result.duration_ms
            ),
            (96000, 1, 96000, 1000)
        );
        assert!(matches!(
            compute(std::io::Cursor::new(bytes), &mut || Err(Error::Cancelled)),
            Err(Error::Cancelled)
        ));
    }
}
#[cfg(test)]
mod independent_vectors {
    use super::*;
    #[test]
    fn independently_decoded_wav_matches_the_original_flac_fingerprint() {
        if std::env::var_os("COLLECTOR_TEST_FLAC").is_none() {
            return;
        }
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../collector-store/tests/fixtures/digital-matching/01.flac");
        let output = std::process::Command::new("flac")
            .args(["--decode", "--stdout", "--silent"])
            .arg(path)
            .output()
            .unwrap();
        assert!(output.status.success());
        let wav = compute(std::io::Cursor::new(output.stdout), &mut || Ok(())).unwrap();
        let flac = junk_libs_music::fingerprint::compute_window(
            junk_libs_music::fingerprint::ByteSource::new(
                std::io::Cursor::new(include_bytes!(
                    "../../collector-store/tests/fixtures/digital-matching/01.flac"
                )),
                junk_libs_music::fingerprint::MediaHint::new(None, Some("flac"), None),
            ),
            wav.total_frames(),
            &mut || Ok(()),
        )
        .unwrap();
        assert_eq!(wav, flac);
    }
}
