//! An immutable group marker is the sole archive commit point for grouped revisions.
//! Members use ordinary entity envelopes; archive traversal hides them until commit.
use super::*;
use std::collections::BTreeSet;
use std::io::Read;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GroupMember {
    pub path: String,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GroupManifest {
    pub schema_version: u32,
    pub transaction_id: TransactionId,
    pub collection_id: CollectionId,
    pub members: Vec<GroupMember>,
    pub decision: RawJson,
}
pub(crate) struct GroupRevision {
    pub path: String,
    pub files: Vec<(String, Vec<u8>)>,
}
impl GroupManifest {
    fn validate(&self) -> Result<()> {
        if self.schema_version != 1 || self.members.is_empty() || self.members.len() > 2048 {
            return Err(Error::Storage("invalid archive publication group".into()));
        }
        let mut paths = BTreeSet::new();
        for member in &self.members {
            publication::validate_relative_path(&member.path)?;
            Digest::new(HashAlgorithm::Sha256, member.sha256.clone())?;
            if !paths.insert(&member.path) {
                return Err(Error::Storage("duplicate publication group member".into()));
            }
        }
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MemberMarker {
    manifest: String,
    transaction_id: TransactionId,
}

fn bounded_metadata(path: &Path, maximum: u64) -> Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path).map_err(storage)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > maximum {
        return Err(Error::Storage(
            "group metadata is not a bounded regular file".into(),
        ));
    }
    let mut file = File::open(path).map_err(storage)?;
    let mut bytes = Vec::new();
    std::io::Read::by_ref(&mut file)
        .take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(storage)?;
    if bytes.len() as u64 > maximum {
        return Err(Error::Storage("group metadata exceeds its limit".into()));
    }
    Ok(bytes)
}

fn sync_parents(path: &Path, archive: &Path) -> Result<()> {
    for directory in path.ancestors().take_while(|p| p.starts_with(archive)) {
        sync_directory(directory)?;
    }
    Ok(())
}

pub(crate) fn visible_revision(path: &Path) -> Result<bool> {
    let marker_path = path.join("group.json");
    if !marker_path.try_exists().map_err(storage)? {
        return Ok(true);
    }
    let marker: MemberMarker =
        serde_json::from_slice(&bounded_metadata(&marker_path, 4096)?).map_err(storage)?;
    publication::validate_relative_path(&marker.manifest)?;
    let archive = path
        .ancestors()
        .find(|ancestor| ancestor.join(".transactions").is_dir())
        .ok_or_else(|| Error::Storage("group member has no archive root".into()))?;
    let commit = archive.join(&marker.manifest);
    if !commit.try_exists().map_err(storage)? {
        return Ok(false);
    }
    let manifest: GroupManifest =
        serde_json::from_slice(&bounded_metadata(&commit, 32 * 1024 * 1024)?).map_err(storage)?;
    manifest.validate()?;
    if marker.transaction_id != manifest.transaction_id
        || !manifest
            .members
            .iter()
            .any(|member| member.path == relative(archive, &marker_path).unwrap_or_default())
    {
        return Err(Error::Storage(
            "group member does not belong to its committed manifest".into(),
        ));
    }
    let collection_prefix = marker
        .manifest
        .strip_suffix(&format!(
            "/publication-groups/{}.json",
            manifest.transaction_id
        ))
        .ok_or_else(|| Error::Storage("invalid publication group location".into()))?;
    if !path.starts_with(archive.join(collection_prefix)) {
        return Err(Error::Storage(
            "group member crosses collection boundary".into(),
        ));
    }
    // Verify all member metadata before exposing any of this group. No media payloads.
    for member in &manifest.members {
        if !member.path.starts_with(&format!("{collection_prefix}/")) {
            return Err(Error::Storage(
                "group manifest crosses collection boundary".into(),
            ));
        }
        let member_path = archive.join(&member.path);
        if !fs::canonicalize(&member_path)
            .map_err(storage)?
            .starts_with(fs::canonicalize(archive.join(collection_prefix)).map_err(storage)?)
        {
            return Err(Error::Storage(
                "group metadata escaped its collection".into(),
            ));
        }
        if fs::symlink_metadata(&member_path)
            .map_err(storage)?
            .file_type()
            .is_symlink()
            || Digest::sha256(&bounded_metadata(&member_path, 32 * 1024 * 1024)?).hex
                != member.sha256
        {
            return Err(Error::Storage(
                "committed group member is missing or changed".into(),
            ));
        }
    }
    Ok(true)
}

pub(crate) fn publish(
    store: &Store,
    transaction: TransactionId,
    collection: CollectionId,
    revisions: Vec<GroupRevision>,
    decision: RawJson,
    guard: &mut dyn FnMut() -> Result<()>,
) -> Result<Vec<String>> {
    if revisions.is_empty() || revisions.len() > 256 {
        return Err(Error::Invalid(
            "publication group requires 1–256 revisions".into(),
        ));
    }
    let collection_path = collection_path(&*lock(&store.db)?, &store.archive, collection)?;
    let prefix = relative(&store.archive, &collection_path)?;
    let manifest_path = format!("{prefix}/publication-groups/{transaction}.json");
    let marker = serde_json::to_vec(&MemberMarker {
        manifest: manifest_path.clone(),
        transaction_id: transaction,
    })
    .map_err(storage)?;
    let stage_path = format!(".staging/{transaction}");
    let mut artifacts = Vec::new();
    let mut members = Vec::new();
    let mut files = Vec::new();
    for (index, mut revision) in revisions.into_iter().enumerate() {
        publication::validate_relative_path(&revision.path)?;
        if !revision.path.starts_with(&format!("{prefix}/"))
            || !revision.path.contains("/revisions/")
        {
            return Err(Error::Invalid(
                "group revision is outside its collection".into(),
            ));
        }
        revision.files.push(("group.json".into(), marker.clone()));
        for (name, bytes) in revision.files {
            if name.contains('/') || bytes.len() > 32 * 1024 * 1024 {
                return Err(Error::Invalid(
                    "group metadata file is invalid or too large".into(),
                ));
            }
            let destination = format!("{}/{name}", revision.path);
            let staged = format!("{stage_path}/{index}/{name}");
            let sha256 = Digest::sha256(&bytes).hex;
            members.push(GroupMember {
                path: destination.clone(),
                sha256: sha256.clone(),
            });
            artifacts.push(publication::JournalArtifact {
                kind: publication::ArtifactKind::GroupRevision,
                staged_path: staged.clone(),
                published_path: destination,
                sha256,
            });
            files.push((staged, bytes));
        }
    }
    let manifest = GroupManifest {
        schema_version: 1,
        transaction_id: transaction,
        collection_id: collection,
        members,
        decision,
    };
    manifest.validate()?;
    let bytes = serde_json::to_vec_pretty(&manifest).map_err(storage)?;
    if bytes.len() > 32 * 1024 * 1024 {
        return Err(Error::Invalid("This group exceeds the 32 MiB metadata publication limit. Merge fewer entries at a time.".into()));
    }
    let digest = Digest::sha256(&bytes);
    let staged_commit = format!("{stage_path}/commit.json");
    artifacts.push(publication::JournalArtifact {
        kind: publication::ArtifactKind::GroupCommit,
        staged_path: staged_commit.clone(),
        published_path: manifest_path.clone(),
        sha256: digest.hex.clone(),
    });
    let journal = publication::PublicationJournal::new(
        transaction,
        "publication_group",
        transaction.to_string(),
        Some(collection),
        Revision(0),
        Revision(1),
        artifacts,
    )?;
    store.prepare_command(
        "archive",
        &manifest_path,
        &digest.hex,
        &store.command_result,
    )?;
    let journals = publication::PublicationJournalStore::new(&store.archive);
    journals.write_new(&journal)?;
    store
        .publication_fault
        .check(PublicationFaultPoint::Prepared)?;
    files.push((staged_commit, bytes));
    for (path, bytes) in files {
        let path = store.archive.join(path);
        fs::create_dir_all(path.parent().ok_or(Error::Conflict)?).map_err(storage)?;
        write_new(&path, &bytes)?;
        sync_parents(path.parent().ok_or(Error::Conflict)?, &store.archive)?;
    }
    journals.advance(transaction, publication::PublicationPhase::BytesStaged)?;
    store
        .publication_fault
        .check(PublicationFaultPoint::BytesStaged)?;
    guard()?;
    for artifact in journal
        .artifacts
        .iter()
        .filter(|artifact| artifact.kind == publication::ArtifactKind::GroupRevision)
    {
        let target = store.archive.join(&artifact.published_path);
        fs::create_dir_all(target.parent().ok_or(Error::Conflict)?).map_err(storage)?;
        if target.try_exists().map_err(storage)? {
            return Err(Error::Conflict);
        }
        fs::rename(store.archive.join(&artifact.staged_path), &target).map_err(storage)?;
        sync_parents(target.parent().ok_or(Error::Conflict)?, &store.archive)?;
    }
    journals.advance(transaction, publication::PublicationPhase::PayloadPublished)?;
    store
        .publication_fault
        .check(PublicationFaultPoint::PayloadPublished)?;
    guard()?;
    let commit = store.archive.join(&manifest_path);
    fs::create_dir_all(commit.parent().ok_or(Error::Conflict)?).map_err(storage)?;
    fs::rename(
        store
            .archive
            .join(&journal.artifacts.last().ok_or(Error::Conflict)?.staged_path),
        &commit,
    )
    .map_err(storage)?;
    sync_directory(commit.parent().ok_or(Error::Conflict)?)?;
    sync_directory(&collection_path)?;
    journals.advance(
        transaction,
        publication::PublicationPhase::RevisionCommitted,
    )?;
    store
        .publication_fault
        .check(PublicationFaultPoint::RevisionCommitted)?;
    Ok(journal
        .artifacts
        .into_iter()
        .map(|artifact| artifact.published_path)
        .collect())
}
