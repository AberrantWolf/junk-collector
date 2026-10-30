use collector_app::*;
use collector_core::*;
use collector_modules::Registry;
use collector_store::Store;
use std::sync::{Arc, Mutex};
struct Mail {
    profile: MailProfile,
    batch: Mutex<MailBatch>,
}
impl MailProviderPort for Mail {
    fn profiles(&self) -> Vec<MailProfile> {
        vec![self.profile.clone()]
    }
    fn fetch(&self, _: &str, _: &str) -> Result<MailBatch> {
        Ok(self.batch.lock().unwrap().clone())
    }
}
fn context() -> CommandContext {
    CommandContext::new(uuid_key()).unwrap()
}
fn uuid_key() -> String {
    ItemId::new().to_string()
}
fn purchase(key: &str, status: PurchaseStatus) -> PurchaseOrder {
    PurchaseOrder {
        id: key.repeat(64),
        retailer: "joshin".into(),
        order_reference: "J12345".into(),
        title: "ゼルダの伝説 Switch 2".into(),
        platform: "Nintendo Switch 2".into(),
        edition: "Collector".into(),
        format: "physical".into(),
        status,
        observed_at: "2026-10-30T10:00:00Z".parse().unwrap(),
        intentional_duplicate: false,
        source_keys: vec![key.repeat(64)],
    }
}
#[test]
fn mail_review_is_private_durable_idempotent_and_does_not_invent_possession() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    let state = root.path().join("state");
    let store = Arc::new(Store::open(&archive, &state).unwrap());
    let user = store
        .create_user("owner", "a long test password", true)
        .unwrap();
    let auth = store
        .login("owner", "a long test password", "setup")
        .unwrap()
        .1;
    let actor = auth.principal.clone();
    let base = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
    let collection = base
        .create_collection(
            &auth,
            CreateCollectionCommand {
                name: "Games".into(),
                module: ModuleId("org.junk.games".into()),
            },
            &context(),
        )
        .unwrap();
    let provider = Arc::new(Mail {
        profile: MailProfile {
            id: "proton-owner".into(),
            owner: user,
            label: "Private Proton".into(),
            provider: "proton_bridge".into(),
        },
        batch: Mutex::new(MailBatch {
            cursor: "first".into(),
            purchases: vec![purchase("a", PurchaseStatus::Preordered)],
        }),
    });
    let service = base.with_mail_intake(store.clone(), provider.clone());
    let configure = ConfigureMail {
        collection_id: collection.id,
        enabled: true,
        expected_revision: Revision(0),
    };
    service
        .configure_mail(&actor, "proton-owner", configure, &context())
        .unwrap();
    service.sync_purchase_mail().unwrap();
    service.sync_purchase_mail().unwrap();
    let inbox = service
        .purchase_inbox(&actor, collection.id, "", PurchaseInboxView::All)
        .unwrap();
    assert_eq!(inbox.items.len(), 1);
    let s = &inbox.items[0];
    let id = ItemId::new();
    let command = ReviewPurchase {
        item_id: id,
        expected_revision: Revision(0),
        title: "The Legend of Zelda".into(),
        existing_order_id: None,
        intentional_duplicate: false,
        purchase: s.purchase.clone(),
    };
    let receipt = context();
    let saved = service
        .review_purchase(&actor, &s.id, command.clone(), &receipt)
        .unwrap();
    assert!(saved.specimens.is_empty());
    assert!(saved.captures.is_empty());
    assert_eq!(saved.purchase_orders.len(), 1);
    let replay = service
        .review_purchase(&actor, &s.id, command, &receipt)
        .unwrap();
    assert_eq!(saved.revision, replay.revision);
    // Ordinary entry editing preserves accepted order facts.
    let saved = service
        .save_item(
            &actor,
            id,
            SaveItemCommand::from(saved.clone()),
            saved.revision,
            &context(),
        )
        .unwrap();
    assert_eq!(saved.purchase_orders.len(), 1);

    assert_eq!(
        service
            .purchase_inbox(&actor, collection.id, "", PurchaseInboxView::All)
            .unwrap()
            .items[0]
            .accepted_item,
        Some(id)
    );
    let other = store
        .create_user("other", "another long test password", false)
        .unwrap();
    let outsider = Principal {
        user_id: other,
        administrator: false,
        device_id: None,
    };
    assert!(service.mail_profiles(&outsider).unwrap().is_empty());
    assert!(matches!(
        service.dismiss_purchase(&outsider, &s.id, &context()),
        Err(Error::NotFound)
    ));
    assert!(
        service
            .purchase_inbox(&outsider, collection.id, "", PurchaseInboxView::All)
            .is_err()
    );
    let mut viewer = saved.clone();
    viewer.redact_for(Role::Viewer);
    assert!(viewer.purchase_orders.is_empty());
    let mut update = purchase("b", PurchaseStatus::Cancelled);
    update.observed_at = "2026-10-31T10:00:00Z".parse().unwrap();
    *provider.batch.lock().unwrap() = MailBatch {
        cursor: "second".into(),
        purchases: vec![update.clone()],
    };
    service.sync_purchase_mail().unwrap();
    let cancel = service
        .purchase_inbox(&actor, collection.id, "", PurchaseInboxView::All)
        .unwrap()
        .items
        .into_iter()
        .find(|v| v.id == "b".repeat(64))
        .unwrap();
    assert_eq!(cancel.matches, vec![id]);
    let cancelled = service
        .review_purchase(
            &actor,
            &cancel.id,
            ReviewPurchase {
                item_id: id,
                expected_revision: saved.revision,
                title: "Do not overwrite manual title".into(),
                existing_order_id: Some(saved.purchase_orders[0].id.clone()),
                intentional_duplicate: true,
                purchase: update,
            },
            &context(),
        )
        .unwrap();
    assert_eq!(cancelled.title, "The Legend of Zelda");
    assert_eq!(cancelled.purchase_orders.len(), 1);
    assert_eq!(
        cancelled.purchase_orders[0].status,
        PurchaseStatus::Cancelled
    );
    assert_eq!(cancelled.purchase_orders[0].source_keys.len(), 2);
    // Invalid/untrusted adapter output is visible and cannot advance history.
    let mut invalid = purchase("c", PurchaseStatus::Ordered);
    invalid.title = "invalid\0title".into();
    *provider.batch.lock().unwrap() = MailBatch {
        cursor: "must-not-commit".into(),
        purchases: vec![invalid],
    };
    service.sync_purchase_mail().unwrap();
    let failed = service.mail_connections(&actor).unwrap().remove(0);
    assert_eq!(failed.cursor, "second");
    assert!(failed.problem.is_some());
    // A paused connection fences a stale batch and keeps the last committed cursor.
    let current = service.mail_connections(&actor).unwrap().remove(0);
    service
        .configure_mail(
            &actor,
            "proton-owner",
            ConfigureMail {
                collection_id: collection.id,
                enabled: false,
                expected_revision: current.revision,
            },
            &context(),
        )
        .unwrap();
    assert!(matches!(
        store.commit_mail_batch(
            &current,
            MailBatch {
                cursor: "stale".into(),
                purchases: vec![]
            }
        ),
        Err(Error::Conflict)
    ));
    drop(service);
    drop(store);
    let reopened = Arc::new(Store::open(&archive, &state).unwrap());
    assert_eq!(
        Catalog::item(&*reopened, id).unwrap().purchase_orders[0].status,
        PurchaseStatus::Cancelled
    );
    drop(reopened);
    let fresh = Store::open(&archive, root.path().join("archive-only-state")).unwrap();
    assert_eq!(
        Catalog::item(&fresh, id).unwrap().purchase_orders[0]
            .source_keys
            .len(),
        2
    );
}

