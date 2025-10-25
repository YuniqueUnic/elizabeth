use axum::Router;
use logrs::Level;
use tower_http::trace::{
    DefaultMakeSpan, DefaultOnEos, DefaultOnFailure, DefaultOnRequest, DefaultOnResponse,
    TraceLayer,
};

// Re-export TracingConfig from configrs
pub use configrs::TracingConfig;

/// Apply request tracing middleware to the router
pub fn apply_tracing_layer<S>(config: &TracingConfig, router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    if !config.enabled {
        logrs::info!("Request logrs middleware disabled");
        return router;
    }

    logrs::info!(
        "Applying request logrs middleware with level: {}",
        config.level
    );

    let level = parse_level(&config.level).unwrap_or(Level::INFO);
    let make_span = DefaultMakeSpan::new().level(level);
    let on_response = DefaultOnResponse::new().level(level);
    let on_failure = DefaultOnFailure::new().level(Level::WARN);
    let on_request = DefaultOnRequest::new().level(level);
    let on_eos = DefaultOnEos::new().level(level);

    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(make_span.clone())
        .on_eos(on_eos.clone())
        .on_request(on_request.clone())
        .on_response(on_response.clone())
        .on_failure(on_failure.clone());

    // Configure header and body inclusion if supported
    if config.include_headers {
        logrs::info!("Including headers in trace output");
    }
    if config.include_body {
        logrs::info!("Including body in trace output (warning: may impact performance)");
    }

    router.layer(trace_layer)
}

fn parse_level(level_str: &str) -> Option<Level> {
    match level_str.to_lowercase().as_str() {
        "trace" => Some(Level::TRACE),
        "debug" => Some(Level::DEBUG),
        "info" => Some(Level::INFO),
        "warn" => Some(Level::WARN),
        "error" => Some(Level::ERROR),
        _ => {
            logrs::warn!("Invalid log level '{}', defaulting to INFO", level_str);
            Some(Level::INFO)
        }
    }
}
