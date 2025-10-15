use logrs::LevelFilter;

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

    logrs::init_logger(log_level);
    logrs::info!("Logger initialized with level: {:?}", log_level);
}
