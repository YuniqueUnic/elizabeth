use tracing::level_filters::LevelFilter;

pub(crate) fn init(cfg: &configrs::Config) {
    // Initialize logger with appropriate level based on verbose count
    let match_log_level = |level: &str| match level.to_lowercase().as_str() {
        "error" => LevelFilter::ERROR,
        "warn" => LevelFilter::WARN,
        "info" => LevelFilter::INFO,
        "debug" => LevelFilter::DEBUG,
        "trace" => LevelFilter::TRACE,
        _ => LevelFilter::OFF,
    };
    let log_level = match_log_level(&cfg.app.log_level);

    log_service::init_logger_with_level(log_level);
    tracing::info!("Logger initialized with level: {:?}", log_level);
}
