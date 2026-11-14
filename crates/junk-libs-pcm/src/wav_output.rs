//! A seekable RIFF/WAVE view over a bounded window of integer PCM.
use crate::IntegerPcmReader;
use std::io::{self, Read, Seek, SeekFrom};

pub struct PcmWave {
    pcm: Box<dyn IntegerPcmReader>,
    header: [u8; 44],
    first_frame: u64,
    length: u64,
    position: u64,
    next_frame: Option<u64>,
    buffer: Vec<u8>,
}

impl PcmWave {
    pub fn new(pcm: Box<dyn IntegerPcmReader>, first_frame: u64, frames: u64) -> io::Result<Self> {
        let format = pcm.format();
        let frame_bytes = format.frame_bytes() as u16;
        let riff_size = if frame_bytes != 0 && frame_bytes.is_multiple_of(2) {
            frames
                .checked_mul(u64::from(frame_bytes))
                .and_then(|bytes| u32::try_from(bytes).ok())
                .and_then(|bytes| bytes.checked_add(36))
        } else {
            None
        }
        .filter(|_| {
            first_frame
                .checked_add(frames)
                .is_some_and(|end| end <= pcm.total_frames())
        })
        .ok_or_else(|| crate::invalid("PCM window exceeds WAV bounds"))?;
        let byte_rate = format
            .sample_rate
            .checked_mul(u32::from(frame_bytes))
            .ok_or_else(|| crate::invalid("invalid WAV byte rate"))?;
        let mut header =
            *b"RIFF\0\0\0\0WAVEfmt \x10\0\0\0\x01\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0data\0\0\0\0";
        header[4..8].copy_from_slice(&riff_size.to_le_bytes());
        header[22..24].copy_from_slice(&format.channels.to_le_bytes());
        header[24..28].copy_from_slice(&format.sample_rate.to_le_bytes());
        header[28..32].copy_from_slice(&byte_rate.to_le_bytes());
        header[32..34].copy_from_slice(&frame_bytes.to_le_bytes());
        header[34..36].copy_from_slice(&format.bits.to_le_bytes());
        header[40..44].copy_from_slice(&(riff_size - 36).to_le_bytes());
        Ok(Self {
            pcm,
            header,
            first_frame,
            length: u64::from(riff_size) + 8,
            position: 0,
            next_frame: None,
            buffer: Vec::new(),
        })
    }

    pub fn byte_length(&self) -> u64 {
        self.length
    }
}

impl Read for PcmWave {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let count = output
            .len()
            .min(self.length.saturating_sub(self.position) as usize);
        if count == 0 {
            return Ok(0);
        }
        if self.position < self.header.len() as u64 {
            let start = self.position as usize;
            let read = count.min(self.header.len() - start);
            output[..read].copy_from_slice(&self.header[start..start + read]);
            self.position += read as u64;
            return Ok(read);
        }
        let width = self.pcm.format().frame_bytes();
        let payload = self.position - self.header.len() as u64;
        let frame = self.first_frame + payload / width as u64;
        let skip = payload as usize % width;
        let wanted = (count + skip).div_ceil(width).min(16384);
        if self.next_frame != Some(frame) {
            self.pcm.seek_frame(frame)?;
        }
        self.buffer.resize(wanted * width, 0);
        let frames = self.pcm.read_pcm(&mut self.buffer)?;
        if frames == 0 || frames > wanted {
            return Err(crate::invalid("PCM window ended early"));
        }
        self.next_frame = Some(frame + frames as u64);
        let read = count.min(frames * width - skip);
        output[..read].copy_from_slice(&self.buffer[skip..skip + read]);
        self.position += read as u64;
        Ok(read)
    }
}

impl Seek for PcmWave {
    fn seek(&mut self, seek: SeekFrom) -> io::Result<u64> {
        let position = match seek {
            SeekFrom::Start(position) => Some(position),
            SeekFrom::Current(offset) => self.position.checked_add_signed(offset),
            SeekFrom::End(offset) => self.length.checked_add_signed(offset),
        }
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid WAV seek"))?;
        self.position = position;
        Ok(position)
    }
}
