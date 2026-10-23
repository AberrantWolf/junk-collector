use collector_core::*;
use collector_modules::{games::*, *};
use std::{collections::BTreeMap, io::Cursor};

struct Host {
    access: PackageAccess,
    files: Vec<(PackageFileId, String, Vec<u8>)>,
    catalogs: BTreeMap<SnapshotId, CatalogSnapshotInput>,
}
impl Host {
    fn new(files: Vec<(&str, Vec<u8>)>) -> Self {
        Self {
            access: PackageAccess {
                collection_id: CollectionId::new(),
                scope: PackageScope::Representation(RepresentationId::new()),
            },
            files: files
                .into_iter()
                .map(|(name, bytes)| (PackageFileId::new(), name.into(), bytes))
                .collect(),
            catalogs: BTreeMap::new(),
        }
    }
    fn catalog(&mut self, system: &str, domain: &str, label: &str, units: &[(u64, &str)]) {
        let id = SnapshotId::new();
        let rows = vec![RawJson::from_value(
            serde_json::json!({"record_id":"game:0","label":label,"files":units.iter().enumerate().map(|(i,(bytes,sha1))|serde_json::json!({"name":format!("track-{}.bin",i+1),"bytes":bytes,"sha1":sha1})).collect::<Vec<_>>()}),
        )];
        let digest = Digest::sha256(format!("{system}:{domain}:{label}:{id}").as_bytes());
        self.catalogs.insert(id,CatalogSnapshotInput {id,provider:ModuleId("org.junk.providers.local-dat".into()),digest,observation:RawJson::from_value(serde_json::json!({"schema_version":1,"system":system,"catalog":if domain=="org.junk.game.cd.raw-2352-track" {"Redump"} else {"No-Intro"},"version":"independent fixture","byte_domain":domain,"byte_domain_version":1,"game_count":1})),entries:rows});
    }
    fn identify(&self) -> GameIdentification {
        identify_game(
            self,
            self.access,
            &self.catalogs.keys().copied().collect::<Vec<_>>(),
        )
        .unwrap()
    }
}
impl ModuleHost for Host {
    fn package_inventory(&self, access: PackageAccess) -> ModuleResult<PackageInventory> {
        assert_eq!(access, self.access);
        PackageInventory::new(
            access.collection_id,
            access.scope,
            self.files
                .iter()
                .map(|(id, name, bytes)| PackageInventoryFile {
                    id: *id,
                    display_name: PackageFileName::new(name).unwrap(),
                    bytes: ByteLength(bytes.len() as u64),
                })
                .collect(),
        )
        .map_err(|e| ModuleError::Invalid(e.to_string()))
    }
    fn package_file(
        &self,
        _: PackageAccess,
        id: PackageFileId,
    ) -> ModuleResult<Box<dyn RepresentationReader>> {
        Ok(Box::new(Cursor::new(
            self.files.iter().find(|f| f.0 == id).unwrap().2.clone(),
        )))
    }
    fn catalog_snapshot(&self, id: SnapshotId) -> ModuleResult<CatalogSnapshotInput> {
        Ok(self.catalogs[&id].clone())
    }
    fn representation_access(&self, _: RepresentationId) -> ModuleResult<PackageAccess> {
        Ok(self.access)
    }
    fn submit_job(&self, _: JobSubmission) -> ModuleResult<JobId> {
        unreachable!()
    }
}
fn assert_recognized(host: &Host) {
    let result = host.identify();
    assert!(
        matches!(result.status, GameDetectionStatus::Recognized),
        "{result:?}"
    );
    assert!(result.candidates.iter().all(|c| c.complete_identity));
    Games
        .validate_operation_result(
            &CapabilityId::new("org.junk.games.identify").unwrap(),
            &RawJson::from_value(serde_json::to_value(result).unwrap()),
        )
        .unwrap();
}
#[test]
fn automatic_cartridge_profiles_require_complete_independent_catalog_vectors() {
    let mut nes = vec![0; 16];
    nes[..4].copy_from_slice(b"NES\x1a");
    nes[4] = 1;
    nes.extend(vec![0x5a; 16384]);
    for name in ["disc.bin", "misleading.cue", "wrong.z64"] {
        let mut host = Host::new(vec![(name, nes.clone())]);
        host.catalog(
            "NES",
            "org.junk.game.nes.ines-payload",
            "NES fixture",
            &[(16384, "3e039b4b6cdd54fbe4c4259157894de80305c199")],
        );
        assert_recognized(&host);
    }
    let mut payload = vec![0x80, 0x37, 0x12, 0x40];
    payload.resize(4096, 0);
    for width in [0, 2, 4] {
        let mut rom = payload.clone();
        if width > 0 {
            for chunk in rom.chunks_exact_mut(width) {
                chunk.reverse();
            }
        }
        let mut host = Host::new(vec![("wrong.nes", rom)]);
        host.catalog(
            "Nintendo 64",
            "org.junk.game.n64.z64-big-endian",
            "N64 fixture",
            &[(4096, "5b62d92d1ed8e6c7100074fd4b0393249bd08dba")],
        );
        assert_recognized(&host);
    }
    for (system, domain, length, byte, sha1) in [
        (
            "SNES",
            "org.junk.game.snes.headerless-payload",
            32768,
            0x33,
            "52389cdcf079cb19865428bd00e441c333f75b4a",
        ),
        (
            "PC Engine",
            "org.junk.game.pce.hucard-payload",
            8192,
            0x22,
            "d0181bf51c9939e10b84b2006879b984ac96b1dd",
        ),
        (
            "NES",
            "org.junk.game.nes.ines-payload",
            16384,
            0x5a,
            "3e039b4b6cdd54fbe4c4259157894de80305c199",
        ),
    ] {
        for copier in [false, true] {
            if copier && system == "NES" {
                continue;
            }
            let mut rom = if copier { vec![0xa5; 512] } else { vec![] };
            rom.extend(vec![byte; length]);
            let mut host = Host::new(vec![("unhelpful.bin", rom)]);
            assert!(matches!(
                host.identify().status,
                GameDetectionStatus::Unrecognized
            ));
            host.catalog(
                system,
                domain,
                "Headerless fixture",
                &[(length as u64, sha1)],
            );
            assert_recognized(&host);
        }
    }
}
#[test]
fn malformed_and_unsupported_inputs_cannot_get_raw_catalog_identity() {
    for rom in [
        b"NES\x1a\x01\0\0\0\0\0\0\0\0\0\0\0".to_vec(),
        b"UNIFstuff".to_vec(),
        vec![0x80, 0x37, 0x12, 0x40],
    ] {
        let mut host = Host::new(vec![("looks-like-a-game.bin", rom.clone())]);
        let sha1 = match rom.len() {
            4 => "f834b4f983daad0442642b54eaa503d30489ceb7",
            9 => "3707b5c5018259945536e491faf638f504561000",
            _ => "4c245e1fabffdfe688e9422eb8d07329fee81814",
        };
        host.catalog(
            "Game Boy",
            "org.junk.game.raw-bytes",
            "Must not match",
            &[(rom.len() as u64, sha1)],
        );
        let result = host.identify();
        assert!(matches!(
            result.status,
            GameDetectionStatus::Malformed | GameDetectionStatus::Unsupported
        ));
        assert!(result.candidates.is_empty());
        assert!(result.analyses.is_empty());
    }
    for length in [8192, 32768, 33280] {
        let host = Host::new(vec![("megadrive.bin", vec![0x55; length])]);
        assert!(matches!(
            host.identify().status,
            GameDetectionStatus::Unrecognized
        ));
    }
}
#[test]
fn complete_headerless_identity_and_competing_systems_are_explicit() {
    let mut host = Host::new(vec![("disc.bin", b"abc".to_vec())]);
    host.catalog(
        "Game Boy",
        "org.junk.game.raw-bytes",
        "Known game",
        &[(3, "a9993e364706816aba3e25717850c26c9cd0d89d")],
    );
    assert_recognized(&host);
    host.catalog(
        "GB",
        "org.junk.game.raw-bytes",
        "Known game",
        &[(3, "a9993e364706816aba3e25717850c26c9cd0d89d")],
    );
    assert_recognized(&host);
    host.catalog(
        "Mega Drive",
        "org.junk.game.raw-bytes",
        "Different system",
        &[(3, "a9993e364706816aba3e25717850c26c9cd0d89d")],
    );
    assert!(matches!(
        host.identify().status,
        GameDetectionStatus::Ambiguous
    ));
}
#[test]
fn ordered_cue_tracks_have_complete_or_partial_coverage() {
    let cue=b"FILE \"one.bin\" BINARY\n TRACK 01 AUDIO\n INDEX 01 00:00:00\nFILE \"two.bin\" BINARY\n TRACK 02 AUDIO\n INDEX 01 00:00:00\n".to_vec();
    let mut host = Host::new(vec![
        ("renamed.dat", cue.clone()),
        ("one.bin", vec![0; 2352 * 5]),
        ("two.bin", vec![0x33; 2352 * 5]),
    ]);
    host.catalog(
        "PSX",
        "org.junk.game.cd.raw-2352-track",
        "Final Fantasy Tactics (USA)",
        &[
            (11760, "89075aa74e5d8ec7bc850a0930d0deccf320a01f"),
            (11760, "6158cba7c26235b0847e75d923fed50357affcf9"),
        ],
    );
    assert_recognized(&host);
    host.files[2].2[0] = 1;
    let partial = host.identify();
    assert!(matches!(partial.status, GameDetectionStatus::Unrecognized));
    assert!(partial.candidates.iter().all(|c| !c.complete_identity));
    host.files[0].2.extend_from_slice(b" UNSUPPORTED\n");
    let invalid = host.identify();
    assert!(matches!(
        invalid.status,
        GameDetectionStatus::Malformed | GameDetectionStatus::Unsupported
    ));
    assert!(invalid.candidates.is_empty());
}
#[test]
fn catalog_detection_has_finite_aliases_and_constrained_conflicts() {
    for alias in ["PS1", "PSX", "Sony PlayStation", "Sony - PlayStation"] {
        assert_eq!(standard_system(alias).unwrap().id, "ps1");
    }
    assert!(standard_system("made-up console").is_none());
    let detection =
        detect_standard_catalog("Logiqx XML", "PS1", "Saturn", "Redump", "", "20261010");
    assert!(matches!(detection.status, GameDetectionStatus::Ambiguous));
    assert!(corrected_catalog(&detection, "Sony - PlayStation").is_some());
    assert!(corrected_catalog(&detection, "Nintendo - Game Boy").is_none());
    assert!(matches!(
        detect_standard_catalog("Logiqx XML", "PS1", "", "custom", "", "1").status,
        GameDetectionStatus::Unsupported
    ));
}

