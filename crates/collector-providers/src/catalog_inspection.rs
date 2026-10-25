//! Bounded archive traversal and shared DAT parsing. System policy lives in Games.
use collector_app::*;
use collector_core::*;
use collector_modules::games::*;
use std::io::{Cursor, Read};

pub struct StandardCatalogInspector;
impl CatalogUploadInspectionPort for StandardCatalogInspector {
    fn inspect_upload(
        &self,
        bytes: &[u8],
        uploads: &dyn UploadedCatalogPort,
        expanded_limit: u64,
        member_limit: u32,
    ) -> Result<CatalogUploadPreview> {
        let mut result = CatalogUploadPreview {
            expanded_bytes: ByteLength(0),
            member_count: 0,
            entries: vec![],
            ignored: vec![],
        };
        if bytes.starts_with(b"PK") {
            let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
                .map_err(|e| Error::Invalid(format!("Invalid ZIP: {e}")))?;
            if archive.len() > member_limit as usize {
                return Err(Error::Invalid("ZIP exceeds 128 members".into()));
            }
            result.member_count = archive.len() as u32;
            let mut expanded = 0u64;
            for index in 0..archive.len() {
                let mut file = archive
                    .by_index(index)
                    .map_err(|e| Error::Invalid(format!("Unsupported ZIP member: {e}")))?;
                let name = file
                    .name()
                    .map_err(|e| Error::Invalid(format!("Invalid ZIP member name: {e}")))?
                    .into_owned();
                if file.enclosed_name().is_none()
                    || name.starts_with('/')
                    || name.contains('\\')
                    || name.contains(':')
                    || file
                        .unix_mode()
                        .is_some_and(|mode| !matches!(mode & 0o170000, 0 | 0o100000 | 0o040000))
                    || file.encrypted()
                {
                    return Err(Error::Invalid("Unsafe or encrypted ZIP member".into()));
                }
                if file.is_dir() {
                    continue;
                }
                if file.size() > MAX_DAT_UPLOAD_BYTES as u64 {
                    return Err(Error::Invalid("ZIP member exceeds 64 MiB".into()));
                }
                let mut data = Vec::new();
                let mut chunk = [0; 64 * 1024];
                loop {
                    let count = file
                        .read(&mut chunk)
                        .map_err(|e| Error::Invalid(format!("ZIP member read failed: {e}")))?;
                    if count == 0 {
                        break;
                    }
                    expanded = expanded
                        .checked_add(count as u64)
                        .ok_or_else(|| Error::Invalid("ZIP length overflow".into()))?;
                    if expanded > expanded_limit || data.len() + count > MAX_DAT_UPLOAD_BYTES {
                        return Err(Error::Invalid("ZIP expansion limit exceeded".into()));
                    }
                    data.extend_from_slice(&chunk[..count]);
                }
                if data.starts_with(b"PK")
                    || data.starts_with(b"7z\xbc\xaf\x27\x1c")
                    || data.starts_with(b"Rar!")
                    || data.starts_with(&[0x1f, 0x8b])
                    || data.get(257..262) == Some(b"ustar")
                {
                    return Err(Error::Invalid("Nested archives are unsupported".into()));
                }
                let is_document = data
                    .iter()
                    .copied()
                    .find(|c| !c.is_ascii_whitespace())
                    .is_some_and(|c| c == b'<' || c == 0xef)
                    || data
                        .strip_prefix(b"\xef\xbb\xbf")
                        .unwrap_or(&data)
                        .iter()
                        .copied()
                        .skip_while(u8::is_ascii_whitespace)
                        .take(10)
                        .collect::<Vec<_>>()
                        .eq_ignore_ascii_case(b"clrmamepro");
                if !is_document
                    && !name.to_ascii_lowercase().ends_with(".dat")
                    && !name.to_ascii_lowercase().ends_with(".xml")
                {
                    result.ignored.push(name);
                    continue;
                }
                result.entries.push(inspect_dat(&name, &data, uploads));
            }
            result.expanded_bytes = ByteLength(expanded);
        } else {
            if bytes.len() as u64 > expanded_limit || member_limit == 0 {
                return Err(Error::Invalid("Expanded batch limit exceeded".into()));
            }
            result.expanded_bytes = ByteLength(bytes.len() as u64);
            result.member_count = 1;
            result
                .entries
                .push(inspect_dat("Uploaded DAT", bytes, uploads));
        }
        if result.entries.is_empty() {
            return Err(Error::Invalid("No DAT documents found".into()));
        }
        Ok(result)
    }
}
fn inspect_dat(name: &str, bytes: &[u8], uploads: &dyn UploadedCatalogPort) -> CatalogUploadEntry {
    let parsed = junk_libs_dat::parse_dat(std::io::BufReader::new(Cursor::new(bytes)));
    let mut corrections = vec![];
    let (detection, upload) = match parsed {
        Ok(dat) => {
            let dialect = match dat.dialect {
                junk_libs_dat::DatDialect::LogiqxXml => "Logiqx XML",
                junk_libs_dat::DatDialect::ClrMamePro => "ClrMamePro",
            };
            let mut detection = detect_standard_catalog(
                dialect,
                &dat.name,
                &dat.description,
                &dat.author,
                &dat.url,
                &dat.version,
            );
            for choice in &detection.system_choices {
                if let Some(corrected) = corrected_catalog(&detection, &choice.name) {
                    let metadata = DatUploadMetadata {
                        system: choice.name.clone(),
                        catalog: corrected.catalog.unwrap(),
                        byte_domain: corrected.byte_domains[0].clone(),
                    };
                    let source = crate::LocalDatSource {
                        path: Default::default(),
                        system: metadata.system.clone(),
                        catalog: metadata.catalog.clone(),
                        byte_domain: metadata.byte_domain.clone(),
                        byte_domain_version: 1,
                    };
                    if crate::normalized_dat(&source, dat.clone(), &mut || Ok(false)).is_ok()
                        && let Ok(digest) = uploads.stage_uploaded_catalog(bytes)
                    {
                        corrections.push(UploadedDatCatalog {
                            metadata,
                            digest,
                            bytes: ByteLength(bytes.len() as u64),
                        });
                    }
                }
            }
            let upload = if matches!(detection.status, GameDetectionStatus::Recognized) {
                let metadata = DatUploadMetadata {
                    system: detection.system.as_ref().unwrap().name.clone(),
                    catalog: detection.catalog.clone().unwrap(),
                    byte_domain: detection.byte_domains[0].clone(),
                };
                let source = crate::LocalDatSource {
                    path: Default::default(),
                    system: metadata.system.clone(),
                    catalog: metadata.catalog.clone(),
                    byte_domain: metadata.byte_domain.clone(),
                    byte_domain_version: 1,
                };
                match crate::normalized_dat(&source, dat, &mut || Ok(false))
                    .and_then(|_| uploads.stage_uploaded_catalog(bytes))
                {
                    Ok(digest) => Some(UploadedDatCatalog {
                        metadata,
                        digest,
                        bytes: ByteLength(bytes.len() as u64),
                    }),
                    Err(error) => {
                        detection.status = GameDetectionStatus::Malformed;
                        detection.reasons.push(error.to_string());
                        None
                    }
                }
            } else {
                None
            };
            (detection, upload)
        }
        Err(error) => (
            CatalogDetection {
                status: if matches!(error, junk_libs_dat::DatError::Unsupported(_)) {
                    GameDetectionStatus::Unsupported
                } else {
                    GameDetectionStatus::Malformed
                },
                dialect: "Unrecognized".into(),
                system: None,
                system_choices: vec![],
                catalog: None,
                version: String::new(),
                byte_domains: vec![],
                reasons: vec![error.to_string()],
            },
            None,
        ),
    };
    CatalogUploadEntry {
        name: name.into(),
        detection,
        upload,
        corrections,
    }
}
