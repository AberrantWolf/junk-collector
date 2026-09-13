use std::collections::BTreeMap;

use collector_core::{
    ByteLength, CollectionId, Digest, ImportId, MetadataAuthority, ModuleData, ModuleId,
    ObservationId, OperationSupport, PackageFileFingerprint, PackageFileId, PackageFileName,
    PackageInventory, PackageInventoryFile, PackageScope, ProcessingObservation, RawJson,
    SourceFingerprint,
};

fn id<T: std::str::FromStr>(value: &str) -> T
where
    T::Err: std::fmt::Debug,
{
    value.parse().unwrap()
}

fn fingerprint() -> SourceFingerprint {
    SourceFingerprint::calculate(
        PackageScope::Import(ImportId::new()),
        vec![PackageFileFingerprint {
            id: PackageFileId::new(),
            bytes: ByteLength(4),
            sha256: Digest::sha256(b"data"),
        }],
        None,
        BTreeMap::new(),
    )
    .unwrap()
}

#[test]
fn source_fingerprint_has_a_versioned_golden_encoding() {
    let source = PackageScope::Import(id("018f0000-0000-7000-8000-000000000001"));
    let files = vec![
        PackageFileFingerprint {
            id: id("018f0000-0000-7000-8000-000000000011"),
            bytes: ByteLength(5),
            sha256: Digest::sha256(b"alpha"),
        },
        PackageFileFingerprint {
            id: id("018f0000-0000-7000-8000-000000000012"),
            bytes: ByteLength(4),
            sha256: Digest::sha256(b"beta"),
        },
    ];
    let parameters = BTreeMap::from([
        ("system".to_owned(), "cd".to_owned()),
        ("profile".to_owned(), "raw-2352".to_owned()),
    ]);
    let fingerprint =
        SourceFingerprint::calculate(source, files, Some(Digest::sha256(b"layout")), parameters)
            .unwrap();

    assert_eq!(fingerprint.encoding_version, 1);
    assert_eq!(
        hex::encode(fingerprint.canonical_bytes().unwrap()),
        "4a435346000101018f000000007000800000000000000100000002018f000000007000800000000000001100000000000000058ed3f6ad685b959ead7022518e1af76cd816f8e8ec7ccdda1ed4018e8f2223f8018f00000000700080000000000000120000000000000004f44e64e75f3948e9f73f8dfa94721c4ce8cbb4f265c4790c702b2d41cfbf2753011dc5ae5b68174891b6aa9850aa05ee0d9ae8a20468d9517259951a2dd9e9c0f0000000020000000770726f66696c65000000087261772d323335320000000673797374656d000000026364"
    );
    assert_eq!(
        fingerprint.digest.hex,
        "6aadaca7a9c4acd15139391447bfd9e1abfab371219a22be94b110cd1e8499d4"
    );
}

#[test]
fn every_ordered_file_binds_the_fingerprint() {
    let source = PackageScope::Import(ImportId::new());
    let one = PackageFileFingerprint {
        id: PackageFileId::new(),
        bytes: ByteLength(5),
        sha256: Digest::sha256(b"same-identification-track"),
    };
    let second_id = PackageFileId::new();
    let fingerprint = |second: &[u8]| {
        SourceFingerprint::calculate(
            source,
            vec![
                one.clone(),
                PackageFileFingerprint {
                    id: second_id,
                    bytes: ByteLength(second.len() as u64),
                    sha256: Digest::sha256(second),
                },
            ],
            None,
            BTreeMap::new(),
        )
        .unwrap()
    };
    assert_ne!(
        fingerprint(b"track two").digest,
        fingerprint(b"changed!!").digest
    );
}

#[test]
fn package_inventory_rejects_unsafe_ambiguous_or_duplicate_entries() {
    let collection_id = CollectionId::new();
    let scope = PackageScope::Import(ImportId::new());
    let file = |name: &str, id| PackageInventoryFile {
        id,
        display_name: PackageFileName::new(name).unwrap(),
        bytes: ByteLength(1),
    };
    assert!(PackageFileName::new("../master.bin").is_err());
    assert!(PackageFileName::new("folder/master.bin").is_ok());
    assert!(PackageFileName::new("C:\\master.bin").is_err());
    assert!(PackageFileName::new("bad\0name").is_err());
    assert!(PackageInventory::new(collection_id, scope, vec![]).is_err());

    let duplicate_id = PackageFileId::new();
    assert!(
        PackageInventory::new(
            collection_id,
            scope,
            vec![file("a.bin", duplicate_id), file("b.bin", duplicate_id)],
        )
        .is_err()
    );
    assert!(
        PackageInventory::new(
            collection_id,
            scope,
            vec![
                file("same.bin", PackageFileId::new()),
                file("same.bin", PackageFileId::new())
            ],
        )
        .is_err()
    );
}

