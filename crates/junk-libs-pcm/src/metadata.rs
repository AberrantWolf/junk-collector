//! RIFF metadata traversal and bounded ID3 interpretation. Originals are never rewritten.
use crate::invalid;
use junk_libs_audio::FileAudioInfo;
use std::io::{self, Cursor, Read};
use symphonia_core::{
    io::MediaSourceStream,
    meta::{MetadataOptions, MetadataReader, StandardTagKey},
};

pub(crate) fn read_bounded(
    reader: &mut dyn Read,
    mut bytes: &mut [u8],
    checkpoint: &mut dyn FnMut() -> io::Result<()>,
) -> io::Result<()> {
    while !bytes.is_empty() {
        checkpoint()?;
        let n = bytes.len().min(65536);
        reader.read_exact(&mut bytes[..n])?;
        bytes = &mut bytes[n..];
    }
    Ok(())
}
fn text(info: &mut FileAudioInfo, key: &str, value: String) -> io::Result<()> {
    let value = value.trim_matches('\0').trim().to_string();
    if value.len() > 4096 || value.chars().any(char::is_control) || value.contains('\u{fffd}') {
        return Err(invalid("invalid or oversized WAV metadata text"));
    }
    let values = info.tags.entry(key.into()).or_default();
    if !value.is_empty() && !values.contains(&value) {
        values.push(value);
    }
    if info.tags.values().map(Vec::len).sum::<usize>() > 256 {
        return Err(invalid("too many WAV metadata values"));
    }
    Ok(())
}
fn standard(key: StandardTagKey) -> Option<&'static str> {
    Some(match key {
        StandardTagKey::TrackTitle => "title",
        StandardTagKey::Artist => "artist",
        StandardTagKey::Album => "album",
        StandardTagKey::AlbumArtist => "album_artist",
        StandardTagKey::DiscNumber => "disc_number",
        StandardTagKey::TrackNumber => "track_number",
        StandardTagKey::Date | StandardTagKey::ReleaseDate => "date",
        StandardTagKey::MusicBrainzAlbumId => "musicbrainz_release_id",
        StandardTagKey::MusicBrainzRecordingId => "musicbrainz_recording_id",
        _ => return None,
    })
}
pub(crate) fn validate_codepage(codepage: Option<u16>) -> io::Result<()> {
    if !matches!(
        codepage,
        None | Some(0 | 65001 | 1252 | 28591 | 932 | 936 | 949 | 950 | 1200 | 1201)
    ) {
        return Err(invalid("unsupported RIFF INFO character set"));
    }
    Ok(())
}
fn decode_info(bytes: &[u8], codepage: Option<u16>) -> io::Result<String> {
    validate_codepage(codepage)?;
    if matches!(codepage, None | Some(0))
        && let Ok(value) = std::str::from_utf8(bytes)
    {
        return Ok(value.into());
    }
    if codepage == Some(28591) {
        return Ok(bytes.iter().map(|b| char::from(*b)).collect());
    }
    let encoding = match codepage {
        Some(65001) => encoding_rs::UTF_8,
        Some(932) => encoding_rs::SHIFT_JIS,
        Some(936) => encoding_rs::GBK,
        Some(949) => encoding_rs::EUC_KR,
        Some(950) => encoding_rs::BIG5,
        Some(1200) => encoding_rs::UTF_16LE,
        Some(1201) => encoding_rs::UTF_16BE,
        _ => encoding_rs::WINDOWS_1252,
    };
    let (value, _, errors) = encoding.decode(bytes);
    if errors {
        return Err(invalid("invalid RIFF INFO encoding"));
    }
    Ok(value.into_owned())
}
pub(crate) fn read_info(
    reader: &mut dyn Read,
    mut remaining: u64,
    info: &mut FileAudioInfo,
    codepage: Option<u16>,
    checkpoint: &mut dyn FnMut() -> io::Result<()>,
) -> io::Result<()> {
    let mut entries = 0;
    while remaining != 0 {
        checkpoint()?;
        entries += 1;
        if entries > 256 || remaining < 8 {
            return Err(invalid("invalid RIFF INFO entries"));
        }
        let mut header = [0; 8];
        reader.read_exact(&mut header)?;
        let len = u64::from(u32::from_le_bytes(header[4..].try_into().unwrap()));
        let span = 8 + len + len % 2;
        if span > remaining || len > 65536 {
            return Err(invalid("invalid or oversized RIFF INFO value"));
        }
        let mut bytes = vec![0; len as usize];
        read_bounded(reader, &mut bytes, checkpoint)?;
        if len % 2 != 0 {
            reader.read_exact(&mut [0])?;
        }
        let tag = symphonia_metadata::riff::parse(header[..4].try_into().unwrap(), &bytes);
        // Widely used INFO extensions are not all recognized by Symphonia.
        let key = match &header[..4] {
            b"IPRD" => Some("album"),
            b"IPRT" | b"ITRK" => Some("track_number"),
            _ => tag.std_key.and_then(standard),
        };
        if let Some(key) = key {
            let value = decode_info(&bytes, codepage)?;
            text(info, key, value)?;
        }
        remaining -= span;
    }
    Ok(())
}
fn syncsafe(bytes: &[u8]) -> io::Result<usize> {
    if bytes.iter().any(|b| b & 128 != 0) {
        return Err(invalid("invalid ID3 size"));
    }
    Ok(bytes.iter().fold(0usize, |n, b| (n << 7) | usize::from(*b)))
}
pub(crate) fn read_id3(
    bytes: Vec<u8>,
    info: &mut FileAudioInfo,
    checkpoint: &mut dyn FnMut() -> io::Result<()>,
) -> io::Result<()> {
    if bytes.len() < 10 || &bytes[..3] != b"ID3" || !matches!(bytes[3], 2..=4) {
        return Err(invalid("unsupported WAV ID3 header"));
    }
    let version = bytes[3];
    let size = syncsafe(&bytes[6..10])?;
    let end = 10usize
        .checked_add(size)
        .ok_or_else(|| invalid("ID3 overflow"))?;
    let footer = if version == 4 && bytes[5] & 16 != 0 {
        10
    } else {
        0
    };
    if end + footer > bytes.len() || bytes[end + footer..].iter().any(|b| *b != 0) {
        return Err(invalid("truncated or inconsistent WAV ID3 size"));
    }
    if version == 2 && bytes[5] & 64 != 0 {
        return Err(invalid("compressed ID3 is unsupported"));
    }
    // v2.2/v2.3 global unsynchronization sizes describe the de-unsynchronized payload.
    let mut scan = Vec::with_capacity(size);
    let mut pos = 10;
    while pos < end {
        if pos % 65536 == 0 {
            checkpoint()?;
        }
        scan.push(bytes[pos]);
        if version < 4
            && bytes[5] & 128 != 0
            && bytes[pos] == 255
            && pos + 1 < end
            && bytes[pos + 1] == 0
        {
            pos += 1;
        }
        pos += 1;
    }
    let mut pos = 0;
    if version >= 3 && bytes[5] & 64 != 0 {
        if scan.len() < 4 {
            return Err(invalid("short extended ID3 header"));
        }
        pos = if version == 4 {
            syncsafe(&scan[..4])?
        } else {
            4 + u32::from_be_bytes(scan[..4].try_into().unwrap()) as usize
        };
        if pos < 4 || pos > scan.len() {
            return Err(invalid("invalid extended ID3 header"));
        }
    }
    let mut frames = 0;
    let mut legacy_pictures = Vec::new();
    let mut v22_frames = Vec::new();
    while pos < scan.len() {
        checkpoint()?;
        if scan[pos] == 0 {
            if scan[pos..].iter().any(|b| *b != 0) {
                return Err(invalid("invalid ID3 padding"));
            }
            break;
        }
        frames += 1;
        let header = if version == 2 { 6 } else { 10 };
        let idlen = if version == 2 { 3 } else { 4 };
        if frames > 256
            || scan.len() - pos < header
            || !scan[pos..pos + idlen]
                .iter()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
        {
            return Err(invalid("invalid or excessive ID3 frames"));
        }
        let len = if version == 2 {
            scan[pos + 3..pos + 6]
                .iter()
                .fold(0usize, |n, b| (n << 8) | usize::from(*b))
        } else if version == 4 {
            syncsafe(&scan[pos + 4..pos + 8])?
        } else {
            u32::from_be_bytes(scan[pos + 4..pos + 8].try_into().unwrap()) as usize
        };
        if version == 3 && scan[pos + 9] & 0xe0 != 0 || version == 4 && scan[pos + 9] & 0x4c != 0 {
            return Err(invalid(
                "compressed/encrypted/grouped ID3 frames are unsupported",
            ));
        }
        let limit = if &scan[pos..pos + idlen] == b"APIC" || &scan[pos..pos + idlen] == b"PIC" {
            20 * 1024 * 1024
        } else {
            65536
        };
        if len == 0 || len > limit || header + len > scan.len() - pos {
            return Err(invalid("invalid or oversized ID3 frame"));
        }
        if version == 2 {
            let value = &scan[pos + header..pos + header + len];
            if &scan[pos..pos + 3] == b"PIC" {
                if value.len() < 6 {
                    return Err(invalid("short ID3v2.2 picture"));
                }
                let mime = match &value[1..4] {
                    b"PNG" => "image/png",
                    b"JPG" => "image/jpeg",
                    _ => return Err(invalid("unsupported ID3v2.2 picture format")),
                };
                let description = &value[5..];
                let offset = match value[0] {
                    0 => description.iter().position(|b| *b == 0).map(|n| n + 1),
                    1 => description
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .position(|b| *b == [0, 0])
                        .map(|n| (n + 1) * 2),
                    _ => None,
                }
                .ok_or_else(|| invalid("invalid ID3 picture description"))?;
                let picture = &description[offset..];
                if picture.is_empty() || legacy_pictures.len() + info.pictures.len() >= 16 {
                    return Err(invalid("invalid ID3 picture data"));
                }
                legacy_pictures.push((mime.to_owned(), picture.to_vec()));
            } else {
                v22_frames.extend_from_slice(&scan[pos..pos + header + len]);
            }
        }
        pos = pos
            .checked_add(header + len)
            .ok_or_else(|| invalid("ID3 frame overflow"))?;
        if pos > scan.len() {
            return Err(invalid("truncated ID3 frame"));
        }
    }
    checkpoint()?;
    let mut reader = symphonia_metadata::id3v2::Id3v2Reader::new(&MetadataOptions::default());
    let bytes = if version == 2 {
        let size = v22_frames.len();
        let mut bytes = b"ID3\x02\0\0".to_vec();
        bytes.extend([
            (size >> 21) as u8 & 127,
            (size >> 14) as u8 & 127,
            (size >> 7) as u8 & 127,
            size as u8 & 127,
        ]);
        bytes.extend(v22_frames);
        bytes
    } else {
        bytes
    };
    let revision = reader
        .read_all(&mut MediaSourceStream::new(
            Box::new(Cursor::new(bytes)),
            Default::default(),
        ))
        .map_err(io::Error::other)?;
    if revision.tags().len() > 256 || revision.visuals().len() + info.pictures.len() > 16 {
        return Err(invalid("excessive WAV ID3 metadata"));
    }
    for tag in revision.tags() {
        checkpoint()?;
        if let Some(key) = tag.std_key.and_then(standard) {
            text(info, key, tag.value.to_string())?;
        }
    }
    if version == 2 {
        info.pictures.extend(legacy_pictures);
        return Ok(());
    }
    for picture in revision.visuals() {
        checkpoint()?;
        if picture.data.len() > 20 * 1024 * 1024 {
            return Err(invalid("WAV artwork exceeds 20 MiB"));
        }
        info.pictures
            .push((picture.media_type.clone(), picture.data.to_vec()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{IntegerPcmReader, WavReader};
    #[test]
    fn trailing_info_id3_tags_and_cover_are_interpreted_without_changing_pcm() {
        let bytes =
            include_bytes!("../../collector-store/tests/fixtures/digital-audio/03-tagged.wav");
        let mut reader = WavReader::open(Cursor::new(bytes), &mut || Ok(())).unwrap();
        assert_eq!(reader.metadata.tag("title"), Some("WAV title"));
        assert_eq!(reader.metadata.tag("album"), Some("Tagged WAV album"));
        assert_eq!(reader.metadata.tag("artist"), Some("Tagged WAV artist"));
        assert_eq!(reader.metadata.tag("track_number"), Some("7"));
        assert_eq!(reader.metadata.tag("disc_number"), Some("2"));
        assert_eq!(reader.metadata.pictures.len(), 1);
        assert_eq!(reader.metadata.pictures[0].0, "image/png");
        let mut pcm = [0; 15];
        assert_eq!(reader.read_pcm(&mut pcm).unwrap(), 5);
        assert_eq!(
            pcm,
            [0, 0, 128, 255, 255, 255, 0, 0, 0, 1, 0, 0, 255, 255, 127]
        );
    }
    #[test]
    fn malformed_id3_headers_frames_and_excessive_metadata_fail_closed() {
        for bytes in [
            b"ID3\x03\0\0\x7f\x7f\x7f\x7f".to_vec(),
            b"ID3\x03\0\0\0\0\0\x0bTIT2\0\0\0\x10\0\0x".to_vec(),
        ] {
            assert!(read_id3(bytes, &mut FileAudioInfo::default(), &mut || Ok(())).is_err());
        }
        let info = b"INAM\xff\xff\xff\x7f";
        assert!(
            read_info(
                &mut Cursor::new(info),
                info.len() as u64,
                &mut FileAudioInfo::default(),
                None,
                &mut || Ok(())
            )
            .is_err()
        );
    }
}

#[cfg(test)]
mod additional_tests {
    use super::*;
    use crate::WavReader;
    fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut v = id.to_vec();
        v.extend((body.len() as u32).to_le_bytes());
        v.extend(body);
        if !body.len().is_multiple_of(2) {
            v.push(0);
        }
        v
    }
    fn with_chunks(chunks: &[Vec<u8>]) -> Vec<u8> {
        let mut bytes =
            include_bytes!("../../collector-store/tests/fixtures/digital-audio/03.wav").to_vec();
        for c in chunks {
            bytes.extend(c);
        }
        let len = (bytes.len() - 8) as u32;
        bytes[4..8].copy_from_slice(&len.to_le_bytes());
        bytes
    }
    #[test]
    fn declared_riff_charset_after_info_is_honored_for_japanese_text() {
        let (encoded, _, errors) = encoding_rs::SHIFT_JIS.encode("曲名");
        assert!(!errors);
        let mut value = encoded.into_owned();
        value.push(0);
        let mut info = b"INFO".to_vec();
        info.extend(chunk(b"INAM", &value));
        let mut cset = 932u16.to_le_bytes().to_vec();
        cset.extend([0; 6]);
        let bytes = with_chunks(&[chunk(b"LIST", &info), chunk(b"CSET", &cset)]);
        assert_eq!(
            WavReader::open(Cursor::new(bytes), &mut || Ok(()))
                .unwrap()
                .metadata
                .tag("title"),
            Some("曲名")
        );
        assert!(validate_codepage(Some(65535)).is_err());
    }
    #[test]
    fn legacy_pic_and_global_unsynchronization_keep_exact_artwork() {
        let picture = [1, 255, 224, 0];
        let mut value = b"\0PNG\x03\0".to_vec();
        value.extend(picture);
        let mut payload = b"TT2\0\0\x06\0Title".to_vec();
        payload.extend(b"PIC");
        payload.extend([0, 0, value.len() as u8]);
        payload.extend(value);
        let mut encoded = Vec::new();
        for b in payload {
            encoded.push(b);
            if b == 255 {
                encoded.push(0);
            }
        }
        let n = encoded.len();
        let mut id3 = b"ID3\x02\0\x80".to_vec();
        id3.extend([
            (n >> 21) as u8 & 127,
            (n >> 14) as u8 & 127,
            (n >> 7) as u8 & 127,
            n as u8 & 127,
        ]);
        id3.extend(encoded);
        let mut info = FileAudioInfo::default();
        read_id3(id3, &mut info, &mut || Ok(())).unwrap();
        assert_eq!(info.tag("title"), Some("Title"));
        assert_eq!(info.pictures, vec![("image/png".into(), picture.to_vec())]);
    }
}
