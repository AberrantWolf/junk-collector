//! Policy for reversible Item visibility. Stored originals and inventory are
//! deliberately outside this command's mutation surface.
use super::*;

impl CollectionService {
    pub(crate) fn asset_read_role(
        &self,
        actor: &Principal,
        collection_id: CollectionId,
        subject: AssetSubject,
    ) -> Result<Role> {
        let role = self.authorize(actor, collection_id, Action::Browse)?;
        if let Some(item_id) = self.content.asset_subject_item(subject)? {
            let item = self.reader.item(item_id)?;
            if item.collection_id != collection_id {
                return Err(Error::NotFound);
            }
            if item.deleted {
                self.authorize(actor, collection_id, Action::Curate)?;
            }
        }
        Ok(role)
    }

    pub fn set_item_removed(
        &self,
        actor: &Principal,
        id: ItemId,
        expected: Revision,
        removed: bool,
        context: &CommandContext,
    ) -> Result<PresentedItem> {
        let item = self.reader.item(id)?;
        if item.lifecycle.merged_into.is_some() {
            return Err(Error::Invalid(
                "Use guarded group undo to restore merged entries.".into(),
            ));
        }
        self.content_write_eligibility(actor, item.collection_id)?;
        self.execute_command(
            actor,
            context,
            if removed {
                "item.remove"
            } else {
                "item.restore"
            },
            &(id, expected, removed),
            |effects| effects.set_item_removed(id, expected, removed),
        )?;
        self.item_presented(actor, id)
    }

    /// Permanently delete a removed entry: its original files, saved formats,
    /// covers, metadata, evidence, import records and history. This can't be
    /// undone. Entries that were part of a combined album are refused; they
    /// can only stay removed.
    pub fn purge_item(
        &self,
        actor: &Principal,
        id: ItemId,
        expected: Revision,
    ) -> Result<ItemPurgeResult> {
        let item = self.reader.item(id)?;
        self.authorize(actor, item.collection_id, Action::Curate)?;
        self.content_write_eligibility(actor, item.collection_id)?;
        let freed_bytes = self.writer.purge_removed_item(id, expected)?;
        Ok(ItemPurgeResult { freed_bytes })
    }

    /// Permanently delete chosen originals of an entry, keeping the entry. Used
    /// when a disc was given away, or imported twice by mistake. This can't be
    /// undone; combines that included these files can no longer be undone.
    pub fn purge_item_files(
        &self,
        actor: &Principal,
        id: ItemId,
        command: PurgeItemFiles,
    ) -> Result<ItemPurgeResult> {
        let item = self.reader.item(id)?;
        self.authorize(actor, item.collection_id, Action::Curate)?;
        self.content_write_eligibility(actor, item.collection_id)?;
        let freed_bytes = self.writer.purge_item_files(
            id,
            command.expected_revision,
            &command.captures,
            command.remove_copies,
        )?;
        Ok(ItemPurgeResult { freed_bytes })
    }
}

/// Originals to delete permanently from an entry that stays.
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PurgeItemFiles {
    pub expected_revision: Revision,
    /// The originals (discs or purchases) to delete.
    pub captures: Vec<CaptureId>,
    /// Also delete the physical copy records these originals alone came from.
    pub remove_copies: bool,
}

/// What a permanent deletion did.
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ItemPurgeResult {
    /// Disk space given back by deleting the entry's files.
    pub freed_bytes: u64,
}

/// Evaluate replacement scope and authority resets from writer-locked facts.
pub fn prepare_clean_reimport(
    mut item: Item,
    record: &ImportRecord,
    options: &CleanReimportOptions,
    metadata_revision: Revision,
) -> Result<Item> {
    if Some(item.revision) != record.view.expected_item_revision
        || options.replace_captures.is_empty()
        || options.replace_captures.len() > item.captures.len()
        || options
            .replace_captures
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != options.replace_captures.len()
        || options
            .replace_captures
            .iter()
            .any(|id| !item.captures.iter().any(|capture| capture.id == *id))
    {
        return Err(Error::Conflict);
    }
    if metadata_revision != options.expected_metadata_revision {
        return Err(Error::Conflict);
    }
    if let Some(representation) = options.retained_representation {
        let source = item
            .captures
            .iter()
            .filter(|capture| options.replace_captures.contains(&capture.id))
            .flat_map(|capture| &capture.representations)
            .find(|value| {
                value.id == representation && value.role == RepresentationRole::PreservationMaster
            })
            .ok_or(Error::Conflict)?;
        if source.files.len() != record.view.files.len()
            || source
                .files
                .iter()
                .zip(&record.view.files)
                .any(|(source, target)| {
                    source.name != target.name
                        || source.bytes != target.bytes
                        || Some(&source.digest) != target.digest.as_ref()
                })
        {
            return Err(Error::Conflict);
        }
    }
    item.lifecycle.generation = item
        .lifecycle
        .generation
        .checked_add(1)
        .ok_or(Error::Conflict)?;
    // A previous replacement may already have committed a Capture while its
    // final Item switch is unfinished. Superseding that intent must keep the
    // reserved Capture hidden even after its bookkeeping recovers.
    if let Some(previous) = &item.lifecycle.reimport
        && !previous.completed
        && !item
            .lifecycle
            .historical_captures
            .contains(&previous.capture_id)
    {
        item.lifecycle.historical_captures.push(previous.capture_id);
    }
    item.lifecycle.reimport = Some(ItemReimport {
        import_id: record.view.id,
        capture_id: record.view.capture_id,
        replaced_captures: options.replace_captures.clone(),
        completed: false,
    });
    // Revision cutoffs reset effective authority, never its historical records.
    item.lifecycle.automatic_metadata_through = metadata_revision.0;
    if options.reset_metadata {
        item.lifecycle.reset_fields_through = metadata_revision.0;
    }
    if options.reset_artwork {
        item.lifecycle.reset_artwork_through = metadata_revision.0;
    }
    Ok(item)
}
