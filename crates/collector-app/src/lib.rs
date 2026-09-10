//! Shared use cases. Every adapter and frontend crosses this authorization boundary.
pub mod security;
use chrono::{DateTime, Utc};
use collector_core::*;
use collector_modules::ModulePresentation;
use collector_modules::Registry;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek};
use std::sync::Arc;

/// Transitional combined Wave 1 adapter. Application services consume the
/// narrow ports below; A04.2 removes this compatibility surface after moving
/// the remaining command families behind application policy.
pub trait Catalog: Send + Sync {
    fn collections(&self, user: UserId) -> Result<Vec<Collection>>;
    fn create_collection(&self, user: UserId, collection: Collection) -> Result<Collection>;
    fn role(&self, user: UserId, collection: CollectionId) -> Result<Role>;
    fn grant(&self, collection: CollectionId, user: UserId, role: Role) -> Result<()>;
    fn browse(&self, user: UserId, query: BrowseQuery) -> Result<Page<Item>>;
    fn item(&self, id: ItemId) -> Result<Item>;
    fn save(&self, item: Item, expected: Revision) -> Result<Item>;
    fn work(&self, id: WorkId) -> Result<Work>;
    fn save_work(&self, work: Work, expected: Revision) -> Result<Work>;
    fn edition(&self, id: EditionId) -> Result<Edition>;
    fn save_edition(&self, edition: Edition, expected: Revision) -> Result<Edition>;
    fn capture(&self, id: CaptureId) -> Result<Capture>;
    fn save_capture(&self, capture: Capture, expected: Revision) -> Result<Capture>;
    fn evidence(&self, item: ItemId) -> Result<Vec<Evidence>>;
}

/// Mechanism-only reads. A read adapter never receives a Principal or decides
/// what a role is allowed to observe.
pub trait CollectionReadPort: Send + Sync {
    fn collections(&self, user: UserId) -> Result<Vec<Collection>>;
    fn role(&self, user: UserId, collection: CollectionId) -> Result<Role>;
    fn browse(&self, user: UserId, query: BrowseQuery) -> Result<Page<Item>>;
    fn item(&self, id: ItemId) -> Result<Item>;
    fn work(&self, id: WorkId) -> Result<Work>;
    fn edition(&self, id: EditionId) -> Result<Edition>;
    fn capture(&self, id: CaptureId) -> Result<Capture>;
    fn evidence(&self, item: ItemId) -> Result<Vec<Evidence>>;
}

impl<T: Catalog + ?Sized> CollectionReadPort for T {
    fn collections(&self, user: UserId) -> Result<Vec<Collection>> {
        Catalog::collections(self, user)
    }
    fn role(&self, user: UserId, collection: CollectionId) -> Result<Role> {
        Catalog::role(self, user, collection)
    }
    fn browse(&self, user: UserId, query: BrowseQuery) -> Result<Page<Item>> {
        Catalog::browse(self, user, query)
    }
    fn item(&self, id: ItemId) -> Result<Item> {
        Catalog::item(self, id)
    }
    fn work(&self, id: WorkId) -> Result<Work> {
        Catalog::work(self, id)
    }
    fn edition(&self, id: EditionId) -> Result<Edition> {
        Catalog::edition(self, id)
    }
    fn capture(&self, id: CaptureId) -> Result<Capture> {
        Catalog::capture(self, id)
    }
    fn evidence(&self, item: ItemId) -> Result<Vec<Evidence>> {
        Catalog::evidence(self, item)
    }
}

pub trait CollectionWritePort: Send + Sync {
    fn create_collection(&self, user: UserId, collection: Collection) -> Result<Collection>;
    fn save(&self, item: Item, expected: Revision) -> Result<Item>;
    fn save_work(&self, work: Work, expected: Revision) -> Result<Work>;
    fn save_edition(&self, edition: Edition, expected: Revision) -> Result<Edition>;
    fn save_capture(&self, capture: Capture, expected: Revision) -> Result<Capture>;
}

pub trait AccessPort: Send + Sync {
    fn grant_for_user(&self, collection: CollectionId, user: UserId) -> Result<Option<GrantView>>;
    fn grants(&self, collection: CollectionId) -> Result<Vec<GrantView>>;
    fn set_grant(
        &self,
        collection: CollectionId,
        user: UserId,
        expected: Revision,
        role: Option<Role>,
    ) -> Result<Option<GrantView>>;
}

pub trait MediaReader: Read + Seek + Send {}
impl<T: Read + Seek + Send> MediaReader for T {}

pub struct MediaResource {
    pub reader: Box<dyn MediaReader>,
    pub length: u64,
    pub media_type: String,
    pub digest: Digest,
    pub download_name: String,
}

