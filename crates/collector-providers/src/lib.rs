//! Bounded provider and catalog parsing adapters. Domain policy consumes the
//! neutral observations produced here; this crate owns filesystem/network and
//! provider-format mechanisms only.

mod accuraterip;
pub use accuraterip::*;
mod conversion;
pub use conversion::*;
mod metadata;
pub use metadata::*;

use collector_app::{
    CatalogParserPort, ParsedCatalogSnapshot, ProcessingExecutionContext, ProviderRequest,
    ProviderStreamResponse, SnapshotRecord, StreamingProviderPort,
};
use collector_core::{ByteLength, Digest, Error, HashAlgorithm, RawJson, Result};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::PathBuf,
};

pub use collector_modules::games::{
    DatCatalogEntryObservation, DatCatalogFileObservation, DatSnapshotObservation,
};

pub const LOCAL_DAT_PROVIDER_ID: &str = "org.junk.providers.local-dat";
pub const LOCAL_DAT_ACQUIRE_OPERATION: &str = "org.junk.providers.local-dat.acquire";
pub const DAT_OBSERVATION_SCHEMA_VERSION: u32 = 1;
const MAX_DAT_BYTES: u64 = 512 * 1024 * 1024;
const MAX_DAT_GAMES: usize = 1_000_000;
const MAX_DAT_ROMS: usize = 4_096;
const MAX_TEXT_BYTES: usize = 4_096;

#[derive(Clone, Debug)]
pub struct LocalDatSource {
    pub path: PathBuf,
    pub system: String,
    pub catalog: String,
    pub byte_domain: String,
    pub byte_domain_version: u32,
}

