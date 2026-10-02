use std::collections::BTreeMap;

use collector_core::{
    AssetId, Digest, Error, LanguageTag, MatchStrength, MetadataCandidateIdentity,
    MetadataCandidateObservation, MetadataText, RawJson, Result,
};
use serde::{Deserialize, Serialize};

pub const MUSICBRAINZ_PROVIDER_ID: &str = "org.junk.providers.musicbrainz";
pub const MUSICBRAINZ_ACQUIRE_OPERATION: &str = "org.junk.providers.musicbrainz.acquire";
pub const COVER_ART_ARCHIVE_PROVIDER_ID: &str = "org.junk.providers.cover-art-archive";
pub const COVER_ART_ARCHIVE_ACQUIRE_OPERATION: &str =
    "org.junk.providers.cover-art-archive.acquire";
pub const MUSICBRAINZ_SNAPSHOT_SCHEMA_VERSION: u32 = 1;
pub const COVER_ART_ARCHIVE_SNAPSHOT_SCHEMA_VERSION: u32 = 1;
pub const MUSICBRAINZ_PROVIDER_PRIORITY: u16 = 20;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MusicBrainzQuery {
    ReleaseId {
        release_id: String,
        #[serde(default)]
        refresh: bool,
    },
    DiscId {
        disc_id: String,
        #[serde(default)]
        refresh: bool,
    },
    Search {
        query: String,
        #[serde(default)]
        refresh: bool,
    },
}

