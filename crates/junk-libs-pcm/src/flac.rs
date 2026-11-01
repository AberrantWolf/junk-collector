//! Bounded metadata-only FLAC export; audio frames and STREAMINFO remain unchanged.
use crate::{invalid, metadata::read_bounded};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{self, Read, Write},
};
pub struct ExportMetadata {
    pub tags: BTreeMap<String, String>,
    pub picture: Option<(String, Vec<u8>)>,
}
pub struct ExportResult {
    pub input_digest: [u8; 32],
    pub output_digest: [u8; 32],
    pub bytes: u64,
}
struct HashedRead<'a> {
    reader: &'a mut dyn Read,
    hash: Sha256,
}
impl Read for HashedRead<'_> {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        let n = self.reader.read(b)?;
        self.hash.update(&b[..n]);
        Ok(n)
    }
}
struct HashedWrite<'a> {
    writer: &'a mut dyn Write,
    hash: Sha256,
    bytes: u64,
}
impl Write for HashedWrite<'_> {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        if self.bytes + b.len() as u64 > 8 * 1024 * 1024 * 1024 {
            return Err(invalid("FLAC export exceeds 8 GiB"));
        }
        let n = self.writer.write(b)?;
        self.hash.update(&b[..n]);
        self.bytes += n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
pub fn repack(
    reader: &mut dyn Read,
    output: &mut dyn Write,
    metadata: &ExportMetadata,
    checkpoint: &mut dyn FnMut() -> io::Result<()>,
) -> io::Result<ExportResult> {
    checkpoint()?;
    if metadata.tags.len() > 256 {
        return Err(invalid("too many FLAC export tags"));
    }
    let mut source = HashedRead {
        reader,
        hash: Sha256::new(),
    };
    let mut signature = [0; 4];
    source.read_exact(&mut signature)?;
    if &signature != b"fLaC" {
        return Err(invalid("FLAC export requires a native FLAC stream"));
    }
    let mut blocks = Vec::new();
    let mut total = 0usize;
    let mut count = 0;
    loop {
        checkpoint()?;
        count += 1;
        if count > 256 {
            return Err(invalid("too many FLAC metadata blocks"));
        }
        let mut header = [0; 4];
        source.read_exact(&mut header)?;
        let kind = header[0] & 127;
        let len = u32::from_be_bytes([0, header[1], header[2], header[3]]) as usize;
        total = total
            .checked_add(len + 4)
            .ok_or_else(|| invalid("FLAC metadata overflow"))?;
        if total > 32 * 1024 * 1024
            || count == 1 && (kind != 0 || len != 34)
            || count != 1 && kind == 0
            || kind == 127
        {
            return Err(invalid("invalid FLAC metadata structure"));
        }
        let mut block = vec![0; len];
        read_bounded(&mut source, &mut block, checkpoint)?;
        if kind != 1 && kind != 4 && kind != 6 {
            blocks.push((kind, block));
        }
        if header[0] & 128 != 0 {
            break;
        }
    }
    let vendor = b"Junk Collector metadata export";
    let mut comments = Vec::new();
    comments.extend((vendor.len() as u32).to_le_bytes());
    comments.extend(vendor);
    comments.extend((metadata.tags.len() as u32).to_le_bytes());
    for (key, value) in &metadata.tags {
        checkpoint()?;
        if key.is_empty()
            || !key.bytes().all(|b| b.is_ascii_uppercase() || b == b'_')
            || value.len() > 4096
            || value.chars().any(char::is_control)
        {
            return Err(invalid("invalid FLAC export tag"));
        }
        let tag = format!("{key}={value}");
        comments.extend((tag.len() as u32).to_le_bytes());
        comments.extend(tag.as_bytes());
    }
    blocks.push((4, comments));
    if let Some((mime, bytes)) = &metadata.picture {
        if mime.len() > 128
            || !mime.is_ascii()
            || mime.chars().any(char::is_control)
            || bytes.len() > 15 * 1024 * 1024
        {
            return Err(invalid("FLAC export artwork exceeds block limits"));
        }
        let mut picture = Vec::new();
        picture.extend(3u32.to_be_bytes());
        picture.extend((mime.len() as u32).to_be_bytes());
        picture.extend(mime.as_bytes());
        picture.extend(0u32.to_be_bytes());
        let dimensions = if bytes.len() >= 24 && &bytes[..8] == b"\x89PNG\r\n\x1a\n" {
            (
                u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
                u32::from_be_bytes(bytes[20..24].try_into().unwrap()),
            )
        } else {
            (0, 0)
        };
        for n in [dimensions.0, dimensions.1, 0, 0] {
            picture.extend(n.to_be_bytes());
        }
        picture.extend((bytes.len() as u32).to_be_bytes());
        picture.extend(bytes);
        blocks.push((6, picture));
    }
    let mut output = HashedWrite {
        writer: output,
        hash: Sha256::new(),
        bytes: 0,
    };
    output.write_all(b"fLaC")?;
    for (index, (kind, block)) in blocks.iter().enumerate() {
        checkpoint()?;
        if block.len() > 0xffffff {
            return Err(invalid("FLAC metadata block overflow"));
        }
        let len = (block.len() as u32).to_be_bytes();
        output.write_all(&[
            *kind | if index + 1 == blocks.len() { 128 } else { 0 },
            len[1],
            len[2],
            len[3],
        ])?;
        for chunk in block.chunks(65536) {
            checkpoint()?;
            output.write_all(chunk)?;
        }
    }
    let mut bytes = [0; 65536];
    let mut audio = 0u64;
    loop {
        checkpoint()?;
        let n = source.read(&mut bytes)?;
        if n == 0 {
            break;
        }
        audio += n as u64;
        output.write_all(&bytes[..n])?;
    }
    if audio == 0 {
        return Err(invalid("FLAC stream has no audio frames"));
    }
    output.flush()?;
    checkpoint()?;
    Ok(ExportResult {
        input_digest: source.hash.finalize().into(),
        output_digest: output.hash.finalize().into(),
        bytes: output.bytes,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_export_is_readable_and_cancellable_with_exact_hashes() {
        let original = include_bytes!("../../collector-store/tests/fixtures/digital-audio/01.flac");
        let mut bytes = Vec::new();
        let metadata = ExportMetadata {
            tags: BTreeMap::from([
                ("TITLE".into(), "Exported title".into()),
                ("ALBUM".into(), "Album".into()),
            ]),
            picture: None,
        };
        let result = repack(&mut &original[..], &mut bytes, &metadata, &mut || Ok(())).unwrap();
        assert_eq!(
            result.input_digest.as_slice(),
            Sha256::digest(original).as_slice()
        );
        assert_eq!(
            result.output_digest.as_slice(),
            Sha256::digest(&bytes).as_slice()
        );
        assert_eq!(result.bytes, bytes.len() as u64);
        let inspected = junk_libs_audio::inspect_file_audio(
            junk_libs_audio::ByteSource::new(
                std::io::Cursor::new(bytes),
                junk_libs_audio::MediaHint::new(None, Some("flac"), None),
            ),
            &mut || Ok(()),
        )
        .unwrap();
        assert_eq!(inspected.tag("title"), Some("Exported title"));
        assert_eq!(inspected.decoded_frames, Some(24000));
        let mut calls = 0;
        let error = repack(&mut &original[..], &mut Vec::new(), &metadata, &mut || {
            calls += 1;
            if calls == 4 {
                Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"))
            } else {
                Ok(())
            }
        })
        .err()
        .unwrap();
        assert_eq!(error.kind(), io::ErrorKind::Interrupted);
    }
}