#[test]
fn package_inventory_resolves_bounded_relative_references() {
    let cue_id = PackageFileId::new();
    let audio_id = PackageFileId::new();
    let inventory = PackageInventory::new(
        CollectionId::new(),
        PackageScope::Import(ImportId::new()),
        vec![
            PackageInventoryFile {
                id: cue_id,
                display_name: PackageFileName::new("disc/layout/disc.cue").unwrap(),
                bytes: ByteLength(1),
            },
            PackageInventoryFile {
                id: audio_id,
                display_name: PackageFileName::new("disc/audio.bin").unwrap(),
                bytes: ByteLength(1),
            },
        ],
    )
    .unwrap();

    assert_eq!(
        inventory
            .resolve_relative(cue_id, "../audio.bin")
            .unwrap()
            .id,
        audio_id
    );
    assert!(
        inventory
            .resolve_relative(cue_id, "../../../outside.bin")
            .is_err()
    );
    assert!(inventory.resolve_relative(cue_id, "missing.bin").is_err());
}

#[test]
fn processing_and_metadata_envelopes_preserve_unknown_raw_json() {
    let raw = r#"{"z":1e+02,"a":null,"ordered":[3,2,1]}"#;
    let observation = ProcessingObservation {
        id: ObservationId::new(),
        collection_id: CollectionId::new(),
        operation: "org.example.stamps.inspect".parse().unwrap(),
        operation_version: 1,
        source_fingerprint: fingerprint(),
        snapshot_id: None,
        support: OperationSupport::ExternallyUnqualified {
            reason: "oracle not supplied".into(),
        },
        raw_observations: vec![ModuleData {
            module: ModuleId("org.example.provider".into()),
            schema_version: 99,
            payload: RawJson::from_json(raw).unwrap(),
        }],
        typed_result: ModuleData {
            module: ModuleId("org.example.stamps".into()),
            schema_version: 7,
            payload: RawJson::from_json(raw).unwrap(),
        },
        limitations: vec!["not verification".into()],
        created_at: chrono::Utc::now(),
    };
    observation.validate().unwrap();
    let encoded = serde_json::to_string(&observation).unwrap();
    let decoded: ProcessingObservation = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded.raw_observations[0].payload.get(), raw);
    assert_eq!(decoded.typed_result.payload.get(), raw);

    let authority = MetadataAuthority {
        retained_unknown_fields: BTreeMap::from([(
            "future".into(),
            RawJson::from_json(raw).unwrap(),
        )]),
        ..Default::default()
    };
    let decoded: MetadataAuthority =
        serde_json::from_str(&serde_json::to_string(&authority).unwrap()).unwrap();
    assert_eq!(decoded.retained_unknown_fields["future"].get(), raw);
}

#[test]
fn package_inventory_rejects_case_and_file_directory_collisions() {
    for names in [["Disc.bin", "disc.bin"], ["disc", "disc/audio.bin"]] {
        let files = names
            .into_iter()
            .map(|name| PackageInventoryFile {
                id: PackageFileId::new(),
                display_name: PackageFileName::new(name).unwrap(),
                bytes: ByteLength(1),
            })
            .collect();
        assert!(
            PackageInventory::new(
                CollectionId::new(),
                PackageScope::Import(ImportId::new()),
                files
            )
            .is_err()
        );
    }
}

#[test]
fn metadata_text_override_rejects_non_text_json() {
    let value = r#"{"field":"title","value":{"nested":true},"language":null}"#;
    assert!(serde_json::from_str::<collector_core::MetadataFieldOverride>(value).is_err());
}

