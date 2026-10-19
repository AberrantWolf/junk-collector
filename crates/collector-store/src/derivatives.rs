//! Infrastructure-owned scratch and atomic Capture derivative publication.
use super::*;
use collector_app::{ConversionScratch, ConversionValidation, FrozenDerivativeJob, ProcessingPort};
use std::io::{Read, Seek, SeekFrom};

struct FileScratch(File);
impl Read for FileScratch {
    fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(b)
    }
}
impl Write for FileScratch {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.write(b)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}
impl Seek for FileScratch {
    fn seek(&mut self, p: SeekFrom) -> std::io::Result<u64> {
        self.0.seek(p)
    }
}
impl ConversionScratch for FileScratch {
    fn discard_partial(&mut self) -> std::io::Result<()> {
        self.0.set_len(0)?;
        self.0.rewind()
    }
}

pub(crate) fn scratch(store: &Store) -> Result<Box<dyn ConversionScratch>> {
    Ok(Box::new(FileScratch(
        tempfile::tempfile_in(&store.state).map_err(storage)?,
    )))
}

impl Store {
    pub(crate) fn derivative_is_committed(&self, frozen: &FrozenDerivativeJob) -> Result<bool> {
        frozen.validate()?;
        let _writer = lock(&self.writer)?;
        let mut capture = self.capture(frozen.capture_id)?;
        if !capture
            .representations
            .iter()
            .any(|r| r.id == frozen.representation_id)
        {
            let collection = {
                let db = lock(&self.db)?;
                collection_path(&db, &self.archive, frozen.collection_id)?
            };
            // A committed revision can precede its projection (including a lost
            // response in this process). Recover the original immutable records.
            if let Some(record) = migration_v3::read_captures(&collection, frozen.collection_id)?
                .into_iter()
                .find(|r| {
                    r.capture.id == frozen.capture_id
                        && r.capture
                            .representations
                            .iter()
                            .any(|r| r.id == frozen.representation_id)
                })
            {
                let recipe: ConversionRecipe = processing::read_v4(
                    &collection.join(format!(
                        "conversion_recipes/{}/revisions/{:020}",
                        frozen.recipe.id, 1
                    )),
                    frozen.collection_id,
                    "conversion_recipe",
                )?;
                if recipe.digest != frozen.recipe.digest {
                    return Err(Error::Conflict);
                }
                let evidence = migration_v3::read_evidence(&collection, frozen.collection_id)?
                    .into_iter()
                    .find(|e| e.id.0 == frozen.representation_id.0)
                    .ok_or(Error::NotFound)?;
                let mut db = lock(&self.db)?;
                let tx = db.transaction().map_err(storage)?;
                project_evidence_record(&tx, &evidence)?;
                tx.execute("INSERT OR IGNORE INTO conversion_recipes(id,collection_id,record) VALUES(?,?,?)", params![recipe.id.to_string(), frozen.collection_id.to_string(), serde_json::to_string(&recipe).map_err(storage)?]).map_err(storage)?;
                project_capture(&tx, &record.capture)?;
                self.validate_derivative_projection(&tx, true)?;
                tx.commit().map_err(storage)?;
                capture = record.capture;
            }
        }
        if capture.collection_id != frozen.collection_id {
            return Err(Error::Conflict);
        }
        let Some(output) = capture
            .representations
            .iter()
            .find(|r| r.id == frozen.representation_id)
        else {
            return Ok(false);
        };
        let recipe = self.recipe(frozen.recipe.id)?;
        if recipe.digest != frozen.recipe.digest
            || output.source
                != match frozen.recipe.source_fingerprint.source {
                    PackageScope::Representation(id) => Some(id),
                    _ => None,
                }
            || output.files.len() != 1
        {
            return Err(Error::Conflict);
        }
        let evidence: archive_v3::EvidenceRecordV3 = {
            let db = lock(&self.db)?;
            projected_record(&db, "evidence", &frozen.representation_id.to_string())?
        };
        validate_retained_derivative(&capture, output, &recipe, &evidence)?;
        Ok(true)
    }

