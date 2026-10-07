//! Shared transport contracts; no application policy or storage mechanisms.
use collector_core::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CollectionListSummary {
    pub id: CollectionListId,
    pub collection_id: CollectionId,
    pub revision: Revision,
    pub name: String,
    pub kind: CollectionListKind,
    pub entry_count: u32,
    pub can_edit: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CollectionListsQuery {
    pub search: Option<String>,
    pub cursor: Option<CollectionListId>,
    pub limit: Option<u16>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ListEntriesQuery {
    pub cursor: Option<String>,
    pub limit: Option<u16>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateCollectionList {
    pub name: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum CollectionListChange {
    Rename {
        name: String,
    },
    Delete,
    Remove {
        entry_id: ListEntryId,
    },
    Move {
        entry_id: ListEntryId,
        before: Option<ListEntryId>,
    },
    Append {
        item_id: ItemId,
        source_representation_id: Option<RepresentationId>,
        track_position: Option<u16>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateCollectionList {
    pub expected_revision: Revision,
    pub change: CollectionListChange,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ListEntryAvailability {
    Entry,
    Preparing,
    NeedsPreparation,
    SourceUnavailable,
    EntryRemoved,
    ModuleUnavailable,
    Available {
        representation_id: RepresentationId,
        file_index: usize,
        media_type: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PresentedListEntry {
    pub source: Option<ListTarget>,
    pub audio: Option<MusicTrackAvailability>,
    pub id: ListEntryId,
    pub position: u32,
    pub item_id: Option<ItemId>,
    pub title: String,
    pub album: Option<String>,
    pub disc_position: Option<u16>,
    pub track_position: Option<u16>,
    pub artwork: Option<AssetId>,
    pub availability: ListEntryAvailability,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CollectionListView {
    pub list: CollectionListSummary,
    pub entries: Page<PresentedListEntry>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum MusicTrackAvailabilityState {
    Preparing,
    Unavailable {
        reason: String,
    },
    Available {
        representation_id: RepresentationId,
        file_id: collector_core::PackageFileId,
        file_index: usize,
        length: ByteLength,
        media_type: String,
        digest: Digest,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MusicTrackAvailability {
    pub capture_id: CaptureId,
    pub source_label: String,
    pub disc_position: Option<u16>,
    pub source_representation_id: RepresentationId,
    pub track_position: u16,
    /// Exact coordinate in the source's integer stereo PCM timeline.
    pub start_frame: PcmFrame,
    /// Exact track-local integer stereo PCM frame count.
    pub frame_count: PcmFrame,
    pub title: String,
    pub profile: ConversionProfile,
    pub state: MusicTrackAvailabilityState,
    /// Current actor, module, source and encoder permit explicit preparation.
    pub can_prepare: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MusicSearchQuery {
    pub search: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<u16>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MusicSearchRecord {
    pub duration_ms: Option<u64>,
    pub item_id: ItemId,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub disc_position: Option<u16>,
    pub track_position: u16,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MusicSearchHit {
    pub song: MusicSearchRecord,
    pub artwork: Option<AssetId>,
    /// Source-specific coordinates; empty when no original has been observed.
    pub sources: Vec<MusicTrackAvailability>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ClientCollection {
    pub id: CollectionId,
    pub name: String,
    pub can_browse: bool,
    pub can_play: bool,
    pub can_edit_playlists: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ClientAlbum {
    pub id: ItemId,
    pub collection_id: CollectionId,
    pub title: String,
    pub artist: String,
    pub artwork: Option<AssetId>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AlbumPlaybackSnapshot {
    pub item_id: ItemId,
    pub collection_id: CollectionId,
    pub title: String,
    pub artist: String,
    pub tracks: Vec<MusicTrackAvailability>,
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
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct EnrollmentView {
    pub user_id: Option<UserId>,
    pub id: EnrollmentId,
    pub requested_label: String,
    pub phase: EnrollmentPhase,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct StartEnrollment {
    pub device_name: String,
}
#[derive(Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ApproveEnrollment {
    pub user_code: String,
}
#[derive(Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CompleteEnrollment {
    pub completion_secret: String,
}
#[derive(Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct NativeCredentialIssue {
    pub user_id: UserId,
    pub username: String,
    pub display_name: String,
    pub device_id: NativeCredentialId,
    pub credential: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}
#[derive(Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct EnrollmentCompletion<P> {
    #[serde(flatten)]
    pub issue: NativeCredentialIssue,
    pub pairing: P,
}
