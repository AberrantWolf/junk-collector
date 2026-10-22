use collector_app::{JobPort, JobSettings, JobSpec};
use collector_core::*;
use collector_store::{Clock, Store};
use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};

struct TestClock(AtomicI64);
impl Clock for TestClock {
    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::from_timestamp(self.0.load(Ordering::SeqCst), 0).unwrap()
    }
    fn monotonic_millis(&self) -> u64 {
        self.0.load(Ordering::SeqCst) as u64 * 1000
    }
}
fn enqueue(store: &Store, kind: &str, user: UserId, import: Option<ImportId>) -> JobId {
    let id = JobId::new();
    store
        .enqueue_job(JobSpec {
            id,
            user_id: user,
            collection_id: None,
            item_id: None,
            kind: kind.into(),
            phase: "queued".into(),
            total_units: 1,
            request: if kind == "preservation_import" {
                serde_json::json!({"inbox_source_locator":"fixture-only"})
            } else {
                serde_json::json!({"import_id":import})
            },
            frozen_input: None,
        })
        .unwrap();
    id
}
fn finish(store: &Store, job: &collector_app::JobClaim) {
    store
        .complete_job(job.id, &job.claim_token, serde_json::json!({}))
        .unwrap();
}

#[test]
fn copy_caps_and_reserved_processing_work_for_every_supported_budget() {
    for processing in 1..=8 {
        for copies in 1..=processing {
            let root = tempfile::tempdir().unwrap();
            let store = Store::open(root.path().join("archive"), root.path().join("state"))
                .unwrap()
                .with_job_settings(JobSettings {
                    processing_workers: processing,
                    copy_workers: copies,
                })
                .unwrap();
            let user = UserId::new();
            for index in 0..processing + 1 {
                enqueue(
                    &store,
                    if index % 2 == 0 {
                        "import_staging"
                    } else {
                        "preservation_import"
                    },
                    user,
                    None,
                );
            }
            let mut active = Vec::new();
            for _ in 0..copies {
                active.push(store.claim_job("worker", 30).unwrap().unwrap());
            }
            assert!(
                store.claim_job("worker", 30).unwrap().is_none(),
                "copy cap {copies}/{processing}"
            );
            let work = enqueue(&store, "test.processing", user, None);
            if copies == processing {
                finish(&store, &active.pop().unwrap());
            }
            assert_eq!(store.claim_job("worker", 30).unwrap().unwrap().id, work);
        }
    }
}

#[test]
fn single_worker_finishes_copied_disc_before_next_copy_and_does_not_pin_blocked_work() {
    let root = tempfile::tempdir().unwrap();
    let store = Store::open(root.path().join("archive"), root.path().join("state"))
        .unwrap()
        .with_job_settings(JobSettings {
            processing_workers: 1,
            copy_workers: 1,
        })
        .unwrap();
    let user = UserId::new();
    let a = ImportId::new();
    let b = ImportId::new();
    let copy_a = enqueue(&store, "import_staging", user, Some(a));
    let copy = store.claim_job("worker", 30).unwrap().unwrap();
    assert_eq!(copy.id, copy_a);
    finish(&store, &copy);
    let copy_b = enqueue(&store, "import_staging", user, Some(b));
    let ordinary = enqueue(&store, "test.processing", user, None);
    let analyze_b = enqueue(&store, "import_analysis", user, Some(b));
    let copy_c = enqueue(&store, "import_staging", user, Some(ImportId::new()));
    let analyze_a = enqueue(&store, "import_analysis", user, Some(a));
    let analyze = store.claim_job("worker", 30).unwrap().unwrap();
    assert_eq!(analyze.id, analyze_a);
    finish(&store, &analyze);
    let publish_a = enqueue(&store, "import_publication", user, Some(a));
    let publication = store.claim_job("worker", 30).unwrap().unwrap();
    assert_eq!(publication.id, publish_a);
    finish(&store, &publication);
    let next = store.claim_job("worker", 30).unwrap().unwrap();
    assert_eq!(next.id, copy_b);
    finish(&store, &next);
    let regular = store.claim_job("worker", 30).unwrap().unwrap();
    assert_eq!(
        regular.id, ordinary,
        "finishing discs cannot starve older ordinary processing"
    );
    finish(&store, &regular);
    let analyze = store.claim_job("worker", 30).unwrap().unwrap();
    assert_eq!(analyze.id, analyze_b);
    finish(&store, &analyze);
    let publish_b = enqueue(&store, "import_publication", user, Some(b));
    let published = store.claim_job("worker", 30).unwrap().unwrap();
    assert_eq!(
        published.id, publish_b,
        "finish ready originals before starting the next copy, even after a regular task's turn"
    );
    finish(&store, &published);
    let next = store.claim_job("worker", 30).unwrap().unwrap();
    assert_eq!(next.id, copy_c);
    store
        .fail_job(next.id, &next.claim_token, "fixture failure")
        .unwrap();
    let copy_d = enqueue(&store, "import_staging", user, Some(ImportId::new()));
    assert_eq!(store.claim_job("worker", 30).unwrap().unwrap().id, copy_d);
}

