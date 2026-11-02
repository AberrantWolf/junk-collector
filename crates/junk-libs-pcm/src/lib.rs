//! Bounded integer PCM readers and independent lossless validation.
//! No package identities, paths, storage or application policy.
pub mod fingerprint;
pub mod flac;
pub mod metadata;
use std::io::{self, Read, Seek, SeekFrom};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Format {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits: u16,
}
impl Format {
    pub fn frame_bytes(self) -> usize {
        usize::from(self.channels) * usize::from(self.bits / 8)
    }
}
pub trait IntegerPcmReader: Send {
    fn format(&self) -> Format;
    fn total_frames(&self) -> u64;
    fn seek_frame(&mut self, frame: u64) -> io::Result<()>;
    /// Returns complete frames encoded as signed little-endian integer PCM.
    fn read_pcm(&mut self, output: &mut [u8]) -> io::Result<usize>;
}
/// Adapt existing exact CD readers without changing their CD-specific contract.
pub struct CdReader(pub Box<dyn junk_libs_disc::CdPcmReader + Send>);
impl<T: junk_libs_disc::CdPcmReader + Send> IntegerPcmReader for T {
    fn format(&self) -> Format {
        Format {
            sample_rate: 44100,
            channels: 2,
            bits: 16,
        }
    }
    fn total_frames(&self) -> u64 {
        junk_libs_disc::CdPcmReader::total_frames(self)
    }
    fn seek_frame(&mut self, frame: u64) -> io::Result<()> {
        junk_libs_disc::CdPcmReader::seek_frame(self, frame).map_err(io::Error::other)
    }
    fn read_pcm(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let wanted = (output.len() / 4).min(16384);
        let mut frames = vec![[0i16; 2]; wanted];
        let count = junk_libs_disc::CdPcmReader::read_frames(self, &mut frames)
            .map_err(io::Error::other)?;
        if count > wanted {
            return Err(invalid("PCM reader exceeded request"));
        }
        for (frame, bytes) in frames[..count]
            .iter()
            .zip(output.as_chunks_mut::<4>().0.iter_mut())
        {
            bytes[..2].copy_from_slice(&frame[0].to_le_bytes());
            bytes[2..].copy_from_slice(&frame[1].to_le_bytes());
        }
        Ok(count)
    }
}
/// Owning adapter for an opaque CD reader.
impl junk_libs_disc::CdPcmReader for CdReader {
    fn total_frames(&self) -> u64 {
        self.0.total_frames()
    }
    fn seek_frame(&mut self, frame: u64) -> Result<(), junk_libs_core::AnalysisError> {
        self.0.seek_frame(frame)
    }
    fn read_frames(
        &mut self,
        output: &mut [[i16; 2]],
    ) -> Result<usize, junk_libs_core::AnalysisError> {
        self.0.read_frames(output)
    }
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

pub struct WavReader<R> {
    reader: R,
    format: Format,
    data: u64,
    frames: u64,
    position: u64,
    trailing_bytes: u64,
    pub metadata: junk_libs_audio::FileAudioInfo,
}
impl<R: Read + Seek + Send> WavReader<R> {
    /// Checked RIFF/WAVE PCM, including WAVE_FORMAT_EXTENSIBLE integer PCM.
    /// Bytes outside the declared RIFF segment are reported, never interpreted.
    /// Chunk sizes are checked before reads/allocation; metadata is skipped in
    /// bounded seeks, never allocated. The caller supplies cooperative cancellation.
    pub fn open(mut reader: R, checkpoint: &mut dyn FnMut() -> io::Result<()>) -> io::Result<Self> {
        checkpoint()?;
        let file_end = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(0))?;
        let mut header = [0; 12];
        reader.read_exact(&mut header)?;
        let end = u64::from(u32::from_le_bytes(header[4..8].try_into().unwrap())) + 8;
        if &header[..4] != b"RIFF" || &header[8..] != b"WAVE" || end < 12 || end > file_end {
            return Err(invalid("invalid RIFF/WAVE length or signature"));
        }
        let mut format = None;
        let mut data = None;
        let mut position = 12u64;
        let mut chunks = 0;
        let mut metadata = junk_libs_audio::FileAudioInfo::default();
        let mut metadata_bytes = 0u64;
        let mut info_chunks = Vec::new();
        let mut codepage = None;
        while position < end {
            checkpoint()?;
            chunks += 1;
            if chunks > 4096 || end - position < 8 {
                return Err(invalid("invalid WAV chunks"));
            }
            reader.seek(SeekFrom::Start(position))?;
            let mut chunk = [0; 8];
            reader.read_exact(&mut chunk)?;
            let size = u64::from(u32::from_le_bytes(chunk[4..].try_into().unwrap()));
            let next = position
                .checked_add(8 + size + size % 2)
                .ok_or_else(|| invalid("WAV chunk overflow"))?;
            if next > end {
                return Err(invalid("truncated WAV chunk"));
            }
            match &chunk[..4] {
                b"fmt " => {
                    if format.is_some() || !matches!(size, 16 | 18 | 40) {
                        return Err(invalid("unsupported WAV format chunk"));
                    }
                    let mut raw = [0u8; 40];
                    reader.read_exact(&mut raw[..size as usize])?;
                    let u16_at = |n| u16::from_le_bytes(raw[n..n + 2].try_into().unwrap());
                    let u32_at = |n| u32::from_le_bytes(raw[n..n + 4].try_into().unwrap());
                    let f = Format {
                        channels: u16_at(2),
                        sample_rate: u32_at(4),
                        bits: u16_at(14),
                    };
                    if !matches!(f.channels, 1 | 2)
                        || !matches!(f.bits, 16 | 24)
                        || !(8000..=192000).contains(&f.sample_rate)
                        || usize::from(u16_at(12)) != f.frame_bytes()
                        || u32_at(8) != f.sample_rate * f.frame_bytes() as u32
                    {
                        return Err(invalid("unsupported or inconsistent WAV PCM format"));
                    }
                    match u16_at(0) {
                        // Some mastering tools emit a 40-byte format chunk for
                        // ordinary PCM, with an empty extension and zero padding.
                        // Accept only that unambiguous layout, not unknown extensions.
                        1 if size == 16 || raw[16..size as usize].iter().all(|b| *b == 0) => {}
                        0xfffe
                            if size == 40
                                && u16_at(16) == 22
                                && u16_at(18) == f.bits
                                && (u32_at(20) == 0
                                    || u32_at(20) == if f.channels == 1 { 4 } else { 3 })
                                && raw[24..40]
                                    == [
                                        1, 0, 0, 0, 0, 0, 16, 0, 128, 0, 0, 170, 0, 56, 155, 113,
                                    ] => {}
                        _ => return Err(invalid("only integer PCM WAV is supported")),
                    }
                    format = Some(f);
                }
                b"data" => {
                    if data.is_some() || format.is_none() {
                        return Err(invalid("duplicate or misplaced WAV data"));
                    }
                    data = Some((position + 8, size));
                }
                b"CSET" => {
                    if size != 8 || codepage.is_some() {
                        return Err(invalid("invalid or duplicate WAV character set"));
                    }
                    let mut charset = [0; 8];
                    reader.read_exact(&mut charset)?;
                    codepage = Some(u16::from_le_bytes(charset[..2].try_into().unwrap()));
                    metadata::validate_codepage(codepage)?;
                }
                b"id3 " | b"ID3 " => {
                    metadata_bytes = metadata_bytes
                        .checked_add(size)
                        .ok_or_else(|| invalid("metadata overflow"))?;
                    if metadata_bytes > 32 * 1024 * 1024 {
                        return Err(invalid("WAV metadata exceeds 32 MiB"));
                    }
                    let mut bytes = vec![0; size as usize];
                    metadata::read_bounded(&mut reader, &mut bytes, checkpoint)?;
                    metadata::read_id3(bytes, &mut metadata, checkpoint)?;
                }
                b"LIST" => {
                    if size < 4 {
                        return Err(invalid("truncated WAV LIST type"));
                    }
                    let mut kind = [0; 4];
                    reader.read_exact(&mut kind)?;
                    if &kind == b"INFO" {
                        metadata_bytes = metadata_bytes
                            .checked_add(size)
                            .ok_or_else(|| invalid("metadata overflow"))?;
                        if metadata_bytes > 32 * 1024 * 1024 {
                            return Err(invalid("WAV metadata exceeds 32 MiB"));
                        }
                        info_chunks.push((position + 12, size - 4));
                    }
                    if &kind == b"wavl" {
                        return Err(invalid("unsupported WAV sample list"));
                    }
                }
                // These alter sample interpretation; never silently ignore them.
                b"wavl" | b"slnt" => return Err(invalid("unsupported WAV sample layout")),
                _ => {}
            }
            position = next;
        }
        for (offset, len) in info_chunks {
            checkpoint()?;
            reader.seek(SeekFrom::Start(offset))?;
            metadata::read_info(&mut reader, len, &mut metadata, codepage, checkpoint)?;
        }
        let format = format.ok_or_else(|| invalid("missing WAV format"))?;
        let (data, size) = data.ok_or_else(|| invalid("missing WAV data"))?;
        let frame_bytes = format.frame_bytes() as u64;
        if size == 0
            || size % frame_bytes != 0
            || size / frame_bytes > u64::from(format.sample_rate) * 86400
        {
            return Err(invalid("invalid WAV PCM length"));
        }
        checkpoint()?;
        reader.seek(SeekFrom::Start(data))?;
        Ok(Self {
            reader,
            format,
            data,
            frames: size / frame_bytes,
            position: 0,
            trailing_bytes: file_end - end,
            metadata,
        })
    }