    pub(crate) fn publish_music_derivative(
        &self,
        claim: &JobClaim,
        frozen: &FrozenDerivativeJob,
        validation: &ConversionValidation,
        scratch: &mut dyn ConversionScratch,
        checkpoint: &mut dyn FnMut() -> Result<()>,
    ) -> Result<()> {
        frozen.validate()?;
        validation.validate_recipe(&frozen.recipe)?;
        checkpoint()?;
        // Copy and verify the exact returned bytes outside the writer lock. All
        // temporary names/roots belong to this adapter; providers see handles only.
        let scratch_root = self.state.join("derivative-scratch");
        fs::create_dir_all(&scratch_root).map_err(storage)?;
        let spool = tempfile::Builder::new()
            .prefix(&format!("{}.{}.", claim.id, claim.claim_token))
            .tempdir_in(&scratch_root)
            .map_err(storage)?;
        let encoded = spool.path().join("encoded");
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&encoded)
            .map_err(storage)?;
        scratch.rewind().map_err(storage)?;
        let mut bytes = 0_u64;
        let mut buffer = vec![0; 1024 * 1024];
        loop {
            checkpoint()?;
            let count = scratch.read(&mut buffer).map_err(storage)?;
            if count == 0 {
                break;
            }
            bytes = bytes.checked_add(count as u64).ok_or(Error::Conflict)?;
            if bytes > validation.bytes.0 {
                return Err(Error::Conflict);
            }
            output.write_all(&buffer[..count]).map_err(storage)?;
        }
        output.sync_all().map_err(storage)?;
        if bytes != validation.bytes.0
            || hash_publication_file(&encoded, checkpoint)? != validation.digest
        {
            return Err(Error::Invalid(
                "validated derivative scratch changed".into(),
            ));
        }
        drop(output);
        let _writer = lock(&self.writer)?;
        let mut capture = self.capture(frozen.capture_id)?;
        if let Some(output) = capture
            .representations
            .iter()
            .find(|r| r.id == frozen.representation_id)
        {
            let recipe = self.recipe(frozen.recipe.id)?;
            if recipe.digest != frozen.recipe.digest {
                return Err(Error::Conflict);
            }
            let evidence = {
                let db = lock(&self.db)?;
                projected_record(&db, "evidence", &frozen.representation_id.to_string())?
            };
            validate_retained_derivative(&capture, output, &recipe, &evidence)?;
            return Ok(());
        }
        checkpoint()?;
        let collection = {
            let db = lock(&self.db)?;
            collection_path(&db, &self.archive, frozen.collection_id)?
        };
        let source = match frozen.recipe.source_fingerprint.source {
            PackageScope::Representation(id) => id,
            _ => return Err(Error::Conflict),
        };
        if !capture
            .representations
            .iter()
            .any(|r| r.id == source && matches!(r.role, RepresentationRole::PreservationMaster))
        {
            return Err(Error::Conflict);
        }
        let (format, role, profile) = match frozen.recipe.settings {
            ConversionSettings::Flac { .. } => (
                "flac",
                RepresentationRole::LosslessDerivative,
                ConversionProfile::Flac,
            ),
            ConversionSettings::Opus { bitrate_kbps, .. } => (
                "opus",
                RepresentationRole::LossyDerivative,
                ConversionProfile::Opus { bitrate_kbps },
            ),
        };
        let name = format!("track-{:02}.{format}", frozen.recipe.tracks[0].position);
        capture.representations.push(Representation {
            id: frozen.representation_id,
            role,
            format: format.into(),
            files: vec![StoredFile {
                visibility: Default::default(),
                name: name.clone(),
                bytes: validation.bytes,
                digest: validation.digest.clone(),
            }],
            source: Some(source),
            profile: Some(profile),
        });
        let expected = capture.revision;
        capture.revision = Revision(expected.0.checked_add(1).ok_or(Error::Conflict)?);
        capture.validate()?;
        let txid = TransactionId(claim.id.0);
        let journals = publication::PublicationJournalStore::new(&self.archive);
        if let Some(phase) = journals.phase(txid)? {
            if phase >= publication::PublicationPhase::RevisionCommitted {
                return Err(Error::Unavailable(
                    "committed derivative requires archive recovery".into(),
                ));
            }
            journals.abort(txid)?;
        }
        let evidence = archive_v3::EvidenceRecordV3 {
            id: EvidenceId(frozen.representation_id.0),
            collection_id: frozen.collection_id,
            item_id: None,
            representation_ids: vec![frozen.representation_id],
            kind: EvidenceKind::DerivativeValidation,
            outcome: EvidenceOutcome::Passed,
            inputs: vec![
                frozen.recipe.source_fingerprint.digest.clone(),
                frozen.recipe.digest.clone(),
            ],
            implementation: frozen.recipe.encoder.clone(),
            implementation_version: frozen.recipe.encoder_version.clone(),
            catalog: None,
            details: serde_json::json!({"recipe_id":frozen.recipe.id,"validation":validation,"claim": if validation.lossless_pcm_equal { "independently decoded integer PCM equals the selected source track" } else { "Opus structure and duration only; no lossless PCM equality" }}),
            created_at: frozen.created_at,
        };
        evidence.validate()?;
        let recipe_bytes = serde_json::to_vec_pretty(&frozen.recipe).map_err(storage)?;
        let recipe_manifest = processing::ManifestV4 {
            schema_version: 4,
            entity_type: "conversion_recipe".into(),
            entity_id: frozen.recipe.id.to_string(),
            collection_id: frozen.collection_id,
            revision: Revision(1),
            record_sha256: Digest::sha256(&recipe_bytes).hex,
            transaction_id: txid,
        };
        let evidence_bytes = serde_json::to_vec_pretty(&archive_v3::RevisionEnvelopeV3::new(
            txid,
            &evidence,
            vec![],
        ))
        .map_err(storage)?;
        let evidence_manifest = V3TypedManifest {
            schema_version: 3,
            entity_type: "evidence".into(),
            entity_id: evidence.id.to_string(),
            collection_id: frozen.collection_id,
            revision: Revision(1),
            record_sha256: Digest::sha256(&evidence_bytes).hex.clone(),
            transaction_id: Some(txid),
        };
        let root = relative(&self.archive, &collection)?;
        let mut payloads = vec![V3PayloadPublication {
            source: encoded,
            published_path: format!(
                "{root}/captures/{}/payload/{}/{name}",
                capture.id, frozen.representation_id
            ),
            sha256: validation.digest.hex.clone(),
        }];
        for (index, (path, data)) in [
            (
                format!(
                    "{root}/conversion_recipes/{}/revisions/{:020}/record.v4.json",
                    frozen.recipe.id, 1
                ),
                recipe_bytes,
            ),
            (
                format!(
                    "{root}/conversion_recipes/{}/revisions/{:020}/manifest.v4.toml",
                    frozen.recipe.id, 1
                ),
                toml::to_string_pretty(&recipe_manifest)
                    .map_err(storage)?
                    .into_bytes(),
            ),
            (
                format!(
                    "{root}/evidence/{}/revisions/{:020}/record.v3.json",
                    evidence.id, 1
                ),
                evidence_bytes.clone(),
            ),
            (
                format!(
                    "{root}/evidence/{}/revisions/{:020}/manifest.v3.toml",
                    evidence.id, 1
                ),
                toml::to_string_pretty(&evidence_manifest)
                    .map_err(storage)?
                    .into_bytes(),
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let source = spool.path().join(index.to_string());
            write_new(&source, &data)?;
            // Every artifact is new. Never overwrite an independently retained record.
            if self.archive.join(&path).exists() {
                return Err(Error::Conflict);
            }
            payloads.push(V3PayloadPublication {
                source,
                published_path: path,
                sha256: Digest::sha256(&data).hex,
            });
        }
        if payloads
            .iter()
            .any(|payload| self.archive.join(&payload.published_path).exists())
        {
            return Err(Error::Conflict);
        }
        let legacy_root = {
            let db = lock(&self.db)?;
            db.query_row(
                "SELECT item_id,relative_path FROM legacy_payload_locators WHERE capture_id=?",
                [capture.id.to_string()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(storage)?
            .map(|(id, relative_path)| {
                Ok::<_, Error>(archive_v3::LegacyCaptureRootV3 {
                    item_id: id.parse().map_err(storage)?,
                    relative_path,
                })
            })
            .transpose()?
        };
        let result = publish_v3_revision_controlled(
            self,
            V3RevisionTarget {
                collection: &collection,
                directory_kind: "captures",
                entity_id: capture.id.to_string(),
                collection_id: frozen.collection_id,
                expected,
                revision: capture.revision,
            },
            archive_v3::CaptureRecordV3 {
                capture: capture.clone(),
                legacy_root,
            },
            vec![archive_v3::EvidenceReferenceV3 {
                id: evidence.id,
                sha256: Digest::sha256(&evidence_bytes).hex,
            }],
            payloads,
            txid,
            Some(claim.id),
            checkpoint,
        );
        if let Err(error) = result {
            if journals
                .phase(txid)?
                .is_some_and(|phase| phase < publication::PublicationPhase::RevisionCommitted)
            {
                journals.abort(txid)?;
            }
            return Err(error);
        }
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        project_evidence_record(&tx, &evidence)?;
        tx.execute(
            "INSERT INTO conversion_recipes(id,collection_id,record) VALUES(?,?,?)",
            params![
                frozen.recipe.id.to_string(),
                frozen.collection_id.to_string(),
                serde_json::to_string(&frozen.recipe).map_err(storage)?
            ],
        )
        .map_err(storage)?;
        project_capture(&tx, &capture)?;
        tx.commit().map_err(storage)?;
        drop(db);
        finish_v3_publication(self, txid)
    }
}

impl Store {
    /// Startup checks derivative record relationships without reading every payload.
    /// Explicit reindex and recovery also check retained bytes. Neither is a new
    /// codec decode or a new claim of catalog verification.
    pub(crate) fn validate_derivative_projection(
        &self,
        db: &Connection,
        verify_payload: bool,
    ) -> Result<()> {
        let mut statement = db.prepare("SELECT record FROM captures").map_err(storage)?;
        let rows = statement
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(storage)?;
        for row in rows {
            let capture: Capture = serde_json::from_str(&row.map_err(storage)?).map_err(storage)?;
            for representation in &capture.representations {
                let recipe: Option<String> = db
                    .query_row(
                        "SELECT record FROM conversion_recipes WHERE id=? AND collection_id=?",
                        params![
                            representation.id.to_string(),
                            capture.collection_id.to_string()
                        ],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(storage)?;
                let Some(recipe) = recipe else {
                    let retained_validation: bool = db
                        .query_row(
                            "SELECT EXISTS(SELECT 1 FROM evidence WHERE id=? AND collection_id=?)",
                            params![
                                representation.id.to_string(),
                                capture.collection_id.to_string()
                            ],
                            |row| row.get(0),
                        )
                        .map_err(storage)?;
                    if retained_validation {
                        return Err(Error::Storage(
                            "derivative validation has no retained recipe".into(),
                        ));
                    }
                    continue;
                };
                let recipe: ConversionRecipe = serde_json::from_str(&recipe).map_err(storage)?;
                let evidence: archive_v3::EvidenceRecordV3 =
                    projected_record(db, "evidence", &representation.id.to_string())?;
                let validation =
                    validate_retained_derivative(&capture, representation, &recipe, &evidence)?;
                if !verify_payload {
                    continue;
                }
                let file = &representation.files[0];
                crate::files::safe_name(&file.name)?;
                let path = collection_path(db, &self.archive, capture.collection_id)?
                    .join("captures")
                    .join(capture.id.to_string())
                    .join("payload")
                    .join(representation.id.to_string())
                    .join(&file.name);
                let (length, digest) = crate::files::hash_file(&path)?;
                if length != validation.bytes || digest != validation.digest {
                    return Err(Error::Storage(
                        "archived derivative payload does not match validation evidence".into(),
                    ));
                }
            }
        }
        Ok(())
    }
}

impl Store {
    pub(crate) fn cleanup_derivative_scratch(&self) -> Result<()> {
        let root = self.state.join("derivative-scratch");
        if !root.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(&root).map_err(storage)? {
            let entry = entry.map_err(storage)?;
            let name = entry.file_name();
            let parts = name
                .to_str()
                .unwrap_or_default()
                .split('.')
                .collect::<Vec<_>>();
            if entry.file_type().map_err(storage)?.is_dir()
                && parts.len() == 3
                && parts[0].parse::<JobId>().is_ok()
                && parts[1].parse::<TransactionId>().is_ok()
                && !parts[2].is_empty()
            {
                // Both process locks are held and journal recovery has already
                // settled publication. These are disposable copies, never masters.
                fs::remove_dir_all(entry.path()).map_err(storage)?;
            }
        }
        sync_directory(&root)
    }
}

fn validate_retained_derivative(
    capture: &Capture,
    representation: &Representation,
    recipe: &ConversionRecipe,
    evidence: &archive_v3::EvidenceRecordV3,
) -> Result<ConversionValidation> {
    let validation: ConversionValidation =
        serde_json::from_value(evidence.details["validation"].clone()).map_err(storage)?;
    validation.validate_recipe(recipe)?;
    let source = match recipe.source_fingerprint.source {
        PackageScope::Representation(id) => id,
        _ => return Err(Error::Conflict),
    };
    let source_rep = capture
        .representations
        .iter()
        .find(|r| r.id == source)
        .ok_or(Error::Conflict)?;
    let (format, role, profile) = match recipe.settings {
        ConversionSettings::Flac { .. } => (
            "flac",
            RepresentationRole::LosslessDerivative,
            ConversionProfile::Flac,
        ),
        ConversionSettings::Opus { bitrate_kbps, .. } => (
            "opus",
            RepresentationRole::LossyDerivative,
            ConversionProfile::Opus { bitrate_kbps },
        ),
    };
    if recipe.tracks.len() != 1
        || recipe.id.0 != representation.id.0
        || representation.source != Some(source)
        || representation.format != format
        || representation.role != role
        || representation.profile.as_ref() != Some(&profile)
        || !matches!(source_rep.role, RepresentationRole::PreservationMaster)
        || evidence.id.0 != representation.id.0
        || evidence.collection_id != capture.collection_id
        || evidence.representation_ids != vec![representation.id]
        || !matches!(evidence.kind, EvidenceKind::DerivativeValidation)
        || evidence.outcome != EvidenceOutcome::Passed
        || evidence.details["recipe_id"] != serde_json::json!(recipe.id)
        || evidence.inputs
            != vec![
                recipe.source_fingerprint.digest.clone(),
                recipe.digest.clone(),
            ]
        || evidence.implementation != recipe.encoder
        || evidence.implementation_version != recipe.encoder_version
        || representation.files.len() != 1
        || representation.files[0].bytes != validation.bytes
        || representation.files[0].digest != validation.digest
    {
        return Err(Error::Storage(
            "inconsistent archived derivative evidence".into(),
        ));
    }
    Ok(validation)
}
