use collector_app::resolve_metadata_display;
use collector_core::*;

fn item() -> Item {
    Item {
        id: ItemId::new(),
        collection_id: CollectionId::new(),
        revision: Revision(1),
        title: "Inventory title".into(),
        subtitle: String::new(),
        identity_link: ItemIdentityLink::None,
        domain: ModuleData {
            module: ModuleId("org.junk.music".into()),
            schema_version: 1,
            payload: RawJson::from_json("{}").unwrap(),
        },
        identity: IdentityState::Unidentified,
        translations: vec![],
        identifiers: vec![],
        specimens: vec![],
        wishlist: false,
        captures: vec![],
        assets: vec![],
        metadata: MetadataState::default(),
        deleted: false,
        updated_at: chrono::Utc::now(),
    }
}

fn candidate(
    provider: &str,
    record: &str,
    title: &str,
    language: &str,
    complete_identity: bool,
    strength: MatchStrength,
    priority: u16,
) -> MetadataCandidateObservation {
    MetadataCandidateObservation {
        identity: MetadataCandidateIdentity {
            provider: ModuleId(provider.into()),
            provider_record_id: record.into(),
            provider_record_version: Some("1".into()),
        },
        title: MetadataText {
            value: title.into(),
            language: Some(LanguageTag::new(language).unwrap()),
        },
        subtitle: None,
        artwork: None,
        complete_identity,
        strength,
        provider_priority: priority,
        observation: RawJson::from_json(r#"{"raw":1e+02,"unknown":null}"#).unwrap(),
    }
}

#[test]
fn resolver_orders_evidence_before_language_and_is_stable_under_reordering() {
    let japanese = candidate(
        "org.example.musicbrainz",
        "jp-strong",
        "強い一致",
        "ja",
        true,
        MatchStrength::Identifier,
        20,
    );
    let english = candidate(
        "org.example.names",
        "en-name",
        "English name",
        "en",
        false,
        MatchStrength::Name,
        1,
    );
    let authority = MetadataAuthority::default();
    for candidates in [
        vec![english.clone(), japanese.clone()],
        vec![japanese.clone(), english.clone()],
    ] {
        let display = resolve_metadata_display(&item(), &authority, &candidates, "en").unwrap();
        assert_eq!(display.title.value, "強い一致");
        assert_eq!(display.title.language.unwrap().as_str(), "ja");
        assert_eq!(display.resolution, MetadataResolutionKind::Automatic);
    }
}

#[test]
fn field_override_then_selection_then_automatic_and_ambiguity_are_explicit() {
    let fallback =
        resolve_metadata_display(&item(), &MetadataAuthority::default(), &[], "en").unwrap();
    assert_eq!(fallback.title.value, "Inventory title");
    assert_eq!(fallback.resolution, MetadataResolutionKind::Manual);

    let first = candidate(
        "org.example.provider",
        "a",
        "Automatic A",
        "en",
        false,
        MatchStrength::DiscLayout,
        10,
    );
    let second = candidate(
        "org.example.provider",
        "b",
        "Automatic B",
        "en",
        false,
        MatchStrength::DiscLayout,
        10,
    );
    let candidates = vec![second.clone(), first.clone()];
    let automatic =
        resolve_metadata_display(&item(), &MetadataAuthority::default(), &candidates, "en")
            .unwrap();
    assert_eq!(automatic.title.value, "Automatic A");
    assert_eq!(automatic.resolution, MetadataResolutionKind::Tentative);

    let selected = MetadataAuthority {
        selected_candidate: Some(second.identity.clone()),
        ..Default::default()
    };
    let display = resolve_metadata_display(&item(), &selected, &candidates, "en").unwrap();
    assert_eq!(display.title.value, "Automatic B");
    assert_eq!(display.resolution, MetadataResolutionKind::Selected);

    let overridden = MetadataAuthority {
        selected_candidate: Some(second.identity),
        field_overrides: vec![MetadataFieldOverride {
            field: MetadataField::Title,
            value: "Manual title".into(),
            language: Some(LanguageTag::new("ja").unwrap()),
        }],
        ..Default::default()
    };
    let display = resolve_metadata_display(&item(), &overridden, &candidates, "en").unwrap();
    assert_eq!(display.title.value, "Manual title");
    assert_eq!(display.resolution, MetadataResolutionKind::Manual);
}

#[test]
fn ambiguity_survives_unrelated_manual_choices_and_inventory_subtitle_is_retained() {
    let mut item = item();
    item.subtitle = "Inventory subtitle".into();
    let mut authority = MetadataAuthority::default();
    assert_eq!(
        resolve_metadata_display(&item, &authority, &[], "en")
            .unwrap()
            .subtitle
            .unwrap()
            .value,
        "Inventory subtitle"
    );
    let a = candidate(
        "org.example.names",
        "a",
        "A",
        "en",
        false,
        MatchStrength::Name,
        1,
    );
    let mut b = a.clone();
    b.identity.provider_record_id = "b".into();
    authority.artwork_choice = Some(AssetId::new());
    assert_eq!(
        resolve_metadata_display(&item, &authority, &[a.clone(), b], "en")
            .unwrap()
            .resolution,
        MetadataResolutionKind::Tentative
    );
    let mut changed = a.clone();
    changed.title.value = "Changed".into();
    assert!(matches!(
        collector_app::merge_metadata_candidates(&mut vec![a], [&changed]),
        Err(Error::Conflict)
    ));
}

#[test]
fn dat_metadata_uses_match_evidence_and_raw_snapshot_version_not_policy_order() {
    use collector_modules::games::{
        CatalogCandidate, CatalogMatchResult, CatalogMatchedUnit, dat_metadata_candidates,
    };

    let provider = ModuleId("org.junk.providers.local-dat".into());
    let row = RawJson::from_json(
        r#"{"record_id":"fixture:1","label":"強い DAT 名","region":"Japan","version":"Rev A","category":null,"files":[{"name":"game.bin","bytes":4,"serial":"ABC","crc32":"12345678","sha1":null,"md5":null}]}"#,
    )
    .unwrap();
    let result = CatalogMatchResult::UniqueCompleteMatch {
        candidate: CatalogCandidate {
            provider: provider.clone(),
            provider_record_id: "fixture:1".into(),
            label: "強い DAT 名".into(),
            matched_units: vec![CatalogMatchedUnit {
                source_file_id: PackageFileId::new(),
                track_number: None,
                digest: Digest::new(HashAlgorithm::Crc32, "12345678").unwrap(),
            }],
        },
    };
    let first = dat_metadata_candidates(
        provider.clone(),
        &Digest::sha256(b"raw-a"),
        std::slice::from_ref(&row),
        &result,
    )
    .unwrap();
    let second = dat_metadata_candidates(
        provider,
        &Digest::sha256(b"raw-b"),
        std::slice::from_ref(&row),
        &result,
    )
    .unwrap();
    assert_eq!(first[0].title.value, "強い DAT 名");
    assert_eq!(first[0].subtitle.as_ref().unwrap().value, "Japan · Rev A");
    assert!(first[0].complete_identity);
    assert_eq!(first[0].provider_priority, 10);
    assert_ne!(
        first[0].identity.provider_record_version, second[0].identity.provider_record_version,
        "changed exact DAT bytes cannot reuse a candidate identity"
    );

    let inconsistent = CatalogMatchResult::UniqueCompleteMatch {
        candidate: CatalogCandidate {
            provider: ModuleId("org.junk.providers.local-dat".into()),
            provider_record_id: "fixture:1".into(),
            label: "Mutable label".into(),
            matched_units: vec![CatalogMatchedUnit {
                source_file_id: PackageFileId::new(),
                track_number: None,
                digest: Digest::new(HashAlgorithm::Crc32, "12345678").unwrap(),
            }],
        },
    };
    assert!(
        dat_metadata_candidates(
            ModuleId("org.junk.providers.local-dat".into()),
            &Digest::sha256(b"raw-a"),
            std::slice::from_ref(&row),
            &inconsistent,
        )
        .is_err()
    );

    let musicbrainz = candidate(
        "org.junk.providers.musicbrainz",
        "release",
        "English provider disagreement",
        "en",
        true,
        MatchStrength::DiscLayout,
        1,
    );
    let display = resolve_metadata_display(
        &item(),
        &MetadataAuthority::default(),
        &[musicbrainz, first[0].clone()],
        "en",
    )
    .unwrap();
    assert_eq!(display.title.value, "強い DAT 名");
}