struct FailAt(collector_store::PublicationFaultPoint);
impl collector_store::PublicationFaultInjector for FailAt {
    fn check(&self, point: collector_store::PublicationFaultPoint) -> Result<()> {
        if point == self.0 {
            Err(Error::Unavailable(
                "controlled email review interruption".into(),
            ))
        } else {
            Ok(())
        }
    }
}
#[test]
fn interrupted_review_recovers_the_order_and_its_command_as_one_effect() {
    use collector_store::PublicationFaultPoint as Point;
    for point in [Point::BytesStaged, Point::RevisionCommitted] {
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join("archive");
        let state = root.path().join("state");
        let store = Arc::new(Store::open(&archive, &state).unwrap());
        let user = store
            .create_user("owner", "a long test password", true)
            .unwrap();
        let auth = store
            .login("owner", "a long test password", "setup")
            .unwrap()
            .1;
        let actor = auth.principal.clone();
        let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
        let collection = service
            .create_collection(
                &auth,
                CreateCollectionCommand {
                    name: "Games".into(),
                    module: ModuleId("org.junk.games".into()),
                },
                &context(),
            )
            .unwrap();
        let provider = Arc::new(Mail {
            profile: MailProfile {
                id: "proton-owner".into(),
                owner: user,
                label: "Private".into(),
                provider: "proton_bridge".into(),
            },
            batch: Mutex::new(MailBatch {
                cursor: "complete".into(),
                purchases: vec![purchase("a", PurchaseStatus::Preordered)],
            }),
        });
        let service = service.with_mail_intake(store.clone(), provider.clone());
        service
            .configure_mail(
                &actor,
                "proton-owner",
                ConfigureMail {
                    collection_id: collection.id,
                    enabled: true,
                    expected_revision: Revision(0),
                },
                &context(),
            )
            .unwrap();
        service.sync_purchase_mail().unwrap();
        drop(service);
        drop(store);
        let broken = Arc::new(
            Store::open_with_publication_fault(&archive, &state, Arc::new(FailAt(point))).unwrap(),
        );
        let service = CollectionService::new(broken.clone(), Arc::new(Registry::builtins()))
            .with_mail_intake(broken.clone(), provider.clone());
        let suggestion = service
            .purchase_inbox(&actor, collection.id, "", PurchaseInboxView::All)
            .unwrap()
            .items
            .remove(0);
        let command = ReviewPurchase {
            item_id: ItemId::new(),
            expected_revision: Revision(0),
            title: "Reviewed game".into(),
            existing_order_id: None,
            intentional_duplicate: false,
            purchase: suggestion.purchase.clone(),
        };
        let receipt = context();
        assert!(
            service
                .review_purchase(&actor, &suggestion.id, command.clone(), &receipt)
                .is_err()
        );
        drop(service);
        drop(broken);
        let recovered = Arc::new(Store::open(&archive, &state).unwrap());
        let service = CollectionService::new(recovered.clone(), Arc::new(Registry::builtins()))
            .with_mail_intake(recovered.clone(), provider);
        let committed = point == Point::RevisionCommitted;
        assert_eq!(
            service
                .purchase_inbox(&actor, collection.id, "", PurchaseInboxView::All)
                .unwrap()
                .items[0]
                .accepted_item
                .is_some(),
            committed
        );
        let saved = service
            .review_purchase(&actor, &suggestion.id, command, &receipt)
            .unwrap();
        assert_eq!(saved.purchase_orders.len(), 1);
        assert_eq!(saved.revision, Revision(1));
    }
}

