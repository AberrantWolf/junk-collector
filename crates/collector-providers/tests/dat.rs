use collector_app::{
    FrozenJobInput, ProcessingExecutionContext, ProviderRequest, StreamingProviderPort,
};
use collector_core::{CollectionId, ModuleId, OperationId, RawJson};
use collector_providers::{
    DatCatalogEntryObservation, DatSnapshotObservation, LOCAL_DAT_ACQUIRE_OPERATION,
    LOCAL_DAT_PROVIDER_ID, LocalDatProvider, LocalDatSource,
};
use std::{cell::Cell, collections::BTreeMap, fs, io::Read};

struct Execution {
    cancelled: bool,
    progress: Vec<(String, u64, u64)>,
}

impl ProcessingExecutionContext for Execution {
    fn frozen_job(&self) -> &FrozenJobInput {
        panic!("provider fixture does not inspect the frozen job")
    }

    fn report_progress(
        &mut self,
        phase: &str,
        completed: u64,
        total: u64,
    ) -> collector_core::Result<()> {
        self.progress.push((phase.into(), completed, total));
        Ok(())
    }

    fn cancellation_requested(&self) -> collector_core::Result<bool> {
        Ok(self.cancelled)
    }
}

struct ParserCancellation(Cell<usize>);

impl ProcessingExecutionContext for ParserCancellation {
    fn frozen_job(&self) -> &FrozenJobInput {
        panic!("provider fixture does not inspect the frozen job")
    }

    fn report_progress(
        &mut self,
        _phase: &str,
        _completed: u64,
        _total: u64,
    ) -> collector_core::Result<()> {
        Ok(())
    }

    fn cancellation_requested(&self) -> collector_core::Result<bool> {
        let checks = self.0.get() + 1;
        self.0.set(checks);
        Ok(checks > 2)
    }
}

fn source(path: std::path::PathBuf) -> LocalDatSource {
    LocalDatSource {
        path,
        system: "nes".into(),
        catalog: "fixture".into(),
        byte_domain: "org.junk.game.raw-bytes".into(),
        byte_domain_version: 1,
    }
}

fn xml() -> &'static [u8] {
    br#"<datafile><header><name>Fixture</name><version>1</version></header><game name="Title &amp; More"><rom name="one.bin" size="5" crc="9271EE57" sha1="e0996a37c13d44c3b06074939d43fa3759bd32c1"/></game></datafile>"#
}

fn text() -> &'static [u8] {
    br#"clrmamepro (
 name "Fixture"
 version 1
)
game (
 name "Title & More"
 rom ( name "one.bin" size 5 crc 9271EE57 sha1 e0996a37c13d44c3b06074939d43fa3759bd32c1 )
)
"#
}

fn redump_tracks() -> &'static [u8] {
    br#"<datafile><header><name>Redump fixture</name><version>1</version></header><game name="Disc"><rom name="Disc.cue" size="81" crc="11111111" sha1="1111111111111111111111111111111111111111"/><rom name="Disc (Track 1).bin" size="2352" crc="22222222" sha1="2222222222222222222222222222222222222222"/><rom name="Disc (Track 2).BIN" size="4704" crc="33333333" sha1="3333333333333333333333333333333333333333"/></game></datafile>"#
}

#[test]
fn equivalent_xml_and_text_dat_bytes_produce_equivalent_observations() {
    let source = source("unused".into());
    let (xml_header, xml_rows) = LocalDatProvider::parse_observation(&source, xml()).unwrap();
    let (text_header, text_rows) = LocalDatProvider::parse_observation(&source, text()).unwrap();
    assert_eq!(xml_header, text_header);
    assert_eq!(xml_rows, text_rows);
    assert_eq!(xml_rows[0].label, "Title & More");
    assert_eq!(xml_rows[0].files[0].crc32.as_deref(), Some("9271ee57"));
}

