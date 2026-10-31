use super::*;
use collector_app::{
    MailBatch, MailConnection, MailFailure, MailFailureKind, PurchaseInbox, PurchaseInboxPort,
    PurchaseInboxView, PurchaseSuggestion,
};

pub(crate) fn install(db: &Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS private.mail_connections(id TEXT PRIMARY KEY, owner TEXT NOT NULL, record TEXT NOT NULL);
    CREATE TABLE IF NOT EXISTS private.purchase_suggestions(id TEXT PRIMARY KEY, owner TEXT NOT NULL, collection_id TEXT NOT NULL, record TEXT NOT NULL, dismissed INTEGER NOT NULL DEFAULT 0);
    CREATE INDEX IF NOT EXISTS private.purchase_inbox_owner ON purchase_suggestions(owner,collection_id,id);").map_err(storage)
}
fn connection(db: &Connection, id: &str) -> Result<Option<MailConnection>> {
    let raw: Option<String> = db
        .query_row(
            "SELECT record FROM private.mail_connections WHERE id=?",
            [id],
            |r| r.get(0),
        )
        .optional()
        .map_err(storage)?;
    raw.map(|v| serde_json::from_str(&v).map_err(storage))
        .transpose()
}
fn accepted_item(db: &Connection, key: &str) -> Result<Option<ItemId>> {
    let id:Option<String>=db.query_row("SELECT items.id FROM items,json_each(items.record,'$.purchase_orders') orders,json_each(orders.value,'$.source_keys') sources WHERE sources.value=? LIMIT 1",[key],|r|r.get(0)).optional().map_err(storage)?;
    id.map(|s| s.parse().map_err(storage)).transpose()
}
fn suggestion(db: &Connection, id: &str) -> Result<PurchaseSuggestion> {
    let (raw, dismissed): (String, bool) = db
        .query_row(
            "SELECT record,dismissed FROM private.purchase_suggestions WHERE id=?",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(storage)?
        .ok_or(Error::NotFound)?;
    let mut value: PurchaseSuggestion = serde_json::from_str(&raw).map_err(storage)?;
    value.dismissed = dismissed;
    value.accepted_item = accepted_item(db, id)?;
    Ok(value)
}
impl PurchaseInboxPort for Store {
    fn mail_connections(&self, owner: UserId) -> Result<Vec<MailConnection>> {
        let db = lock(&self.db)?;
        let mut q = db
            .prepare(
                "SELECT record FROM private.mail_connections WHERE owner=? ORDER BY id LIMIT 64",
            )
            .map_err(storage)?;
        q.query_map([owner.to_string()], |r| r.get::<_, String>(0))
            .map_err(storage)?
            .map(|r| serde_json::from_str(&r.map_err(storage)?).map_err(storage))
            .collect()
    }
    fn mail_connection(&self, id: &str) -> Result<Option<MailConnection>> {
        connection(&*lock(&self.db)?, id)
    }
    fn purchase_suggestion(&self, id: &str) -> Result<PurchaseSuggestion> {
        suggestion(&*lock(&self.db)?, id)
    }
    fn purchase_inbox(
        &self,
        owner: UserId,
        collection: CollectionId,
        after: &str,
        view: PurchaseInboxView,
    ) -> Result<PurchaseInbox> {
        let db = lock(&self.db)?;
        let view = match view {
            PurchaseInboxView::Pending => "pending",
            PurchaseInboxView::Accepted => "accepted",
            PurchaseInboxView::Dismissed => "dismissed",
            PurchaseInboxView::All => "all",
        };
        let mut q=db.prepare("WITH accepted AS (SELECT sources.value AS key FROM items,json_each(items.record,'$.purchase_orders') orders,json_each(orders.value,'$.source_keys') sources WHERE items.collection_id=?) SELECT s.id FROM private.purchase_suggestions s WHERE s.owner=? AND s.collection_id=? AND s.id>? AND (?='all' OR (?='pending' AND s.dismissed=0 AND s.id NOT IN (SELECT key FROM accepted)) OR (?='accepted' AND s.id IN (SELECT key FROM accepted)) OR (?='dismissed' AND s.dismissed=1)) ORDER BY s.id LIMIT 51").map_err(storage)?;
        let ids = q
            .query_map(
                params![
                    collection.to_string(),
                    owner.to_string(),
                    collection.to_string(),
                    after,
                    view,
                    view,
                    view,
                    view
                ],
                |r| r.get::<_, String>(0),
            )
            .map_err(storage)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(storage)?;
        let next_cursor = if ids.len() > 50 {
            ids.get(49).cloned()
        } else {
            None
        };
        let mut titles=db.prepare("SELECT id,title,json_extract(record,'$.purchase_orders') FROM items WHERE collection_id=? AND deleted=0 ORDER BY id LIMIT 5000").map_err(storage)?;
        let candidates = titles
            .query_map([collection.to_string()], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            })
            .map_err(storage)?
            .map(|row| {
                let (id, title, orders) = row.map_err(storage)?;
                let orders: Vec<PurchaseOrder> =
                    serde_json::from_str(orders.as_deref().unwrap_or("[]")).map_err(storage)?;
                Ok((
                    id.parse::<ItemId>().map_err(storage)?,
                    collector_app::purchase_search_keys(&title, &orders),
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let mut items = Vec::new();
        for id in ids.iter().take(50) {
            let mut s = suggestion(&db, id)?;
            let key = collector_app::normalized_purchase_title(&s.purchase.title);
            s.matches = candidates
                .iter()
                .filter(|(_, keys)| !key.is_empty() && keys.contains(&key))
                .take(20)
                .map(|(id, _)| *id)
                .collect();
            items.push(s);
        }
        Ok(PurchaseInbox { items, next_cursor })
    }
    fn commit_mail_batch(&self, c: &MailConnection, batch: MailBatch) -> Result<()> {
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage)?;
        let account_enabled: Option<bool> = tx
            .query_row(
                "SELECT enabled FROM private.users WHERE id=?",
                [c.profile.owner.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        collector_app::security::mail_ingestion(account_enabled.unwrap_or(false))?;
        let mut current = connection(&tx, &c.profile.id)?.ok_or(Error::Conflict)?;
        if !current.enabled
            || current.revision != c.revision
            || current.cursor != c.cursor
            || current.profile.owner != c.profile.owner
            || current.collection_id != c.collection_id
        {
            return Err(Error::Conflict);
        }
        for purchase in batch.purchases {
            let detail = batch.details.iter().find(|d| d.key == purchase.id);
            purchase.validate()?;
            let id = purchase.source_keys[0].clone();
            let s=PurchaseSuggestion{id:id.clone(),connection_id:c.profile.id.clone(),owner:c.profile.owner,collection_id:c.collection_id,purchase,reason:"Review the original email, then confirm the item, edition, platform, order reference and status. Email sender names alone do not verify a purchase.".into(),dismissed:false,revision:Revision(0),source:detail.map(|d| d.source.clone()),issue:detail.and_then(|d| d.issue),matches:vec![],accepted_item:None};
            let inserted = tx.execute("INSERT OR IGNORE INTO private.purchase_suggestions(id,owner,collection_id,record) VALUES(?,?,?,?)",params![id,c.profile.owner.to_string(),c.collection_id.to_string(),serde_json::to_string(&s).map_err(storage)?]).map_err(storage)?;
            if inserted == 0 && detail.is_some() {
                let mut existing = suggestion(&tx, &id)?;
                // Source coordinates are current private diagnostics, not an immutable
                // claim or a replacement for the original reviewed receipt content.
                if existing.owner == s.owner
                    && existing.collection_id == s.collection_id
                    && serde_json::to_value(&existing.purchase).map_err(storage)?
                        == serde_json::to_value(&s.purchase).map_err(storage)?
                {
                    existing.source = s.source;
                    existing.issue = s.issue;
                    tx.execute(
                        "UPDATE private.purchase_suggestions SET record=? WHERE id=?",
                        params![serde_json::to_string(&existing).map_err(storage)?, id],
                    )
                    .map_err(storage)?;
                }
            }
        }
        for progress in batch.progress {
            current.progress.retain(|p| p.folder != progress.folder);
            current.progress.push(progress);
        }
        current.profile = c.profile.clone();
        current.cursor = batch.cursor;
        current.last_checked = Some(Utc::now());
        current.problem = None;
        current.failure = None;
        tx.execute(
            "UPDATE private.mail_connections SET record=? WHERE id=?",
            params![
                serde_json::to_string(&current).map_err(storage)?,
                c.profile.id
            ],
        )
        .map_err(storage)?;
        tx.commit().map_err(storage)
    }
    fn mail_problem(&self, c: &MailConnection, failure: &MailFailure) -> Result<()> {
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        if let Some(mut current) = connection(&tx, &c.profile.id)?
            && current.revision == c.revision
            && current.enabled
            && current.cursor == c.cursor
        {
            let message = match failure.kind {
                MailFailureKind::Authentication => {
                    "Bridge rejected the local credentials. Revalidate the connection with its current IMAP password."
                }
                MailFailureKind::Certificate => {
                    "Bridge certificate changed. Confirm its fingerprint and revalidate the connection."
                }
                MailFailureKind::Folder => {
                    "A configured folder is unavailable. Check its exact IMAP name in Edit scanned folders."
                }
                MailFailureKind::Transport => {
                    "Bridge connection interrupted or timed out. The scan will retry from its saved position."
                }
                MailFailureKind::InvalidResponse => {
                    "Bridge returned an incomplete or invalid response. The saved scan position has not advanced."
                }
                MailFailureKind::Adapter => {
                    "Mail adapter could not complete. Check the Bridge process and server Python installation."
                }
            };
            current.profile = c.profile.clone();
            current.problem = Some(message.into());
            current.failure = Some(failure.clone());
            tx.execute(
                "UPDATE private.mail_connections SET record=? WHERE id=?",
                params![
                    serde_json::to_string(&current).map_err(storage)?,
                    c.profile.id
                ],
            )
            .map_err(storage)?;
        }
        tx.commit().map_err(storage)
    }
}
impl Store {
    pub(crate) fn save_mail_connection_record(
        &self,
        c: &MailConnection,
        rescan: bool,
    ) -> Result<MailConnection> {
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        let previous = connection(&tx, &c.profile.id)?;
        if previous.as_ref().map(|v| v.revision).unwrap_or(Revision(0)) != c.revision {
            return Err(Error::Conflict);
        }
        if previous.as_ref().is_some_and(|p| {
            p.profile.owner != c.profile.owner || p.collection_id != c.collection_id
        }) {
            return Err(Error::Conflict);
        }
        let mut result = c.clone();
        if let Some(previous) = previous {
            if rescan {
                result.cursor.clear();
                result.progress.clear();
                result.last_checked = None;
            } else {
                result.cursor = previous.cursor;
                result.last_checked = previous.last_checked;
                result.progress = previous.progress;
                let folders = result.folders.as_ref().unwrap_or(&result.profile.folders);
                result.progress.retain(|p| folders.contains(&p.folder));
            }
        }
        result.revision = Revision(c.revision.0.checked_add(1).ok_or(Error::Conflict)?);
        tx.execute("INSERT INTO private.mail_connections(id,owner,record) VALUES(?,?,?) ON CONFLICT(id) DO UPDATE SET record=excluded.record",params![c.profile.id,c.profile.owner.to_string(),serde_json::to_string(&result).map_err(storage)?]).map_err(storage)?;
        self.record_command(&tx, &result)?;
        tx.commit().map_err(storage)?;
        Ok(result)
    }
    pub(crate) fn set_purchase_dismissed(
        &self,
        s: &PurchaseSuggestion,
        dismissed: bool,
    ) -> Result<()> {
        let _writer = lock(&self.writer)?;
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        let mut current = suggestion(&tx, &s.id)?;
        if current.revision != s.revision
            || current.dismissed == dismissed
            || accepted_item(&tx, &s.id)?.is_some()
        {
            return Err(Error::Conflict);
        }
        current.revision = Revision(current.revision.0.checked_add(1).ok_or(Error::Conflict)?);
        current.dismissed = dismissed;
        tx.execute(
            "UPDATE private.purchase_suggestions SET dismissed=?,record=? WHERE id=? AND owner=?",
            params![
                dismissed,
                serde_json::to_string(&current).map_err(storage)?,
                s.id,
                s.owner.to_string()
            ],
        )
        .map_err(storage)?;
        self.record_command(&tx, &())?;
        tx.commit().map_err(storage)
    }
    pub(crate) fn reopen_purchase_record(
        &self,
        s: &PurchaseSuggestion,
        item: Item,
        expected: Revision,
    ) -> Result<Item> {
        let _writer = lock(&self.writer)?;
        let current_item = self.item(item.id)?;
        if current_item.revision != expected || current_item.collection_id != s.collection_id {
            return Err(Error::Conflict);
        }
        let order = current_item
            .purchase_orders
            .iter()
            .find(|o| o.source_keys.contains(&s.id))
            .ok_or(Error::Conflict)?;
        {
            let mut db = lock(&self.db)?;
            let tx = db.transaction().map_err(storage)?;
            let current = suggestion(&tx, &s.id)?;
            if current.revision != s.revision || current.accepted_item != Some(item.id) {
                return Err(Error::Conflict);
            }
            // Reserve new review generations before publication. An interrupted attempt
            // can require refresh, but cannot allow old review forms to become valid again.
            for key in &order.source_keys {
                match suggestion(&tx, key) {
                    Ok(mut source) => {
                        source.revision =
                            Revision(source.revision.0.checked_add(1).ok_or(Error::Conflict)?);
                        tx.execute(
                            "UPDATE private.purchase_suggestions SET record=? WHERE id=?",
                            params![serde_json::to_string(&source).map_err(storage)?, key],
                        )
                        .map_err(storage)?;
                    }
                    Err(Error::NotFound) => {}
                    Err(e) => return Err(e),
                }
            }
            tx.commit().map_err(storage)?;
        }
        self.publish_under_writer(item, expected, vec![])
    }
    pub(crate) fn accept_purchase_record(
        &self,
        s: &PurchaseSuggestion,
        item: Item,
        expected: Revision,
    ) -> Result<Item> {
        let _writer = lock(&self.writer)?;
        {
            let db = lock(&self.db)?;
            let current = suggestion(&db, &s.id)?;
            if current.revision != s.revision
                || current.dismissed
                || current.accepted_item.is_some()
                || current.owner != s.owner
                || current.collection_id != item.collection_id
            {
                return Err(Error::Conflict);
            }
        }
        // The existing archive journal commits the Item and its exact command receipt.
        // Inbox acceptance is derived from retained source keys, so a crash cannot split them.
        self.publish_under_writer(item, expected, vec![])
    }
}