impl MusicBrainzQuery {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::ReleaseId { release_id, .. } => validate_mbid(release_id),
            Self::DiscId { disc_id, .. } => {
                if disc_id.is_empty()
                    || disc_id.len() > 255
                    || !disc_id.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
                    })
                {
                    return Err(Error::Invalid("invalid MusicBrainz disc identifier".into()));
                }
                Ok(())
            }
            Self::Search { query, .. } => {
                if query.trim().is_empty()
                    || query.len() > 255
                    || query.chars().any(char::is_control)
                {
                    return Err(Error::Invalid("invalid MusicBrainz search".into()));
                }
                Ok(())
            }
        }
    }

    pub fn refresh(&self) -> bool {
        match self {
            Self::ReleaseId { refresh, .. }
            | Self::DiscId { refresh, .. }
            | Self::Search { refresh, .. } => *refresh,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetadataProviderOutcome {
    Candidates,
    NoMatch,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MusicBrainzSnapshotObservation {
    pub http_status: u16,
    pub schema_version: u32,
    pub query: MusicBrainzQuery,
    pub outcome: MetadataProviderOutcome,
    pub release_count: u16,
}

impl MusicBrainzSnapshotObservation {
    pub fn validate(&self) -> Result<()> {
        self.query.validate()?;
        if !matches!(self.http_status, 200 | 404)
            || (self.http_status == 404 && self.outcome != MetadataProviderOutcome::NoMatch)
            || self.schema_version != MUSICBRAINZ_SNAPSHOT_SCHEMA_VERSION
            || (matches!(self.outcome, MetadataProviderOutcome::Candidates)
                != (self.release_count > 0))
            || self.release_count > 128
        {
            return Err(Error::Invalid(
                "invalid MusicBrainz snapshot observation".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MusicBrainzReleaseObservation {
    pub release_id: String,
    pub title: String,
    pub disambiguation: Option<String>,
    pub language: Option<String>,
    pub script: Option<String>,
    pub artist_credit: Option<String>,
    pub artist_ids: Vec<String>,
    pub release_group_id: Option<String>,
    pub disc_ids: Vec<String>,
    #[serde(default)]
    pub media: Vec<MusicBrainzMediumObservation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MusicBrainzMediumObservation {
    pub position: u16,
    pub disc_ids: Vec<String>,
    pub tracks: Vec<MusicBrainzTrackObservation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MusicBrainzTrackObservation {
    pub position: u16,
    pub title: String,
}

impl MusicBrainzReleaseObservation {
    pub fn validate(&self) -> Result<()> {
        validate_mbid(&self.release_id)?;
        validate_text(&self.title, false)?;
        for value in self
            .disambiguation
            .iter()
            .chain(self.language.iter())
            .chain(self.script.iter())
            .chain(self.artist_credit.iter())
            .chain(self.release_group_id.iter())
            .chain(self.artist_ids.iter())
            .chain(self.disc_ids.iter())
        {
            validate_text(value, false)?;
        }
        if self.artist_ids.len() > 128 || self.disc_ids.len() > 128 {
            return Err(Error::Invalid(
                "MusicBrainz release identifiers are oversized".into(),
            ));
        }
        if self.media.len() > 128 {
            return Err(Error::Invalid("too many MusicBrainz media".into()));
        }
        let mut positions = std::collections::BTreeSet::new();
        for medium in &self.media {
            if medium.position == 0
                || !positions.insert(medium.position)
                || medium.tracks.len() > 256
                || medium.disc_ids.len() > 128
            {
                return Err(Error::Invalid("invalid MusicBrainz medium".into()));
            }
            let mut track_positions = std::collections::BTreeSet::new();
            for track in &medium.tracks {
                if track.position == 0
                    || track.position > 256
                    || !track_positions.insert(track.position)
                {
                    return Err(Error::Invalid("invalid MusicBrainz track position".into()));
                }
                validate_text(&track.title, false)?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "query", rename_all = "snake_case", deny_unknown_fields)]
pub enum CoverArtArchiveQuery {
    Listing {
        release_id: String,
    },
    Image {
        release_id: String,
        image_id: String,
    },
}

impl CoverArtArchiveQuery {
    pub fn validate(&self) -> Result<()> {
        let (release_id, image_id) = match self {
            Self::Listing { release_id } => (release_id, None),
            Self::Image {
                release_id,
                image_id,
            } => (release_id, Some(image_id)),
        };
        validate_mbid(release_id)?;
        if image_id.is_some_and(|value| {
            value.is_empty() || value.len() > 32 || !value.bytes().all(|byte| byte.is_ascii_digit())
        }) {
            return Err(Error::Invalid("invalid Cover Art Archive image id".into()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverArtArchiveSnapshotObservation {
    pub http_status: u16,
    pub schema_version: u32,
    pub query: CoverArtArchiveQuery,
    pub outcome: MetadataProviderOutcome,
    pub image_count: u16,
    pub media_type: Option<String>,
}

impl CoverArtArchiveSnapshotObservation {
    pub fn validate(&self) -> Result<()> {
        self.query.validate()?;
        let listing = matches!(self.query, CoverArtArchiveQuery::Listing { .. });
        if !matches!(self.http_status, 200 | 404)
            || (self.http_status == 404 && self.outcome != MetadataProviderOutcome::NoMatch)
            || (listing && self.media_type.is_some())
            || (!listing
                && self.http_status == 200
                && !matches!(
                    self.media_type.as_deref(),
                    Some("image/png" | "image/jpeg" | "image/webp")
                ))
            || self.schema_version != COVER_ART_ARCHIVE_SNAPSHOT_SCHEMA_VERSION
            || self.image_count > 128
            || (listing
                && (matches!(self.outcome, MetadataProviderOutcome::Candidates)
                    != (self.image_count > 0)))
            || (!listing && self.image_count != 0)
            || self
                .media_type
                .as_ref()
                .is_some_and(|value| value.len() > 255 || value.chars().any(char::is_control))
        {
            return Err(Error::Invalid(
                "invalid Cover Art Archive snapshot observation".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverArtObservation {
    pub release_id: String,
    pub image_id: String,
    pub front: bool,
    pub back: bool,
    pub approved: bool,
    pub types: Vec<String>,
    pub comment: String,
    pub edit_id: Option<u64>,
}

impl CoverArtObservation {
    pub fn validate(&self) -> Result<()> {
        CoverArtArchiveQuery::Image {
            release_id: self.release_id.clone(),
            image_id: self.image_id.clone(),
        }
        .validate()?;
        validate_text(&self.comment, true)?;
        if self.types.len() > 32 {
            return Err(Error::Invalid("too many Cover Art Archive types".into()));
        }
        for value in &self.types {
            validate_text(value, false)?;
        }
        Ok(())
    }
}

/// Convert neutral provider observations into music-owned candidate semantics.
/// Search scores and array order never affect authority. The exact response digest
/// participates in the derived version so a raw response cannot silently mutate a
/// previously observed candidate identity.
pub fn musicbrainz_metadata_candidates(
    response_digest: &Digest,
    snapshot: &MusicBrainzSnapshotObservation,
    releases: &[RawJson],
    artwork_by_release: &BTreeMap<String, AssetId>,
) -> Result<Vec<MetadataCandidateObservation>> {
    response_digest.validate()?;
    snapshot.validate()?;
    if releases.len() != usize::from(snapshot.release_count) {
        return Err(Error::Invalid(
            "MusicBrainz entries disagree with snapshot observation".into(),
        ));
    }
    let (complete_identity, strength) = match &snapshot.query {
        MusicBrainzQuery::ReleaseId { .. } => (true, MatchStrength::Identifier),
        MusicBrainzQuery::DiscId { disc_id, .. } => {
            for raw in releases {
                let release: MusicBrainzReleaseObservation = raw.parse()?;
                if !release.disc_ids.iter().any(|value| value == disc_id) {
                    return Err(Error::Invalid(
                        "MusicBrainz disc response does not contain the queried disc".into(),
                    ));
                }
            }
            (true, MatchStrength::DiscLayout)
        }
        MusicBrainzQuery::Search { .. } => (false, MatchStrength::Name),
    };
    let requested_release = match &snapshot.query {
        MusicBrainzQuery::ReleaseId { release_id, .. } => Some(release_id),
        _ => None,
    };
    let mut candidates = Vec::with_capacity(releases.len());
    for raw in releases {
        let release: MusicBrainzReleaseObservation = raw.parse()?;
        release.validate()?;
        if requested_release.is_some_and(|expected| expected != &release.release_id) {
            return Err(Error::Invalid(
                "MusicBrainz release response is misdirected".into(),
            ));
        }
        let mut version_input = response_digest.hex.as_bytes().to_vec();
        version_input.push(0);
        version_input.extend_from_slice(raw.get().as_bytes());
        // Rank and artwork are part of the immutable candidate, even when the
        // upstream response bytes did not change (for example, adding CAA art).
        version_input.extend_from_slice(
            &serde_json::to_vec(&(
                complete_identity,
                strength,
                artwork_by_release.get(&release.release_id),
            ))
            .map_err(|error| Error::Invalid(error.to_string()))?,
        );
        let version = Digest::sha256(&version_input);
        let language = release.language.as_deref().and_then(|value| {
            LanguageTag::new(match value {
                "eng" => "en",
                "jpn" => "ja",
                other => other,
            })
            .ok()
        });
        candidates.push(MetadataCandidateObservation {
            identity: MetadataCandidateIdentity {
                provider: collector_core::ModuleId(MUSICBRAINZ_PROVIDER_ID.into()),
                provider_record_id: release.release_id.clone(),
                provider_record_version: Some(format!("sha256:{}", version.hex)),
            },
            title: MetadataText {
                value: release.title.clone(),
                language: language.clone(),
            },
            subtitle: release.artist_credit.as_ref().map(|value| MetadataText {
                value: value.clone(),
                language,
            }),
            artwork: artwork_by_release.get(&release.release_id).copied(),
            complete_identity,
            strength,
            provider_priority: MUSICBRAINZ_PROVIDER_PRIORITY,
            observation: raw.clone(),
        });
    }
    candidates.sort_by(|left, right| left.identity.cmp(&right.identity));
    Ok(candidates)
}

fn validate_mbid(value: &str) -> Result<()> {
    let valid = value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        });
    if valid {
        Ok(())
    } else {
        Err(Error::Invalid("invalid MusicBrainz identifier".into()))
    }
}

fn validate_text(value: &str, empty_allowed: bool) -> Result<()> {
    if (!empty_allowed && value.trim().is_empty())
        || value.len() > 4096
        || value.chars().any(char::is_control)
    {
        Err(Error::Invalid("invalid provider metadata text".into()))
    } else {
        Ok(())
    }
}
