pub mod cmd;
pub mod db;
mod handlers;
mod init;
pub mod models;
pub mod repository;
pub mod route;

use std::net::SocketAddr;
use std::sync::Arc;

use clap::Parser;
use shadow_rs::shadow;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::db::{init_db, run_migrations};
use crate::init::{cfg_service, const_service, log_service};
use configrs::Config;

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

    // 初始化数据库
    let database_url = &cfg.app.db_url;
    let db_pool = init_db(database_url).await?;
    run_migrations(&db_pool).await?;

    let addr: SocketAddr = format!("{}:{}", cfg.app.addr, cfg.app.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    let (scalar_path, router) = build_api_router(Arc::new(db_pool));

    println!("Server listening on http://{addr}");
    println!("Scalar listening on http://{addr}{}", &scalar_path);

    axum::serve(listener, router.into_make_service())
        .await
        .map_err(anyhow::Error::new)
}

fn build_api_router(db_pool: Arc<crate::db::DbPool>) -> (String, axum::Router) {
    let (root_router, mut api) = OpenApiRouter::new()
        .routes(routes!(route::openapi))
        .split_for_parts();
    let (status_router, status_api) = route::status::api_router().split_for_parts();
    let (room_router, room_api) = route::room::api_router(db_pool).split_for_parts();

    let router = root_router.merge(status_router).merge(room_router);
    api.merge(status_api);
    api.merge(room_api);

    let (scalar, scalar_path) = route::scalar(api);
    let router = router.merge(scalar);
    (scalar_path, router)
}
