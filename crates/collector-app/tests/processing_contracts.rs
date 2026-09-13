use std::collections::BTreeMap;

use chrono::{TimeZone, Utc};
use collector_app::{FrozenProcessingJob, ProviderRequest, ProviderResponse, SnapshotQuery};
use collector_core::{
    ByteLength, CollectionBinding, CollectionId, Digest, ImportId, ModuleId, ObservationId,
    PackageFileFingerprint, PackageFileId, PackageScope, RawJson, SnapshotId, SourceFingerprint,
};

fn games_binding() -> CollectionBinding {
    CollectionBinding::Assigned {
        module: ModuleId("org.junk.games".into()),
    }
}

#[test]
fn processing_job_freezes_source_snapshot_outputs_and_time() {
    let created_at = Utc.with_ymd_and_hms(2026, 9, 11, 1, 2, 3).unwrap();
    let source = PackageScope::Import(ImportId::new());
    let fingerprint = SourceFingerprint::calculate(
        source,
        vec![PackageFileFingerprint {
            id: PackageFileId::new(),
            bytes: ByteLength(6),
            sha256: Digest::sha256(b"source"),
        }],
        None,
        BTreeMap::new(),
    )
    .unwrap();

    let job = FrozenProcessingJob {
        collection_id: CollectionId::new(),
        collection_binding: games_binding(),
        operation: "org.example.stamps.inspect".parse().unwrap(),
        operation_version: 2,
        source,
        source_fingerprint: fingerprint,
        snapshot_id: Some(SnapshotId::new()),
        observation_id: ObservationId::new(),
        selection: RawJson::from_json(r#"{"candidate":"fixed"}"#).unwrap(),
        created_at,
    };
    assert_eq!(job.created_at, created_at);
    assert!(job.validate().is_ok());
    let encoded = serde_json::to_value(&job).unwrap();
    assert_eq!(encoded["collection_binding"]["module"], "org.junk.games");

    let mut legacy = encoded;
    legacy.as_object_mut().unwrap().remove("collection_binding");
    let legacy: FrozenProcessingJob = serde_json::from_value(legacy).unwrap();
    assert!(matches!(
        legacy.collection_binding,
        CollectionBinding::LegacyUnassigned
    ));
    assert!(legacy.validate().is_err());
}

#[test]
fn provider_and_snapshot_queries_are_source_bound_and_path_free() {
    let collection_id = CollectionId::new();
    let snapshot = SnapshotQuery {
        collection_id,
        snapshot_id: SnapshotId::new(),
    };
    let request = ProviderRequest {
        collection_id,
        provider: ModuleId("org.example.catalog".into()),
        operation: "org.example.catalog.lookup".parse().unwrap(),
        source_fingerprint: None,
        query: RawJson::from_json(r#"{"disc":"abc"}"#).unwrap(),
        maximum_response_bytes: 1024,
    };
    assert_eq!(snapshot.collection_id, collection_id);
    assert!(request.validate().is_ok());
    assert!(!serde_json::to_string(&request).unwrap().contains("path"));

    let response = ProviderResponse {
        media_type: "application/json".into(),
        bytes: b"{}".to_vec(),
        digest: Digest::sha256(b"{}"),
        parsed_observation: RawJson::from_json("{}").unwrap(),
        limitations: vec![],
    };
    assert!(response.validate(2).is_ok());
    assert!(response.validate(1).is_err());

    let oversized_parsed = ProviderResponse {
        parsed_observation: RawJson::from_json(format!(
            r#"{{"value":"{}"}}"#,
            "x".repeat(1024 * 1024)
        ))
        .unwrap(),
        ..response.clone()
    };
    assert!(oversized_parsed.validate(2).is_err());

    let unsafe_limitation = ProviderResponse {
        limitations: vec!["unsafe\0text".into()],
        ..response
    };
    assert!(unsafe_limitation.validate(2).is_err());
}

#[test]
fn provider_acquisition_does_not_invent_a_file_source() {
    use collector_app::{FrozenAcquisitionJob, FrozenJobInput};
    let acquisition = FrozenJobInput::Acquisition(FrozenAcquisitionJob {
        collection_binding: games_binding(),
        request: ProviderRequest {
            collection_id: CollectionId::new(),
            provider: ModuleId("org.example.catalog".into()),
            operation: "org.example.catalog.fetch".parse().unwrap(),
            source_fingerprint: None,
            query: RawJson::from_json(r#"{"snapshot":"edition-1"}"#).unwrap(),
            maximum_response_bytes: 512 * 1024 * 1024,
        },
        snapshot_id: SnapshotId::new(),
        created_at: Utc.with_ymd_and_hms(2026, 9, 11, 1, 2, 3).unwrap(),
    });
    acquisition.validate().unwrap();
    let encoded = serde_json::to_string(&acquisition).unwrap();
    let decoded: FrozenJobInput = serde_json::from_str(&encoded).unwrap();
    assert_eq!(serde_json::to_string(&decoded).unwrap(), encoded);
    let response = ProviderResponse {
        media_type: "application/json".into(),
        bytes: b"{}".to_vec(),
        digest: Digest::sha256(b"{}"),
        parsed_observation: RawJson::from_json("{}").unwrap(),
        limitations: vec![],
    };
    assert!(
        response.validate(512 * 1024 * 1024).is_err(),
        "catalog-sized buffering must use the stream port"
    );
}
