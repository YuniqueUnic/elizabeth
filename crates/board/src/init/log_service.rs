use logrs::LevelFilter;

fn parse_log_level(level: &str) -> LevelFilter {
    match level.trim().to_lowercase().as_str() {
        "error" => LevelFilter::ERROR,
        "warn" | "warning" => LevelFilter::WARN,
        "info" | "" => LevelFilter::INFO,
        "debug" => LevelFilter::DEBUG,
        "trace" => LevelFilter::TRACE,
        // Fallback to INFO instead of OFF to avoid silent logging
        _ => LevelFilter::INFO,
    }
}

pub(crate) fn init(cfg: &configrs::Config) {
    let log_level = parse_log_level(&cfg.app.logging.level);

    logrs::init_logger(log_level);
    logrs::info!("Logger initialized with level: {:?}", log_level);
}
