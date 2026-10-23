//! Shared transport contracts; no application policy or storage mechanisms.
use collector_core::*;
use serde::{Deserialize, Serialize};

/// Authenticated control-plane advertisement; each endpoint has the same server identity.
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LocalPlaybackDescriptor {
    pub server_id: ServerId,
    pub endpoints: Vec<LocalPlaybackEndpoint>,
    pub remote_media_allowed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_media: Option<RemoteMediaEndpoint>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RemoteMediaEndpoint {
    pub base_url: String,
    pub server_id: ServerId,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RemoteGrantRequest {
    /// None requests a bounded synthetic reachability probe, never library access.
    pub target: Option<PlaybackTarget>,
}

#[derive(Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RemoteMediaGrant {
    pub token: String,
    pub url: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub server_id: ServerId,
    pub length: u64,
    pub etag: String,
}

#[derive(Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RemoteMediaSample {
    pub start: u64,
    pub end: u64,
    pub digest: Digest,
}
#[derive(Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RemoteMediaTest {
    pub grant: RemoteMediaGrant,
    pub samples: Vec<RemoteMediaSample>,
}

/// Socket-only DTOs; a media service credential cannot issue user grants.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaAuthorizeRequest {
    pub token: String,
    pub origin: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaFile {
    /// Relative to the separately configured read-only archive root.
    pub relative_path: Option<String>,
    /// Synthetic probe only; bounded by the authority client before decoding.
    pub inline: Option<String>,
    pub length: u64,
    pub digest: Digest,
    pub media_type: String,
    pub verify_on_transfer: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaReadTicket {
    pub handle: String,
    pub file: MediaFile,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaHandleRequest {
    pub handle: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LocalPlaybackEndpoint {
    pub https_url: String,
    pub webtransport_url: String,
    pub certificate_pem: String,
    /// SHA-256 of the DER leaf, as required by WebTransport serverCertificateHashes.
    pub certificate_sha256: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PlaybackTarget {
    pub item_id: ItemId,
    pub representation_id: RepresentationId,
    pub file_index: usize,
}

#[derive(Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BrowserPlaybackGrant {
    pub token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

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
    pub kind: Option<CollectionListKind>,
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
    pub artist: Option<String>,
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
    #[serde(default)]
    pub album_selection: MusicAlbumSelection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_position: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_format: Option<MusicAudioFormat>,
    pub capture_id: CaptureId,
    pub source_label: String,
    pub disc_position: Option<u16>,
    pub source_representation_id: RepresentationId,
    pub track_position: u16,
    /// Coordinate in the source PCM timeline at sample_rate(), with native channels.
    pub start_frame: PcmFrame,
    /// Decoded track-local frames; MP3 gapless knowledge is separate in audio_format.
    pub frame_count: PcmFrame,
    pub title: String,
    pub profile: Option<ConversionProfile>,
    pub state: MusicTrackAvailabilityState,
    /// Current actor, module, source and encoder permit explicit preparation.
    pub can_prepare: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MusicAlbumSelection {
    #[default]
    Included,
    Excluded,
    NeedsReview,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MusicAudioFormat {
    pub codec: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: Option<u32>,
    pub delay_frames: Option<u32>,
    pub padding_frames: Option<u32>,
}
impl MusicTrackAvailability {
    pub fn sample_rate(&self) -> u32 {
        self.audio_format
            .as_ref()
            .map_or(44_100, |format| format.sample_rate)
    }
    pub fn primary_playback(&self) -> bool {
        self.profile.is_none() || self.profile == Some(ConversionProfile::Flac)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MusicSearchQuery {
    pub search: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<u16>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MusicSearchRecord {
    #[serde(default)]
    pub source_representation_id: Option<RepresentationId>,
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
