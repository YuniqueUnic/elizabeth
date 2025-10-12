mod cmd;
mod init;

use std::net::SocketAddr;

use axum::{Json, routing::get};
use axum_responses::http::HttpResponse;
use clap::Parser;
use configrs::Config;
use shadow_rs::shadow;
use utoipa::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_scalar::{Scalar, Servable};

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

const API_PREFIX: &str = "/api/v1";

async fn start_server(cfg: &Config) -> anyhow::Result<()> {
    println!("Starting server with args: {cfg:#?}");
    log_service::init(cfg);

    let addr: SocketAddr = format!("{}:{}", cfg.app.addr, cfg.app.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    let (router, api) = OpenApiRouter::new()
        .routes(routes!(openapi))
        .split_for_parts();

    // let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi()).nest(API_PREFIX, router);
    let scalar_path = format!("{}/scalar", API_PREFIX);
    let app = router.merge(Scalar::with_url(scalar_path.clone(), api));

    println!("Server listening on http://{addr}");
    println!("Scalar listening on http://{addr}{}", &scalar_path);

    axum::serve(listener, app.into_make_service())
        .await
        .map_err(anyhow::Error::new)
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "API Docs",
        description = "API documentation for the server",
        version = build::PKG_VERSION
    ),
    tags((name=API_PREFIX,description="API v1")),
    paths(openapi)
)]
struct ApiDoc;

/// Return JSON version of an OpenAPI schema
#[utoipa::path(
    get,
    path = format!("{}/openapi.json",API_PREFIX),
    responses(
        (status = 200, description = "JSON file", body = ())
    )
)]
async fn openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

fn health() -> HttpResponse {
    HttpResponse::Ok()
}

fn status() -> HttpResponse {
    HttpResponse::Ok()
}
