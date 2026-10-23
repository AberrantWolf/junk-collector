//! Game identification policy over shared content inspectors and qualified hashes.
use super::*;
use crate::{CatalogSnapshotInput, ModuleHost};
use collector_core::SnapshotId;
use junk_libs_game_formats::{DetectedFormat, DetectionStatus, inspect_game_format};

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GameFormatInspection {
    pub file_id: PackageFileId,
    pub status: GameDetectionStatus,
    pub reason: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GameCatalogIdentification {
    pub snapshot_id: SnapshotId,
    pub snapshot_digest: Digest,
    pub byte_domain: String,
    pub system: String,
    pub catalog: CatalogMatchResult,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GameIdentification {
    pub analyses: Vec<GameProcessingResult>,
    pub status: GameDetectionStatus,
    pub inspections: Vec<GameFormatInspection>,
    pub catalogs: Vec<GameCatalogIdentification>,
    pub candidates: Vec<MetadataCandidateObservation>,
    pub limitations: Vec<String>,
}

pub fn identify_game(
    host: &dyn ModuleHost,
    access: PackageAccess,
    snapshots: &[SnapshotId],
) -> ModuleResult<GameIdentification> {
    let inventory = host.package_inventory(access)?;
    let mut result = GameIdentification {
        analyses: vec![],
        status: GameDetectionStatus::Unrecognized,
        inspections: vec![],
        catalogs: vec![],
        candidates: vec![],
        limitations: vec![],
    };
    let mut profiles = Vec::new();
    let mut cue = None;
    // Companion filenames locate bytes after an anchor is recognized. Renaming
    // a valid cartridge to .cue cannot force it into a permissive text parser.
    for (index, file) in inventory.files.iter().enumerate() {
        host.checkpoint()?;
        let mut reader = host.package_file(access, file.id)?;
        let mut prefix = vec![0; (file.bytes.0.min(64 * 1024)) as usize];
        std::io::Read::read_exact(reader.as_mut(), &mut prefix)
            .map_err(|e| ModuleError::FailedOperation(e.to_string()))?;
        if let Ok(text) = std::str::from_utf8(&prefix) {
            let lines = text.lines().map(str::trim).collect::<Vec<_>>();
            if lines.iter().any(|line| {
                line.split_whitespace()
                    .next()
                    .is_some_and(|v| v.eq_ignore_ascii_case("FILE"))
            }) && lines.iter().any(|line| {
                line.split_whitespace()
                    .next()
                    .is_some_and(|v| v.eq_ignore_ascii_case("TRACK"))
            }) && cue.replace(index).is_some()
            {
                result.status = GameDetectionStatus::Ambiguous;
                result
                    .limitations
                    .push("Multiple CUE anchors require a package choice".into());
                return Ok(result);
            }
        }
    }
    for (index, file) in inventory.files.iter().enumerate().filter(|_| cue.is_none()) {
        host.checkpoint()?;
        let mut reader = host.package_file(access, file.id)?;
        let inspection = inspect_game_format(reader.as_mut()).map_err(module_hash_error)?;
        let status = match inspection.status {
            DetectionStatus::Recognized => GameDetectionStatus::Recognized,
            DetectionStatus::Malformed => GameDetectionStatus::Malformed,
            DetectionStatus::Unsupported => GameDetectionStatus::Unsupported,
            DetectionStatus::Ambiguous => GameDetectionStatus::Ambiguous,
            DetectionStatus::Unrecognized => GameDetectionStatus::Unrecognized,
        };
        let stops = matches!(
            status,
            GameDetectionStatus::Malformed | GameDetectionStatus::Unsupported
        );
        result.inspections.push(GameFormatInspection {
            file_id: file.id,
            status: status.clone(),
            reason: inspection.reason,
        });
        if stops {
            result.status = status;
            result.limitations.push("Recognized invalid or unsupported input cannot fall through to raw catalog matching".into());
            return Ok(result);
        }
        profiles.push((index, inspection.format));
    }
    let mut hashes = BTreeMap::<String, GameProcessingResult>::new();
    if let Some(index) = cue {
        match hash_inventory_cue(host, access, &inventory, index) {
            Ok(value) => {
                result.inspections.push(GameFormatInspection {file_id:inventory.files[index].id,status:GameDetectionStatus::Recognized,reason:"Validated CUE directives, companion references and ordered raw-2352 track boundaries".into()});
                hashes.insert("org.junk.game.cd.raw-2352-track".into(), value);
            }
            Err(error) => {
                host.checkpoint()?;
                result.status = if matches!(error, ModuleError::UnavailableCapability(_)) {
                    GameDetectionStatus::Unsupported
                } else {
                    GameDetectionStatus::Malformed
                };
                result.inspections.push(GameFormatInspection {
                    file_id: inventory.files[index].id,
                    status: result.status.clone(),
                    reason: error.to_string(),
                });
                result.limitations.push(error.to_string());
                return Ok(result);
            }
        }
    } else if profiles.len() == 1 {
        let (index, format) = &profiles[0];
        let file = &inventory.files[*index];
        let choices: Vec<GameHashProfile> = match format {
            Some(DetectedFormat::Ines) => vec![GameHashProfile::InesPayload, GameHashProfile::Raw],
            Some(DetectedFormat::N64) => vec![GameHashProfile::N64BigEndian],
            Some(DetectedFormat::RawSystem(_)) => vec![GameHashProfile::Raw],
            _ => {
                let mut values = vec![GameHashProfile::Raw];
                if file.bytes.0 >= 8192 && file.bytes.0 % 8192 == 0 {
                    values.push(GameHashProfile::HeaderlessInes);
                }
                if let Ok(header) = junk_libs_game_formats::detect_snes_copier_header(file.bytes.0)
                {
                    values.push(GameHashProfile::SnesPayload {
                        copier_header: matches!(header, CopierHeader::Present512),
                    });
                }
                if let Ok(header) = junk_libs_game_formats::detect_pce_copier_header(file.bytes.0) {
                    values.push(GameHashProfile::PcePayload {
                        copier_header: matches!(header, CopierHeader::Present512),
                    });
                }
                values
            }
        };
        for profile in choices {
            host.checkpoint()?;
            let value = hash_inventory_file(host, access, &inventory, *index, profile)?;
            if let GameProcessingResult::Hashes { files } = &value {
                hashes.insert(files[0].domain.name.clone(), value);
            }
        }
    } else {
        result.limitations.push(
            "Unanchored multi-file packages have no supported automatic interpretation".into(),
        );
        return Ok(result);
    }
    result.analyses = hashes.values().cloned().collect();
    if let Some(anchor) = cue {
        let discovered_audio = crate::disc_audio::inspect(host, access, &inventory, anchor);
        host.checkpoint()?;
        match discovered_audio {
            Ok(audio) => result
                .analyses
                .push(GameProcessingResult::CdAudio { audio }),
            Err(e) => result.limitations.push(format!(
                "CD audio unavailable: {}",
                e.to_string()
                    .chars()
                    .filter(|c| !c.is_control())
                    .take(512)
                    .collect::<String>()
            )),
        }
    }

    for id in snapshots {
        host.checkpoint()?;
        let snapshot: CatalogSnapshotInput = host.catalog_snapshot(*id)?;
        let header: DatSnapshotObservation = snapshot
            .observation
            .parse()
            .map_err(|e| ModuleError::Invalid(e.to_string()))?;
        let Some(system) = standard_system(&header.system) else {
            continue;
        };
        if system.id == "nes"
            && header.byte_domain == "org.junk.game.raw-bytes"
            && !matches!(
                profiles.first().and_then(|p| p.1.as_ref()),
                Some(DetectedFormat::Ines)
            )
        {
            continue;
        }
        let Some(hash) = hashes.get(&header.byte_domain) else {
            continue;
        };
        // A recognized cartridge cannot be reinterpreted as a different platform.
        if cue.is_none() && profiles.len() == 1 {
            match &profiles[0].1 {
                Some(DetectedFormat::Ines) if system.id != "nes" => continue,
                Some(DetectedFormat::N64) if system.id != "n64" => continue,
                Some(DetectedFormat::RawSystem(expected)) if system.id != *expected => continue,
                _ => {}
            }
        }
        let units = match hash {
            GameProcessingResult::Hashes { files } => files
                .iter()
                .map(|file| CatalogInputUnit {
                    source_file_id: file.file_id,
                    track_number: None,
                    bytes: file.bytes,
                    domain: &file.domain,
                    digests: &file.digests,
                })
                .collect::<Vec<_>>(),
            GameProcessingResult::TrackHashes { tracks } => tracks
                .iter()
                .map(|track| CatalogInputUnit {
                    source_file_id: track.source_file_id,
                    track_number: Some(track.track_number),
                    bytes: track.bytes,
                    domain: &track.domain,
                    digests: &track.digests,
                })
                .collect::<Vec<_>>(),
            _ => unreachable!(),
        };
        let matched = match_dat_catalog(
            snapshot.provider.clone(),
            &header.system,
            &snapshot.observation,
            snapshot.entries.iter(),
            &units,
            &mut || host.checkpoint(),
        )?;
        let GameProcessingResult::CatalogMatch { catalog } = matched else {
            unreachable!()
        };
        let candidates = dat_metadata_candidates(
            snapshot.provider,
            &snapshot.digest,
            &snapshot.entries,
            &catalog,
        )
        .map_err(|e| ModuleError::Invalid(e.to_string()))?;
        for mut candidate in candidates {
            // Scope provider record IDs by immutable catalog content, never a row number alone.
            candidate.identity.provider_record_id = format!(
                "{}:{}",
                snapshot.digest.hex, candidate.identity.provider_record_id
            );
            let mut observation = candidate
                .observation
                .value()
                .map_err(|e| ModuleError::Invalid(e.to_string()))?;
            observation
                .as_object_mut()
                .ok_or_else(|| ModuleError::Invalid("invalid catalog row".into()))?
                .insert("system".into(), system.name.clone().into());
            candidate.observation = RawJson::from_value(observation);
            candidate.identity.provider_record_version = Some(format!(
                "sha256:{}",
                Digest::sha256(
                    &serde_json::to_vec(&("game-identification-v1", &candidate))
                        .map_err(|e| ModuleError::Invalid(e.to_string()))?
                )
                .hex
            ));
            result.candidates.push(candidate);
        }
        let matched_candidates = match &catalog {
            CatalogMatchResult::UniqueCompleteMatch { candidate } => {
                std::slice::from_ref(candidate)
            }
            CatalogMatchResult::AmbiguousCompleteMatches { candidates }
            | CatalogMatchResult::PartialIdentification { candidates, .. } => candidates.as_slice(),
            _ => &[],
        };
        if matched_candidates.iter().any(|c| {
            c.matched_units
                .iter()
                .all(|unit| unit.digest.algorithm == HashAlgorithm::Crc32)
        }) {
            result.limitations.push(
                "CRC32-only catalog evidence is weaker than cryptographic checksum evidence".into(),
            );
        }
        result.catalogs.push(GameCatalogIdentification {
            snapshot_id: *id,
            snapshot_digest: snapshot.digest.clone(),
            byte_domain: header.byte_domain.clone(),
            system: system.name,
            catalog,
        });
    }
    let mut distinct = Vec::new();
    for candidate in result.candidates.iter().filter(|c| c.complete_identity) {
        if !distinct
            .iter()
            .any(|other| equivalent_game_candidates(candidate, other))
        {
            distinct.push(candidate.clone());
        }
    }
    let complete = distinct.len();
    result.status = if complete == 1 {
        GameDetectionStatus::Recognized
    } else if complete > 1 {
        GameDetectionStatus::Ambiguous
    } else {
        GameDetectionStatus::Unrecognized
    };
    if complete == 0 {
        result
            .limitations
            .push("No unambiguous complete catalog identity was established".into());
    }
    Ok(result)
}

/// Equivalent complete rows may converge without discarding their catalog sources.
/// Different systems, labels, editions or checksum sets remain competing identities.
pub fn equivalent_game_candidates(
    left: &MetadataCandidateObservation,
    right: &MetadataCandidateObservation,
) -> bool {
    if !left.complete_identity
        || !right.complete_identity
        || left.identity.provider != right.identity.provider
    {
        return false;
    }
    let (Ok(mut a), Ok(mut b)) = (
        left.observation.parse::<DatCatalogEntryObservation>(),
        right.observation.parse::<DatCatalogEntryObservation>(),
    ) else {
        return false;
    };
    if a.system.is_none() || b.system.is_none() {
        return false;
    }
    a.record_id.clear();
    b.record_id.clear();
    a == b
        && left.title == right.title
        && left.subtitle == right.subtitle
        && left.strength == right.strength
}

/// Check reported matches against computed coordinates and immutable catalog rows.
pub(super) fn validate_identification_evidence(result: &GameIdentification) -> ModuleResult<()> {
    let invalid = || {
        ModuleError::Invalid(
            "Identification evidence disagrees with computed inputs or catalog rows".into(),
        )
    };
    let mut expected = std::collections::BTreeSet::new();
    let mut snapshots = std::collections::BTreeSet::new();
    for catalog in &result.catalogs {
        catalog.snapshot_digest.validate().map_err(|_| invalid())?;
        if !snapshots.insert(catalog.snapshot_id) || standard_system(&catalog.system).is_none() {
            return Err(invalid());
        }
        let units = result
            .analyses
            .iter()
            .find_map(|analysis| match analysis {
                GameProcessingResult::Hashes { files }
                    if files.iter().all(|f| f.domain.name == catalog.byte_domain) =>
                {
                    Some(
                        files
                            .iter()
                            .map(|f| (f.file_id, None, &f.digests))
                            .collect::<Vec<_>>(),
                    )
                }
                GameProcessingResult::TrackHashes { tracks }
                    if tracks.iter().all(|t| t.domain.name == catalog.byte_domain) =>
                {
                    Some(
                        tracks
                            .iter()
                            .map(|t| (t.source_file_id, Some(t.track_number), &t.digests))
                            .collect::<Vec<_>>(),
                    )
                }
                _ => None,
            })
            .ok_or_else(invalid)?;
        let (matches, complete): (&[CatalogCandidate], bool) = match &catalog.catalog {
            CatalogMatchResult::UniqueCompleteMatch { candidate } => {
                (std::slice::from_ref(candidate), true)
            }
            CatalogMatchResult::AmbiguousCompleteMatches { candidates } => (candidates, true),
            CatalogMatchResult::PartialIdentification {
                candidates,
                total_files,
                ..
            } => {
                if *total_files as usize != units.len() {
                    return Err(invalid());
                }
                (candidates, false)
            }
            _ => (&[], false),
        };
        for matched in matches {
            if complete && matched.matched_units.len() != units.len() {
                return Err(invalid());
            }
            for coordinate in &matched.matched_units {
                if !units.iter().any(|(file, track, digests)| {
                    *file == coordinate.source_file_id
                        && *track == coordinate.track_number
                        && digests.contains(&coordinate.digest)
                }) {
                    return Err(invalid());
                }
            }
            let id = format!(
                "{}:{}",
                catalog.snapshot_digest.hex, matched.provider_record_id
            );
            if !expected.insert((matched.provider.clone(), id.clone())) {
                return Err(invalid());
            }
            let candidate = result
                .candidates
                .iter()
                .find(|c| {
                    c.identity.provider == matched.provider && c.identity.provider_record_id == id
                })
                .ok_or_else(invalid)?;
            let row: DatCatalogEntryObservation =
                candidate.observation.parse().map_err(|_| invalid())?;
            if candidate.complete_identity != complete
                || candidate.title.value != matched.label
                || row.label != matched.label
                || row.record_id != matched.provider_record_id
                || row.system.as_deref() != Some(catalog.system.as_str())
            {
                return Err(invalid());
            }
        }
    }
    let actual = result
        .candidates
        .iter()
        .map(|c| {
            (
                c.identity.provider.clone(),
                c.identity.provider_record_id.clone(),
            )
        })
        .collect::<std::collections::BTreeSet<_>>();
    if actual != expected || actual.len() != result.candidates.len() {
        return Err(invalid());
    }
    Ok(())
}
