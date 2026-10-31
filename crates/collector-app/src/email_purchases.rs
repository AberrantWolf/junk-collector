//! Mail is an optional intake adapter. No mailbox credentials cross this boundary.
use crate::*;

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MailProfile {
    pub id: String,
    pub owner: UserId,
    pub label: String,
    pub provider: String,
    #[serde(default)]
    pub configuration_revision: Revision,
    #[serde(default)]
    pub folders: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MailConnection {
    pub profile: MailProfile,
    pub collection_id: CollectionId,
    pub enabled: bool,
    pub revision: Revision,
    /// None uses the provisioned profile folders, preserving existing installations.
    #[serde(default)]
    pub folders: Option<Vec<String>>,
    pub cursor: String,
    pub last_checked: Option<DateTime<Utc>>,
    pub problem: Option<String>,
    #[serde(default)]
    pub failure: Option<MailFailure>,
    #[serde(default)]
    pub progress: Vec<MailFolderProgress>,
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
    pub revision: Revision,
    #[serde(default)]
    pub source: Option<MailSource>,
    #[serde(default)]
    pub issue: Option<MailParseIssue>,
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
    #[serde(default)]
    pub rescan: bool,
    /// Omit to retain the current folders.
    pub folders: Option<Vec<String>>,
    pub collection_id: CollectionId,
    pub enabled: bool,
    pub expected_revision: Revision,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewPurchase {
    #[serde(default)]
    pub suggestion_revision: Revision,
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
    #[serde(default)]
    pub details: Vec<MailReceiptDetails>,
    #[serde(default)]
    pub progress: Vec<MailFolderProgress>,
    #[serde(default)]
    pub failure: Option<MailFailure>,
    pub cursor: String,
    pub purchases: Vec<PurchaseOrder>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MailSource {
    pub folder: String,
    pub uid_validity: u64,
    pub uid: u64,
    pub message_id: String,
    pub subject: String,
    pub date: String,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MailParseIssue {
    MalformedDate,
    UnsupportedContent,
    TooLarge,
    TooManyParts,
    UnrecognizedLayout,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MailReceiptDetails {
    pub key: String,
    pub source: MailSource,
    pub issue: Option<MailParseIssue>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MailFolderProgress {
    pub checked_at: DateTime<Utc>,
    pub folder: String,
    pub uid_validity: u64,
    pub scanned_through: u64,
    pub available_through: u64,
    pub caught_up: bool,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MailFailureKind {
    Authentication,
    Certificate,
    Folder,
    Transport,
    InvalidResponse,
    Adapter,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MailFailure {
    pub kind: MailFailureKind,
    pub folder: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReopenPurchase {
    pub suggestion_revision: Revision,
    pub accepted_item: ItemId,
    pub expected_item_revision: Revision,
}
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RepairBridge {
    pub username: String,
    pub password: String,
    pub certificate_sha256: String,
    pub expected_revision: Revision,
}

/// Installed mail adapters expose bounded batches and opaque durable cursors.
pub trait MailProviderPort: Send + Sync {
    fn profiles(&self) -> Vec<MailProfile>;
    /// Serialize publication with credential replacement, without exposing secrets.
    fn with_configuration(
        &self,
        profile: &str,
        revision: Revision,
        commit: &mut dyn FnMut() -> Result<()>,
    ) -> Result<()>;
    fn fetch(&self, profile: &str, cursor: &str, folders: Option<&[String]>) -> Result<MailBatch>;
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BridgeSyncStatus {
    pub schema: u32,
    pub imap_port: u16,
    pub observed_at: u64,
    pub event_at: Option<u64>,
    pub phase: BridgeSyncPhase,
    pub percent: Option<f64>,
    pub elapsed_seconds: Option<f64>,
    pub remaining_seconds: Option<f64>,
    pub certificate_sha256: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BridgeSyncPhase {
    Starting,
    Unknown,
    Syncing,
    Synced,
    Interrupted,
    Stopped,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MailSetupStatus {
    pub can_connect: bool,
    pub bridge: Option<BridgeSyncStatus>,
    pub problem: Option<String>,
}
/// Write-only secrets: intentionally no Debug implementation or read DTO.
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConnectBridge {
    pub username: String,
    pub password: String,
    pub folders: Vec<String>,
}
pub trait MailSetupPort: Send + Sync {
    fn repair_bridge(
        &self,
        owner: UserId,
        profile: &str,
        command: &RepairBridge,
        request_key: &str,
    ) -> Result<MailProfile>;
    fn setup_status(&self) -> Result<MailSetupStatus>;
    fn connect_bridge(
        &self,
        owner: UserId,
        command: &ConnectBridge,
        request_key: &str,
    ) -> Result<MailProfile>;
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
    fn mail_problem(&self, connection: &MailConnection, failure: &MailFailure) -> Result<()>;
}
impl CollectionService {
    pub fn with_mail_setup(mut self, port: Arc<dyn MailSetupPort>) -> Self {
        self.mail_setup = Some(port);
        self
    }
    pub fn mail_setup_status(&self, actor: &Principal) -> Result<MailSetupStatus> {
        self.mail_enabled_user(actor)?;
        self.require_administrator(actor)?;
        self.mail_setup.as_ref().map_or_else(
            || {
                Ok(MailSetupStatus {
                    can_connect: false,
                    bridge: None,
                    problem: Some("Bridge setup is not installed on this server.".into()),
                })
            },
            |p| p.setup_status(),
        )
    }
    pub fn connect_mail_bridge(
        &self,
        actor: &Principal,
        command: ConnectBridge,
        context: &CommandContext,
    ) -> Result<MailProfile> {
        self.mail_enabled_user(actor)?;
        self.require_administrator(actor)?;
        if command.username.is_empty()
            || command.username.len() > 320
            || command.username.chars().any(char::is_control)
            || command.password.is_empty()
            || command.password.len() > 1024
            || command.password.chars().any(char::is_control)
            || validate_mail_folders(&command.folders).is_err()
        {
            return Err(Error::Invalid(
                "Enter local Bridge IMAP credentials and 1–16 valid folder names.".into(),
            ));
        }
        self.mail_setup
            .as_ref()
            .ok_or_else(|| {
                Error::Unavailable("Bridge setup is not installed on this server.".into())
            })?
            .connect_bridge(actor.user_id, &command, context.key())
    }
    pub fn repair_mail_bridge(
        &self,
        actor: &Principal,
        profile: &str,
        command: RepairBridge,
        context: &CommandContext,
    ) -> Result<MailProfile> {
        self.mail_enabled_user(actor)?;
        self.require_administrator(actor)?;
        if command.username.is_empty()
            || command.username.len() > 320
            || command.username.chars().any(char::is_control)
            || command.password.is_empty()
            || command.password.len() > 1024
            || command.password.chars().any(char::is_control)
            || command.certificate_sha256.len() != 64
            || !command
                .certificate_sha256
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
        {
            return Err(Error::Invalid(
                "Enter local Bridge credentials and confirm its current certificate fingerprint."
                    .into(),
            ));
        }
        self.mail_setup
            .as_ref()
            .ok_or_else(|| Error::Unavailable("Bridge setup unavailable".into()))?
            .repair_bridge(actor.user_id, profile, &command, context.key())
    }
    pub fn reopen_purchase(
        &self,
        actor: &Principal,
        id: &str,
        command: ReopenPurchase,
        context: &CommandContext,
    ) -> Result<Item> {
        let suggestion = self.owned_purchase(actor, id)?;
        self.content_write_eligibility(actor, suggestion.collection_id)?;
        self.execute_command(actor, context, "mail.reopen", &(id, &command), |effects| {
            if suggestion.revision != command.suggestion_revision
                || suggestion.accepted_item != Some(command.accepted_item)
            {
                return Err(Error::Conflict);
            }
            let mut item = self.reader.item(command.accepted_item)?;
            if item.deleted
                || item.collection_id != suggestion.collection_id
                || item.revision != command.expected_item_revision
            {
                return Err(Error::Conflict);
            }
            let index = item
                .purchase_orders
                .iter()
                .position(|o| o.source_keys.contains(&suggestion.id))
                .ok_or(Error::Conflict)?;
            item.purchase_orders.remove(index);
            item.validate()?;
            effects.reopen_purchase_suggestion(&suggestion, item, command.expected_item_revision)
        })
    }
    pub fn restore_purchase(
        &self,
        actor: &Principal,
        id: &str,
        expected: Revision,
        context: &CommandContext,
    ) -> Result<()> {
        let suggestion = self.owned_purchase(actor, id)?;
        self.content_write_eligibility(actor, suggestion.collection_id)?;
        self.execute_command(actor, context, "mail.restore", &(id, expected), |effects| {
            if suggestion.revision != expected
                || !suggestion.dismissed
                || suggestion.accepted_item.is_some()
            {
                return Err(Error::Conflict);
            }
            effects.restore_purchase_suggestion(&suggestion)
        })
    }
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
        if command.enabled || command.folders.is_some() || command.rescan {
            self.content_write_eligibility(actor, command.collection_id)?;
        }
        if let Some(folders) = &command.folders {
            validate_mail_folders(folders)?;
        }
        let (inbox, _) = self.mail_ports()?;
        let current = inbox.mail_connection(&profile.id)?;
        if current.as_ref().is_some_and(|c| {
            c.profile.owner != actor.user_id || c.collection_id != command.collection_id
        }) {
            return Err(Error::Conflict);
        }
        let connection = MailConnection {
            folders: command
                .folders
                .clone()
                .or_else(|| current.as_ref().and_then(|c| c.folders.clone())),
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
            failure: None,
            progress: current
                .as_ref()
                .map(|c| c.progress.clone())
                .unwrap_or_default(),
        };
        self.execute_command(
            actor,
            context,
            "mail.configure",
            &(&connection.profile.id, &command),
            |effects| effects.save_mail_connection(&connection, command.rescan),
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
        expected: Revision,
        context: &CommandContext,
    ) -> Result<()> {
        let suggestion = self.owned_purchase(actor, id)?;
        self.execute_command(actor, context, "mail.dismiss", &(id, expected), |effects| {
            if suggestion.accepted_item.is_some()
                || suggestion.dismissed
                || suggestion.revision != expected
            {
                return Err(Error::Conflict);
            }
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
            if suggestion.revision != command.suggestion_revision || suggestion.dismissed || suggestion.accepted_item.is_some() { return Err(Error::Conflict); }
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
                if order.retailer!=purchase.retailer || order.order_reference!=purchase.order_reference { return Err(Error::Conflict); }
                if !order.source_keys.contains(&suggestion.id) { order.source_keys.push(suggestion.id.clone()); }
                if order.observed_at <= purchase.observed_at {
                order.title=purchase.title;
                order.platform=purchase.platform;
                order.edition=purchase.edition;
                order.format=purchase.format;
                order.status=purchase.status;
                order.observed_at=purchase.observed_at;
                order.intentional_duplicate=command.intentional_duplicate;
                }
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
            let Some(mut connection) = inbox.mail_connection(&profile.id)? else {
                continue;
            };
            if !connection.enabled || connection.profile.owner != profile.owner {
                continue;
            }
            connection.profile = profile.clone();
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
            let fetched = provider.fetch(
                &profile.id,
                &connection.cursor,
                connection.folders.as_deref(),
            );
            if self.mail_enabled_user(&actor).is_err() {
                continue;
            }
            let publish_failure = |failure: &MailFailure| -> Result<()> {
                match provider.with_configuration(
                    &profile.id,
                    profile.configuration_revision,
                    &mut || inbox.mail_problem(&connection, failure),
                ) {
                    Ok(()) | Err(Error::Conflict) => Ok(()),
                    Err(e) => Err(e),
                }
            };
            match fetched {
                Ok(batch) => {
                    if let Some(failure) = &batch.failure {
                        let sanitized = if failure
                            .folder
                            .as_ref()
                            .is_none_or(|f| f.len() <= 256 && !f.chars().any(char::is_control))
                        {
                            failure.clone()
                        } else {
                            MailFailure {
                                kind: MailFailureKind::InvalidResponse,
                                folder: None,
                            }
                        };
                        publish_failure(&sanitized)?;
                        continue;
                    }
                    let valid = batch.purchases.len() <= 100
                        && batch.cursor.len() <= 65536
                        && batch.details.len() <= 100
                        && batch.progress.len() <= 16
                        && batch.details.iter().all(|d| {
                            batch.purchases.iter().any(|p| p.id == d.key)
                                && !d.source.folder.is_empty()
                                && d.source.folder.len() <= 256
                                && d.source.uid > 0
                                && batch.progress.iter().any(|p| {
                                    p.folder == d.source.folder
                                        && p.uid_validity == d.source.uid_validity
                                        && d.source.uid <= p.scanned_through
                                })
                                && d.source.message_id.len() <= 512
                                && d.source.subject.len() <= 512
                                && d.source.date.len() <= 128
                                && [
                                    &d.source.folder,
                                    &d.source.message_id,
                                    &d.source.subject,
                                    &d.source.date,
                                ]
                                .iter()
                                .all(|v| !v.chars().any(char::is_control))
                        })
                        && batch.progress.iter().all(|p| {
                            connection
                                .folders
                                .as_ref()
                                .unwrap_or(&profile.folders)
                                .contains(&p.folder)
                                && p.uid_validity > 0
                                && !p.folder.is_empty()
                                && p.folder.len() <= 256
                                && !p.folder.chars().any(char::is_control)
                                && p.scanned_through <= p.available_through
                                && p.caught_up == (p.scanned_through == p.available_through)
                        })
                        && batch.purchases.iter().all(|purchase| {
                            purchase.validate().is_ok()
                                && purchase.source_keys.len() == 1
                                && purchase.id == purchase.source_keys[0]
                        });
                    if !valid {
                        publish_failure(&MailFailure {
                            kind: MailFailureKind::InvalidResponse,
                            folder: None,
                        })?;
                        continue;
                    }
                    // Check authorization again after the blocking fetch.
                    self.content_write_eligibility(&actor, connection.collection_id)?;
                    let mut pending = Some(batch);
                    match provider.with_configuration(
                        &profile.id,
                        profile.configuration_revision,
                        &mut || {
                            inbox.commit_mail_batch(
                                &connection,
                                pending.take().ok_or(Error::Conflict)?,
                            )
                        },
                    ) {
                        Ok(()) | Err(Error::Conflict) | Err(Error::Forbidden) => {}
                        Err(e) => return Err(e),
                    }
                }
                Err(_) => publish_failure(&MailFailure {
                    kind: MailFailureKind::Adapter,
                    folder: None,
                })?, // Never retain adapter errors containing credentials/mail.
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

fn validate_mail_folders(folders: &[String]) -> Result<()> {
    if folders.is_empty()
        || folders.len() > 16
        || folders.iter().any(|f| {
            f.is_empty() || f.len() > 256 || f.trim().is_empty() || f.chars().any(char::is_control)
        })
    {
        return Err(Error::Invalid("Enter 1–16 exact IMAP folder names, each at most 256 bytes and without control characters.".into()));
    }
    Ok(())
}
