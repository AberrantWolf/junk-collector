//! Saved formats: extra copies (FLAC, Opus) kept beside the originals because
//! someone asked for them. Originals are never listed or removed here.
use super::*;
use crate::library::LibraryArtwork;

/// One saved format of one album.
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SavedFormatSummary {
    pub profile: ConversionProfile,
    /// Tracks saved in this format.
    pub tracks: u32,
    pub bytes: u64,
    /// Who asked for it; empty when every copy predates this record.
    pub requested_by: Vec<FormatRequest>,
    /// Some copies were made before the reason was recorded.
    pub unknown_origin: bool,
    /// Captures holding this format, with the revision each was listed at.
    pub captures: Vec<SavedFormatCapture>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SavedFormatCapture {
    pub capture_id: CaptureId,
    pub revision: Revision,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SavedFormatAlbum {
    pub item_id: ItemId,
    pub collection_id: CollectionId,
    pub title: String,
    pub subtitle: String,
    pub artwork: Option<LibraryArtwork>,
    /// Tracks the album plays, for spotting partial formats.
    pub album_tracks: u32,
    pub formats: Vec<SavedFormatSummary>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SavedFormatTotal {
    pub profile: ConversionProfile,
    pub albums: u64,
    pub bytes: u64,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct SavedFormatsPage {
    pub items: Vec<SavedFormatAlbum>,
    pub next_cursor: Option<String>,
    /// Totals across every album the query matches, not just this page.
    pub totals: Vec<SavedFormatTotal>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SavedFormatsQuery {
    pub collection_id: Option<CollectionId>,
    #[serde(default)]
    pub search: String,
    /// "flac" or "opus:<kbps>".
    pub format: Option<String>,
    pub requested_by: Option<FormatRequest>,
    /// Only albums where a saved format covers fewer tracks than the album has.
    #[serde(default)]
    pub partial_only: bool,
    /// Only albums with more than one Opus bitrate.
    #[serde(default)]
    pub several_opus: bool,
    pub cursor: Option<String>,
    pub limit: Option<u16>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RemoveSavedFormat {
    pub capture_id: CaptureId,
    pub expected_revision: Revision,
    pub profile: ConversionProfile,
}

/// What a removal did.
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct SavedFormatRemoval {
    pub album: SavedFormatAlbum,
    /// Disk space given back by deleting the removed copy's files.
    pub freed_bytes: u64,
}

/// The format key used by filters and totals.
pub fn saved_format_key(profile: &ConversionProfile) -> String {
    match profile {
        ConversionProfile::Flac => "flac".into(),
        ConversionProfile::Opus { bitrate_kbps } => format!("opus:{bitrate_kbps}"),
    }
}

/// Summarise the saved formats on an item's captures. Only copies this server
/// made from an original count (`made_here`: the copy has a conversion
/// recipe); originals never appear.
pub fn saved_formats_of(
    captures: &[Capture],
    made_here: &mut dyn FnMut(RepresentationId) -> Result<bool>,
) -> Result<Vec<SavedFormatSummary>> {
    let mut formats: Vec<SavedFormatSummary> = Vec::new();
    for capture in captures {
        for representation in &capture.representations {
            if representation.role == RepresentationRole::PreservationMaster
                || representation.source.is_none()
                || representation.profile.is_none()
                || !made_here(representation.id)?
            {
                continue;
            }
            let Some(profile) = representation.profile.clone() else {
                continue;
            };
            let index = match formats.iter().position(|f| f.profile == profile) {
                Some(index) => index,
                None => {
                    formats.push(SavedFormatSummary {
                        profile,
                        tracks: 0,
                        bytes: 0,
                        requested_by: vec![],
                        unknown_origin: false,
                        captures: vec![],
                    });
                    formats.len() - 1
                }
            };
            let summary = &mut formats[index];
            summary.tracks += 1;
            summary.bytes += representation.files.iter().map(|f| f.bytes.0).sum::<u64>();
            match representation.requested_by {
                Some(request) if !summary.requested_by.contains(&request) => {
                    summary.requested_by.push(request)
                }
                Some(_) => {}
                None => summary.unknown_origin = true,
            }
            if !summary.captures.iter().any(|c| c.capture_id == capture.id) {
                summary.captures.push(SavedFormatCapture {
                    capture_id: capture.id,
                    revision: capture.revision,
                });
            }
        }
    }
    formats.sort_by_key(|f| saved_format_key(&f.profile));
    Ok(formats)
}

impl CollectionService {
    pub fn saved_formats(
        &self,
        actor: &Principal,
        query: SavedFormatsQuery,
    ) -> Result<SavedFormatsPage> {
        if let Some(collection) = query.collection_id {
            self.authorize(actor, collection, Action::Curate)?;
        }
        let page = self.care.saved_format_albums(actor.user_id, &query)?;
        for album in &page.items {
            self.authorize(actor, album.collection_id, Action::Curate)?;
        }
        Ok(page)
    }

    /// Remove one saved format from one capture and delete its files. Only
    /// copies this server made can be removed; originals (including FLAC or
    /// MP3 files that came with an import) go only with the whole entry.
    /// Playback falls back to live conversion of the original.
    pub fn remove_saved_format(
        &self,
        actor: &Principal,
        item_id: ItemId,
        command: RemoveSavedFormat,
        context: &CommandContext,
    ) -> Result<SavedFormatRemoval> {
        let item = self.reader.item(item_id)?;
        self.authorize(actor, item.collection_id, Action::Curate)?;
        self.execute_command(
            actor,
            context,
            "saved_format.remove",
            &(item_id, &command),
            |effects| {
                self.content_write_eligibility(actor, item.collection_id)?;
                let mut capture = self.reader.capture(command.capture_id)?;
                if capture.collection_id != item.collection_id
                    || capture.association.item_id != Some(item_id)
                    || item.deleted
                {
                    return Err(Error::Forbidden);
                }
                let before = capture.representations.len();
                let mut kept = Vec::new();
                for representation in std::mem::take(&mut capture.representations) {
                    let candidate = representation.role != RepresentationRole::PreservationMaster
                        && representation.source.is_some()
                        && representation.profile.as_ref() == Some(&command.profile);
                    // Only a copy with a conversion recipe was made here.
                    let made_here = candidate
                        && match self.processing.recipe(RecipeId(representation.id.0)) {
                            Ok(_) => true,
                            Err(Error::NotFound) => false,
                            Err(error) => return Err(error),
                        };
                    if !made_here {
                        kept.push(representation);
                    }
                }
                capture.representations = kept;
                if capture.representations.len() == before {
                    return Err(Error::Invalid(
                        "This capture has no saved copy in that format.".into(),
                    ));
                }
                effects.save_capture(capture, command.expected_revision)?;
                Ok(())
            },
        )?;
        // Runs on a replayed request too, so a crash after the record was
        // saved still frees the space on retry.
        let freed_bytes = self
            .processing
            .discard_unlisted_derivatives(command.capture_id)?;
        let item = self.reader.item(item_id)?;
        let captures = item
            .captures
            .iter()
            .map(|c| self.reader.capture(c.id))
            .collect::<Result<Vec<_>>>()?;
        Ok(SavedFormatRemoval {
            album: SavedFormatAlbum {
                item_id,
                collection_id: item.collection_id,
                title: item.title.clone(),
                subtitle: item.subtitle.clone(),
                artwork: None,
                album_tracks: 0,
                formats: saved_formats_of(&captures, &mut |id| match self
                    .processing
                    .recipe(RecipeId(id.0))
                {
                    Ok(_) => Ok(true),
                    Err(Error::NotFound) => Ok(false),
                    Err(error) => Err(error),
                })?,
            },
            freed_bytes,
        })
    }
}
