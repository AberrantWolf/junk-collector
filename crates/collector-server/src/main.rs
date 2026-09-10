use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use collector_client::{Endpoint, Pairing, Route};
use collector_core::*;
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
    /// Create storage, local TLS identity and the initial account. Password is read from stdin.
    Init {
        #[arg(long, default_value = "owner")]
        username: String,
    },
    /// Create or repair local browser setup without changing existing accounts. Stop the server first.
    Setup {
        #[arg(long, default_value = "owner")]
        username: String,
    },
    /// Check configuration without opening or locking the archive.
    Doctor,
    Serve,
    Reindex,
    /// Reset an account password locally and revoke its devices. New password is read from stdin.
    ResetPassword {
        #[arg(long, default_value = "owner")]
        username: String,
    },
    /// Recover exactly one account while the server is stopped. New password is read from stdin.
    RecoverAccount {
        #[arg(long)]
        private_state: PathBuf,
        #[arg(long)]
        user_id: UserId,
    },
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
    multi_user: bool,
    exposure: Exposure,
    authentication_origin: Option<String>,
    inboxes: std::collections::BTreeMap<String, PathBuf>,
    tls_names: Vec<String>,
    allowed_origins: Vec<String>,
    endpoints: Vec<String>,
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
            multi_user: false,
            exposure: Exposure::default(),
            authentication_origin: None,
            inboxes: std::collections::BTreeMap::new(),
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
fn instructions(config: &Config, path: &std::path::Path) {
    println!(
        "\nLocal management: {} (open this exact address on the server computer)",
        local_origin(config)
    );
    println!(
        "For first use: sign in here, choose Create collection (or open Administration), then choose Add item."
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
    if !config.allowed_origins.contains(&local_origin(config)) {
        println!(
            "WARNING: Local management origin is not allowed. Stop the server and run setup to repair it."
        );
    }
    println!(
        "Use localhost exactly for local browser access. Remote browsers require trusted HTTPS and its exact allowed_origins entry. Local HTTP onboarding is Chromium-tested; other browsers may require trusted HTTPS. Never publish the private listener through a public tunnel.\n"
    );
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
            bail!("TLS identity missing; run init first");
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
#[tokio::main]
async fn main() -> Result<()> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let cli = Cli::parse();
    let mut config: Config = if cli.config.exists() {
        toml::from_str(&std::fs::read_to_string(&cli.config)?)?
    } else {
        Config::default()
    };
    match &cli.command {
        Command::RecoverAccount {
            private_state,
            user_id,
        } => {
            Store::local_recover(private_state, *user_id, &password()?)?;
            println!(
                "Recovered account {user_id}; browser sessions and passkeys were revoked. Grants and administrator status were preserved."
            );
            return Ok(());
        }
        Command::ResetPassword { .. } => {
            bail!(
                "reset-password is retired because it cannot identify private state and account safely; use recover-account --private-state PATH --user-id USER_ID"
            );
        }
        _ => {}
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
        println!(
            "Configuration checks passed. This does not test passwords, collection grants or network reachability."
        );
        return Ok(());
    }
    if !cli.config.exists() && !matches!(cli.command, Command::Init { .. } | Command::Setup { .. })
    {
        bail!("Configuration missing. Run setup first.");
    }
    let store = Arc::new(Store::open(&config.archive, &config.state)?);
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
        Command::Init { username } => {
            if store.user_count()? > 0 {
                bail!("already initialized; use reset-password for local recovery");
            }
            let origin = local_origin(&config);
            if !config.allowed_origins.contains(&origin) {
                config.allowed_origins.push(origin);
            }
            let password = password()?;
            store.create_user(&username, &password, true)?;
            let mut pairing = tls(&config, true)?;
            pairing.server_id = store.server_id()?;
            std::fs::write(&cli.config, toml::to_string_pretty(&config)?)?;
            instructions(&config, &cli.config);
            println!(
                "Initialized server {}. Local certificate SHA-256: {}",
                pairing.server_id, pairing.certificate_sha256
            );
        }
        Command::ResetPassword { .. } | Command::RecoverAccount { .. } => unreachable!(),
        Command::Reindex => {
            store.reindex()?;
            println!("Catalog rebuilt. Access grants were not reconstructed from archive records.");
        }
        Command::Pairing => {
            let mut pairing = tls(&config, false)?;
            pairing.server_id = store.server_id()?;
            println!("{}", serde_json::to_string_pretty(&pairing)?);
        }
        Command::Serve => {
            if store.user_count()? == 0 {
                bail!("run init before serving");
            }
            instructions(&config, &cli.config);
            let mut pairing = tls(&config, false)?;
            pairing.server_id = store.server_id()?;
            let default_username = store.default_username()?;
            let mut state = AppState::new(store, pairing);
            state.default_username = default_username;
            state.exposure = config.exposure;
            state.inboxes = Arc::new(config.inboxes.clone());
            state.multi_user = config.multi_user;
            state.allowed_origins = Arc::new(config.allowed_origins.clone());
            if let Some(origin) = &config.authentication_origin {
                state.passkeys = Some(Arc::new(collector_server::passkeys::Passkeys::new(origin)?));
            }
            let _job_runtime = collector_server::start_job_workers(state.service.clone());
            let public = router(state.clone());
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
                _=tokio::signal::ctrl_c()=>{tracing::info!("stopping listeners; durable uploads remain resumable");}
            }
        }
    }
    Ok(())
}