#[test]
fn identification_envelope_cannot_invent_matches_or_completeness() {
    let mut host = Host::new(vec![("disc.bin", b"abc".to_vec())]);
    host.catalog(
        "GB",
        "org.junk.game.raw-bytes",
        "Known game",
        &[(3, "a9993e364706816aba3e25717850c26c9cd0d89d")],
    );
    let valid = host.identify();
    for mutation in 0..5 {
        let mut result = valid.clone();
        match mutation {
            0 => result.candidates[0].title.value = "Invented title".into(),
            1 => result.candidates[0].complete_identity = false,
            2 => result.analyses.clear(),
            4 => result.inspections[0].status = GameDetectionStatus::Malformed,
            _ => {
                if let CatalogMatchResult::UniqueCompleteMatch { candidate } =
                    &mut result.catalogs[0].catalog
                {
                    candidate.matched_units[0].digest.hex = "0".repeat(40);
                }
            }
        }
        assert!(
            Games
                .validate_operation_result(
                    &CapabilityId::new("org.junk.games.identify").unwrap(),
                    &RawJson::from_value(serde_json::to_value(result).unwrap())
                )
                .is_err()
        );
    }
}

#[test]
fn mixed_game_audio_keeps_disc_numbers_and_never_decodes_data() {
    use collector_modules::disc_audio::*;
    let cue = b"FILE \"data.bin\" BINARY\n TRACK 01 MODE2/2352\n INDEX 01 00:00:00\nFILE \"audio.bin\" BINARY\n TRACK 02 AUDIO\n INDEX 00 00:00:00\n INDEX 01 00:00:01\n TRACK 03 AUDIO\n INDEX 00 00:00:02\n INDEX 01 00:00:03\n";
    let bytes = (0..5 * 588)
        .flat_map(|frame| {
            let left = (frame as i16).to_le_bytes();
            let right = (-(frame as i16)).to_le_bytes();
            [left[0], left[1], right[0], right[1]]
        })
        .collect();
    let host = Host::new(vec![
        ("game.cue", cue.to_vec()),
        ("data.bin", vec![0xa5; 2352]),
        ("audio.bin", bytes),
    ]);
    let audio = inspect(
        &host,
        host.access,
        &host.package_inventory(host.access).unwrap(),
        0,
    )
    .unwrap();
    assert_eq!(audio.data_tracks, vec![1]);
    assert_eq!(
        audio
            .tracks
            .iter()
            .map(|t| (t.position, t.start_frame, t.frame_count))
            .collect::<Vec<_>>(),
        vec![(2, 0, 2 * 588), (3, 2 * 588, 2 * 588)]
    );
    let mut pcm = open_conversion(
        &host,
        host.access,
        &host.package_inventory(host.access).unwrap(),
        0,
    )
    .unwrap();
    let mut out = [[0i16; 2]; 700];
    let mut actual = vec![];
    loop {
        let n = pcm.reader.read_frames(&mut out).unwrap();
        if n == 0 {
            break;
        }
        actual.extend_from_slice(&out[..n]);
    }
    let expected = (588..5 * 588)
        .map(|frame| [frame as i16, -(frame as i16)])
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
    pcm.reader.seek_frame(2 * 588).unwrap();
    assert!(pcm.reader.read_frames(&mut out).unwrap() > 0);
    assert_eq!(out[0], [3 * 588, -(3 * 588)]);
    let mut identification = host.identify();
    let capability = CapabilityId::new("org.junk.games.identify").unwrap();
    Games
        .validate_operation_result(
            &capability,
            &RawJson::from_value(serde_json::to_value(&identification).unwrap()),
        )
        .unwrap();
    identification.analyses.push(GameProcessingResult::CdAudio {
        audio: audio.clone(),
    });
    let duplicate = RawJson::from_value(serde_json::to_value(&identification).unwrap());
    assert!(
        Games
            .validate_operation_result(&capability, &duplicate)
            .is_err()
    );
    assert!(recorded_disc_audio(capability.as_str(), &duplicate).is_err());
    let mut inconsistent = audio.clone();
    inconsistent.tracks[1].start_frame += 1;
    assert!(inconsistent.validate().is_err());
    let mut inconsistent = audio;
    inconsistent.data_tracks.push(2);
    assert!(inconsistent.validate().is_err());
}

#[test]
fn game_audio_rejects_malformed_or_truncated_cue_sources() {
    use collector_modules::disc_audio::*;
    for (cue, length) in [
        (
            "FILE \"audio.bin\" BINARY\n TRACK 01 AUDIO\n INDEX 01 00:00:00\n",
            2351,
        ),
        (
            "FILE \"audio.bin\" BINARY\n TRACK 01 AUDIO\n INDEX 01 00:00:00\nFILE \"./audio.bin\" BINARY\n TRACK 02 AUDIO\n INDEX 01 00:00:00\n",
            2352,
        ),
        (
            "FILE \"audio.bin\" BINARY\n TRACK 01 AUDIO\n INDEX 01 00:00:00\n UNSUPPORTED 1\n",
            2352,
        ),
    ] {
        let host = Host::new(vec![
            ("game.cue", cue.as_bytes().to_vec()),
            ("audio.bin", vec![0; length]),
        ]);
        assert!(
            inspect(
                &host,
                host.access,
                &host.package_inventory(host.access).unwrap(),
                0
            )
            .is_err()
        );
    }
}
