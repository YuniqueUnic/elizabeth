mod cmd;
mod init;

use axum_responses::http::HttpResponse;
use clap::Parser;
use configrs::Config;
use shadow_rs::shadow;

use crate::init::{cfg_service, const_service, log_service};

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

    Ok(())
}

fn health() -> HttpResponse {
    HttpResponse::Ok()
}
