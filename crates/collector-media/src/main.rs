use clap::Parser;
use std::{net::SocketAddr, path::PathBuf};
#[derive(Parser)]
struct Args {
    #[arg(long, conflicts_with = "http_socket")]
    bind: Option<SocketAddr>,
    #[arg(long, conflicts_with = "bind")]
    http_socket: Option<PathBuf>,
    #[arg(long)]
    authority_socket: PathBuf,
    #[arg(long)]
    archive: PathBuf,
    #[arg(long)]
    require_read_only_archive: bool,
    #[arg(long)]
    allowed_origin: Vec<String>,
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.require_read_only_archive {
        collector_media::require_read_only_archive(&args.archive)?;
    }
    let key = std::fs::read_to_string(
        args.authority_socket
            .parent()
            .ok_or_else(|| anyhow::anyhow!("socket directory missing"))?
            .join("service-key"),
    )?;
    let state = collector_media::MediaState::new(
        &args.archive,
        &args.authority_socket,
        key,
        args.allowed_origin,
    )?;
    let router = collector_media::router(state);
    if let Some(socket) = args.http_socket {
        use std::os::unix::fs::{FileTypeExt, PermissionsExt};
        if let Ok(metadata) = std::fs::symlink_metadata(&socket) {
            anyhow::ensure!(
                metadata.file_type().is_socket(),
                "refusing non-socket HTTP path"
            );
            anyhow::ensure!(
                tokio::net::UnixStream::connect(&socket).await.is_err(),
                "media HTTP socket already in use"
            );
            std::fs::remove_file(&socket)?;
        }
        let listener = tokio::net::UnixListener::bind(&socket)?;
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o660))?;
        serve(listener, router).await?;
        std::fs::remove_file(socket)?;
    } else {
        serve(
            tokio::net::TcpListener::bind(
                args.bind
                    .unwrap_or_else(|| "127.0.0.1:9846".parse().expect("constant bind")),
            )
            .await?,
            router,
        )
        .await?;
    }
    Ok(())
}
async fn serve<L>(mut listener: L, router: axum::Router) -> anyhow::Result<()>
where
    L: axum::serve::Listener,
{
    let mut tasks = tokio::task::JoinSet::new();
    let stop = async {
        #[cfg(unix)]
        {
            let mut terminate =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("termination signal");
            tokio::select! {_=tokio::signal::ctrl_c()=>{},_=terminate.recv()=>{}}
        }
        #[cfg(not(unix))]
        {
            let _ = tokio::signal::ctrl_c().await;
        }
    };
    tokio::pin!(stop);
    loop {
        tokio::select! {
            _=&mut stop=>break,
            (stream,_)=listener.accept()=>{if tasks.len()<64 {tasks.spawn(collector_media::connection(stream,router.clone()));}},
            _=tasks.join_next(),if !tasks.is_empty()=>{},
        }
    }
    tasks.abort_all();
    while tasks.join_next().await.is_some() {}
    Ok(())
}
