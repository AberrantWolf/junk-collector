use collector_app::resolve_metadata_display;
use collector_core::*;

fn item() -> Item {
    Item {
        lifecycle: Default::default(),
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
    // Incomplete candidates are suggestions; an arbitrary first result must
    // never become the displayed album.
    assert_eq!(automatic.title.value, "Inventory title");
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

fn album_candidate(
    release: &str,
    group: Option<&str>,
    language: &str,
    title: &str,
) -> MetadataCandidateObservation {
    let mut value = candidate(
        "org.junk.providers.musicbrainz",
        release,
        title,
        language,
        true,
        MatchStrength::DiscLayout,
        10,
    );
    let observation = collector_modules::music::MusicBrainzReleaseObservation {
        release_id: release.into(),
        release_group_id: group.map(str::to_string),
        title: title.into(),
        language: Some(language.into()),
        script: None,
        artist_credit: None,
        artist_ids: vec![],
        disambiguation: None,
        disc_ids: vec![],
        media: vec![],
    };
    value.observation = RawJson::from_value(serde_json::to_value(observation).unwrap());
    value
}

#[test]
fn pressings_and_languages_of_one_album_resolve_without_album_ambiguity() {
    let group = "11111111-1111-4111-8111-111111111111";
    let english = album_candidate(
        "22222222-2222-4222-8222-222222222222",
        Some(group),
        "en",
        "Album",
    );
    let japanese = album_candidate(
        "33333333-3333-4333-8333-333333333333",
        Some(group),
        "ja",
        "アルバム",
    );
    for values in [
        vec![english.clone(), japanese.clone()],
        vec![japanese.clone(), english.clone()],
    ] {
        for (language, title) in [("en", "Album"), ("ja", "アルバム")] {
            let display =
                resolve_metadata_display(&item(), &MetadataAuthority::default(), &values, language)
                    .unwrap();
            assert_eq!(display.resolution, MetadataResolutionKind::Automatic);
            assert_eq!(display.title.value, title);
        }
    }
}

#[test]
fn shared_disc_id_on_different_albums_needs_review_even_in_different_languages() {
    let first = album_candidate(
        "22222222-2222-4222-8222-222222222222",
        Some("11111111-1111-4111-8111-111111111111"),
        "en",
        "Album A",
    );
    let other = album_candidate(
        "33333333-3333-4333-8333-333333333333",
        Some("44444444-4444-4444-8444-444444444444"),
        "ja",
        "Album B",
    );
    let values = [first.clone(), other];
    let display =
        resolve_metadata_display(&item(), &MetadataAuthority::default(), &values, "en").unwrap();
    assert_eq!(display.resolution, MetadataResolutionKind::Tentative);
    assert_eq!(display.candidate, None);
    let authority = MetadataAuthority {
        automatic_candidate: Some(first.identity.clone()),
        ..Default::default()
    };
    let display = resolve_metadata_display(&item(), &authority, &values, "ja").unwrap();
    assert_eq!(display.candidate, Some(first.identity));
    assert_eq!(display.title.value, "Album A");
    assert_eq!(authority.selected_candidate, None);
}

#[test]
fn missing_album_identity_is_inconclusive_and_name_search_never_auto_resolves() {
    let first = album_candidate(
        "22222222-2222-4222-8222-222222222222",
        None,
        "en",
        "Album A",
    );
    let (resolved, reason) =
        collector_app::automatic_metadata_candidate(std::slice::from_ref(&first), "en");
    assert!(resolved.is_none());
    assert_eq!(
        reason,
        Some(collector_app::MetadataReviewReason::IncompleteProviderInformation)
    );
    let second = album_candidate(
        "33333333-3333-4333-8333-333333333333",
        None,
        "en",
        "Album A",
    );
    assert!(
        collector_app::automatic_metadata_candidate(&[first, second], "en")
            .0
            .is_none()
    );
    let text = candidate(
        "org.junk.providers.musicbrainz",
        "search",
        "Filename guess",
        "en",
        true,
        MatchStrength::Name,
        1,
    );
    assert!(
        collector_app::automatic_metadata_candidate(&[text], "en")
            .0
            .is_none()
    );
}

#[test]
fn domain_override_command_decodes_nested_payload_from_real_json() {
    let command:collector_app::UpdateMetadataAuthorityCommand=serde_json::from_str(r#"{"expected_revision":1,"change":{"change":"set_domain_overrides","overrides":{"module":"org.junk.games","schema_version":1,"payload":{"platform":"Sega - Saturn","automatic_platform":false}}}}"#).unwrap();
    let collector_app::MetadataAuthorityChange::SetDomainOverrides { overrides } = command.change
    else {
        panic!("wrong command");
    };
    let value: collector_modules::games::GameMetadataOverrides = overrides.payload.parse().unwrap();
    assert_eq!(value.platform.as_deref(), Some("Sega - Saturn"));
}
