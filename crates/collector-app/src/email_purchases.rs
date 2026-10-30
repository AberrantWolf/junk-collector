//! Mail is an optional intake adapter. No mailbox credentials cross this boundary.
use crate::*;

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MailProfile {
    pub id: String,
    pub owner: UserId,
    pub label: String,
    pub provider: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MailConnection {
    pub profile: MailProfile,
    pub collection_id: CollectionId,
    pub enabled: bool,
    pub revision: Revision,
    pub cursor: String,
    pub last_checked: Option<DateTime<Utc>>,
    pub problem: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PurchaseSuggestion {
    pub id: String,
    pub connection_id: String,
    pub owner: UserId,
    pub collection_id: CollectionId,
    pub purchase: PurchaseOrder,
    pub reason: String,
    pub dismissed: bool,
    #[serde(default)]
    pub matches: Vec<ItemId>,
    #[serde(default)]
    pub accepted_item: Option<ItemId>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PurchaseInbox {
    pub items: Vec<PurchaseSuggestion>,
    pub next_cursor: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfigureMail {
    pub collection_id: CollectionId,
    pub enabled: bool,
    pub expected_revision: Revision,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewPurchase {
    pub item_id: ItemId,
    pub expected_revision: Revision,
    /// User-reviewed title for a new collection entry; existing metadata is retained.
    pub title: String,
    /// Set only when explicitly updating an existing order (including cancellations).
    pub existing_order_id: Option<String>,
    pub intentional_duplicate: bool,
    pub purchase: PurchaseOrder,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MailBatch {
    pub cursor: String,
    pub purchases: Vec<PurchaseOrder>,
}
/// Installed mail adapters expose bounded batches and opaque durable cursors.
pub trait MailProviderPort: Send + Sync {
    fn profiles(&self) -> Vec<MailProfile>;
    fn fetch(&self, profile: &str, cursor: &str) -> Result<MailBatch>;
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PurchaseInboxView {
    #[default]
    Pending,
    Accepted,
    Dismissed,
    All,
}
pub trait PurchaseInboxPort: Send + Sync {
    fn mail_connections(&self, owner: UserId) -> Result<Vec<MailConnection>>;
    fn mail_connection(&self, profile: &str) -> Result<Option<MailConnection>>;
    fn purchase_inbox(
        &self,
        owner: UserId,
        collection: CollectionId,
        after: &str,
        view: PurchaseInboxView,
    ) -> Result<PurchaseInbox>;
    fn purchase_suggestion(&self, id: &str) -> Result<PurchaseSuggestion>;
    /// CAS fences disabling/reconfiguring a connection while a fetch is in flight.
    /// Account enablement must also be checked in the transaction that commits the batch.
    fn commit_mail_batch(&self, connection: &MailConnection, batch: MailBatch) -> Result<()>;
    fn mail_problem(&self, connection: &MailConnection) -> Result<()>;
}
impl CollectionService {
    fn mail_enabled_user(&self, actor: &Principal) -> Result<()> {
        security::mail_ingestion(self.administration.account(actor.user_id)?.enabled)
    }

    pub fn with_purchase_inbox(mut self, inbox: Arc<dyn PurchaseInboxPort>) -> Self {
        self.purchase_inbox = Some(inbox);
        self
    }
    pub fn mail_intake_configured(&self) -> bool {
        self.mail_provider.is_some()
    }
    fn mail_inbox_port(&self) -> Result<&dyn PurchaseInboxPort> {
        self.purchase_inbox
            .as_deref()
            .ok_or_else(|| Error::Unavailable("Email intake unavailable".into()))
    }
    pub fn with_mail_intake(
        mut self,
        inbox: Arc<dyn PurchaseInboxPort>,
        provider: Arc<dyn MailProviderPort>,
    ) -> Self {
        self.purchase_inbox = Some(inbox);
        self.mail_provider = Some(provider);
        self
    }
    fn mail_ports(&self) -> Result<(&dyn PurchaseInboxPort, &dyn MailProviderPort)> {
        Ok((
            self.purchase_inbox.as_deref().ok_or_else(|| {
                Error::Unavailable("Email purchase intake is not configured".into())
            })?,
            self.mail_provider.as_deref().ok_or_else(|| {
                Error::Unavailable("Email purchase intake is not configured".into())
            })?,
        ))
    }
    pub fn mail_profiles(&self, actor: &Principal) -> Result<Vec<MailProfile>> {
        self.mail_enabled_user(actor)?;
        Ok(self
            .mail_provider
            .as_ref()
            .map(|p| p.profiles())
            .unwrap_or_default()
            .into_iter()
            .filter(|p| p.owner == actor.user_id)
            .collect())
    }
    pub fn mail_connections(&self, actor: &Principal) -> Result<Vec<MailConnection>> {
        self.mail_enabled_user(actor)?;
        self.mail_inbox_port()?.mail_connections(actor.user_id)
    }
    pub fn configure_mail(
        &self,
        actor: &Principal,
        profile: &str,
        command: ConfigureMail,
        context: &CommandContext,
    ) -> Result<MailConnection> {
        let profile = self
            .mail_profiles(actor)?
            .into_iter()
            .find(|p| p.id == profile)
            .ok_or(Error::NotFound)?;
        if command.enabled {
            self.content_write_eligibility(actor, command.collection_id)?;
        }
        let (inbox, _) = self.mail_ports()?;
        let current = inbox.mail_connection(&profile.id)?;
        if current.as_ref().is_some_and(|c| {
            c.profile.owner != actor.user_id || c.collection_id != command.collection_id
        }) {
            return Err(Error::Conflict);
        }
        let connection = MailConnection {
            profile,
            collection_id: command.collection_id,
            enabled: command.enabled,
            revision: command.expected_revision,
            cursor: current
                .as_ref()
                .map(|c| c.cursor.clone())
                .unwrap_or_default(),
            last_checked: current.as_ref().and_then(|c| c.last_checked),
            problem: None,
        };
        self.execute_command(
            actor,
            context,
            "mail.configure",
            &(&connection.profile.id, &command),
            |effects| effects.save_mail_connection(&connection),
        )
    }
    pub fn purchase_inbox(
        &self,
        actor: &Principal,
        collection: CollectionId,
        after: &str,
        view: PurchaseInboxView,
    ) -> Result<PurchaseInbox> {
        self.authorize(actor, collection, Action::Curate)?;
        if after.len() > 64 || !after.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(Error::Invalid("Invalid inbox cursor".into()));
        }
        self.mail_inbox_port()?
            .purchase_inbox(actor.user_id, collection, after, view)
    }
    fn owned_purchase(&self, actor: &Principal, id: &str) -> Result<PurchaseSuggestion> {
        self.mail_enabled_user(actor)?;
        let suggestion = self.mail_inbox_port()?.purchase_suggestion(id)?;
        if suggestion.owner != actor.user_id {
            return Err(Error::NotFound);
        }
        self.authorize(actor, suggestion.collection_id, Action::Curate)?;
        Ok(suggestion)
    }
    pub fn dismiss_purchase(
        &self,
        actor: &Principal,
        id: &str,
        context: &CommandContext,
    ) -> Result<()> {
        let suggestion = self.owned_purchase(actor, id)?;
        if suggestion.accepted_item.is_some() {
            return Err(Error::Conflict);
        }
        self.execute_command(actor, context, "mail.dismiss", &id, |effects| {
            effects.dismiss_purchase_suggestion(&suggestion)
        })
    }
    pub fn review_purchase(
        &self,
        actor: &Principal,
        id: &str,
        command: ReviewPurchase,
        context: &CommandContext,
    ) -> Result<Item> {
        let suggestion = self.owned_purchase(actor, id)?;
        let eligibility = self.content_write_eligibility(actor, suggestion.collection_id)?;
        command.purchase.validate()?;
        if command.purchase.status == PurchaseStatus::Unknown
            || command.purchase.title.starts_with("Unparsed receipt")
        {
            return Err(Error::Invalid(
                "Review the original receipt and enter its item title and purchase status".into(),
            ));
        }

        if command.purchase.id != suggestion.purchase.id
            || command.purchase.source_keys != suggestion.purchase.source_keys
            || command.purchase.retailer != suggestion.purchase.retailer
            || command.purchase.observed_at != suggestion.purchase.observed_at
        {
            return Err(Error::Invalid("Receipt source cannot be changed".into()));
        }
        // Freeze the caller's entire decision before resolving mutable item state.
        self.execute_command(actor,context,"mail.review",&(id,&command),|effects| {
            if suggestion.dismissed || suggestion.accepted_item.is_some() { return Err(Error::Conflict); }
            let mut item=match self.reader.item(command.item_id) {
                Ok(item)=>{
                    if item.collection_id!=suggestion.collection_id || item.deleted { return Err(Error::Conflict); }
                    self.validate_owned_domain(&eligibility,&item.domain)?;
                    item
                },
                Err(Error::NotFound) if command.expected_revision==Revision(0)=>{
                    if eligibility.owner.0!="org.junk.games" { return Err(Error::Invalid("New email purchase entries currently require a Games collection; link other purchases to an existing entry".into())); }
                    Item {purchase_orders:vec![],lifecycle:ItemLifecycle::default(),id:command.item_id,collection_id:suggestion.collection_id,revision:Revision(0),title:command.title.trim().to_owned(),subtitle:String::new(),identity_link:ItemIdentityLink::None,domain:ModuleData { module: eligibility.owner.clone(), schema_version: eligibility.schema_version, payload: RawJson::from_json("{}")? },identity:IdentityState::Unidentified,translations:vec![],identifiers:vec![],specimens:vec![],wishlist:false,captures:vec![],assets:vec![],metadata:MetadataState::default(),deleted:false,updated_at:Utc::now()}
                },
                Err(e)=>return Err(e),
            };
            let mut purchase=command.purchase.clone();
            purchase.intentional_duplicate=command.intentional_duplicate;
            if let Some(existing)=&command.existing_order_id {
                let order=item.purchase_orders.iter_mut().find(|o|&o.id==existing).ok_or(Error::NotFound)?;
                if order.retailer!=purchase.retailer || order.order_reference!=purchase.order_reference || order.observed_at>purchase.observed_at { return Err(Error::Conflict); }
                if !order.source_keys.contains(&suggestion.id) { order.source_keys.push(suggestion.id.clone()); }
                order.title=purchase.title;
                order.platform=purchase.platform;
                order.edition=purchase.edition;
                order.format=purchase.format;
                order.status=purchase.status;
                order.observed_at=purchase.observed_at;
                order.intentional_duplicate=command.intentional_duplicate;
            } else {
                if item.purchase_orders.iter().any(|o|o.source_keys.contains(&suggestion.id)) { return Err(Error::Conflict); }
                item.purchase_orders.push(purchase);
            }
            item.validate()?;
            effects.accept_purchase_suggestion(&suggestion,item,command.expected_revision)
        })
    }
    /// A single bounded page per installed account. Repeated ticks drain history.
    pub fn sync_purchase_mail(&self) -> Result<()> {
        let (inbox, provider) = self.mail_ports()?;
        for profile in provider.profiles() {
            let Some(connection) = inbox.mail_connection(&profile.id)? else {
                continue;
            };
            if !connection.enabled || connection.profile.owner != profile.owner {
                continue;
            }
            let actor = Principal {
                user_id: profile.owner,
                administrator: false,
                device_id: None,
            };
            if self.mail_enabled_user(&actor).is_err() {
                continue;
            }
            if self
                .content_write_eligibility(&actor, connection.collection_id)
                .is_err()
            {
                continue;
            }
            let fetched = provider.fetch(&profile.id, &connection.cursor);
            if self.mail_enabled_user(&actor).is_err() {
                continue;
            }
            match fetched {
                Ok(batch) => {
                    let valid = batch.purchases.len() <= 100
                        && batch.cursor.len() <= 65536
                        && batch.purchases.iter().all(|purchase| {
                            purchase.validate().is_ok()
                                && purchase.source_keys.len() == 1
                                && purchase.id == purchase.source_keys[0]
                        });
                    if !valid {
                        inbox.mail_problem(&connection)?;
                        continue;
                    }
                    // Check authorization again after the blocking fetch.
                    self.content_write_eligibility(&actor, connection.collection_id)?;
                    match inbox.commit_mail_batch(&connection, batch) {
                        Ok(()) | Err(Error::Conflict) | Err(Error::Forbidden) => {}
                        Err(e) => return Err(e),
                    }
                }
                Err(_) => inbox.mail_problem(&connection)?, // Never retain adapter errors containing credentials/mail.
            }
        }
        Ok(())
    }
}

/// Conservative normalization retains edition/platform words; reviewed links act as aliases.
pub fn normalized_purchase_title(value: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    value
        .nfkc()
        .flat_map(char::to_lowercase)
        .filter(|c| c.is_alphanumeric())
        .collect()
}
pub fn purchase_search_keys(title: &str, orders: &[PurchaseOrder]) -> Vec<String> {
    std::iter::once(title)
        .chain(orders.iter().map(|p| p.title.as_str()))
        .map(normalized_purchase_title)
        .collect()
}
