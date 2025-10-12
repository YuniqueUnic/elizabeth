mod cmd;
mod init;
mod route;

use std::net::SocketAddr;

use clap::Parser;
use shadow_rs::shadow;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::init::{cfg_service, const_service, log_service};
use configrs::Config;
// use route::*;

shadow!(build);

pub async fn run() -> anyhow::Result<()> {
    const_service::init();

    let cli = cmd::Cli::parse();
    println!("Parsed CLI arguments: {cli:?}");
    match cli {
        cmd::Cli::Start(args) => {
            let cfg = cfg_service::init(&args)?;
            start_server(&cfg).await?
        }
        #[cfg(feature = "completions")]
        cmd::Cli::Completions { shell } => cmd::output_completions(shell)?,
    }
    Ok(())
}

async fn start_server(cfg: &Config) -> anyhow::Result<()> {
    println!("Starting server with args: {cfg:#?}");
    log_service::init(cfg);

    let addr: SocketAddr = format!("{}:{}", cfg.app.addr, cfg.app.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    let (router, api) = OpenApiRouter::new()
        .routes(routes!(route::openapi))
        .routes(routes!(route::health))
        .routes(routes!(route::status))
        .split_for_parts();

    let (scalar, scalar_path) = route::scalar(api);
    let app = router.merge(scalar);

    println!("Server listening on http://{addr}");
    println!("Scalar listening on http://{addr}{}", &scalar_path);

    axum::serve(listener, app.into_make_service())
        .await
        .map_err(anyhow::Error::new)
}
