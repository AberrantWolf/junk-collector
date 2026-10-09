use collector_app::IngestPlanPackage;

#[test]
fn retained_private_intake_plans_do_not_require_new_presentation_fields() {
    let old = serde_json::json!({
        "position":0,
        "source":{"kind":"browser","label":null,"package_name":"Retained originals"},
        "files":[{"name":"disc.bin","bytes":2352,"sha256":null}],
        "default_title":"Retained title",
        "anchors":[],"blockers":[],"analysis_available":true,
        "metadata_available":false,"artwork_available":false,
        "preparation_available":false,"preparation_reason":null
    });
    let retained: IngestPlanPackage = serde_json::from_value(old.clone()).unwrap();
    assert!(!retained.music_actions_available);
    let mut round_trip = serde_json::to_value(retained).unwrap();
    round_trip
        .as_object_mut()
        .unwrap()
        .remove("music_actions_available");
    assert_eq!(round_trip, old);
}