#[test]
fn raw_2352_catalog_rows_exclude_but_retain_non_track_provider_files() {
    let source = LocalDatSource {
        path: "unused".into(),
        system: "sony-playstation".into(),
        catalog: "fixture".into(),
        byte_domain: "org.junk.game.cd.raw-2352-track".into(),
        byte_domain_version: 1,
    };
    let (header, rows) = LocalDatProvider::parse_observation(&source, redump_tracks()).unwrap();
    assert_eq!(header.game_count, 1);
    assert_eq!(rows[0].files.len(), 2);
    assert_eq!(rows[0].files[0].name, "Disc (Track 1).bin");
    assert_eq!(rows[0].files[1].name, "Disc (Track 2).BIN");
    assert_eq!(rows[0].excluded_files.len(), 1);
    assert_eq!(rows[0].excluded_files[0].name, "Disc.cue");
}

#[test]
fn raw_2352_catalog_rows_fail_closed_without_a_qualified_track() {
    let source = LocalDatSource {
        path: "unused".into(),
        system: "sony-playstation".into(),
        catalog: "fixture".into(),
        byte_domain: "org.junk.game.cd.raw-2352-track".into(),
        byte_domain_version: 1,
    };
    let cue_only = br#"<datafile><header><name>Fixture</name><version>1</version></header><game name="No track"><rom name="only.cue" size="81" crc="11111111"/></game></datafile>"#;
    assert!(LocalDatProvider::parse_observation(&source, cue_only.as_slice()).is_err());
}

#[test]
fn local_provider_preserves_exact_bytes_and_rejects_cancelled_or_oversized_sources() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("fixture.dat");
    fs::write(&path, xml()).unwrap();
    let provider =
        LocalDatProvider::new(BTreeMap::from([("nes".into(), source(path.clone()))])).unwrap();
    let request = ProviderRequest {
        collection_id: CollectionId::new(),
        provider: ModuleId(LOCAL_DAT_PROVIDER_ID.into()),
        operation: OperationId::new(LOCAL_DAT_ACQUIRE_OPERATION).unwrap(),
        source_fingerprint: None,
        query: RawJson::from_value(serde_json::json!({"source":"nes"})),
        maximum_response_bytes: xml().len() as u64,
    };
    let mut execution = Execution {
        cancelled: false,
        progress: vec![],
    };
    let mut response = provider.acquire_stream(&request, &mut execution).unwrap();
    let header: DatSnapshotObservation = response.parsed_observation.parse().unwrap();
    assert_eq!(header.system, "nes");
    let mut bytes = Vec::new();
    response.reader.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, xml());
    assert_eq!(response.digest, collector_core::Digest::sha256(xml()));
    assert!(
        execution
            .progress
            .iter()
            .any(|(phase, _, _)| phase == "parsing_catalog")
    );

    let mut cancelled = Execution {
        cancelled: true,
        progress: vec![],
    };
    assert!(matches!(
        provider.acquire_stream(&request, &mut cancelled),
        Err(collector_core::Error::JobCancelled)
    ));
    let mut parser_cancelled = ParserCancellation(Cell::new(0));
    assert!(matches!(
        provider.acquire_stream(&request, &mut parser_cancelled),
        Err(collector_core::Error::JobCancelled)
    ));
    let mut too_small = request;
    too_small.maximum_response_bytes -= 1;
    let mut execution = Execution {
        cancelled: false,
        progress: vec![],
    };
    assert!(provider.acquire_stream(&too_small, &mut execution).is_err());
}

#[test]
fn malformed_catalogs_and_checksums_fail_closed() {
    let source = source("unused".into());
    assert!(LocalDatProvider::parse_observation(&source, b"not a dat".as_slice()).is_err());
    let invalid = br#"<datafile><header><name>Fixture</name><version>1</version></header><game name="Bad"><rom name="bad.bin" size="1" sha1="not-a-hash"/></game></datafile>"#;
    assert!(LocalDatProvider::parse_observation(&source, invalid.as_slice()).is_err());
}

