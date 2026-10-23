use std::io::{Cursor, SeekFrom};

use collector_app::MediaResource;
use collector_core::Digest;

#[test]
fn authorized_resource_becomes_opaque_audio_source_without_a_path() {
    let bytes = b"RIFF fixture".to_vec();
    let resource = MediaResource {
        visibility: Default::default(),
        reader: Box::new(Cursor::new(bytes.clone())),
        length: bytes.len() as u64,
        media_type: "audio/wav".into(),
        digest: Digest::sha256(&bytes),
        download_name: "curator name.WAV".into(),
        verify_on_transfer: false,
    };

    let mut source = resource.into_audio_source();
    assert_eq!(source.hint().length_bytes, Some(bytes.len() as u64));
    assert_eq!(source.hint().extension.as_deref(), Some("WAV"));
    assert_eq!(source.hint().media_type.as_deref(), Some("audio/wav"));
    assert_eq!(source.reader_mut().seek(SeekFrom::Start(5)).unwrap(), 5);
    let mut rest = Vec::new();
    source.reader_mut().read_to_end(&mut rest).unwrap();
    assert_eq!(rest, b"fixture");
}