impl LocalDatSource {
    pub fn validate(&self) -> Result<()> {
        for (name, value, maximum) in [
            ("system", self.system.as_str(), 120),
            ("catalog", self.catalog.as_str(), 255),
            ("byte domain", self.byte_domain.as_str(), 120),
        ] {
            if value.trim().is_empty()
                || value.len() > maximum
                || value.chars().any(char::is_control)
            {
                return Err(Error::Invalid(format!("invalid DAT source {name}")));
            }
        }
        if self.byte_domain_version == 0 {
            return Err(Error::Invalid(
                "DAT source byte-domain version must be non-zero".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalDatQuery {
    source: Option<String>,
    upload: Option<collector_app::UploadedDatCatalog>,
}

#[derive(Clone)]
pub struct LocalDatProvider {
    sources: BTreeMap<String, LocalDatSource>,
    uploads: Option<std::sync::Arc<dyn collector_app::UploadedCatalogPort>>,
}

impl LocalDatProvider {
    pub fn new(sources: BTreeMap<String, LocalDatSource>) -> Result<Self> {
        if sources.len() > 128 {
            return Err(Error::Invalid("too many configured DAT sources".into()));
        }
        for (id, source) in &sources {
            if id.is_empty()
                || id.len() > 120
                || !id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
            {
                return Err(Error::Invalid("invalid DAT source identifier".into()));
            }
            source.validate()?;
        }
        Ok(Self {
            sources,
            uploads: None,
        })
    }

    pub fn with_uploads(
        mut self,
        uploads: std::sync::Arc<dyn collector_app::UploadedCatalogPort>,
    ) -> Self {
        self.uploads = Some(uploads);
        self
    }

    pub fn configured_sources(&self) -> impl Iterator<Item = (&str, &LocalDatSource)> {
        self.sources
            .iter()
            .map(|(id, source)| (id.as_str(), source))
    }

    pub fn parse_observation(
        source: &LocalDatSource,
        reader: impl Read,
    ) -> Result<(DatSnapshotObservation, Vec<DatCatalogEntryObservation>)> {
        source.validate()?;
        let dat = junk_libs_dat::parse_dat(BufReader::new(reader))
            .map_err(|error| Error::Invalid(format!("invalid DAT catalog: {error}")))?;
        normalized_dat(source, dat, &mut || Ok(false))
    }
}

impl StreamingProviderPort for LocalDatProvider {
    fn acquire_stream(
        &self,
        request: &ProviderRequest,
        execution: &mut dyn ProcessingExecutionContext,
    ) -> Result<ProviderStreamResponse> {
        request.validate()?;
        if request.provider.0 != LOCAL_DAT_PROVIDER_ID
            || request.operation.as_str() != LOCAL_DAT_ACQUIRE_OPERATION
        {
            return Err(Error::Invalid(
                "unsupported local DAT provider request".into(),
            ));
        }
        let query: LocalDatQuery = request.query.parse()?;
        let (source, mut file, uploaded): (LocalDatSource, Box<dyn collector_app::MediaReader>, _) =
            match (query.source, query.upload) {
                (Some(id), None) => {
                    let source = self
                        .sources
                        .get(&id)
                        .ok_or_else(|| {
                            Error::Unavailable("configured DAT source is unavailable".into())
                        })?
                        .clone();
                    let file = File::open(&source.path).map_err(|error| {
                        Error::Unavailable(format!("DAT source outage: {error}"))
                    })?;
                    (source, Box::new(file), None)
                }
                (None, Some(upload)) => {
                    upload.validate()?;
                    let source = LocalDatSource {
                        path: PathBuf::new(),
                        system: upload.metadata.system.clone(),
                        catalog: upload.metadata.catalog.clone(),
                        byte_domain: upload.metadata.byte_domain.clone(),
                        byte_domain_version: 1,
                    };
                    let file = self
                        .uploads
                        .as_ref()
                        .ok_or_else(|| Error::Unavailable("DAT uploads are unavailable".into()))?
                        .uploaded_catalog(&upload)?;
                    (source, file, Some(upload))
                }
                _ => return Err(Error::Invalid("Choose exactly one DAT source.".into())),
            };
        let declared = file.seek(SeekFrom::End(0)).map_err(provider_io)?;
        file.seek(SeekFrom::Start(0)).map_err(provider_io)?;
        if declared == 0 || declared > request.maximum_response_bytes || declared > MAX_DAT_BYTES {
            return Err(Error::Invalid("DAT source is empty or oversized".into()));
        }

        let mut hasher = Sha256::new();
        let mut measured = 0_u64;
        let mut buffer = vec![0; 1024 * 1024];
        loop {
            if execution.cancellation_requested()? {
                return Err(Error::JobCancelled);
            }
            let read = file.read(&mut buffer).map_err(provider_io)?;
            if read == 0 {
                break;
            }
            measured = measured
                .checked_add(read as u64)
                .ok_or_else(|| Error::Invalid("DAT length overflow".into()))?;
            if measured > declared || measured > request.maximum_response_bytes {
                return Err(Error::Invalid(
                    "DAT source changed or exceeded its limit".into(),
                ));
            }
            hasher.update(&buffer[..read]);
        }
        if measured != declared {
            return Err(Error::Invalid("DAT source changed while being read".into()));
        }
        let digest = Digest::new(HashAlgorithm::Sha256, hex::encode(hasher.finalize()))?;
        if uploaded
            .as_ref()
            .is_some_and(|upload| upload.digest != digest || upload.bytes.0 != measured)
        {
            return Err(Error::Invalid(
                "Saved DAT upload failed integrity validation; upload the original DAT again."
                    .into(),
            ));
        }
        file.seek(SeekFrom::Start(0)).map_err(provider_io)?;
        let mut bounded = (&mut file).take(declared.saturating_add(1));
        let reporting = ReportingReader::new(&mut bounded, execution, declared);
        let buffered = BufReader::new(reporting);
        let dat = match junk_libs_dat::parse_dat(buffered) {
            Ok(dat) => dat,
            Err(_) if execution.cancellation_requested()? => return Err(Error::JobCancelled),
            Err(error) => {
                return Err(Error::Invalid(format!("invalid DAT catalog: {error}")));
            }
        };
        let (header, entries) =
            normalized_dat(&source, dat, &mut || execution.cancellation_requested())?;
        let mut index_bytes = 0_usize;
        for entry in &entries {
            if execution.cancellation_requested()? {
                return Err(Error::JobCancelled);
            }
            let bytes = serde_json::to_vec(entry)
                .map_err(|error| Error::Invalid(error.to_string()))?
                .len();
            index_bytes = index_bytes.saturating_add(bytes);
            if bytes > 64 * 1024 || index_bytes > 64 * 1024 * 1024 {
                return Err(Error::Invalid(
                    "parsed catalog index exceeds its resource bound".into(),
                ));
            }
        }
        file.seek(SeekFrom::Start(0)).map_err(provider_io)?;
        Ok(ProviderStreamResponse {
            media_type: "application/vnd.logiqx.dat".into(),
            bytes: ByteLength(measured),
            digest,
            parsed_observation: RawJson::from_value(
                serde_json::to_value(header).map_err(|error| Error::Invalid(error.to_string()))?,
            ),
            reader: Box::new(file.take(measured.saturating_add(1))),
        })
    }
}

impl CatalogParserPort for LocalDatProvider {
    fn parse_snapshot(
        &self,
        record: &SnapshotRecord,
        reader: &mut dyn Read,
        cancelled: &mut dyn FnMut() -> Result<bool>,
    ) -> Result<ParsedCatalogSnapshot> {
        record.validate()?;
        if record.provider.0 != LOCAL_DAT_PROVIDER_ID {
            return Err(Error::Invalid(
                "snapshot belongs to another provider".into(),
            ));
        }
        let header: DatSnapshotObservation = record.parsed_observation.parse()?;
        let source = LocalDatSource {
            path: PathBuf::new(),
            system: header.system.clone(),
            catalog: header.catalog.clone(),
            byte_domain: header.byte_domain.clone(),
            byte_domain_version: header.byte_domain_version,
        };
        let bounded = reader.take(record.bytes.0.saturating_add(1));
        let mut cancellable = CancellableReader {
            reader: bounded,
            cancelled,
        };
        let mut verified = DigestReader {
            reader: &mut cancellable,
            hasher: Sha256::new(),
            bytes: 0,
        };
        let parsed = junk_libs_dat::parse_dat(BufReader::new(&mut verified))
            .map_err(|error| Error::Invalid(format!("invalid DAT catalog: {error}")));
        // Parsers may stop at the document terminator. Drain the bounded stream
        // so trailing bytes and length changes also invalidate the snapshot.
        if parsed.is_ok() {
            std::io::copy(&mut verified, &mut std::io::sink()).map_err(provider_io)?;
        }
        let measured = verified.bytes;
        let digest = Digest::new(
            HashAlgorithm::Sha256,
            hex::encode(verified.hasher.finalize()),
        )?;
        let dat = match parsed {
            Ok(parsed) => parsed,
            Err(_) if cancelled()? => return Err(Error::JobCancelled),
            Err(error) => return Err(error),
        };
        if measured != record.bytes.0 || digest != record.digest {
            return Err(Error::Invalid(
                "archived DAT bytes fail snapshot integrity validation".into(),
            ));
        }
        let (mut actual, entries) = normalized_dat(&source, dat, cancelled)?;
        if header.preview_entries.is_empty() {
            actual.preview_entries.clear();
        }
        if actual != header {
            return Err(Error::Invalid(
                "archived DAT bytes disagree with snapshot metadata".into(),
            ));
        }
        let parsed = ParsedCatalogSnapshot {
            observation: record.parsed_observation.clone(),
            entries: entries
                .into_iter()
                .map(|entry| {
                    serde_json::to_value(entry)
                        .map(RawJson::from_value)
                        .map_err(|error| Error::Invalid(error.to_string()))
                })
                .collect::<Result<Vec<_>>>()?,
        };
        parsed.validate()?;
        Ok(parsed)
    }
}

fn normalized_dat(
    source: &LocalDatSource,
    dat: junk_libs_dat::DatFile,
    cancelled: &mut dyn FnMut() -> Result<bool>,
) -> Result<(DatSnapshotObservation, Vec<DatCatalogEntryObservation>)> {
    if dat.version.trim().is_empty() {
        return Err(Error::Invalid("DAT catalog declares no version".into()));
    }
    safe_text(&dat.name, "DAT name")?;
    safe_optional_text(&dat.description, "DAT description")?;
    safe_text(&dat.version, "DAT version")?;
    if dat.games.is_empty() || dat.games.len() > MAX_DAT_GAMES {
        return Err(Error::Invalid("DAT game count is out of bounds".into()));
    }
    let mut entries = Vec::with_capacity(dat.games.len());
    for (index, game) in dat.games.into_iter().enumerate() {
        if cancelled()? {
            return Err(Error::JobCancelled);
        }
        safe_text(&game.name, "DAT game name")?;
        for (field, value) in [
            ("DAT game region", game.region.as_deref()),
            ("DAT game serial", game.serial.as_deref()),
            ("DAT game version", game.version.as_deref()),
            ("DAT game category", game.category.as_deref()),
        ] {
            if let Some(value) = value {
                safe_text(value, field)?;
            }
        }
        if game.roms.is_empty() || game.roms.len() > MAX_DAT_ROMS {
            return Err(Error::Invalid(
                "DAT game file count is out of bounds".into(),
            ));
        }
        let mut files = Vec::with_capacity(game.roms.len());
        let mut excluded_files = Vec::new();
        for rom in game.roms {
            if cancelled()? {
                return Err(Error::JobCancelled);
            }
            safe_text(&rom.name, "DAT file name")?;
            if let Some(serial) = &rom.serial {
                safe_text(serial, "DAT file serial")?;
            }
            let file = DatCatalogFileObservation {
                name: rom.name,
                bytes: ByteLength(rom.size),
                serial: rom.serial,
                crc32: normalized_hash(&rom.crc, 8)?,
                sha1: normalized_optional_hash(rom.sha1, 40)?,
                md5: normalized_optional_hash(rom.md5, 32)?,
            };
            if collector_modules::games::dat_file_in_byte_domain(&file, &source.byte_domain)? {
                files.push(file);
            } else {
                excluded_files.push(file);
            }
        }
        if files.is_empty() {
            return Err(Error::Invalid(
                "DAT game has no files in the configured byte domain".into(),
            ));
        }
        entries.push(DatCatalogEntryObservation {
            record_id: format!("{}:{index}", source.catalog),
            label: game.name,
            region: game.region,
            serial: game.serial,
            version: game.version,
            category: game.category,
            files,
            excluded_files,
        });
    }
    Ok((
        DatSnapshotObservation {
            schema_version: DAT_OBSERVATION_SCHEMA_VERSION,
            system: source.system.clone(),
            catalog: source.catalog.clone(),
            document_name: dat.name,
            description: dat.description,
            version: dat.version,
            byte_domain: source.byte_domain.clone(),
            byte_domain_version: source.byte_domain_version,
            game_count: entries.len() as u64,
            preview_entries: entries
                .iter()
                .take(3)
                .map(|entry| collector_modules::games::DatCatalogPreviewEntry {
                    label: entry.label.clone(),
                    files: entry
                        .files
                        .iter()
                        .take(3)
                        .map(|file| file.name.clone())
                        .collect(),
                })
                .collect(),
        },
        entries,
    ))
}

fn safe_text(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty()
        || value.len() > MAX_TEXT_BYTES
        || value.chars().any(char::is_control)
    {
        return Err(Error::Invalid(format!("invalid {field}")));
    }
    Ok(())
}

fn safe_optional_text(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        Ok(())
    } else {
        safe_text(value, field)
    }
}

fn normalized_optional_hash(value: Option<String>, digits: usize) -> Result<Option<String>> {
    match value {
        Some(value) => normalized_hash(&value, digits),
        None => Ok(None),
    }
}

fn normalized_hash(value: &str, digits: usize) -> Result<Option<String>> {
    if value.is_empty() {
        return Ok(None);
    }
    if value.len() != digits || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::Invalid("DAT contains an invalid checksum".into()));
    }
    Ok(Some(value.to_ascii_lowercase()))
}

fn provider_io(error: std::io::Error) -> Error {
    Error::Unavailable(format!("DAT source I/O failed: {error}"))
}

struct ReportingReader<'a> {
    reader: &'a mut dyn Read,
    execution: &'a mut dyn ProcessingExecutionContext,
    completed: u64,
    total: u64,
}

impl<'a> ReportingReader<'a> {
    fn new(
        reader: &'a mut dyn Read,
        execution: &'a mut dyn ProcessingExecutionContext,
        total: u64,
    ) -> Self {
        Self {
            reader,
            execution,
            completed: 0,
            total,
        }
    }
}

impl Read for ReportingReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self
            .execution
            .cancellation_requested()
            .map_err(std::io::Error::other)?
        {
            return Err(std::io::Error::other("DAT parsing cancelled"));
        }
        let read = self.reader.read(buffer)?;
        self.completed = self.completed.saturating_add(read as u64);
        if self.completed > self.total {
            return Err(std::io::Error::other("DAT source changed while parsing"));
        }
        self.execution
            .report_progress("parsing_catalog", self.completed, self.total)
            .map_err(std::io::Error::other)?;
        Ok(read)
    }
}

struct CancellableReader<'a, R> {
    reader: R,
    cancelled: &'a mut dyn FnMut() -> Result<bool>,
}

impl<R: Read> Read for CancellableReader<'_, R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if (self.cancelled)().map_err(std::io::Error::other)? {
            return Err(std::io::Error::other("DAT parsing cancelled"));
        }
        self.reader.read(buffer)
    }
}

/// Hash the exact stream consumed by the parser, without buffering the catalog.
struct DigestReader<R> {
    reader: R,
    hasher: Sha256,
    bytes: u64,
}

impl<R: Read> Read for DigestReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let count = self.reader.read(buffer)?;
        self.bytes += count as u64;
        self.hasher.update(&buffer[..count]);
        Ok(count)
    }
}
