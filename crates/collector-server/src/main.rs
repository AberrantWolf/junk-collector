use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use collector_app::CollectionTypeAdministrationService;
use collector_client::{Endpoint, Pairing, Route};
use collector_core::*;
use collector_providers::{
    ACCURATERIP_PROVIDER_ID, AccurateRipProvider, ConfiguredEncoder, CoverArtArchiveProvider,
    EncoderKind, LOCAL_DAT_PROVIDER_ID, LocalDatProvider, LocalDatSource, MusicBrainzProvider,
    ProcessAudioConverter,
};
use collector_server::{AppState, router};
use collector_store::Store;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};

#[derive(Parser)]
#[command(
    name = "junk-collector",
    version,
    about = "Preservation-first collection server"
)]
struct Cli {
    #[arg(long, default_value = "collector.toml")]
    config: PathBuf,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Create or repair local browser setup without changing existing accounts. Stop the server first.
    Setup {
        #[arg(long, default_value = "owner")]
        username: String,
    },
    /// Check configuration without opening or locking the archive.
    Doctor,
    Serve,
    Reindex,
    /// Write a verified complete backup. Stop the server first.
    Backup {
        #[arg(long)]
        destination: PathBuf,
    },
    /// Export portable collection records and originals without accounts/config. Stop the server first.
    ExportArchive {
        #[arg(long)]
        destination: PathBuf,
    },
    /// Check every portable export byte without opening storage or reading configuration.
    ValidateArchiveExport {
        #[arg(long)]
        export: PathBuf,
    },
    /// Copy a portable export into an empty archive; create fresh configuration/accounts afterward.
    ImportArchive {
        #[arg(long)]
        export: PathBuf,
        #[arg(long)]
        archive: PathBuf,
    },
    /// Validate every backup byte without opening or migrating a database.
    ValidateBackup {
        #[arg(long)]
        backup: PathBuf,
    },
    /// Restore into empty roots; --config names a new output configuration.
    Restore {
        #[arg(long)]
        backup: PathBuf,
        #[arg(long)]
        archive: PathBuf,
        #[arg(long)]
        state: PathBuf,
        /// Recover archive history with a fresh private identity and explicit grants.
        #[arg(long)]
        archive_only: bool,
    },
    /// List collections without any Owner grant. Stop the server first.
    UnownedCollections {
        #[arg(long, default_value_t = 50)]
        limit: u16,
        #[arg(long)]
        cursor: Option<CollectionId>,
    },
    /// Explicitly assign the first Owner of a recovered collection. Stop the server first.
    RecoverCollectionOwner {
        #[arg(long)]
        collection_id: CollectionId,
        #[arg(long)]
        username: String,
    },
    /// Inspect exact duplicate provider payloads; stop the server first.
    InspectSnapshotDuplicates,
    /// Apply the reviewed JSON plan using a verified backup; stop the server first.
    ApplySnapshotDuplicates {
        #[arg(long)]
        plan: PathBuf,
        #[arg(long)]
        backup: PathBuf,
    },
    /// Inspect durable collection-type assignment eligibility without changing data.
    InspectCollectionType {
        #[arg(long)]
        collection_id: CollectionId,
    },
    /// Assign a legacy collection to an installed module. Stop the server first.
    AssignCollectionType {
        #[arg(long)]
        collection_id: CollectionId,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        module: String,
    },
    /// Recover an account by its sign-in username. Stop the server first; reads the new password from stdin.
    RecoverAccount {
        /// Sign-in username of the account to recover.
        #[arg(long)]
        username: String,
        /// Account-storage folder override when configuration is unavailable (contains private.sqlite).
        #[arg(long)]
        state: Option<PathBuf>,
    },
    /// List sign-in usernames and account status. Stop the server first.
    Accounts {
        /// Account-storage folder override when configuration is unavailable (contains private.sqlite).
        #[arg(long)]
        state: Option<PathBuf>,
        #[arg(long, default_value_t = 50)]
        limit: u16,
        #[arg(long)]
        cursor: Option<String>,
    },
    /// Show configured storage locations without opening them. Safe while the server is running.
    Paths,
    /// Export trusted enrollment material. Transfer this file through a trusted channel.
    Pairing,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Config {
    archive: PathBuf,
    state: PathBuf,
    public_bind: SocketAddr,
    private_bind: SocketAddr,
    private_tls_bind: Option<SocketAddr>,
    lan_bind: SocketAddr,
    local_playback_bind: Option<SocketAddr>,
    local_playback_enabled: bool,
    remote_media_allowed: bool,
    remote_media: Option<collector_server::remote_media::Configuration>,
    multi_user: bool,
    exposure: Exposure,
    authentication_origin: Option<String>,
    inboxes: std::collections::BTreeMap<String, PathBuf>,
    dat_sources: std::collections::BTreeMap<String, DatSourceConfig>,
    providers: ProviderConfig,
    encoders: EncoderConfig,
    jobs: collector_app::JobSettings,
    backup_reminder: Option<collector_app::BackupReminderSettings>,
    tls_names: Vec<String>,
    allowed_origins: Vec<String>,
    endpoints: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DatSourceConfig {
    path: PathBuf,
    system: String,
    catalog: String,
    byte_domain: String,
    #[serde(default = "default_domain_version")]
    byte_domain_version: u32,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct EncoderConfig {
    /// Optional path or executable name for the FLAC encoder. No derivative is
    /// generated unless a curator explicitly requests it.
    flac: Option<PathBuf>,
    /// Optional path or executable name for opusenc.
    opus: Option<PathBuf>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ProviderConfig {
    /// MusicBrainz requires a versioned application name plus a maintainer contact.
    /// When absent, metadata acquisition is honestly unavailable and preservation
    /// remains operational.
    musicbrainz_user_agent: Option<String>,
    acoustid_api_key: Option<String>,
}

fn default_domain_version() -> u32 {
    1
}
impl Default for Config {
    fn default() -> Self {
        Self {
            archive: ".collector/archive".into(),
            state: ".collector/state".into(),
            public_bind: "127.0.0.1:9841".parse().unwrap(),
            private_bind: "127.0.0.1:9842".parse().unwrap(),
            private_tls_bind: None,
            lan_bind: "0.0.0.0:9843".parse().unwrap(),
            local_playback_bind: Some("0.0.0.0:9845".parse().unwrap()),
            local_playback_enabled: true,
            remote_media_allowed: false,
            remote_media: None,
            multi_user: false,
            exposure: Exposure::default(),
            authentication_origin: None,
            inboxes: std::collections::BTreeMap::new(),
            dat_sources: std::collections::BTreeMap::new(),
            providers: ProviderConfig::default(),
            encoders: EncoderConfig::default(),
            jobs: collector_app::JobSettings::default(),
            backup_reminder: None,
            tls_names: vec![
                "localhost".into(),
                "junk-collector.local".into(),
                "127.0.0.1".into(),
            ],
            allowed_origins: vec![
                "https://localhost:9843".into(),
                "https://junk-collector.local:9843".into(),
            ],
            endpoints: vec!["https://junk-collector.local:9843".into()],
        }
    }
}
fn local_origin(config: &Config) -> String {
    format!("http://localhost:{}", config.private_bind.port())
}
fn metadata_readiness(config: &ProviderConfig) {
    if config.musicbrainz_user_agent.is_some() {
        println!(
            "Optional metadata/artwork: MusicBrainz and Cover Art Archive are configured. A matching release and successful provider responses are still required; this is not a network availability check."
        );
    } else {
        println!(
            "Optional metadata/artwork: not configured. Set providers.musicbrainz_user_agent with a monitored contact address; a MusicBrainz account is not required. Saving originals and manual metadata/artwork remain available."
        );
    }
}

fn instructions(config: &Config, path: &std::path::Path) {
    println!(
        "Background job defaults: {} processing workers; inbox copy limit {}; provider limit 1. Saved Administration worker settings override these defaults.",
        config.jobs.processing_workers, config.jobs.copy_workers
    );
    println!(
        "\nLocal management: {} (open this exact address on the server computer)",
        local_origin(config)
    );
    println!(
        "For first use: sign in here, choose Create collection (or open Administration), then choose Add / import."
    );
    if config.tls_names.iter().any(|name| name == "localhost") {
        println!(
            "Local client HTTPS: https://localhost:{} (same computer only; the certificate must be trusted)",
            config.lan_bind.port()
        );
    }
    for endpoint in &config.endpoints {
        println!(
            "Advertised client HTTPS: {endpoint}. Requires working DNS; setup does not create DNS records."
        );
    }
    if let Some(bind) = config
        .local_playback_bind
        .filter(|_| config.local_playback_enabled)
    {
        println!(
            "Automatic private media: {bind} (TCP native HTTPS / UDP browser WebTransport). Trust is supplied through the signed-in/paired connection; no internal certificate import is needed. Allow this port on the private host firewall."
        );
    }
    println!(
        "Public media fallback: {}. Login/library control remains available.",
        if config.remote_media_allowed {
            "enabled"
        } else {
            "disabled"
        }
    );
    let certificate = config.state.join("tls/cert.pem");
    if let Ok(file) = std::fs::File::open(&certificate) {
        use std::io::Read;
        let mut bytes = Vec::new();
        if file.take(65_537).read_to_end(&mut bytes).is_ok() && bytes.len() <= 65_536 {
            let location = std::path::absolute(&certificate).unwrap_or(certificate);
            println!("Public certificate: {}", location.display());
            println!(
                "Certificate PEM-file SHA-256: {}",
                Digest::sha256(&bytes).hex
            );
            println!(
                "Certificate download and help: sign in at {} → Administration → Server settings → HTTPS certificate.",
                local_origin(config)
            );
            println!(
                "For a self-signed certificate on macOS: import cert.pem into Keychain Access (login keychain), open its Trust settings, set Secure Sockets Layer (SSL) to Always Trust, close and authenticate, then restart Trash Amp and your browser."
            );
            println!(
                "Trash Amp: Settings → Servers; enter the Collector server URL. For an owner-provided certificate, independently compare the PEM-file SHA-256 above, then enter its path in Owner-provided trust file and choose Connect / Reconnect. Leave the trust file empty for system-trusted HTTPS. This does not establish browser trust; local browser approval can use {}/pair with the approval code.",
                local_origin(config)
            );
            println!(
                "Obtain trust material from the server computer or a separately trusted owner channel. Keep key.pem private; never disable certificate validation."
            );
        }
    }
    metadata_readiness(&config.providers);
    println!(
        "After sign-in: open Account & security to save recovery codes, then create a collection and import."
    );
    println!(
        "Public proxy upstream: http://{} (client surface; curation={}, administration={})",
        config.public_bind,
        config.exposure.public_curation || config.exposure.public_administration,
        config.exposure.public_administration
    );
    println!(
        "LAN client HTTPS listener: {} (client surface by default; browser certificate trust is separate from native pairing)",
        config.lan_bind
    );
    println!(
        "Configuration: {}. Changes require a server restart.",
        path.display()
    );
    println!(
        "Optional explicit music preparation: set values in `[encoders]` (create that section if absent): `flac = \"/path/to/flac\"` and/or `opus = \"/path/to/opusenc\"`. Tools are probed at server start; imports and original downloads do not require them."
    );
    if !config.allowed_origins.contains(&local_origin(config)) {
        println!(
            "WARNING: Local management origin is not allowed. Stop the server and run setup to repair it."
        );
    }
    println!(
        "Use localhost exactly for local browser access. Remote browsers require trusted HTTPS and its exact allowed_origins entry. Local HTTP onboarding is Chromium-tested; other browsers may require trusted HTTPS. Never publish the private listener through a public tunnel.\n"
    );
}

fn probe_optional_encoder(
    kind: EncoderKind,
    configured: Option<&PathBuf>,
) -> Option<ConfiguredEncoder> {
    let Some(path) = configured else {
        println!(
            "Optional {} encoder: not configured",
            match kind {
                EncoderKind::Flac => "FLAC",
                EncoderKind::Opus => "Opus",
            }
        );
        return None;
    };
    match ConfiguredEncoder::probe(kind, path) {
        Ok(tool) => {
            println!(
                "Optional {} encoder available: {}",
                match kind {
                    EncoderKind::Flac => "FLAC",
                    EncoderKind::Opus => "Opus",
                },
                tool.version()
            );
            Some(tool)
        }
        Err(error) => {
            eprintln!(
                "WARNING: optional {} encoder is unavailable: {error}. Preservation and original reads remain available.",
                match kind {
                    EncoderKind::Flac => "FLAC",
                    EncoderKind::Opus => "Opus",
                }
            );
            None
        }
    }
}
async fn shutdown_signal() -> Result<()> {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .context("install SIGTERM handler")?;
        tokio::select! {
            result = tokio::signal::ctrl_c() => result.context("wait for interrupt"),
            _ = terminate.recv() => Ok(()),
        }
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c().await.context("wait for interrupt")
}

fn password() -> Result<String> {
    use std::io::{IsTerminal, Read};
    if std::io::stdin().is_terminal() {
        let password = rpassword::prompt_password("Choose a password (at least 12 characters): ")?;
        let confirmation = rpassword::prompt_password("Repeat password: ")?;
        if password != confirmation {
            bail!("Passwords did not match; no account changes made");
        }
        return Ok(password);
    }
    let mut password = String::new();
    std::io::stdin().take(1026).read_to_string(&mut password)?;
    Ok(password.trim_end_matches(['\r', '\n']).to_owned())
}
fn tls(config: &Config, create: bool) -> Result<Pairing> {
    let root = config.state.join("tls");
    if !root.join("cert.pem").exists() {
        if !create {
            bail!("TLS identity missing; run setup first");
        }
        std::fs::create_dir_all(&root)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))?;
        }
        let certified = rcgen::generate_simple_self_signed(config.tls_names.clone())?;
        std::fs::write(root.join("key.pem"), certified.key_pair.serialize_pem())?;
        std::fs::write(root.join("cert.pem"), certified.cert.pem())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(root.join("key.pem"), std::fs::Permissions::from_mode(0o600))?;
        }
    }
    let pem = std::fs::read_to_string(root.join("cert.pem"))?;
    let endpoints = config
        .endpoints
        .iter()
        .map(|e| {
            Ok(Endpoint {
                url: e.parse()?,
                route: Route::Lan,
                latency_ms: None,
                healthy: true,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    // Filled from private operational state by the caller; it is never inferred from a hostname.
    Ok(Pairing {
        server_id: ServerId::new(),
        certificate_sha256: Digest::sha256(pem.as_bytes()).hex,
        certificate_pem: pem,
        endpoints,
    })
}
// Read only storage locations: unrelated listener/provider settings cannot block recovery.
#[derive(Deserialize)]
#[serde(default)]
struct StoragePaths {
    state: PathBuf,
    archive: PathBuf,
}
impl Default for StoragePaths {
    fn default() -> Self {
        let config = Config::default();
        Self {
            state: config.state,
            archive: config.archive,
        }
    }
}
fn storage_paths(config: &std::path::Path) -> Result<StoragePaths> {
    if !config.exists() {
        return Ok(StoragePaths::default());
    }
    toml::from_str(&std::fs::read_to_string(config).with_context(|| format!("Cannot read configuration {}. For account commands, use --state PATH to select the existing account-storage folder.", config.display()))?)
        .with_context(|| format!("Cannot read storage locations from {}. For account commands, use --state PATH to select the existing folder containing private.sqlite.", config.display()))
}
fn account_storage(config: &std::path::Path, state: Option<&std::path::Path>) -> Result<PathBuf> {
    match state {
        Some(path) => Ok(path.to_owned()),
        None => Ok(storage_paths(config)?.state),
    }
}
#[tokio::main]
async fn main() -> Result<()> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let cli = Cli::parse();
    if let Command::ValidateArchiveExport { export } = &cli.command {
        let inventory = collector_store::validate_archive_export(export)?;
        println!(
            "Archive export valid: {} files. Checked bytes only; no database was opened.",
            inventory.files.len()
        );
        return Ok(());
    }
    if let Command::ImportArchive { export, archive } = &cli.command {
        collector_store::import_archive(export, archive)?;
        println!("Archive import complete: {}", archive.display());
        println!(
            "Create a configuration pointing archive at this directory and state at a new, separate directory. Run setup, then unowned-collections and recover-collection-owner --collection-id ID --username NAME."
        );
        println!(
            "No accounts, grants, TLS, server identity or pending jobs were copied. Collection records and original files may contain private information."
        );
        return Ok(());
    }
    if let Command::ExportArchive { destination } = &cli.command {
        let paths = storage_paths(&cli.config)?;
        anyhow::ensure!(
            cli.config.is_file()
                && paths.archive.is_dir()
                && paths.state.join("private.sqlite").is_file(),
            "export requires an existing installation and configuration; check paths before retrying"
        );
        let store = Store::open(&paths.archive, &paths.state)?;
        store.export_archive(destination)?;
        println!(
            "Verified portable archive export: {}",
            destination.display()
        );
        println!(
            "Includes collection records and originals, which may contain private information. No accounts, grants, TLS, server identity, pending jobs or configuration were copied."
        );
        return Ok(());
    }
    if matches!(
        cli.command,
        Command::UnownedCollections { .. } | Command::RecoverCollectionOwner { .. }
    ) {
        let paths = storage_paths(&cli.config)?;
        anyhow::ensure!(
            cli.config.is_file()
                && paths.archive.is_dir()
                && paths.state.join("private.sqlite").is_file(),
            "ownership recovery requires an existing installation and account state; run setup first"
        );
        let store = Arc::new(Store::open(&paths.archive, &paths.state)?);
        let service = collector_app::LocalOwnershipRecoveryService::new(store);
        match &cli.command {
            Command::UnownedCollections { cursor, limit } => {
                let page = service.unowned_collections(*cursor, *limit)?;
                for collection in page.items {
                    println!("{}\t{:?}", collection.id, collection.name);
                }
                if let Some(cursor) = page.next_cursor {
                    println!("More collections: unowned-collections --cursor {cursor}");
                }
            }
            Command::RecoverCollectionOwner {
                collection_id,
                username,
            } => {
                let grant = service.recover_owner(*collection_id, username)?;
                println!(
                    "Owner access confirmed: {:?} for collection {} (grant revision {}).",
                    grant.username, grant.collection_id, grant.revision.0
                );
                println!(
                    "Sign in to manage this collection and grant other accounts access through its normal controls."
                );
            }
            _ => unreachable!(),
        }
        return Ok(());
    }
    if let Command::Backup { destination } = &cli.command {
        let paths = storage_paths(&cli.config)?;
        anyhow::ensure!(
            cli.config.is_file()
                && paths.archive.is_dir()
                && paths.state.join("private.sqlite").is_file(),
            "backup requires an existing installation and configuration; check paths before retrying"
        );
        let store = Store::open(&paths.archive, &paths.state)?;
        store.backup(destination, &cli.config)?;
        println!("Verified backup: {}", destination.display());
        println!(
            "Includes private credentials, TLS and pending-work scratch. Keep the backup private."
        );
        return Ok(());
    }
    if let Command::ValidateBackup { backup } = &cli.command {
        let inventory = collector_store::validate_backup(backup)?;
        println!(
            "Backup valid: {} files; original archive: {}; original private state: {}",
            inventory.files.len(),
            inventory.archive_root,
            inventory.state_root
        );
        println!("Validation checked bytes only; no database was opened or migrated.");
        return Ok(());
    }
    if let Command::Restore {
        backup,
        archive,
        state,
        archive_only,
    } = &cli.command
    {
        let inventory = collector_store::validate_backup(backup)?;
        let expected = inventory
            .files
            .iter()
            .find(|f| f.path == "collector.toml")
            .context("configuration missing from backup")?;
        anyhow::ensure!(
            expected.bytes.0 <= 4 * 1024 * 1024,
            "configuration exceeds 4 MiB"
        );
        let mut original = Vec::new();
        std::io::Read::read_to_end(
            &mut std::io::Read::take(
                std::fs::File::open(backup.join("collector.toml"))?,
                4 * 1024 * 1024 + 1,
            ),
            &mut original,
        )?;
        anyhow::ensure!(
            Digest::sha256(&original) == expected.digest,
            "configuration changed after validation"
        );
        let mut value: toml::Value = toml::from_str(std::str::from_utf8(&original)?)?;
        let archive = std::path::absolute(archive)?;
        let state = std::path::absolute(state)?;
        value["archive"] =
            toml::Value::String(archive.to_str().context("non-UTF8 archive path")?.into());
        value["state"] = toml::Value::String(state.to_str().context("non-UTF8 state path")?.into());
        let configuration = toml::to_string_pretty(&value)?;
        let output = std::path::absolute(&cli.config)?;
        collector_store::restore_backup(
            backup,
            &archive,
            &state,
            &output,
            configuration.as_bytes(),
            if *archive_only {
                collector_app::RestoreScope::ArchiveOnly
            } else {
                collector_app::RestoreScope::Full
            },
        )?;
        println!("Restore complete. Configuration: {}", output.display());
        println!(
            "Archive: {}\nPrivate state: {}",
            archive.display(),
            state.display()
        );
        println!(
            "Private directories use mode 0700 and copied files/configuration use mode 0600 on Unix."
        );
        if *archive_only {
            println!(
                "Fresh private identity required: run setup, then unowned-collections and recover-collection-owner --collection-id ID --username NAME. No accounts, grants, TLS or pending jobs were copied."
            );
        } else {
            println!(
                "Server identity, credentials and TLS were restored from the backup. Retire the original instance; never run both copies."
            );
        }
        for field in ["inboxes", "dat_sources", "encoders"] {
            if let Some(paths) = value.get(field) {
                println!("Review external {field} paths for this host (not copied): {paths}");
            }
        }
        println!(
            "Review listener addresses, TLS names, allowed origins and endpoints, then run doctor before serve."
        );
        return Ok(());
    }
    if matches!(cli.command, Command::Paths) {
        let paths = storage_paths(&cli.config)?;
        for (label, path) in [
            ("Configuration", cli.config.clone()),
            ("Account storage", paths.state.clone()),
            ("Public HTTPS certificate", paths.state.join("tls/cert.pem")),
            ("Archive", paths.archive),
        ] {
            let absolute = std::path::absolute(&path)?;
            println!(
                "{label}: {} ({})",
                absolute.display(),
                if absolute.exists() {
                    "exists"
                } else {
                    "missing"
                }
            );
        }
        return Ok(());
    }
    if let Command::Accounts {
        state,
        limit,
        cursor,
    } = &cli.command
    {
        let state = account_storage(&cli.config, state.as_deref())?;
        let accounts = Store::local_accounts(&state, cursor.as_deref(), *limit)?;
        for account in accounts.items {
            println!(
                "{}: {}, {}{}",
                account.username,
                if account.enabled {
                    "enabled"
                } else {
                    "disabled"
                },
                if account.administrator {
                    "administrator"
                } else {
                    "regular account"
                },
                if account.is_default {
                    ", default account"
                } else {
                    ""
                }
            );
        }
        if let Some(cursor) = accounts.next_cursor {
            println!("More accounts: accounts --cursor {cursor:?}");
        }
        return Ok(());
    }
    if let Command::RecoverAccount { state, username } = &cli.command {
        let state = account_storage(&cli.config, state.as_deref())?;
        println!(
            "Recovering username {username:?} from {}. Browser sessions, apps, pending approvals, recovery codes and passkeys will be revoked. Roles and enabled status are not affected.",
            std::path::absolute(&state)?.display()
        );
        Store::local_recover(
            &state,
            collector_app::LocalAccountSelector::Username(username.clone()),
            &password()?,
        )?;
        println!(
            "Recovered username {username:?}. Sign in with the new password, save new recovery codes, register passkeys and reconnect apps."
        );
        return Ok(());
    }
    let mut config: Config = if cli.config.exists() {
        toml::from_str(&std::fs::read_to_string(&cli.config)?)?
    } else {
        Config::default()
    };
    config.jobs.validate()?;
    if let Some(remote) = &config.remote_media {
        remote.validate()?;
        anyhow::ensure!(
            !config.remote_media_allowed,
            "a separate remote media endpoint requires control-origin media fallback disabled"
        );
    }
    if let Some(reminder) = &config.backup_reminder {
        reminder.validate()?;
    }
    if !config.public_bind.ip().is_loopback() || !config.private_bind.ip().is_loopback() {
        bail!(
            "HTTP listeners must bind loopback; use a dedicated trusted proxy for public/private access. Direct LAN clients use the TLS listener."
        );
    }
    if config
        .private_tls_bind
        .is_some_and(|address| address.ip().is_unspecified())
    {
        bail!("private_tls_bind must name an explicit interface address, not a wildcard");
    }
    let mut passkeys = if matches!(
        cli.command,
        Command::Setup { .. } | Command::Doctor | Command::Serve
    ) {
        let configuration = collector_server::passkeys::Configuration::from_origin(
            config.authentication_origin.as_deref(),
        )?;
        if let Some(reason) = configuration.unavailable_reason() {
            println!("{}", reason.message());
        } else {
            println!(
                "Passkeys configured. Browser trust and the exact authentication origin remain required."
            );
        }
        Some(configuration)
    } else {
        None
    };
    if matches!(cli.command, Command::Doctor) {
        println!("Configuration: {}", cli.config.display());
        if !cli.config.exists() {
            bail!("Configuration missing. Run setup first.");
        }
        instructions(&config, &cli.config);
        let origin = local_origin(&config);
        if !config.allowed_origins.contains(&origin) {
            bail!(
                "Local browser origin {origin} is missing from allowed_origins. Stop the server, run setup, then serve."
            );
        }
        if !config.state.join("private.sqlite").exists()
            || !config.state.join("tls/cert.pem").exists()
        {
            bail!("Account state or TLS identity missing. Run setup.");
        }
        if let Some(settings) = &config.backup_reminder {
            let history = if settings.externally_managed {
                None
            } else {
                collector_store::read_backup_history(&config.state)
                    .ok()
                    .flatten()
            };
            println!(
                "Backup reminder (record only, no revalidation): {}",
                serde_json::to_string(&collector_app::backup_reminder(
                    settings,
                    history.as_ref(),
                    chrono::Utc::now()
                ))?
            );
        }
        probe_optional_encoder(EncoderKind::Flac, config.encoders.flac.as_ref());
        probe_optional_encoder(EncoderKind::Opus, config.encoders.opus.as_ref());
        if let Some(user_agent) = &config.providers.musicbrainz_user_agent {
            MusicBrainzProvider::new(user_agent)?;
            if let Some(key) = config.providers.acoustid_api_key.as_deref() {
                collector_providers::AcoustIdProvider::new(user_agent, key)?;
            }
            CoverArtArchiveProvider::new(user_agent)?;
        }
        println!(
            "Configuration checks passed. This does not test passwords, collection grants or network reachability."
        );
        return Ok(());
    }
    if !cli.config.exists() && !matches!(cli.command, Command::Setup { .. }) {
        bail!("Configuration missing. Run setup first.");
    }
    let store =
        Arc::new(Store::open(&config.archive, &config.state)?.with_job_settings(config.jobs)?);
    match cli.command {
        Command::Doctor => unreachable!(),
        Command::Setup { username } => {
            if store.user_count()? == 0 {
                println!(
                    "Setting up Junk Collector. Originals: {}. Private state: {}.",
                    config.archive.display(),
                    config.state.display()
                );
                store.create_user(&username, &password()?, true)?;
            } else {
                println!("Keeping existing accounts, passwords, collections and TLS identity.");
            }
            tls(&config, true)?;
            let origin = local_origin(&config);
            if !config.allowed_origins.contains(&origin) {
                config.allowed_origins.push(origin);
            }
            if cli.config.exists() {
                let backup = cli
                    .config
                    .with_extension(format!("toml.{}.bak", uuid::Uuid::new_v4()));
                std::fs::copy(&cli.config, &backup)?;
                println!("Previous configuration: {}", backup.display());
            }
            std::fs::write(&cli.config, toml::to_string_pretty(&config)?)?;
            instructions(&config, &cli.config);
            println!(
                "Setup complete. Run serve with this configuration, then sign in with your existing password (or the one just created)."
            );
        }
        Command::RecoverAccount { .. } | Command::Accounts { .. } | Command::Paths => {
            unreachable!()
        }
        Command::Backup { .. }
        | Command::ExportArchive { .. }
        | Command::ValidateArchiveExport { .. }
        | Command::ImportArchive { .. }
        | Command::ValidateBackup { .. }
        | Command::Restore { .. }
        | Command::UnownedCollections { .. }
        | Command::RecoverCollectionOwner { .. } => {
            unreachable!()
        }
        Command::InspectSnapshotDuplicates => {
            println!(
                "{}",
                serde_json::to_string_pretty(&store.inspect_snapshot_duplicates()?)?
            );
        }
        Command::ApplySnapshotDuplicates { plan, backup } => {
            let mut bytes = Vec::new();
            std::io::Read::read_to_end(
                &mut std::io::Read::take(std::fs::File::open(plan)?, 16 * 1024 * 1024 + 1),
                &mut bytes,
            )?;
            anyhow::ensure!(
                bytes.len() <= 16 * 1024 * 1024,
                "conversion plan exceeds 16 MiB"
            );
            let plan: collector_store::SnapshotDuplicatePlan = serde_json::from_slice(&bytes)?;
            store.apply_snapshot_duplicates(&plan, &backup)?;
            println!(
                "Conversion complete: {} payload bytes removed, {} reference bytes added; historical snapshot IDs retained.",
                plan.saved_bytes, plan.reference_bytes
            );
        }
        Command::Reindex => {
            store.reindex()?;
            println!("Catalog rebuilt. Access grants were not reconstructed from archive records.");
        }
        Command::InspectCollectionType { collection_id } => {
            let service = CollectionTypeAdministrationService::new(
                store.clone(),
                Arc::new(collector_modules::Registry::builtins()),
            );
            println!(
                "{}",
                serde_json::to_string_pretty(&service.inspect(collection_id)?)?
            );
        }
        Command::AssignCollectionType {
            collection_id,
            expected_revision,
            module,
        } => {
            let service = CollectionTypeAdministrationService::new(
                store.clone(),
                Arc::new(collector_modules::Registry::builtins()),
            );
            let collection = service.assign(
                collection_id,
                Revision(expected_revision),
                ModuleId(module),
                chrono::Utc::now(),
            )?;
            println!("{}", serde_json::to_string_pretty(&collection)?);
        }
        Command::Pairing => {
            let mut pairing = tls(&config, false)?;
            pairing.server_id = store.server_id()?;
            println!("{}", serde_json::to_string_pretty(&pairing)?);
        }
        Command::Serve => {
            if store.user_count()? == 0 {
                bail!("Run setup before serving.");
            }
            instructions(&config, &cli.config);
            let mut pairing = tls(&config, false)?;
            pairing.server_id = store.server_id()?;
            let mut state = AppState::new(store.clone(), pairing);
            state.exposure = config.exposure;
            state.inboxes = Arc::new(config.inboxes.clone());
            let flac = probe_optional_encoder(EncoderKind::Flac, config.encoders.flac.as_ref());
            let opus = probe_optional_encoder(EncoderKind::Opus, config.encoders.opus.as_ref());
            state.service = state
                .service
                .clone()
                .with_audio_converter(Arc::new(ProcessAudioConverter::new(flac, opus)?));
            let dat_sources = config
                .dat_sources
                .iter()
                .map(|(id, source)| {
                    (
                        id.clone(),
                        LocalDatSource {
                            path: source.path.clone(),
                            system: source.system.clone(),
                            catalog: source.catalog.clone(),
                            byte_domain: source.byte_domain.clone(),
                            byte_domain_version: source.byte_domain_version,
                        },
                    )
                })
                .collect();
            let dat_provider =
                Arc::new(LocalDatProvider::new(dat_sources)?.with_uploads(store.clone()));
            let provider_id = ModuleId(LOCAL_DAT_PROVIDER_ID.into());
            state.service = state
                .service
                .clone()
                .with_streaming_provider(provider_id.clone(), dat_provider.clone())?
                .with_catalog_parser(provider_id, dat_provider.clone())?
                .with_catalog_uploads(store.clone())
                .with_catalog_inspection(Arc::new(collector_providers::StandardCatalogInspector));
            #[cfg(debug_assertions)]
            let accuraterip =
                if let Ok(origin) = std::env::var("JUNK_COLLECTOR_TEST_ACCURATERIP_ORIGIN") {
                    Arc::new(AccurateRipProvider::with_test_origin(
                        "junk-collector-test/0.1",
                        &origin,
                        std::time::Duration::from_secs(5),
                    )?)
                } else {
                    Arc::new(AccurateRipProvider::new("junk-collector/0.1")?)
                };
            #[cfg(not(debug_assertions))]
            let accuraterip = Arc::new(AccurateRipProvider::new("junk-collector/0.1")?);
            let accuraterip_id = ModuleId(ACCURATERIP_PROVIDER_ID.into());
            state.service = state
                .service
                .clone()
                .with_streaming_provider(
                    accuraterip_id.clone(),
                    Arc::new(collector_app::BoundedProviderAdapter(accuraterip.clone())),
                )?
                .with_catalog_parser(accuraterip_id, accuraterip)?;
            if let Some(user_agent) = &config.providers.musicbrainz_user_agent {
                #[cfg(debug_assertions)]
                let musicbrainz =
                    if let Ok(origin) = std::env::var("JUNK_COLLECTOR_TEST_MUSICBRAINZ_ORIGIN") {
                        Arc::new(MusicBrainzProvider::with_test_origin(
                            user_agent,
                            &origin,
                            std::time::Duration::from_secs(5),
                            std::time::Duration::from_millis(1),
                            std::time::Duration::from_secs(60),
                            std::time::Duration::from_secs(10),
                        )?)
                    } else {
                        Arc::new(MusicBrainzProvider::new(user_agent)?)
                    };
                #[cfg(not(debug_assertions))]
                let musicbrainz = Arc::new(MusicBrainzProvider::new(user_agent)?);
                if let Some(key) = config.providers.acoustid_api_key.as_deref() {
                    #[cfg(debug_assertions)]
                    let acoustid =
                        if let Ok(origin) = std::env::var("JUNK_COLLECTOR_TEST_ACOUSTID_ORIGIN") {
                            collector_providers::AcoustIdProvider::with_test_origin(
                                user_agent, key, &origin,
                            )?
                        } else {
                            collector_providers::AcoustIdProvider::new(user_agent, key)?
                        };
                    #[cfg(not(debug_assertions))]
                    let acoustid = collector_providers::AcoustIdProvider::new(user_agent, key)?;
                    state.service = state.service.clone().with_streaming_provider(
                        ModuleId(collector_modules::music::ACOUSTID_PROVIDER_ID.into()),
                        Arc::new(collector_app::BoundedProviderAdapter(Arc::new(acoustid))),
                    )?;
                }
                let musicbrainz_id =
                    ModuleId(collector_modules::music::MUSICBRAINZ_PROVIDER_ID.into());
                state.service = state
                    .service
                    .clone()
                    .with_streaming_provider(
                        musicbrainz_id.clone(),
                        Arc::new(collector_app::BoundedProviderAdapter(musicbrainz.clone())),
                    )?
                    .with_catalog_parser(musicbrainz_id, musicbrainz)?;

                #[cfg(debug_assertions)]
                let cover_art =
                    if let Ok(origin) = std::env::var("JUNK_COLLECTOR_TEST_COVER_ART_ORIGIN") {
                        Arc::new(CoverArtArchiveProvider::with_test_origins(
                            user_agent,
                            &origin,
                            &[origin.as_str()],
                            std::time::Duration::from_secs(5),
                        )?)
                    } else {
                        Arc::new(CoverArtArchiveProvider::new(user_agent)?)
                    };
                #[cfg(not(debug_assertions))]
                let cover_art = Arc::new(CoverArtArchiveProvider::new(user_agent)?);
                let cover_art_id =
                    ModuleId(collector_modules::music::COVER_ART_ARCHIVE_PROVIDER_ID.into());
                state.service = state
                    .service
                    .clone()
                    .with_streaming_provider(cover_art_id.clone(), cover_art.clone())?
                    .with_catalog_parser(cover_art_id, cover_art.clone())?
                    .with_artwork_source(cover_art);
            }
            state.catalog_sources = Arc::new(
                dat_provider
                    .configured_sources()
                    .map(|(id, source)| collector_server::CatalogSourceView {
                        id: id.into(),
                        system: source.system.clone(),
                        catalog: source.catalog.clone(),
                        byte_domain: source.byte_domain.clone(),
                        byte_domain_version: source.byte_domain_version,
                    })
                    .collect(),
            );
            state.multi_user = config.multi_user;
            state.backup_reminder = config.backup_reminder.clone();
            state.allowed_origins = Arc::new(config.allowed_origins.clone());
            state.passkeys = passkeys
                .take()
                .expect("serve validates passkey configuration before opening storage");
            let _job_runtime = collector_server::start_job_workers(state.clone())?;
            let _local_runtime = match config
                .local_playback_bind
                .filter(|_| config.local_playback_enabled)
            {
                Some(bind) => {
                    Some(collector_server::local_playback::start(&mut state, bind).await?)
                }
                None => None,
            };
            state.remote_media_allowed = config.remote_media_allowed;
            state.remote_media = config.remote_media.clone().map(Arc::new);
            let _media_authority = match &config.remote_media {
                Some(remote) => {
                    Some(collector_server::remote_media::start(state.clone(), remote).await?)
                }
                None => None,
            };
            state.direct_media = false;
            let public = router(state.clone());
            state.direct_media = true;
            let lan = router(state.clone());
            state.ingress = Ingress::Private;
            let private = router(state.clone());
            let private_tls = router(state);
            let public_listener = tokio::net::TcpListener::bind(config.public_bind)
                .await
                .context("public listener")?;
            let private_listener = tokio::net::TcpListener::bind(config.private_bind)
                .await
                .context("private listener")?;
            let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(
                config.state.join("tls/cert.pem"),
                config.state.join("tls/key.pem"),
            )
            .await?;
            tracing::info!(public=%config.public_bind,private=%config.private_bind,lan=%config.lan_bind,"serving public, private, and direct LAN TLS listeners");
            let private_tls_config = tls_config.clone();
            let private_tls_server = async {
                match config.private_tls_bind {
                    Some(address) => {
                        axum_server::bind_rustls(address, private_tls_config)
                            .serve(private_tls.into_make_service_with_connect_info::<SocketAddr>())
                            .await
                    }
                    None => std::future::pending::<std::io::Result<()>>().await,
                }
            };
            tokio::select! {
                result=axum::serve(public_listener,public.into_make_service_with_connect_info::<SocketAddr>())=>{result?;},
                result=axum::serve(private_listener,private.into_make_service_with_connect_info::<SocketAddr>())=>{result?;},
                result=axum_server::bind_rustls(config.lan_bind,tls_config).serve(lan.into_make_service_with_connect_info::<SocketAddr>())=>{result?;},
                result=private_tls_server=>{result?;},
                result=shutdown_signal()=>{result?; tracing::info!("stopping listeners; durable uploads remain resumable");}
            }
        }
    }
    Ok(())
}
