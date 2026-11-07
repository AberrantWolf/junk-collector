//! Streaming adaptation of decoded file audio to the existing stereo Opus input.
use crate::{Format, IntegerPcmReader, WavReader};
use junk_libs_audio::{
    AudioDecoder, AudioReadSeek, ByteSource, FileAudioInfo, FramePosition, PcmChunk, PcmFormat,
    SincResampler,
};
use std::io;

const RATE: u32 = 44_100;
const BLOCK: usize = 2048;

pub fn frames_at_input_rate(frames: u64, rate: u32) -> Option<u64> {
    if !(8000..=384000).contains(&rate) {
        return None;
    }
    let rate = u64::from(rate);
    frames
        .checked_mul(u64::from(RATE))?
        .checked_add(rate / 2)
        .map(|frames| frames / rate)
        .filter(|frames| *frames > 0)
}

enum Decoder {
    Compressed(Box<AudioDecoder>),
    Wav(Box<WavReader<Box<dyn AudioReadSeek>>>),
}

pub struct OpusFileInput {
    decoder: Decoder,
    stereo_format: PcmFormat,
    channels: u16,
    resampler: Option<SincResampler>,
    original_frames: u64,
    consumed: u64,
    total: u64,
    position: u64,
    finished: bool,
}

impl OpusFileInput {
    pub fn open(source: ByteSource, recorded: &FileAudioInfo) -> io::Result<Self> {
        let original_frames = recorded
            .decoded_frames
            .ok_or_else(|| io::Error::other("Missing recorded audio length"))?;
        let format_mismatch =
            || io::Error::other("Decoded audio format differs from the recorded original");
        let decoder = if recorded.codec == "wav" {
            // Read only the declared PCM segment, including for WAVs with retained trailers.
            let reader = WavReader::open(source.into_parts().0, &mut || Ok(()))?;
            let format = reader.format();
            if format.sample_rate != recorded.sample_rate
                || format.channels != recorded.channels
                || Some(u32::from(format.bits)) != recorded.bits_per_sample
                || reader.total_frames() != original_frames
            {
                return Err(format_mismatch());
            }
            Decoder::Wav(Box::new(reader))
        } else {
            let decoder = AudioDecoder::open(source).map_err(io::Error::other)?;
            let format = decoder.format();
            if decoder.inspection.codec != recorded.codec
                || format.sample_rate() != recorded.sample_rate
                || format.channels() != recorded.channels
            {
                return Err(format_mismatch());
            }
            Decoder::Compressed(Box::new(decoder))
        };
        if !matches!(recorded.channels, 1 | 2) {
            return Err(format_mismatch());
        }
        let total = frames_at_input_rate(original_frames, recorded.sample_rate)
            .ok_or_else(|| io::Error::other("Unsupported Opus input duration or rate"))?;
        let stereo_format = PcmFormat::new(recorded.sample_rate, 2).map_err(io::Error::other)?;
        let resampler = (recorded.sample_rate != RATE)
            .then(|| SincResampler::new(stereo_format, RATE, BLOCK).map_err(io::Error::other))
            .transpose()?;
        Ok(Self {
            decoder,
            stereo_format,
            channels: recorded.channels,
            resampler,
            original_frames,
            consumed: 0,
            total,
            position: 0,
            finished: false,
        })
    }

    fn decode(&mut self, wanted: usize) -> io::Result<Option<PcmChunk>> {
        let samples = match &mut self.decoder {
            Decoder::Compressed(decoder) => decoder
                .read_frames(wanted)
                .map_err(io::Error::other)?
                .map_or_else(Vec::new, |chunk| chunk.samples),
            Decoder::Wav(reader) => {
                let format = reader.format();
                let mut bytes = vec![0; wanted * format.frame_bytes()];
                let count = reader.read_pcm(&mut bytes)?;
                bytes[..count * format.frame_bytes()]
                    .chunks_exact(usize::from(format.bits / 8))
                    .map(|sample| match sample {
                        [low, high] => f32::from(i16::from_le_bytes([*low, *high])) / 32768.0,
                        [low, middle, high] => {
                            i32::from_le_bytes([0, *low, *middle, *high]) as f32 / 2147483648.0
                        }
                        _ => unreachable!("WAV reader validates 16- or 24-bit samples"),
                    })
                    .collect()
            }
        };
        let start = FramePosition(self.consumed);
        self.consumed += (samples.len() / usize::from(self.channels)) as u64;
        if self.consumed > self.original_frames {
            return Err(io::Error::other(
                "Decoded audio exceeds the recorded length",
            ));
        }
        if samples.is_empty() {
            if self.consumed != self.original_frames {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Decoded audio ended before its recorded length",
                ));
            }
            self.finished = true;
            return Ok(None);
        }
        let stereo = if self.channels == 1 {
            samples
                .into_iter()
                .flat_map(|sample| [sample, sample])
                .collect()
        } else {
            samples
        };
        PcmChunk::new(self.stereo_format, start, stereo)
            .map(Some)
            .map_err(io::Error::other)
    }

    fn next_chunk(&mut self, wanted: usize) -> io::Result<PcmChunk> {
        if self.resampler.is_none() {
            return self.decode(wanted)?.ok_or_else(short_output);
        }
        loop {
            let resampler = self.resampler.as_mut().expect("resampling selected");
            if let Some(chunk) = resampler.read_frames(wanted).map_err(io::Error::other)? {
                return Ok(chunk);
            }
            if self.finished {
                return Err(short_output());
            }
            let chunk = self.decode(BLOCK)?;
            let resampler = self.resampler.as_mut().expect("resampling selected");
            match chunk {
                Some(chunk) => resampler.push(chunk),
                None => resampler.finish(),
            }
            .map_err(io::Error::other)?;
        }
    }
}

fn short_output() -> io::Error {
    io::Error::new(
        io::ErrorKind::UnexpectedEof,
        "Opus input ended before its recorded length",
    )
}

impl IntegerPcmReader for OpusFileInput {
    fn format(&self) -> Format {
        Format {
            sample_rate: RATE,
            channels: 2,
            bits: 16,
        }
    }

    fn total_frames(&self) -> u64 {
        self.total
    }

    fn seek_frame(&mut self, frame: u64) -> io::Result<()> {
        if frame == self.position {
            return Ok(());
        }
        Err(io::Error::other("Opus file input is sequential"))
    }

    fn read_pcm(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let wanted = (output.len() / 4).min(BLOCK);
        let wanted = (wanted as u64).min(self.total - self.position) as usize;
        if wanted == 0 {
            return Ok(0);
        }
        let chunk = self.next_chunk(wanted)?;
        let count = chunk.frame_count() as usize;
        self.position += count as u64;
        if self.position == self.total && !self.finished {
            // Check the decoded tail before delivering the final output block.
            // The resampler can finish producing the rounded duration just
            // before it needs the last few input frames.
            while !self.finished {
                self.decode(BLOCK)?;
            }
        }
        for (sample, bytes) in chunk.samples.into_iter().zip(output.as_chunks_mut::<2>().0) {
            if !sample.is_finite() {
                return Err(io::Error::other(
                    "Decoded audio contains a non-finite sample",
                ));
            }
            let sample = (sample * 32768.0).round().clamp(-32768.0, 32767.0) as i16;
            bytes.copy_from_slice(&sample.to_le_bytes());
        }
        Ok(count)
    }
}