/// Persistence and file mechanisms after application policy has authorized an
/// effect. No method receives a Principal or Role.
pub trait ContentPort: Send + Sync {
    fn activity(&self, user: UserId, item: ItemId) -> Result<serde_json::Value>;
    fn save_activity(&self, user: UserId, item: ItemId, value: serde_json::Value) -> Result<()>;
    fn representation(
        &self,
        item: ItemId,
        representation: RepresentationId,
        index: usize,
    ) -> Result<MediaResource>;
    fn asset(&self, item: ItemId, asset: AssetId, thumbnail: bool) -> Result<MediaResource>;
    fn publish_cover(
        &self,
        item: ItemId,
        asset: AssetId,
        expected: Revision,
        bytes: &[u8],
        alt_text: String,
    ) -> Result<Item>;
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UploadFile {
    pub name: String,
    pub bytes: ByteLength,
    pub sha256: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Upload {
    pub id: JobId,
    pub user_id: UserId,
    pub item_id: ItemId,
    pub expected_revision: Revision,
    pub representation_id: RepresentationId,
    pub capture_id: CaptureId,
    pub files: Vec<UploadFile>,
    pub offsets: Vec<u64>,
    pub state: JobState,
}

pub trait ImportPort: Send + Sync {
    fn create_upload(
        &self,
        user: UserId,
        item: ItemId,
        expected: Revision,
        files: Vec<UploadFile>,
    ) -> Result<Upload>;
    fn upload(&self, id: JobId) -> Result<Upload>;
    fn upload_chunk(&self, id: JobId, index: usize, offset: u64, bytes: &[u8]) -> Result<Upload>;
    fn cancel_upload(&self, id: JobId) -> Result<()>;
    fn commit_upload(&self, id: JobId) -> Result<Item>;
    fn fail_upload(&self, id: JobId, message: &str) -> Result<()>;
    fn verify_integrity(&self, item: ItemId) -> Result<Item>;
    fn jobs(&self, user: UserId) -> Result<Vec<Job>>;
}

pub trait AdministrationPort: Send + Sync {
    fn create_user(&self, username: &str, password: &str, administrator: bool) -> Result<UserId>;
    fn reindex(&self) -> Result<()>;
    fn accounts(&self) -> Result<Vec<AccountView>>;
    fn account(&self, user: UserId) -> Result<AccountView>;
    fn update_account(
        &self,
        user: UserId,
        expected: Revision,
        display_name: &str,
        enabled: bool,
        administrator: bool,
    ) -> Result<AccountView>;
    fn reassign_default(&self, user: UserId) -> Result<()>;
}

pub trait AuthenticationPort: Send + Sync {
    fn now(&self) -> chrono::DateTime<Utc>;
    fn login(
        &self,
        username: &str,
        password: &str,
        label: &str,
    ) -> Result<(String, AuthenticatedPrincipal)>;
    fn authenticate(&self, token: &str) -> Result<AuthenticatedPrincipal>;
    fn authenticate_native(&self, credential: &str) -> Result<AuthenticatedPrincipal>;
    fn revoke_session(&self, user: UserId, device: DeviceId) -> Result<()>;
    fn sessions(&self, user: UserId, current: Option<DeviceId>) -> Result<Vec<SessionView>>;
    fn replace_recovery_codes(&self, user: UserId) -> Result<Vec<String>>;
    fn consume_recovery_code(
        &self,
        username: &str,
        code: &str,
        new_password: &str,
        label: &str,
    ) -> Result<(String, AuthenticatedPrincipal)>;
    fn username(&self, user: UserId) -> Result<String>;
    fn user_by_name(&self, username: &str) -> Result<UserId>;
    fn passkeys(&self, user: UserId) -> Result<Vec<StoredPasskey>>;
    fn insert_passkey(
        &self,
        user: UserId,
        credential_id: &str,
        credential: serde_json::Value,
    ) -> Result<()>;
    fn complete_passkey_authentication(
        &self,
        user: UserId,
        passkey: PasskeyId,
        expected_revision: Revision,
        expected_record: serde_json::Value,
        credential: serde_json::Value,
        label: &str,
    ) -> Result<(String, AuthenticatedPrincipal)>;
    fn revoke_passkey(&self, user: UserId, passkey: PasskeyId, expected: Revision) -> Result<()>;
    fn issue_session(&self, user: UserId, label: &str) -> Result<(String, AuthenticatedPrincipal)>;
}

pub trait EnrollmentPort: Send + Sync {
    fn start_enrollment(
        &self,
        user: UserId,
        label: &str,
        verification_url: &str,
    ) -> Result<EnrollmentStart>;
    fn approve_enrollment(&self, user: UserId, user_code: &str) -> Result<EnrollmentView>;
    fn cancel_enrollment(&self, user: UserId, enrollment: EnrollmentId) -> Result<()>;
    fn complete_enrollment(
        &self,
        enrollment: EnrollmentId,
        completion_secret: &str,
    ) -> Result<NativeCredentialIssue>;
    fn native_devices(&self, user: UserId) -> Result<Vec<NativeDeviceView>>;
    fn revoke_native_device(
        &self,
        user: UserId,
        credential: NativeCredentialId,
        expected: Revision,
    ) -> Result<()>;
}

#[derive(Clone, Debug)]
pub struct CommandContext {
    key: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CredentialKind {
    BrowserSession,
    NativeCredential,
    TestLocal,
}

#[derive(Clone, Debug)]
pub struct AuthenticatedPrincipal {
    pub principal: Principal,
    pub credential_kind: CredentialKind,
    pub authenticated_at: chrono::DateTime<Utc>,
}

impl AuthenticatedPrincipal {
    pub fn require_recent_at(&self, now: chrono::DateTime<Utc>) -> Result<()> {
        const RECENT_SECONDS: i64 = 5 * 60;
        let age = now
            .signed_duration_since(self.authenticated_at)
            .num_seconds();
        if (0..RECENT_SECONDS).contains(&age) {
            Ok(())
        } else {
            Err(Error::RecentAuthenticationRequired)
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AccountView {
    pub id: UserId,
    pub username: String,
    pub display_name: String,
    pub administrator: bool,
    pub enabled: bool,
    pub revision: Revision,
}

#[derive(Clone, Debug, Serialize)]
pub struct AccountUpdate {
    pub user: UserId,
    pub expected_revision: Revision,
    pub display_name: String,
    pub enabled: bool,
    pub administrator: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GrantView {
    pub collection_id: CollectionId,
    pub user_id: UserId,
    pub username: String,
    pub role: Role,
    pub revision: Revision,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SessionView {
    pub id: SessionId,
    pub label: String,
    pub created_at: chrono::DateTime<Utc>,
    pub expires_at: chrono::DateTime<Utc>,
    pub current: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PasskeyView {
    pub id: PasskeyId,
    pub label: String,
    pub revision: Revision,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct StoredPasskey {
    pub view: PasskeyView,
    pub credential_id: String,
    pub record: serde_json::Value,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EnrollmentPhase {
    PendingApproval,
    Approved,
    Issued,
    Cancelled,
    Expired,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct EnrollmentStart {
    pub id: EnrollmentId,
    pub completion_secret: String,
    pub user_code: String,
    pub verification_url: String,
    pub expires_at: chrono::DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct EnrollmentView {
    pub id: EnrollmentId,
    pub requested_label: String,
    pub phase: EnrollmentPhase,
    pub expires_at: chrono::DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NativeCredentialIssue {
    pub device_id: NativeCredentialId,
    pub credential: String,
    pub expires_at: chrono::DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct NativeDeviceView {
    pub id: NativeCredentialId,
    pub label: String,
    pub revision: Revision,
    pub created_at: chrono::DateTime<Utc>,
    pub expires_at: chrono::DateTime<Utc>,
}
impl CommandContext {
    pub fn new(key: impl Into<String>) -> Result<Self> {
        let key = key.into();
        if key.is_empty() || key.len() > 128 || !key.bytes().all(|byte| byte.is_ascii_graphic()) {
            return Err(Error::Invalid(
                "Idempotency-Key must be 1–128 visible ASCII characters".into(),
            ));
        }
        Ok(Self { key })
    }
    pub fn key(&self) -> &str {
        &self.key
    }
}

pub enum ReceiptDecision {
    Execute,
    Replay(serde_json::Value),
}

/// A per-command mechanism scope. Every durable effect must commit its receipt
/// atomically, or prepare an outcome tied to an immutable archive commit point.
/// No ambient thread-local state or authorization decisions live in this scope.
pub trait CommandEffects:
    CollectionWritePort
    + ContentPort
    + ImportPort
    + AdministrationPort
    + AuthenticationPort
    + EnrollmentPort
    + AccessPort
{
}
impl<T> CommandEffects for T where
    T: CollectionWritePort
        + ContentPort
        + ImportPort
        + AdministrationPort
        + AuthenticationPort
        + EnrollmentPort
        + AccessPort
{
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandReceipt {
    pub user: UserId,
    pub api_major: u16,
    pub operation: String,
    pub key: String,
    pub request_sha256: String,
}

pub trait IdempotencyPort: Send + Sync {
    fn command_effects(&self, receipt: CommandReceipt) -> Box<dyn CommandEffects>;

    fn begin_command(
        &self,
        user: UserId,
        api_major: u16,
        operation: &str,
        key: &str,
        request_sha256: &str,
    ) -> Result<ReceiptDecision>;
    fn complete_command(
        &self,
        user: UserId,
        api_major: u16,
        operation: &str,
        key: &str,
        request_sha256: &str,
        result: serde_json::Value,
    ) -> Result<()>;
    fn abandon_command(
        &self,
        user: UserId,
        api_major: u16,
        operation: &str,
        key: &str,
        request_sha256: &str,
    ) -> Result<()>;
}

impl<T: Catalog + ?Sized> CollectionWritePort for T {
    fn create_collection(&self, user: UserId, collection: Collection) -> Result<Collection> {
        Catalog::create_collection(self, user, collection)
    }
    fn save(&self, item: Item, expected: Revision) -> Result<Item> {
        Catalog::save(self, item, expected)
    }
    fn save_work(&self, work: Work, expected: Revision) -> Result<Work> {
        Catalog::save_work(self, work, expected)
    }
    fn save_edition(&self, edition: Edition, expected: Revision) -> Result<Edition> {
        Catalog::save_edition(self, edition, expected)
    }
    fn save_capture(&self, capture: Capture, expected: Revision) -> Result<Capture> {
        Catalog::save_capture(self, capture, expected)
    }
}

#[derive(Clone)]
pub struct CollectionService {
    reader: Arc<dyn CollectionReadPort>,
    access: Arc<dyn AccessPort>,
    content: Arc<dyn ContentPort>,
    imports: Arc<dyn ImportPort>,
    administration: Arc<dyn AdministrationPort>,
    authentication: Arc<dyn AuthenticationPort>,
    enrollments: Arc<dyn EnrollmentPort>,
    idempotency: Arc<dyn IdempotencyPort>,
    modules: Arc<Registry>,
}

#[derive(Clone, Copy, Debug, Serialize, utoipa::ToSchema)]
pub struct EffectivePermissions {
    pub role: Role,
    pub can_browse: bool,
    pub can_curate: bool,
    pub can_manage_access: bool,
}
impl From<Role> for EffectivePermissions {
    fn from(role: Role) -> Self {
        Self {
            role,
            can_browse: role.allows(Action::Browse),
            can_curate: role.allows(Action::Curate),
            can_manage_access: role.allows(Action::ManageAccess),
        }
    }
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct PresentedCollection {
    pub id: CollectionId,
    pub revision: Revision,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub effective_permissions: EffectivePermissions,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct PresentedModuleData {
    pub module: ModuleId,
    pub schema_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<serde_json::Value>)]
    pub payload: Option<RawJson>,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct PresentedSpecimen {
    pub id: SpecimenId,
    pub label: String,
    pub condition: Condition,
    pub ownership: Ownership,
    pub possession: Possession,
    pub components: Vec<Component>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acquisition: Option<Acquisition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loans: Option<Vec<Loan>>,
}

impl PresentedSpecimen {
    fn from_stored(specimen: Specimen, private: bool) -> Self {
        Self {
            id: specimen.id,
            label: specimen.label,
            condition: specimen.condition,
            ownership: specimen.ownership,
            possession: specimen.possession,
            components: specimen.components,
            acquisition: private.then_some(specimen.acquisition).flatten(),
            private_notes: private.then_some(specimen.private_notes).flatten(),
            loans: private.then_some(specimen.loans),
        }
    }
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct PresentedCapture {
    pub id: CaptureId,
    pub collection_id: CollectionId,
    pub revision: Revision,
    pub association: CaptureAssociation,
    pub created_at: DateTime<Utc>,
    pub representations: Vec<Representation>,
}

impl From<Capture> for PresentedCapture {
    fn from(capture: Capture) -> Self {
        Self {
            id: capture.id,
            collection_id: capture.collection_id,
            revision: capture.revision,
            association: capture.association,
            created_at: capture.created_at,
            representations: capture.representations,
        }
    }
}

/// Explicit response shape: unrestricted stored Item records are never
/// flattened into a transport response.
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct PresentedItem {
    pub id: ItemId,
    pub collection_id: CollectionId,
    pub revision: Revision,
    pub title: String,
    pub subtitle: String,
    pub identity_link: ItemIdentityLink,
    pub domain: PresentedModuleData,
    pub identity: IdentityState,
    pub translations: Vec<LocalizedText>,
    pub identifiers: Vec<ProviderIdentity>,
    pub specimens: Vec<PresentedSpecimen>,
    pub wishlist: bool,
    pub captures: Vec<PresentedCapture>,
    pub assets: Vec<Asset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MetadataState>,
    pub deleted: bool,
    pub updated_at: DateTime<Utc>,
    pub presentation: ModulePresentation,
    pub effective_permissions: EffectivePermissions,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct PresentedEvidence {
    pub id: EvidenceId,
    pub item_id: ItemId,
    pub representation_id: RepresentationId,
    pub kind: EvidenceKind,
    pub outcome: EvidenceOutcome,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Vec<Digest>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implementation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implementation_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalog: Option<CatalogSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Editable inventory fields. Revision, timestamps, deletion state, metadata
/// observations, Evidence, Assets, and Capture representations are server-owned.
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveItemCommand {
    pub collection_id: CollectionId,
    pub title: String,
    pub subtitle: String,
    pub identity_link: ItemIdentityLink,
    pub domain: ModuleData,
    pub identity: IdentityState,
    pub translations: Vec<LocalizedText>,
    pub identifiers: Vec<ProviderIdentity>,
    pub specimens: Vec<Specimen>,
    pub wishlist: bool,
}

impl From<Item> for SaveItemCommand {
    fn from(item: Item) -> Self {
        Self {
            collection_id: item.collection_id,
            title: item.title,
            subtitle: item.subtitle,
            identity_link: item.identity_link,
            domain: item.domain,
            identity: item.identity,
            translations: item.translations,
            identifiers: item.identifiers,
            specimens: item.specimens,
            wishlist: item.wishlist,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveWorkCommand {
    pub collection_id: CollectionId,
    pub provisional: bool,
    pub title: String,
    pub translations: Vec<LocalizedText>,
    pub identifiers: Vec<ProviderIdentity>,
}
impl From<Work> for SaveWorkCommand {
    fn from(work: Work) -> Self {
        Self {
            collection_id: work.collection_id,
            provisional: work.provisional,
            title: work.title,
            translations: work.translations,
            identifiers: work.identifiers,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveEditionCommand {
    pub collection_id: CollectionId,
    pub work_id: Option<WorkId>,
    pub provisional: bool,
    pub title: String,
    pub expected_components: Vec<ExpectedComponent>,
    pub identifiers: Vec<ProviderIdentity>,
}
impl From<Edition> for SaveEditionCommand {
    fn from(edition: Edition) -> Self {
        Self {
            collection_id: edition.collection_id,
            work_id: edition.work_id,
            provisional: edition.provisional,
            title: edition.title,
            expected_components: edition.expected_components,
            identifiers: edition.identifiers,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveCaptureCommand {
    pub collection_id: CollectionId,
    pub association: CaptureAssociation,
}
impl From<Capture> for SaveCaptureCommand {
    fn from(capture: Capture) -> Self {
        Self {
            collection_id: capture.collection_id,
            association: capture.association,
        }
    }
}

impl CollectionService {
    pub fn new<C>(catalog: Arc<C>, modules: Arc<Registry>) -> Self
    where
        C: CollectionReadPort
            + CollectionWritePort
            + AccessPort
            + ContentPort
            + ImportPort
            + AdministrationPort
            + AuthenticationPort
            + EnrollmentPort
            + IdempotencyPort
            + 'static,
    {
        let reader: Arc<dyn CollectionReadPort> = catalog.clone();
        let content: Arc<dyn ContentPort> = catalog.clone();
        let imports: Arc<dyn ImportPort> = catalog.clone();
        let administration: Arc<dyn AdministrationPort> = catalog.clone();
        let authentication: Arc<dyn AuthenticationPort> = catalog.clone();
        let enrollments: Arc<dyn EnrollmentPort> = catalog.clone();
        let idempotency: Arc<dyn IdempotencyPort> = catalog.clone();
        let access: Arc<dyn AccessPort> = catalog.clone();
        Self {
            reader,
            access,
            content,
            imports,
            administration,
            authentication,
            enrollments,
            idempotency,
            modules,
        }
    }
    fn execute_command<Request, Response>(
        &self,
        actor: &Principal,
        context: &CommandContext,
        operation: &str,
        request: &Request,
        effect: impl FnOnce(&dyn CommandEffects) -> Result<Response>,
    ) -> Result<Response>
    where
        Request: Serialize,
        Response: Serialize + DeserializeOwned,
    {
        let request = serde_json::to_vec(&(operation, request))
            .map_err(|error| Error::Invalid(error.to_string()))?;
        let request_sha256 = Digest::sha256(&request).hex;
        match self.idempotency.begin_command(
            actor.user_id,
            1,
            operation,
            context.key(),
            &request_sha256,
        )? {
            ReceiptDecision::Replay(value) => serde_json::from_value(value)
                .map_err(|error| Error::Storage(format!("invalid command receipt: {error}"))),
            ReceiptDecision::Execute => match effect(
                self.idempotency
                    .command_effects(CommandReceipt {
                        user: actor.user_id,
                        api_major: 1,
                        operation: operation.into(),
                        key: context.key().into(),
                        request_sha256: request_sha256.clone(),
                    })
                    .as_ref(),
            ) {
                Ok(result) => {
                    let value = serde_json::to_value(&result)
                        .map_err(|error| Error::Storage(error.to_string()))?;
                    self.idempotency.complete_command(
                        actor.user_id,
                        1,
                        operation,
                        context.key(),
                        &request_sha256,
                        value,
                    )?;
                    Ok(result)
                }
                Err(error) => {
                    let _ = self.idempotency.abandon_command(
                        actor.user_id,
                        1,
                        operation,
                        context.key(),
                        &request_sha256,
                    );
                    Err(error)
                }
            },
        }
    }
    pub fn collections(&self, actor: &Principal) -> Result<Vec<PresentedCollection>> {
        self.reader
            .collections(actor.user_id)?
            .into_iter()
            .map(|collection| {
                let role = self.reader.role(actor.user_id, collection.id)?;
                Ok(PresentedCollection {
                    id: collection.id,
                    revision: collection.revision,
                    name: collection.name,
                    created_at: collection.created_at,
                    effective_permissions: role.into(),
                })
            })
            .collect()
    }
    pub fn create_collection(
        &self,
        authenticated: &AuthenticatedPrincipal,
        name: &str,
        context: &CommandContext,
    ) -> Result<Collection> {
        authenticated.require_recent_at(self.authentication.now())?;
        let actor = &authenticated.principal;
        if !actor.administrator {
            return Err(Error::Forbidden);
        }
        if name.trim().is_empty() || name.len() > 256 {
            return Err(Error::Invalid("collection name must be 1–256 bytes".into()));
        }
        let name = name.trim().to_owned();
        let collection = Collection {
            id: deterministic_collection_id(actor.user_id, context.key()),
            revision: Revision(1),
            name: name.clone(),
            created_at: self.authentication.now(),
        };
        self.execute_command(actor, context, "collection.create", &name, |effects| {
            effects.create_collection(actor.user_id, collection.clone())
        })
    }
    pub fn authorize(
        &self,
        actor: &Principal,
        collection: CollectionId,
        action: Action,
    ) -> Result<Role> {
        let role = self.reader.role(actor.user_id, collection)?;
        if !role.allows(action) {
            return Err(Error::Forbidden);
        }
        Ok(role)
    }
    /// Presentation is calculated only after role-based entity redaction. It is a
    /// read model; module commands remain behind A04.
    pub fn browse_presented(
        &self,
        actor: &Principal,
        query: BrowseQuery,
    ) -> Result<Page<PresentedItem>> {
        if let Some(collection) = query.collection_id {
            self.authorize(actor, collection, Action::Browse)?;
        }
        let page = self.reader.browse(actor.user_id, query)?;
        let mut items = Vec::with_capacity(page.items.len());
        for mut item in page.items {
            let role = self.authorize(actor, item.collection_id, Action::Browse)?;
            item.redact_for(role);
            items.push(self.presented(item, role));
        }
        Ok(Page {
            items,
            next_cursor: page.next_cursor,
        })
    }
    pub fn item_presented(&self, actor: &Principal, id: ItemId) -> Result<PresentedItem> {
        let mut item = self.reader.item(id)?;
        let role = self.authorize(actor, item.collection_id, Action::Browse)?;
        item.redact_for(role);
        Ok(self.presented(item, role))
    }
    fn presented(&self, item: Item, role: Role) -> PresentedItem {
        let mut presentation = self.modules.presentation(&item.domain);
        presentation.artwork.retain(|reference| {
            item.assets
                .iter()
                .any(|asset| asset.id == reference.asset_id)
        });
        if role == Role::Viewer && !presentation.available {
            presentation.values.clear();
        }
        let private = role != Role::Viewer;
        PresentedItem {
            id: item.id,
            collection_id: item.collection_id,
            revision: item.revision,
            title: item.title,
            subtitle: item.subtitle,
            identity_link: item.identity_link,
            domain: PresentedModuleData {
                module: item.domain.module,
                schema_version: item.domain.schema_version,
                payload: private.then_some(item.domain.payload),
            },
            identity: item.identity,
            translations: item.translations,
            identifiers: item.identifiers,
            specimens: item
                .specimens
                .into_iter()
                .map(|specimen| PresentedSpecimen::from_stored(specimen, private))
                .collect(),
            wishlist: item.wishlist,
            captures: item.captures.into_iter().map(Into::into).collect(),
            assets: item.assets,
            metadata: private.then_some(item.metadata),
            deleted: item.deleted,
            updated_at: item.updated_at,
            presentation,
            effective_permissions: role.into(),
        }
    }
    pub fn save_item(
        &self,
        actor: &Principal,
        id: ItemId,
        command: SaveItemCommand,
        expected: Revision,
        context: &CommandContext,
    ) -> Result<Item> {
        let request = (id, expected, command.clone());
        self.authorize(actor, command.collection_id, Action::Curate)?;
        let (captures, assets, metadata, deleted, revision, updated_at) = match self.reader.item(id)
        {
            Ok(previous) => {
                if previous.collection_id != command.collection_id {
                    return Err(Error::Invalid(
                        "moving items across collections is not supported".into(),
                    ));
                }
                (
                    previous.captures,
                    previous.assets,
                    previous.metadata,
                    previous.deleted,
                    previous.revision,
                    previous.updated_at,
                )
            }
            Err(Error::NotFound) => (
                vec![],
                vec![],
                MetadataState::default(),
                false,
                Revision(0),
                Utc::now(),
            ),
            Err(error) => return Err(error),
        };
        let item = Item {
            id,
            collection_id: command.collection_id,
            revision,
            title: command.title,
            subtitle: command.subtitle,
            identity_link: command.identity_link,
            domain: command.domain,
            identity: command.identity,
            translations: command.translations,
            identifiers: command.identifiers,
            specimens: command.specimens,
            wishlist: command.wishlist,
            captures,
            assets,
            metadata,
            deleted,
            updated_at,
        };
        item.validate()?;
        self.modules.validate(&item.domain)?;
        self.execute_command(actor, context, "item.save", &request, |effects| {
            effects.save(item, expected)
        })
    }
    pub fn adapt_wave1_item_save(
        &self,
        actor: &Principal,
        path_id: ItemId,
        item: Item,
        expected: Revision,
    ) -> Result<SaveItemCommand> {
        if path_id != item.id {
            return Err(Error::Invalid("item ID mismatch".into()));
        }
        self.authorize(actor, item.collection_id, Action::Curate)?;
        match self.reader.item(path_id) {
            Ok(current) => {
                self.authorize(actor, current.collection_id, Action::Curate)?;
                let submitted_owned = serde_json::to_value((
                    item.revision,
                    &item.captures,
                    &item.assets,
                    &item.metadata,
                    item.deleted,
                    item.updated_at,
                ))
                .map_err(|error| Error::Invalid(error.to_string()))?;
                let canonical_owned = serde_json::to_value((
                    current.revision,
                    &current.captures,
                    &current.assets,
                    &current.metadata,
                    current.deleted,
                    current.updated_at,
                ))
                .map_err(|error| Error::Storage(error.to_string()))?;
                if item.collection_id != current.collection_id
                    || expected != current.revision
                    || submitted_owned != canonical_owned
                {
                    return Err(Error::Invalid(
                        "the Wave 1 item envelope changed server-owned fields; refresh and send only editable command fields"
                            .into(),
                    ));
                }
            }
            Err(Error::NotFound)
                if expected == Revision(0)
                    && item.revision == Revision(0)
                    && item.captures.is_empty()
                    && item.assets.is_empty()
                    && !item.deleted
                    && serde_json::to_value(&item.metadata)
                        .map_err(|error| Error::Invalid(error.to_string()))?
                        == serde_json::to_value(MetadataState::default())
                            .map_err(|error| Error::Storage(error.to_string()))? => {}
            Err(Error::NotFound) => {
                return Err(Error::Invalid(
                    "a new Wave 1 item envelope contains server-owned state".into(),
                ));
            }
            Err(error) => return Err(error),
        }
        Ok(item.into())
    }
    pub fn capture(&self, actor: &Principal, id: CaptureId) -> Result<Capture> {
        let capture = self.reader.capture(id)?;
        self.authorize(actor, capture.collection_id, Action::Browse)?;
        Ok(capture)
    }
    pub fn work(&self, actor: &Principal, id: WorkId) -> Result<Work> {
        let work = self.reader.work(id)?;
        self.authorize(actor, work.collection_id, Action::Browse)?;
        Ok(work)
    }
    pub fn save_work(
        &self,
        actor: &Principal,
        id: WorkId,
        command: SaveWorkCommand,
        expected: Revision,
        context: &CommandContext,
    ) -> Result<Work> {
        let request = (id, expected, command.clone());
        self.authorize(actor, command.collection_id, Action::Curate)?;
        let (revision, migration_provenance, deleted, updated_at) = match self.reader.work(id) {
            Ok(previous) => {
                self.authorize(actor, previous.collection_id, Action::Curate)?;
                if previous.collection_id != command.collection_id {
                    return Err(Error::Invalid("work cannot move across collections".into()));
                }
                (
                    previous.revision,
                    previous.migration_provenance,
                    previous.deleted,
                    previous.updated_at,
                )
            }
            Err(Error::NotFound) => (Revision(0), vec![], false, Utc::now()),
            Err(error) => return Err(error),
        };
        let work = Work {
            id,
            collection_id: command.collection_id,
            revision,
            provisional: command.provisional,
            title: command.title,
            translations: command.translations,
            identifiers: command.identifiers,
            migration_provenance,
            deleted,
            updated_at,
        };
        work.validate()?;
        self.execute_command(actor, context, "work.save", &request, |effects| {
            effects.save_work(work, expected)
        })
    }
    pub fn edition(&self, actor: &Principal, id: EditionId) -> Result<Edition> {
        let edition = self.reader.edition(id)?;
        self.authorize(actor, edition.collection_id, Action::Browse)?;
        Ok(edition)
    }
    pub fn save_edition(
        &self,
        actor: &Principal,
        id: EditionId,
        command: SaveEditionCommand,
        expected: Revision,
        context: &CommandContext,
    ) -> Result<Edition> {
        let request = (id, expected, command.clone());
        self.authorize(actor, command.collection_id, Action::Curate)?;
        let (revision, migration_provenance, deleted, updated_at) = match self.reader.edition(id) {
            Ok(previous) => {
                self.authorize(actor, previous.collection_id, Action::Curate)?;
                if previous.collection_id != command.collection_id {
                    return Err(Error::Invalid(
                        "edition cannot move across collections".into(),
                    ));
                }
                (
                    previous.revision,
                    previous.migration_provenance,
                    previous.deleted,
                    previous.updated_at,
                )
            }
            Err(Error::NotFound) => (Revision(0), vec![], false, Utc::now()),
            Err(error) => return Err(error),
        };
        let edition = Edition {
            id,
            collection_id: command.collection_id,
            revision,
            work_id: command.work_id,
            provisional: command.provisional,
            title: command.title,
            expected_components: command.expected_components,
            identifiers: command.identifiers,
            migration_provenance,
            deleted,
            updated_at,
        };
        edition.validate()?;
        self.execute_command(actor, context, "edition.save", &request, |effects| {
            effects.save_edition(edition, expected)
        })
    }
    pub fn save_capture(
        &self,
        actor: &Principal,
        id: CaptureId,
        command: SaveCaptureCommand,
        expected: Revision,
        context: &CommandContext,
    ) -> Result<Capture> {
        let request = (id, expected, command.clone());
        let (revision, created_at, representations) = match self.reader.capture(id) {
            Ok(previous) => {
                self.authorize(actor, previous.collection_id, Action::Curate)?;
                if previous.collection_id != command.collection_id {
                    return Err(Error::Invalid(
                        "capture cannot move across collections".into(),
                    ));
                }
                (
                    previous.revision,
                    previous.created_at,
                    previous.representations,
                )
            }
            Err(Error::NotFound) => (Revision(0), Utc::now(), vec![]),
            Err(error) => return Err(error),
        };
        self.authorize(actor, command.collection_id, Action::Curate)?;
        let capture = Capture {
            id,
            collection_id: command.collection_id,
            revision,
            association: command.association,
            created_at,
            representations,
        };
        capture.validate()?;
        self.execute_command(actor, context, "capture.save", &request, |effects| {
            effects.save_capture(capture, expected)
        })
    }
    pub fn evidence(&self, actor: &Principal, id: ItemId) -> Result<Vec<PresentedEvidence>> {
        let item = self.reader.item(id)?;
        let role = self.authorize(actor, item.collection_id, Action::Browse)?;
        let private = role != Role::Viewer;
        Ok(self
            .reader
            .evidence(id)?
            .into_iter()
            .map(|evidence| PresentedEvidence {
                id: evidence.id,
                item_id: evidence.item_id,
                representation_id: evidence.representation_id,
                kind: evidence.kind,
                outcome: evidence.outcome,
                created_at: evidence.created_at,
                inputs: private.then_some(evidence.inputs),
                implementation: private.then_some(evidence.implementation),
                implementation_version: private.then_some(evidence.implementation_version),
                catalog: if private { evidence.catalog } else { None },
                details: private.then_some(evidence.details),
            })
            .collect())
    }
    pub fn activity(&self, actor: &Principal, id: ItemId) -> Result<serde_json::Value> {
        let item = self.reader.item(id)?;
        self.authorize(actor, item.collection_id, Action::Browse)?;
        self.content.activity(actor.user_id, id)
    }
    pub fn save_activity(
        &self,
        actor: &Principal,
        id: ItemId,
        value: serde_json::Value,
        context: &CommandContext,
    ) -> Result<()> {
        let item = self.reader.item(id)?;
        self.authorize(actor, item.collection_id, Action::Browse)?;
        if serde_json::to_vec(&value)
            .map_err(|error| Error::Invalid(error.to_string()))?
            .len()
            > 8192
        {
            return Err(Error::Invalid("activity too large".into()));
        }
        let request = (id, value.clone());
        self.execute_command(actor, context, "activity.save", &request, |effects| {
            effects.save_activity(actor.user_id, id, value)
        })
    }
    pub fn representation(
        &self,
        actor: &Principal,
        item_id: ItemId,
        representation: RepresentationId,
        index: usize,
    ) -> Result<MediaResource> {
        let item = self.reader.item(item_id)?;
        self.authorize(actor, item.collection_id, Action::Browse)?;
        if item.deleted {
            return Err(Error::NotFound);
        }
        self.content.representation(item_id, representation, index)
    }
    pub fn asset(
        &self,
        actor: &Principal,
        item_id: ItemId,
        asset_id: AssetId,
        thumbnail: bool,
    ) -> Result<MediaResource> {
        let item = self.reader.item(item_id)?;
        let role = self.authorize(actor, item.collection_id, Action::Browse)?;
        if item.deleted {
            return Err(Error::NotFound);
        }
        let asset = item
            .assets
            .iter()
            .find(|asset| asset.id == asset_id)
            .ok_or(Error::NotFound)?;
        if role == Role::Viewer && asset.visibility == Visibility::Curators {
            return Err(Error::Forbidden);
        }
        self.content.asset(item_id, asset_id, thumbnail)
    }
    pub fn publish_cover(
        &self,
        actor: &Principal,
        item_id: ItemId,
        expected: Revision,
        bytes: &[u8],
        alt_text: String,
        context: &CommandContext,
    ) -> Result<Item> {
        let item = self.reader.item(item_id)?;
        self.authorize(actor, item.collection_id, Action::Curate)?;
        let asset_id = deterministic_asset_id(actor.user_id, context.key());
        if bytes.len() > 20 * 1024 * 1024 || alt_text.len() > 4096 {
            return Err(Error::Invalid("image exceeds upload limit".into()));
        }
        let request = (item_id, expected, Digest::sha256(bytes), alt_text.clone());
        self.execute_command(actor, context, "cover.publish", &request, |effects| {
            effects.publish_cover(item_id, asset_id, expected, bytes, alt_text)
        })
    }
    fn authorized_upload(&self, actor: &Principal, id: JobId) -> Result<Upload> {
        let upload = self.imports.upload(id)?;
        if upload.user_id != actor.user_id {
            return Err(Error::Forbidden);
        }
        let item = self.reader.item(upload.item_id)?;
        self.authorize(actor, item.collection_id, Action::Curate)?;
        Ok(upload)
    }
    pub fn create_upload(
        &self,
        actor: &Principal,
        item_id: ItemId,
        expected: Revision,
        files: Vec<UploadFile>,
        context: &CommandContext,
    ) -> Result<Upload> {
        let item = self.reader.item(item_id)?;
        self.authorize(actor, item.collection_id, Action::Curate)?;
        let request = (item_id, expected, files.clone());
        self.execute_command(actor, context, "upload.create", &request, |effects| {
            effects.create_upload(actor.user_id, item_id, expected, files)
        })
    }
    pub fn authorize_import(
        &self,
        actor: &Principal,
        item_id: ItemId,
        expected: Revision,
    ) -> Result<()> {
        self.require_administrator(actor)?;
        let item = self.reader.item(item_id)?;
        self.authorize(actor, item.collection_id, Action::Curate)?;
        if item.revision != expected {
            return Err(Error::Conflict);
        }
        Ok(())
    }
    pub fn upload(&self, actor: &Principal, id: JobId) -> Result<Upload> {
        self.authorized_upload(actor, id)
    }
    pub fn upload_chunk(
        &self,
        actor: &Principal,
        id: JobId,
        index: usize,
        offset: u64,
        bytes: &[u8],
    ) -> Result<Upload> {
        self.authorized_upload(actor, id)?;
        self.imports.upload_chunk(id, index, offset, bytes)
    }
    pub fn cancel_upload(
        &self,
        actor: &Principal,
        id: JobId,
        context: &CommandContext,
    ) -> Result<()> {
        self.authorized_upload(actor, id)?;
        self.execute_command(actor, context, "upload.cancel", &id, |effects| {
            effects.cancel_upload(id)
        })
    }
    pub fn commit_upload(
        &self,
        actor: &Principal,
        id: JobId,
        context: &CommandContext,
    ) -> Result<Item> {
        self.authorized_upload(actor, id)?;
        let result = self.execute_command(actor, context, "upload.commit", &id, |effects| {
            effects.commit_upload(id)
        });
        if let Err(error) = &result
            && !matches!(error, Error::IdempotencyConflict | Error::CommandInProgress)
        {
            let _ = self.imports.fail_upload(
                id,
                "Archive publication failed. Check server logs and retry the retained upload.",
            );
        }
        result
    }
    pub fn fail_upload(&self, actor: &Principal, id: JobId, message: &str) -> Result<()> {
        self.authorized_upload(actor, id)?;
        self.imports.fail_upload(id, message)
    }
    pub fn verify_integrity(
        &self,
        actor: &Principal,
        item_id: ItemId,
        context: &CommandContext,
    ) -> Result<Item> {
        let item = self.reader.item(item_id)?;
        self.authorize(actor, item.collection_id, Action::Curate)?;
        self.execute_command(actor, context, "integrity.verify", &item_id, |effects| {
            effects.verify_integrity(item_id)
        })
    }
    pub fn jobs(&self, actor: &Principal) -> Result<Vec<Job>> {
        let jobs = self.imports.jobs(actor.user_id)?;
        if jobs.iter().any(|job| job.user_id != actor.user_id) {
            return Err(Error::Storage(
                "job adapter returned another user's private record".into(),
            ));
        }
        Ok(jobs)
    }
    pub fn require_administrator(&self, actor: &Principal) -> Result<()> {
        if actor.administrator {
            Ok(())
        } else {
            Err(Error::Forbidden)
        }
    }
    pub fn require_recent_authentication(
        &self,
        authenticated: &AuthenticatedPrincipal,
    ) -> Result<()> {
        authenticated.require_recent_at(self.authentication.now())
    }
    pub fn create_user(
        &self,
        authenticated: &AuthenticatedPrincipal,
        username: &str,
        password: &str,
        administrator: bool,
        context: &CommandContext,
    ) -> Result<UserId> {
        self.require_recent_authentication(authenticated)?;
        let actor = &authenticated.principal;
        self.require_administrator(actor)?;
        let request = (username, Digest::sha256(password.as_bytes()), administrator);
        self.execute_command(actor, context, "account.create", &request, |effects| {
            effects.create_user(username, password, administrator)
        })
    }
    pub fn accounts(&self, authenticated: &AuthenticatedPrincipal) -> Result<Vec<AccountView>> {
        self.require_administrator(&authenticated.principal)?;
        self.administration.accounts()
    }
    pub fn update_account(
        &self,
        authenticated: &AuthenticatedPrincipal,
        request: AccountUpdate,
        context: &CommandContext,
    ) -> Result<AccountView> {
        self.require_recent_authentication(authenticated)?;
        self.require_administrator(&authenticated.principal)?;
        self.execute_command(
            &authenticated.principal,
            context,
            "account.update",
            &request,
            |effects| {
                effects.update_account(
                    request.user,
                    request.expected_revision,
                    &request.display_name,
                    request.enabled,
                    request.administrator,
                )
            },
        )
    }
    pub fn update_profile(
        &self,
        authenticated: &AuthenticatedPrincipal,
        expected: Revision,
        display_name: &str,
        context: &CommandContext,
    ) -> Result<AccountView> {
        self.require_recent_authentication(authenticated)?;
        let current = self
            .administration
            .account(authenticated.principal.user_id)?;
        let request = (expected, display_name);
        self.execute_command(
            &authenticated.principal,
            context,
            "profile.update",
            &request,
            |effects| {
                effects.update_account(
                    current.id,
                    expected,
                    display_name,
                    current.enabled,
                    current.administrator,
                )
            },
        )
    }
    pub fn profile(&self, authenticated: &AuthenticatedPrincipal) -> Result<AccountView> {
        self.administration.account(authenticated.principal.user_id)
    }
    pub fn reassign_default(
        &self,
        authenticated: &AuthenticatedPrincipal,
        user: UserId,
        context: &CommandContext,
    ) -> Result<()> {
        self.require_recent_authentication(authenticated)?;
        self.require_administrator(&authenticated.principal)?;
        self.execute_command(
            &authenticated.principal,
            context,
            "account.default.reassign",
            &user,
            |effects| effects.reassign_default(user),
        )
    }
    pub fn reindex(
        &self,
        authenticated: &AuthenticatedPrincipal,
        context: &CommandContext,
    ) -> Result<()> {
        self.require_recent_authentication(authenticated)?;
        let actor = &authenticated.principal;
        self.require_administrator(actor)?;
        self.execute_command(actor, context, "projection.reindex", &(), |effects| {
            effects.reindex()
        })
    }
    pub fn login(
        &self,
        username: &str,
        password: &str,
        label: &str,
    ) -> Result<(String, AuthenticatedPrincipal)> {
        self.authentication.login(username, password, label)
    }
    pub fn authenticate(&self, token: &str) -> Result<AuthenticatedPrincipal> {
        self.authentication.authenticate(token)
    }
    pub fn authenticate_native(&self, credential: &str) -> Result<AuthenticatedPrincipal> {
        self.authentication.authenticate_native(credential)
    }
    pub fn revoke_session(
        &self,
        authenticated: &AuthenticatedPrincipal,
        device: DeviceId,
        context: &CommandContext,
    ) -> Result<()> {
        self.require_recent_authentication(authenticated)?;
        let actor = &authenticated.principal;
        self.execute_command(actor, context, "session.revoke", &device, |effects| {
            effects.revoke_session(actor.user_id, device)
        })
    }
    pub fn sessions(&self, authenticated: &AuthenticatedPrincipal) -> Result<Vec<SessionView>> {
        self.authentication.sessions(
            authenticated.principal.user_id,
            authenticated.principal.device_id,
        )
    }
    pub fn replace_recovery_codes(
        &self,
        authenticated: &AuthenticatedPrincipal,
    ) -> Result<Vec<String>> {
        self.require_recent_authentication(authenticated)?;
        self.authentication
            .replace_recovery_codes(authenticated.principal.user_id)
    }
    pub fn recover_account(
        &self,
        username: &str,
        code: &str,
        new_password: &str,
        label: &str,
    ) -> Result<(String, AuthenticatedPrincipal)> {
        self.authentication
            .consume_recovery_code(username, code, new_password, label)
    }
    pub fn start_enrollment(
        &self,
        username: &str,
        label: &str,
        verification_url: &str,
    ) -> Result<EnrollmentStart> {
        let user = self.authentication.user_by_name(username)?;
        self.enrollments
            .start_enrollment(user, label, verification_url)
    }
    pub fn approve_enrollment(
        &self,
        authenticated: &AuthenticatedPrincipal,
        user_code: &str,
    ) -> Result<EnrollmentView> {
        self.require_recent_authentication(authenticated)?;
        self.enrollments
            .approve_enrollment(authenticated.principal.user_id, user_code)
    }
    pub fn cancel_enrollment(
        &self,
        authenticated: &AuthenticatedPrincipal,
        enrollment: EnrollmentId,
    ) -> Result<()> {
        self.enrollments
            .cancel_enrollment(authenticated.principal.user_id, enrollment)
    }
    pub fn complete_enrollment(
        &self,
        enrollment: EnrollmentId,
        completion_secret: &str,
    ) -> Result<NativeCredentialIssue> {
        self.enrollments
            .complete_enrollment(enrollment, completion_secret)
    }
    pub fn native_devices(
        &self,
        authenticated: &AuthenticatedPrincipal,
    ) -> Result<Vec<NativeDeviceView>> {
        self.enrollments
            .native_devices(authenticated.principal.user_id)
    }
    pub fn revoke_native_device(
        &self,
        authenticated: &AuthenticatedPrincipal,
        credential: NativeCredentialId,
        expected: Revision,
        context: &CommandContext,
    ) -> Result<()> {
        self.require_recent_authentication(authenticated)?;
        let actor = &authenticated.principal;
        self.execute_command(
            actor,
            context,
            "native_credential.revoke",
            &(credential, expected),
            |effects| effects.revoke_native_device(actor.user_id, credential, expected),
        )
    }
    pub fn logout_session(&self, actor: &Principal, device: DeviceId) -> Result<()> {
        self.authentication.revoke_session(actor.user_id, device)
    }
    pub fn username(&self, actor: &Principal) -> Result<String> {
        self.authentication.username(actor.user_id)
    }
    pub fn user_by_name_for_authentication(&self, username: &str) -> Result<UserId> {
        self.authentication.user_by_name(username)
    }
    pub fn passkeys(&self, user: UserId) -> Result<Vec<StoredPasskey>> {
        self.authentication.passkeys(user)
    }
    pub fn register_passkey(
        &self,
        authenticated: &AuthenticatedPrincipal,
        credential_id: &str,
        credential: serde_json::Value,
    ) -> Result<()> {
        self.require_recent_authentication(authenticated)?;
        self.authentication.insert_passkey(
            authenticated.principal.user_id,
            credential_id,
            credential,
        )
    }
    pub fn complete_verified_passkey_authentication(
        &self,
        user: UserId,
        passkey: PasskeyId,
        expected_revision: Revision,
        expected_record: serde_json::Value,
        credential: serde_json::Value,
        label: &str,
    ) -> Result<(String, AuthenticatedPrincipal)> {
        self.authentication.complete_passkey_authentication(
            user,
            passkey,
            expected_revision,
            expected_record,
            credential,
            label,
        )
    }
    pub fn revoke_passkey(
        &self,
        authenticated: &AuthenticatedPrincipal,
        passkey: PasskeyId,
        expected: Revision,
        context: &CommandContext,
    ) -> Result<()> {
        self.require_recent_authentication(authenticated)?;
        let actor = &authenticated.principal;
        self.execute_command(
            actor,
            context,
            "passkey.revoke",
            &(passkey, expected),
            |effects| effects.revoke_passkey(actor.user_id, passkey, expected),
        )
    }
    pub fn grant(
        &self,
        authenticated: &AuthenticatedPrincipal,
        collection: CollectionId,
        user: UserId,
        role: Role,
        context: &CommandContext,
    ) -> Result<()> {
        self.require_recent_authentication(authenticated)?;
        let actor = &authenticated.principal;
        self.authorize(actor, collection, Action::ManageAccess)?;
        let current = self.access.grant_for_user(collection, user)?;
        let expected = current.map_or(Revision(0), |grant| grant.revision);
        let request = (collection, user, role);
        self.execute_command(actor, context, "grant.set", &request, |effects| {
            effects.set_grant(collection, user, expected, Some(role))
        })
        .map(|_| ())
    }
    pub fn grants(&self, actor: &Principal, collection: CollectionId) -> Result<Vec<GrantView>> {
        self.authorize(actor, collection, Action::ManageAccess)?;
        self.access.grants(collection)
    }
    pub fn set_grant(
        &self,
        authenticated: &AuthenticatedPrincipal,
        collection: CollectionId,
        username: &str,
        expected: Revision,
        role: Option<Role>,
        context: &CommandContext,
    ) -> Result<Option<GrantView>> {
        self.require_recent_authentication(authenticated)?;
        let actor = &authenticated.principal;
        self.authorize(actor, collection, Action::ManageAccess)?;
        let user = self.authentication.user_by_name(username)?;
        let request = (collection, user, expected, role);
        self.execute_command(actor, context, "grant.update", &request, |effects| {
            effects.set_grant(collection, user, expected, role)
        })
    }
}

fn deterministic_collection_id(user: UserId, key: &str) -> CollectionId {
    use sha2::{Digest as _, Sha256};
    let mut bytes: [u8; 16] =
        Sha256::digest(format!("junk-collector:collection-create:{user}:{key}"))[..16]
            .try_into()
            .expect("SHA-256 prefix has fixed length");
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    CollectionId(uuid::Uuid::from_bytes(bytes))
}

fn deterministic_asset_id(user: UserId, key: &str) -> AssetId {
    use sha2::{Digest as _, Sha256};
    let mut bytes: [u8; 16] = Sha256::digest(format!("junk-collector:cover-publish:{user}:{key}"))
        [..16]
        .try_into()
        .expect("SHA-256 prefix has fixed length");
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    AssetId(uuid::Uuid::from_bytes(bytes))
}