#[test]
fn entry_wire_is_bounded_and_stable() {
    let source = source("unused".into());
    let (_, rows) = LocalDatProvider::parse_observation(&source, xml()).unwrap();
    let encoded = serde_json::to_value(&rows[0]).unwrap();
    let decoded: DatCatalogEntryObservation = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded, rows[0]);
}

#[test]
fn configured_source_outage_fails_without_creating_a_synthetic_snapshot() {
    let provider = LocalDatProvider::new(BTreeMap::from([(
        "missing".into(),
        source("does-not-exist.dat".into()),
    )]))
    .unwrap();
    let request = ProviderRequest {
        collection_id: CollectionId::new(),
        provider: ModuleId(LOCAL_DAT_PROVIDER_ID.into()),
        operation: OperationId::new(LOCAL_DAT_ACQUIRE_OPERATION).unwrap(),
        source_fingerprint: None,
        query: RawJson::from_value(serde_json::json!({"source":"missing"})),
        maximum_response_bytes: 1024,
    };
    let mut execution = Execution {
        cancelled: false,
        progress: vec![],
    };
    assert!(provider.acquire_stream(&request, &mut execution).is_err());
}

#[test]
fn snapshot_parser_checks_exact_bytes_even_when_metadata_is_unchanged() {
    use collector_app::{CatalogParserPort, SnapshotRecord};
    use collector_core::{ByteLength, Digest, SnapshotId};
    let provider = LocalDatProvider::new(BTreeMap::new()).unwrap();
    let (header, _) = LocalDatProvider::parse_observation(&source("unused".into()), xml()).unwrap();
    let record = SnapshotRecord {
        id: SnapshotId::new(),
        collection_id: CollectionId::new(),
        provider: ModuleId(LOCAL_DAT_PROVIDER_ID.into()),
        media_type: "application/vnd.logiqx.dat".into(),
        bytes: ByteLength(xml().len() as u64),
        digest: Digest::sha256(xml()),
        parsed_observation: RawJson::from_value(serde_json::to_value(header).unwrap()),
        created_at: "2026-09-13T00:00:00Z".parse().unwrap(),
    };
    provider
        .parse_snapshot(&record, &mut xml(), &mut || Ok(false))
        .unwrap();
    let changed = String::from_utf8(xml().to_vec())
        .unwrap()
        .replace("9271EE57", "00000000");
    assert!(
        provider
            .parse_snapshot(&record, &mut changed.as_bytes(), &mut || Ok(false))
            .is_err()
    );
    let mut trailing = xml().to_vec();
    trailing.extend_from_slice(b" ");
    assert!(
        provider
            .parse_snapshot(&record, &mut trailing.as_slice(), &mut || Ok(false))
            .is_err()
    );
    assert!(matches!(
        provider.parse_snapshot(&record, &mut xml(), &mut || Ok(true)),
        Err(collector_core::Error::JobCancelled)
    ));
}

#[test]
fn raw_2352_catalog_does_not_hide_bad_tracks_beside_a_valid_track() {
    let source = LocalDatSource {
        path: "unused".into(),
        system: "sony-playstation".into(),
        catalog: "fixture".into(),
        byte_domain: "org.junk.game.cd.raw-2352-track".into(),
        byte_domain_version: 1,
    };
    for (name, size) in [("bad.bin", 2353), ("empty.bin", 0), ("unknown.img", 2352)] {
        let xml = format!(
            r#"<datafile><header><name>Fixture</name><version>1</version></header><game name="Disc"><rom name="valid.bin" size="2352" crc="11111111"/><rom name="{name}" size="{size}" crc="22222222"/></game></datafile>"#
        );
        assert!(
            LocalDatProvider::parse_observation(&source, xml.as_bytes()).is_err(),
            "{name}"
        );
    }
}