#[test]
fn conversion_recipe_rejects_unrelated_digest() {
    use collector_core::*;
    let recipe = ConversionRecipe {
        id: RecipeId::new(),
        version: 1,
        settings: ConversionSettings::Flac { level: 5 },
        source_fingerprint: SourceFingerprint::calculate(
            PackageScope::Representation(id("018f0000-0000-7000-8000-000000000001")),
            vec![PackageFileFingerprint {
                id: id("018f0000-0000-7000-8000-000000000011"),
                bytes: ByteLength(4),
                sha256: Digest::sha256(b"data"),
            }],
            None,
            BTreeMap::new(),
        )
        .unwrap(),
        tracks: vec![ConversionTrack {
            position: 1,
            start: PcmFrame(0),
            length: PcmFrame(1),
        }],
        encoder: "flac".into(),
        encoder_version: "1.5.0".into(),
        input_format: PcmSampleFormat {
            sample_rate: 44100,
            channels: 2,
            bits_per_sample: 16,
            signed: true,
            little_endian: true,
        },
        output_format: PcmSampleFormat {
            sample_rate: 44100,
            channels: 2,
            bits_per_sample: 16,
            signed: true,
            little_endian: true,
        },
        digest: Digest::sha256(b"unrelated bytes"),
    };
    assert!(recipe.validate().is_err());
    let sealed = recipe.seal().unwrap();
    sealed.validate().unwrap();
    // Independently encoded with Python struct/UUID/json/hashlib; recipe v1.
    assert_eq!(
        sealed.digest.hex,
        "27351c36061ce8e2f7084f72f656a6f990c93c6821d56e59dc47441b2d332a9c"
    );
    let mut changed = sealed.clone();
    changed.settings = ConversionSettings::Flac { level: 6 };
    assert!(changed.validate().is_err());
    changed = sealed.clone();
    changed.output_format.sample_rate = 48000;
    assert!(changed.seal().is_err(), "FLAC cannot silently resample");
    changed = sealed.clone();
    changed.tracks.push(changed.tracks[0].clone());
    assert!(changed.seal().is_err(), "duplicate/overlapping tracks");
    changed = sealed.clone();
    changed.id = RecipeId::new();
    changed.validate().unwrap();
    assert_eq!(
        changed.digest, sealed.digest,
        "record ID is not recipe content"
    );
}

#[test]
fn source_fingerprint_deserialization_validates_binding() {
    let original = fingerprint();
    let mut wire = serde_json::to_value(&original).unwrap();
    wire["files"][0]["bytes"] = serde_json::json!(999);
    assert!(serde_json::from_value::<SourceFingerprint>(wire).is_err());
    let roundtrip: SourceFingerprint =
        serde_json::from_str(&serde_json::to_string(&original).unwrap()).unwrap();
    assert_eq!(roundtrip, original);
}

#[test]
fn metadata_authority_has_one_value_per_known_field() {
    use collector_core::{MetadataField, MetadataFieldOverride};
    let text = MetadataFieldOverride {
        field: MetadataField::Title,
        value: "手動".into(),
        language: None,
    };
    let mut authority = MetadataAuthority {
        field_overrides: vec![text.clone()],
        ..Default::default()
    };
    authority.validate().unwrap();
    authority.field_overrides.push(text);
    assert!(authority.validate().is_err());
}

#[test]
fn fingerprint_must_match_the_authorized_inventory() {
    let frozen = fingerprint();
    let mut inventory = PackageInventory::new(
        CollectionId::new(),
        frozen.source,
        vec![PackageInventoryFile {
            id: frozen.files[0].id,
            bytes: frozen.files[0].bytes,
            display_name: PackageFileName::new("disc.bin").unwrap(),
        }],
    )
    .unwrap();
    frozen.validate_inventory(&inventory).unwrap();
    inventory.files[0].id = PackageFileId::new();
    assert!(frozen.validate_inventory(&inventory).is_err());
    inventory.files[0].id = frozen.files[0].id;
    inventory.scope = PackageScope::Import(ImportId::new());
    assert!(frozen.validate_inventory(&inventory).is_err());
}

#[test]
fn historical_field_authority_preserves_language_and_raw_values_without_applying_it() {
    let wire = r#"{"selected_candidate":null,"field_overrides":[{"field":"artist","value":"Artist","language":"ja","extra":1e+02},{"field":"platform","value":"Console","language":"en"}],"artwork_choice":null,"retained_unknown_fields":{}}"#;
    let authority: MetadataAuthority = serde_json::from_str(wire).unwrap();
    assert!(authority.field_overrides.is_empty());
    assert!(authority.domain_overrides.is_none());
    assert_eq!(
        authority.preserved_field_overrides[0].get(),
        r#"{"field":"artist","value":"Artist","language":"ja","extra":1e+02}"#
    );
    let encoded = serde_json::to_string(&authority).unwrap();
    let decoded: MetadataAuthority = serde_json::from_str(&encoded).unwrap();
    assert_eq!(serde_json::to_string(&decoded).unwrap(), encoded);
    assert_eq!(decoded.preserved_field_overrides.len(), 2);
    let invalid = wire.replace("artist", "title");
    assert!(serde_json::from_str::<MetadataAuthority>(&invalid).is_err());
}
