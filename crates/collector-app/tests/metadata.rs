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