#[test]
fn restart_with_lower_limit_preserves_jobs_and_fences_old_claims() {
    let root = tempfile::tempdir().unwrap();
    let clock = Arc::new(TestClock(AtomicI64::new(1_800_000_000)));
    let store = Store::open_with_clock(
        root.path().join("archive"),
        root.path().join("state"),
        clock.clone(),
    )
    .unwrap()
    .with_job_settings(JobSettings {
        processing_workers: 8,
        copy_workers: 1,
    })
    .unwrap();
    let user = UserId::new();
    for _ in 0..8 {
        enqueue(&store, "test.processing", user, None);
    }
    let old = (0..8)
        .map(|_| store.claim_job("old-process", 30).unwrap().unwrap())
        .collect::<Vec<_>>();
    drop(store);
    clock.0.fetch_add(31, Ordering::SeqCst);
    let store = Store::open_with_clock(
        root.path().join("archive"),
        root.path().join("state"),
        clock,
    )
    .unwrap()
    .with_job_settings(JobSettings {
        processing_workers: 1,
        copy_workers: 1,
    })
    .unwrap();
    let mut recovered = std::collections::BTreeSet::new();
    while let Some(job) = store.claim_job("new-process", 30).unwrap() {
        assert!(store.claim_job("new-process", 30).unwrap().is_none());
        let previous = old.iter().find(|old| old.id == job.id).unwrap();
        assert_eq!(previous.created_at, job.created_at);
        assert!(
            store
                .complete_job(job.id, &previous.claim_token, serde_json::json!({}))
                .is_err()
        );
        recovered.insert(job.id);
        finish(&store, &job);
    }
    assert_eq!(recovered.len(), 8);
}

#[test]
fn neither_backlog_monopolizes_any_supported_worker_count() {
    for count in 1..=8 {
        let root = tempfile::tempdir().unwrap();
        let store = Store::open(root.path().join("archive"), root.path().join("state"))
            .unwrap()
            .with_job_settings(JobSettings {
                processing_workers: count,
                copy_workers: 1,
            })
            .unwrap();
        let user = UserId::new();
        for _ in 0..40 {
            enqueue(&store, "import_staging", user, None);
            enqueue(&store, "test.processing", user, None);
        }
        let mut all = Vec::new();
        for _ in 0..3 {
            let mut round = Vec::new();
            while let Some(job) = store.claim_job("worker", 30).unwrap() {
                round.push(job);
            }
            assert_eq!(round.len(), usize::from(count));
            if count > 1 {
                assert_eq!(
                    round
                        .iter()
                        .filter(|job| job.kind == "import_staging")
                        .count(),
                    1
                );
                assert!(round.iter().any(|job| job.kind == "test.processing"));
            }
            for job in round {
                all.push(job.kind.clone());
                finish(&store, &job);
            }
        }
        assert!(all.iter().any(|kind| kind == "import_staging"));
        assert!(all.iter().any(|kind| kind == "test.processing"));
    }
}

#[test]
fn metadata_overtakes_encoding_but_cannot_monopolize_processing() {
    for (count, metadata_first) in (1..=8).flat_map(|count| [(count, false), (count, true)]) {
        let root = tempfile::tempdir().unwrap();
        let clock = Arc::new(TestClock(AtomicI64::new(1_800_000_000)));
        let store = Store::open_with_clock(
            root.path().join("archive"),
            root.path().join("state"),
            clock.clone(),
        )
        .unwrap()
        .with_job_settings(JobSettings {
            processing_workers: count,
            copy_workers: 1,
        })
        .unwrap();
        let user = UserId::new();
        let import = ImportId::new();
        enqueue(&store, "import_staging", user, Some(import));
        let copied = store.claim_job("worker", 30).unwrap().unwrap();
        finish(&store, &copied);
        for metadata in [metadata_first, !metadata_first] {
            for index in 0..24 {
                enqueue(
                    &store,
                    if !metadata {
                        "music_derivative"
                    } else if index % 2 == 0 {
                        "metadata_resolution"
                    } else {
                        "metadata_existing"
                    },
                    user,
                    None,
                );
            }
            clock.0.fetch_add(1, Ordering::SeqCst);
        }
        // An affinity continuation and metadata must not alternate preferences
        // indefinitely while older encoding waits.
        enqueue(&store, "import_analysis", user, Some(import));
        let mut order = Vec::new();
        for _ in 0..3.max(9 / count) {
            let active = (0..count)
                .map(|_| store.claim_job("worker", 30).unwrap().unwrap())
                .collect::<Vec<_>>();
            assert!(store.claim_job("worker", 30).unwrap().is_none());
            for job in active {
                if order.len() == 2 {
                    assert_eq!(
                        job.kind, "music_derivative",
                        "combined preferences starved ordinary processing"
                    );
                }
                order.push(job.kind.starts_with("metadata_"));
                finish(&store, &job);
            }
        }
        assert!(
            order[0] && order[1],
            "metadata must overtake older encoding at {count} workers"
        );
        for window in order.windows(3) {
            assert!(
                window.contains(&false),
                "metadata monopolized processing at {count} workers"
            );
            assert!(
                window.contains(&true),
                "encoding delayed metadata at {count} workers"
            );
        }
    }
}
