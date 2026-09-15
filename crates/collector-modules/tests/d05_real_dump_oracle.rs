use collector_core::{
    ByteLength, CollectionId, Digest, PackageFileId, PackageFileName, PackageInventory,
    PackageInventoryFile, PackageScope, RepresentationId,
};
use collector_modules::music::{
    AccurateRipDatabaseIdentity, AudioVerificationStatus, CdPackageKind, CdProcessingResult,
    InterpretAccurateRipInput, analyze_cue_package, analyze_redumper_package,
    interpret_accuraterip_summary, open_cue_pcm_package, open_redumper_pcm_package,
};
use collector_modules::{ModuleError, RepresentationReader};
use junk_libs_accuraterip::{
    BoundedDiscReader, DbarFile, VerificationOptions, verify_with_offset_reader,
};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Oracle {
    packages: Vec<PackageOracle>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageOracle {
    kind: CdPackageKind,
    anchor: String,
    files: Vec<String>,
    expected_layout_sha256: String,
    #[serde(default)]
    accuraterip: Option<AccurateRipOracle>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AccurateRipOracle {
    dbar: String,
    maximum_offset: i32,
    expected_status: AudioVerificationStatus,
    expected_selected_offset: Option<i32>,
    expected_tracks: Vec<TrackOracle>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TrackOracle {
    position: u8,
    computed_offset: i32,
    arv1: String,
    arv2: String,
}

struct DynPcmReader(Box<dyn junk_libs_disc::CdPcmReader>);

impl junk_libs_disc::CdPcmReader for DynPcmReader {
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

fn safe(root: &Path, name: &str) -> PathBuf {
    assert!(
        !name.is_empty() && !name.starts_with('/') && !name.contains("..") && !name.contains('\\')
    );
    root.join(name)
}

#[test]
#[ignore = "requires JUNK_COLLECTOR_W4_CD_DUMP_ROOT and user-owned oracle.toml"]
fn owned_cd_dump_matches_declared_layout_and_accuraterip_oracles() {
    let root = std::env::var_os("JUNK_COLLECTOR_W4_CD_DUMP_ROOT")
        .map(PathBuf::from)
        .expect("JUNK_COLLECTOR_W4_CD_DUMP_ROOT is required");
    let oracle: Oracle = toml::from_str(
        &std::fs::read_to_string(safe(&root, "oracle.toml")).expect("read oracle.toml"),
    )
    .expect("parse oracle.toml");
    assert!(!oracle.packages.is_empty());
    for package in oracle.packages {
        let collection = CollectionId::new();
        let scope = PackageScope::Representation(RepresentationId::new());
        let mut paths = BTreeMap::new();
        let files = package
            .files
            .iter()
            .map(|name| {
                let path = safe(&root, name);
                let bytes = std::fs::metadata(&path)
                    .expect("read package metadata")
                    .len();
                let id = PackageFileId::new();
                paths.insert(id, path);
                PackageInventoryFile {
                    id,
                    display_name: PackageFileName::new(name.clone()).expect("safe package name"),
                    bytes: ByteLength(bytes),
                }
            })
            .collect();
        let inventory =
            PackageInventory::new(collection, scope, files).expect("valid package inventory");
        let anchor_index = package
            .files
            .iter()
            .position(|name| name == &package.anchor)
            .expect("anchor is listed");
        let anchor_id = inventory.files[anchor_index].id;
        let open = |id| -> Result<Box<dyn RepresentationReader>, ModuleError> {
            Ok(Box::new(
                File::open(paths.get(&id).expect("known file identity"))
                    .map_err(|error| ModuleError::FailedOperation(error.to_string()))?,
            ))
        };
        let (layout, reader, tracks): (
            CdProcessingResult,
            Box<dyn junk_libs_disc::CdPcmReader>,
            _,
        ) = match package.kind {
            CdPackageKind::Cue => {
                let mut cue = String::new();
                open(anchor_id)
                    .unwrap()
                    .read_to_string(&mut cue)
                    .expect("read CUE");
                let layout = analyze_cue_package(&cue, anchor_id, &inventory).expect("analyze CUE");
                let (reader, tracks) =
                    open_cue_pcm_package(&cue, anchor_id, &inventory, open).expect("open CUE PCM");
                (layout, Box::new(reader), tracks)
            }
            CdPackageKind::Redumper => {
                let layout = analyze_redumper_package(anchor_id, &inventory, open)
                    .expect("analyze redumper package");
                let (reader, tracks) = open_redumper_pcm_package(anchor_id, &inventory, open)
                    .expect("open redumper PCM");
                (layout, Box::new(reader), tracks)
            }
        };
        let mut layout_value = serde_json::to_value(&layout).expect("serialize layout");
        if let Some(spans) = layout_value
            .get_mut("source_spans")
            .and_then(serde_json::Value::as_array_mut)
        {
            for span in spans {
                let id = span["source_file_id"]
                    .as_str()
                    .expect("source file identity");
                let name = inventory
                    .files
                    .iter()
                    .position(|file| file.id.to_string() == id)
                    .map(|index| package.files[index].clone())
                    .expect("layout source is in the package");
                span["source_file_id"] = serde_json::Value::String(name);
            }
        }
        let layout_bytes = serde_json::to_vec(&layout_value).expect("serialize normalized layout");
        assert_eq!(
            Digest::sha256(&layout_bytes).hex,
            package.expected_layout_sha256
        );
        if let Some(expected) = package.accuraterip {
            let CdProcessingResult::Layout {
                disc_ids: Some(ids),
                ..
            } = &layout
            else {
                panic!("AccurateRip oracle requires disc IDs")
            };
            let dbar_bytes = std::fs::read(safe(&root, &expected.dbar)).expect("read dBAR oracle");
            let dbar = DbarFile::parse(&dbar_bytes).expect("parse dBAR oracle");
            let mut bounded =
                BoundedDiscReader::new(DynPcmReader(reader), tracks).expect("bounded disc reader");
            let summary = verify_with_offset_reader(
                &dbar,
                &mut bounded,
                VerificationOptions {
                    max_sample_shift: expected.maximum_offset,
                },
                || Ok(()),
            )
            .expect("run shared verifier");
            let result = interpret_accuraterip_summary(
                &dbar,
                &summary,
                InterpretAccurateRipInput {
                    database: AccurateRipDatabaseIdentity {
                        track_count: result_track_count(&layout),
                        accuraterip_id1: u32::from_str_radix(&ids.accuraterip_id1, 16).unwrap(),
                        accuraterip_id2: u32::from_str_radix(&ids.accuraterip_id2, 16).unwrap(),
                        cddb_id: u32::from_str_radix(&ids.freedb, 16).unwrap(),
                    },
                    database_response_digest: Digest::sha256(&dbar_bytes),
                    input_digests: package
                        .files
                        .iter()
                        .map(|name| {
                            let mut input =
                                File::open(safe(&root, name)).expect("open package input");
                            let mut hash = Sha256::new();
                            let mut buffer = [0_u8; 65536];
                            loop {
                                let count = input.read(&mut buffer).expect("hash package input");
                                if count == 0 {
                                    break;
                                }
                                hash.update(&buffer[..count]);
                            }
                            Digest::new(
                                collector_core::HashAlgorithm::Sha256,
                                format!("{:x}", hash.finalize()),
                            )
                            .expect("SHA-256")
                        })
                        .collect(),
                },
            )
            .expect("interpret AccurateRip result");
            assert_eq!(result.status, expected.expected_status);
            assert_eq!(
                result.selected_offset.map(|value| value.0),
                expected.expected_selected_offset
            );
            assert_eq!(result.tracks.len(), expected.expected_tracks.len());
            for (actual, expected) in result.tracks.iter().zip(expected.expected_tracks) {
                assert_eq!(
                    (actual.position, actual.computed_offset.0),
                    (expected.position, expected.computed_offset)
                );
                assert_eq!(actual.checksums[0].checksum.hex, expected.arv1);
                assert_eq!(actual.checksums[1].checksum.hex, expected.arv2);
            }
        }
    }
}

fn result_track_count(layout: &CdProcessingResult) -> u8 {
    let CdProcessingResult::Layout { tracks, .. } = layout else {
        unreachable!()
    };
    u8::try_from(tracks.len()).expect("at most 99 tracks")
}