#[test]
fn inbox_filters_before_paging_so_old_reviews_cannot_hide_new_receipts() {
    let root = tempfile::tempdir().unwrap();
    let store =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let user = store
        .create_user("owner", "a long test password", true)
        .unwrap();
    let auth = store
        .login("owner", "a long test password", "setup")
        .unwrap()
        .1;
    let actor = auth.principal.clone();
    let base = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
    let collection = base
        .create_collection(
            &auth,
            CreateCollectionCommand {
                name: "Games".into(),
                module: ModuleId("org.junk.games".into()),
            },
            &context(),
        )
        .unwrap();
    let purchases = (0..75)
        .map(|i| {
            let mut p = purchase("a", PurchaseStatus::Ordered);
            p.id = format!("{i:064x}");
            p.source_keys = vec![p.id.clone()];
            p
        })
        .collect();
    let provider = Arc::new(Mail {
        profile: MailProfile {
            id: "proton".into(),
            owner: user,
            label: "Private".into(),
            provider: "proton_bridge".into(),
        },
        batch: Mutex::new(MailBatch {
            cursor: "done".into(),
            purchases,
        }),
    });
    let service = base.with_mail_intake(store.clone(), provider);
    service
        .configure_mail(
            &actor,
            "proton",
            ConfigureMail {
                collection_id: collection.id,
                enabled: true,
                expected_revision: Revision(0),
            },
            &context(),
        )
        .unwrap();
    service.sync_purchase_mail().unwrap();
    for i in 0..60 {
        service
            .dismiss_purchase(&actor, &format!("{i:064x}"), &context())
            .unwrap();
    }
    let pending = service
        .purchase_inbox(&actor, collection.id, "", PurchaseInboxView::Pending)
        .unwrap();
    assert_eq!(pending.items.len(), 15);
    assert!(pending.next_cursor.is_none());
    let dismissed = service
        .purchase_inbox(&actor, collection.id, "", PurchaseInboxView::Dismissed)
        .unwrap();
    assert_eq!(dismissed.items.len(), 50);
    let next = service
        .purchase_inbox(
            &actor,
            collection.id,
            dismissed.next_cursor.as_deref().unwrap(),
            PurchaseInboxView::Dismissed,
        )
        .unwrap();
    assert_eq!(next.items.len(), 10);
}