    /// Uninterpreted bytes after the complete declared RIFF segment. These are
    /// part of the original file, but never part of this reader's PCM or metadata.
    pub fn trailing_bytes(&self) -> u64 {
        self.trailing_bytes
    }
}
impl<R: Read + Seek + Send> IntegerPcmReader for WavReader<R> {
    fn format(&self) -> Format {
        self.format
    }
    fn total_frames(&self) -> u64 {
        self.frames
    }
    fn seek_frame(&mut self, frame: u64) -> io::Result<()> {
        if frame > self.frames {
            return Err(invalid("WAV seek beyond PCM"));
        }
        self.reader.seek(SeekFrom::Start(
            self.data + frame * self.format.frame_bytes() as u64,
        ))?;
        self.position = frame;
        Ok(())
    }
    fn read_pcm(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let frame_bytes = self.format.frame_bytes();
        let count = (self.frames - self.position)
            .min((output.len() / frame_bytes).min(16384) as u64) as usize;
        self.reader.read_exact(&mut output[..count * frame_bytes])?;
        self.position += count as u64;
        Ok(count)
    }
}

pub fn validate_flac(
    tracks: &[(u64, u64)],
    source: &mut dyn IntegerPcmReader,
    scratch: &mut dyn Read,
    expected_frames: u64,
    checkpoint: &mut dyn FnMut() -> io::Result<()>,
) -> io::Result<()> {
    checkpoint()?;
    let mut decoder = claxon::FlacReader::new(scratch).map_err(io::Error::other)?;
    let info = decoder.streaminfo();
    let format = source.format();
    if info.sample_rate != format.sample_rate
        || info.channels != u32::from(format.channels)
        || info.bits_per_sample != u32::from(format.bits)
        || info.samples.is_some_and(|n| n != expected_frames)
    {
        return Err(invalid("FLAC format or length differs from source PCM"));
    }
    if !matches!(format.channels, 1 | 2)
        || !matches!(format.bits, 16 | 24)
        || !(8000..=192000).contains(&format.sample_rate)
    {
        return Err(invalid("unsupported source PCM format"));
    }
    let mut samples = decoder.samples();
    let mut bytes = vec![0; 16384 * format.frame_bytes()];
    let mut compared = 0u64;
    for &(start, length) in tracks {
        source.seek_frame(start)?;
        let mut remaining = length;
        while remaining != 0 {
            checkpoint()?;
            let wanted = remaining.min(16384) as usize;
            let count = source.read_pcm(&mut bytes[..wanted * format.frame_bytes()])?;
            if count == 0 || count > wanted {
                return Err(invalid("source PCM length mismatch"));
            }
            for raw in
                bytes[..count * format.frame_bytes()].chunks_exact(usize::from(format.bits / 8))
            {
                let expected = match format.bits {
                    16 => i32::from(i16::from_le_bytes(raw.try_into().unwrap())),
                    24 => i32::from_le_bytes([0, raw[0], raw[1], raw[2]]) >> 8,
                    _ => return Err(invalid("unsupported PCM sample width")),
                };
                let actual = samples
                    .next()
                    .ok_or_else(|| invalid("truncated FLAC PCM"))?
                    .map_err(io::Error::other)?;
                if expected != actual {
                    return Err(invalid("FLAC PCM differs from source"));
                }
            }
            remaining -= count as u64;
            compared += count as u64;
        }
    }
    checkpoint()?;
    if compared != expected_frames || samples.next().is_some() {
        return Err(invalid("FLAC frame count mismatch"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    fn wave(channels: u16, bits: u16, samples: &[u8]) -> Vec<u8> {
        let align = channels * bits / 8;
        let mut bytes = b"RIFF".to_vec();
        bytes.extend((36 + samples.len() as u32 + samples.len() as u32 % 2).to_le_bytes());
        bytes.extend(b"WAVEfmt ");
        bytes.extend(16u32.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(channels.to_le_bytes());
        bytes.extend(96000u32.to_le_bytes());
        bytes.extend((96000u32 * u32::from(align)).to_le_bytes());
        bytes.extend(align.to_le_bytes());
        bytes.extend(bits.to_le_bytes());
        bytes.extend(b"data");
        bytes.extend((samples.len() as u32).to_le_bytes());
        bytes.extend(samples);
        if samples.len() % 2 == 1 {
            bytes.push(0);
        }
        bytes
    }
    #[test]
    fn smallest_mono_24_bit_and_stereo_16_bit_streams_keep_exact_bytes_and_seeks() {
        for (channels, bits, pcm) in [(1, 24, vec![0, 0, 128]), (2, 16, vec![0, 128, 255, 127])] {
            let mut reader =
                WavReader::open(Cursor::new(wave(channels, bits, &pcm)), &mut || Ok(())).unwrap();
            assert_eq!(reader.total_frames(), 1);
            assert_eq!(
                reader.format(),
                Format {
                    sample_rate: 96000,
                    channels,
                    bits
                }
            );
            let mut result = [0; 32];
            assert_eq!(reader.read_pcm(&mut result).unwrap(), 1);
            assert_eq!(&result[..pcm.len()], pcm);
            assert_eq!(reader.read_pcm(&mut result).unwrap(), 0);
            reader.seek_frame(0).unwrap();
            assert_eq!(reader.read_pcm(&mut result).unwrap(), 1);
            assert!(reader.seek_frame(2).is_err());
        }
    }
    #[test]
    fn complete_riff_with_uninterpreted_suffix_keeps_exact_pcm_and_bounds_reads() {
        struct Bounded {
            bytes: Cursor<Vec<u8>>,
            riff_end: u64,
        }
        impl Read for Bounded {
            fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
                assert!(self.bytes.position() + output.len() as u64 <= self.riff_end);
                self.bytes.read(output)
            }
        }
        impl Seek for Bounded {
            fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
                self.bytes.seek(position)
            }
        }
        for (channels, bits, pcm) in [(1, 24, vec![0, 0, 128]), (2, 16, vec![0, 128, 255, 127])] {
            let mut bytes = wave(channels, bits, &pcm);
            let riff_end = bytes.len() as u64;
            // Includes an incomplete frame, fake metadata and a second WAV.
            // None may be visited or used to extend the declared audio.
            bytes.extend(b"\xffID3\xffLIST\xff");
            bytes.extend(wave(channels, bits, &pcm));
            let trailing = bytes.len() as u64 - riff_end;
            let mut reader = WavReader::open(
                Bounded {
                    bytes: Cursor::new(bytes),
                    riff_end,
                },
                &mut || Ok(()),
            )
            .unwrap();
            assert_eq!(reader.trailing_bytes(), trailing);
            assert!(reader.metadata.tags.is_empty());
            assert!(reader.metadata.pictures.is_empty());
            assert_eq!(reader.total_frames(), 1);
            let mut output = [0; 48];
            assert_eq!(reader.read_pcm(&mut output).unwrap(), 1);
            assert_eq!(&output[..pcm.len()], pcm);
            assert_eq!(reader.read_pcm(&mut output).unwrap(), 0);
            reader.seek_frame(0).unwrap();
            assert_eq!(reader.read_pcm(&mut output).unwrap(), 1);
            assert_eq!(&output[..pcm.len()], pcm);
            assert!(reader.seek_frame(2).is_err());
        }
    }
    #[test]
    fn suffix_cannot_complete_or_repair_malformed_declared_riff() {
        let good = wave(2, 16, &[0; 8]);
        // All bytes physically exist, but the PCM chunk crosses the RIFF boundary.
        for declared_end in [0, 8, 11, 12, 36, 43, good.len() - 1, good.len() + 1] {
            let mut bad = good.clone();
            bad[4..8].copy_from_slice(&(declared_end as u32).saturating_sub(8).to_le_bytes());
            bad.extend(b"ID3\0\0\0\0\0");
            assert!(
                WavReader::open(Cursor::new(bad), &mut || Ok(())).is_err(),
                "end {declared_end}"
            );
        }
        let reader = WavReader::open(Cursor::new(good), &mut || Ok(())).unwrap();
        assert_eq!(reader.trailing_bytes(), 0);
        // The required pad exists physically, but is outside the declared RIFF.
        let mut missing_pad = wave(1, 24, &[0, 0, 128]);
        let declared_end = missing_pad.len() - 1;
        missing_pad[4..8].copy_from_slice(&((declared_end - 8) as u32).to_le_bytes());
        assert!(WavReader::open(Cursor::new(missing_pad), &mut || Ok(())).is_err());
    }
    #[test]
    fn padded_pcm_format_keeps_samples_and_refuses_unknown_extensions() {
        let pcm = [0, 0, 128, 255, 255, 127];
        let mut bytes = wave(1, 24, &pcm);
        bytes.splice(36..36, [0; 24]);
        bytes[16..20].copy_from_slice(&40u32.to_le_bytes());
        let length = (bytes.len() - 8) as u32;
        bytes[4..8].copy_from_slice(&length.to_le_bytes());
        let mut reader = WavReader::open(Cursor::new(&bytes), &mut || Ok(())).unwrap();
        let mut output = [0; 6];
        assert_eq!(reader.read_pcm(&mut output).unwrap(), 2);
        assert_eq!(output, pcm);
        for offset in 36..60 {
            let mut bad = bytes.clone();
            bad[offset] = 1;
            assert!(WavReader::open(Cursor::new(bad), &mut || Ok(())).is_err());
        }
    }
    #[test]
    fn truncation_aliases_inconsistent_format_and_duplicate_chunks_fail_closed() {
        let good = wave(2, 16, &[0; 8]);
        for end in 0..good.len() {
            assert!(
                WavReader::open(Cursor::new(&good[..end]), &mut || Ok(())).is_err(),
                "length {end}"
            );
        }
        for (offset, value) in [
            (0, b'X'),
            (8, b'X'),
            (20, 3),
            (22, 3),
            (28, 1),
            (32, 1),
            (34, 8),
            (40, 7),
        ] {
            let mut bad = good.clone();
            bad[offset] = value;
            assert!(
                WavReader::open(Cursor::new(bad), &mut || Ok(())).is_err(),
                "offset {offset}"
            );
        }
        let mut bad = good.clone();
        bad.extend_from_slice(&good[36..]);
        let length = (bad.len() - 8) as u32;
        bad[4..8].copy_from_slice(&length.to_le_bytes());
        assert!(WavReader::open(Cursor::new(bad), &mut || Ok(())).is_err());
        let mut bad = wave(1, 24, &[0; 2]);
        assert!(WavReader::open(Cursor::new(&bad), &mut || Ok(())).is_err());
        bad = wave(1, 24, &[]);
        assert!(WavReader::open(Cursor::new(bad), &mut || Ok(())).is_err());
    }
    #[test]
    fn waveform_lists_cannot_hide_an_unsupported_second_audio_layout() {
        let mut bytes = wave(2, 16, &[0; 4]);
        bytes.extend(b"LIST");
        bytes.extend(4u32.to_le_bytes());
        bytes.extend(b"wavl");
        let length = (bytes.len() - 8) as u32;
        bytes[4..8].copy_from_slice(&length.to_le_bytes());
        assert!(WavReader::open(Cursor::new(bytes), &mut || Ok(())).is_err());
    }
    #[test]
    fn extensible_integer_pcm_is_checked_and_float_or_valid_bit_changes_are_rejected() {
        let mut bytes = wave(1, 24, &[0, 0, 128]);
        let extension: Vec<u8> = [
            22u16.to_le_bytes().as_slice(),
            24u16.to_le_bytes().as_slice(),
            4u32.to_le_bytes().as_slice(),
            &[1, 0, 0, 0, 0, 0, 16, 0, 128, 0, 0, 170, 0, 56, 155, 113],
        ]
        .concat();
        bytes.splice(36..36, extension);
        bytes[16..20].copy_from_slice(&40u32.to_le_bytes());
        bytes[20..22].copy_from_slice(&0xfffeu16.to_le_bytes());
        let length = (bytes.len() - 8) as u32;
        bytes[4..8].copy_from_slice(&length.to_le_bytes());
        assert!(WavReader::open(Cursor::new(&bytes), &mut || Ok(())).is_ok());
        for (offset, value) in [(38, 16), (44, 3), (40, 3)] {
            let mut bad = bytes.clone();
            bad[offset] = value;
            assert!(WavReader::open(Cursor::new(bad), &mut || Ok(())).is_err());
        }
    }
    #[test]
    fn chunk_traversal_is_bounded_and_cancellable() {
        let mut bytes = wave(1, 16, &[0; 2]);
        for _ in 0..4096 {
            bytes.extend(b"JUNK\0\0\0\0");
        }
        let length = (bytes.len() - 8) as u32;
        bytes[4..8].copy_from_slice(&length.to_le_bytes());
        assert!(WavReader::open(Cursor::new(&bytes), &mut || Ok(())).is_err());
        let mut calls = 0;
        let error = WavReader::open(Cursor::new(bytes), &mut || {
            calls += 1;
            if calls == 5 {
                Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"))
            } else {
                Ok(())
            }
        })
        .err()
        .unwrap();
        assert_eq!(error.kind(), io::ErrorKind::Interrupted);
        assert_eq!(calls, 5);
    }
}
