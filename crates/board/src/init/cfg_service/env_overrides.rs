macro_rules! apply_env {
    ($env_fn:ident, $key:expr, $target:expr) => {
        if let Some(value) = $env_fn($key) {
            $target = value;
        }
    };
    ($env_fn:ident, $key:expr, $target:expr, $wrap:expr) => {
        if let Some(value) = $env_fn($key) {
            $target = $wrap(value);
        }
    };
}

pub(super) fn apply_program_env_overrides(cfg: &mut configrs::Config) {
    apply_basic_env_overrides(cfg);
    apply_jwt_env_overrides(cfg);
    apply_room_env_overrides(cfg);
    apply_gc_env_overrides(cfg);
    apply_middleware_env_overrides(cfg);
}

fn apply_basic_env_overrides(cfg: &mut configrs::Config) {
    apply_env!(env_string, "LOG_LEVEL", cfg.app.logging.level);

    // Prefer `DATABASE_URL` (recommended/sqlx), fallback to legacy `DATABASE_PATH`.
    if let Some(database_url) = env_string("DATABASE_URL") {
        cfg.app.database.url = database_url;
    } else if let Some(database_path) = env_string("DATABASE_PATH") {
        cfg.app.database.url = normalize_database_url_from_path(database_path);
    }

    apply_env!(
        env_u32,
        "DB_MAX_CONNECTIONS",
        cfg.app.database.max_connections,
        Some
    );
    apply_env!(
        env_u32,
        "DB_MIN_CONNECTIONS",
        cfg.app.database.min_connections,
        Some
    );
}

fn apply_jwt_env_overrides(cfg: &mut configrs::Config) {
    apply_env!(env_string, "JWT_SECRET", cfg.app.jwt.secret);
    apply_env!(env_i64, "JWT_TTL_SECONDS", cfg.app.jwt.ttl_seconds);
    apply_env!(env_i64, "JWT_LEEWAY_SECONDS", cfg.app.jwt.leeway_seconds);
    apply_env!(
        env_i64,
        "JWT_REFRESH_TTL_SECONDS",
        cfg.app.jwt.refresh_ttl_seconds
    );
    apply_env!(
        env_i64,
        "JWT_CLEANUP_INTERVAL_SECONDS",
        cfg.app.jwt.cleanup_interval_seconds
    );
    apply_env!(
        env_bool,
        "JWT_ENABLE_REFRESH_TOKEN_ROTATION",
        cfg.app.jwt.enable_refresh_token_rotation
    );
}

fn apply_room_env_overrides(cfg: &mut configrs::Config) {
    apply_env!(env_byte_size, "ROOM_MAX_SIZE", cfg.app.room.max_size);
    apply_env!(
        env_i64,
        "ROOM_MAX_TIMES_ENTERED",
        cfg.app.room.max_times_entered
    );
    apply_env!(
        env_duration,
        "ROOM_SHARE_DISABLED_LOCK_DURATION",
        cfg.app.room.share_disabled_lock_duration
    );
    if let Some(allowed_ages) = env_list("ROOM_ALLOWED_AGES") {
        let parsed = allowed_ages
            .into_iter()
            .map(|value| value.parse::<configrs::HumanDuration>().ok())
            .collect::<Option<Vec<_>>>();
        if let Some(parsed) = parsed {
            cfg.app.room.expiry.allowed_ages = parsed
                .into_iter()
                .map(|duration| duration.into_inner().into())
                .collect();
        }
    }
    if let Some(default_age) = env_duration("ROOM_DEFAULT_AGE") {
        cfg.app.room.expiry.default_age = default_age.into_inner().into();
    }
    apply_env!(
        env_i64,
        "UPLOAD_RESERVATION_TTL_SECONDS",
        cfg.app.upload.reservation_ttl_seconds
    );
}

fn apply_gc_env_overrides(cfg: &mut configrs::Config) {
    apply_env!(env_u64, "GC_INTERVAL_SECONDS", cfg.app.gc.interval_seconds);
    apply_env!(env_u32, "GC_BATCH_LIMIT", cfg.app.gc.batch_limit);
}

fn apply_middleware_env_overrides(cfg: &mut configrs::Config) {
    apply_tracing_env_overrides(cfg);
    apply_request_id_env_overrides(cfg);
    apply_compression_env_overrides(cfg);
    apply_cors_env_overrides(cfg);
    apply_security_env_overrides(cfg);
    apply_rate_limit_env_overrides(cfg);
}

fn apply_tracing_env_overrides(cfg: &mut configrs::Config) {
    apply_env!(
        env_bool,
        "MIDDLEWARE_TRACING_ENABLED",
        cfg.app.middleware.tracing.enabled
    );
    apply_env!(
        env_string,
        "MIDDLEWARE_TRACING_LEVEL",
        cfg.app.middleware.tracing.level
    );
    apply_env!(
        env_bool,
        "MIDDLEWARE_TRACING_INCLUDE_HEADERS",
        cfg.app.middleware.tracing.include_headers
    );
    apply_env!(
        env_bool,
        "MIDDLEWARE_TRACING_INCLUDE_BODY",
        cfg.app.middleware.tracing.include_body
    );
}

fn apply_request_id_env_overrides(cfg: &mut configrs::Config) {
    apply_env!(
        env_bool,
        "MIDDLEWARE_REQUEST_ID_ENABLED",
        cfg.app.middleware.request_id.enabled
    );
    apply_env!(
        env_string,
        "MIDDLEWARE_REQUEST_ID_HEADER_NAME",
        cfg.app.middleware.request_id.header_name
    );
    apply_env!(
        env_bool,
        "MIDDLEWARE_REQUEST_ID_GENERATE_IF_MISSING",
        cfg.app.middleware.request_id.generate_if_missing
    );
}

fn apply_compression_env_overrides(cfg: &mut configrs::Config) {
    apply_env!(
        env_bool,
        "MIDDLEWARE_COMPRESSION_ENABLED",
        cfg.app.middleware.compression.enabled
    );
    apply_env!(
        env_usize,
        "MIDDLEWARE_COMPRESSION_MIN_CONTENT_LENGTH",
        cfg.app.middleware.compression.min_content_length
    );
}

fn apply_cors_env_overrides(cfg: &mut configrs::Config) {
    apply_env!(
        env_bool,
        "MIDDLEWARE_CORS_ENABLED",
        cfg.app.middleware.cors.enabled
    );
    apply_env!(
        env_list,
        "MIDDLEWARE_CORS_ALLOWED_ORIGINS",
        cfg.app.middleware.cors.allowed_origins
    );
    apply_env!(
        env_list,
        "MIDDLEWARE_CORS_ALLOWED_METHODS",
        cfg.app.middleware.cors.allowed_methods
    );
    apply_env!(
        env_list,
        "MIDDLEWARE_CORS_ALLOWED_HEADERS",
        cfg.app.middleware.cors.allowed_headers
    );
    apply_env!(
        env_bool,
        "MIDDLEWARE_CORS_ALLOW_CREDENTIALS",
        cfg.app.middleware.cors.allow_credentials
    );
    apply_env!(
        env_u64,
        "MIDDLEWARE_CORS_MAX_AGE",
        cfg.app.middleware.cors.max_age
    );
    apply_env!(
        env_list,
        "MIDDLEWARE_CORS_EXPOSE_HEADERS",
        cfg.app.middleware.cors.expose_headers
    );
}

fn apply_security_env_overrides(cfg: &mut configrs::Config) {
    apply_env!(
        env_bool,
        "MIDDLEWARE_SECURITY_ENABLED",
        cfg.app.middleware.security.enabled
    );
    apply_env!(
        env_bool,
        "MIDDLEWARE_SECURITY_CONTENT_TYPE_OPTIONS",
        cfg.app.middleware.security.content_type_options
    );
    apply_env!(
        env_string,
        "MIDDLEWARE_SECURITY_FRAME_OPTIONS",
        cfg.app.middleware.security.frame_options
    );
    apply_env!(
        env_string,
        "MIDDLEWARE_SECURITY_XSS_PROTECTION",
        cfg.app.middleware.security.xss_protection
    );
    apply_env!(
        env_string,
        "MIDDLEWARE_SECURITY_STRICT_TRANSPORT_SECURITY",
        cfg.app.middleware.security.strict_transport_security
    );
    apply_env!(
        env_string,
        "MIDDLEWARE_SECURITY_REFERRER_POLICY",
        cfg.app.middleware.security.referrer_policy
    );
}

fn apply_rate_limit_env_overrides(cfg: &mut configrs::Config) {
    apply_env!(
        env_bool,
        "MIDDLEWARE_RATE_LIMIT_ENABLED",
        cfg.app.middleware.rate_limit.enabled
    );
    apply_env!(
        env_u64,
        "MIDDLEWARE_RATE_LIMIT_PER_SECOND",
        cfg.app.middleware.rate_limit.per_second
    );
    apply_env!(
        env_u64,
        "MIDDLEWARE_RATE_LIMIT_BURST_SIZE",
        cfg.app.middleware.rate_limit.burst_size
    );
    apply_env!(
        env_u64,
        "MIDDLEWARE_RATE_LIMIT_CLEANUP_INTERVAL_SECONDS",
        cfg.app.middleware.rate_limit.cleanup_interval_seconds
    );
}

fn env_string(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| {
            let trimmed = v.trim();
            trimmed.trim_matches('"').trim_matches('\'').to_string()
        })
        .filter(|v| !v.is_empty())
}

fn normalize_database_url_from_path(value: String) -> String {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("sqlite:")
        || lower.starts_with("postgres:")
        || lower.starts_with("postgresql:")
        || lower.starts_with("mysql:")
    {
        trimmed.to_string()
    } else {
        format!("sqlite://{trimmed}")
    }
}

fn env_list(key: &str) -> Option<Vec<String>> {
    let raw = env_string(key)?;
    let items = raw
        .split(',')
        .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    if items.is_empty() { None } else { Some(items) }
}

fn env_parse<T>(key: &str) -> Option<T>
where
    T: std::str::FromStr,
{
    env_string(key)?.parse().ok()
}

fn env_bool(key: &str) -> Option<bool> {
    env_string(key).and_then(|value| match value.to_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    })
}

fn env_i64(key: &str) -> Option<i64> {
    env_parse::<i64>(key)
}

fn env_byte_size(key: &str) -> Option<bytesize::ByteSize> {
    env_parse::<bytesize::ByteSize>(key)
}

fn env_duration(key: &str) -> Option<configrs::HumanDuration> {
    env_parse::<configrs::HumanDuration>(key)
}

fn env_u32(key: &str) -> Option<u32> {
    env_parse::<u32>(key)
}

fn env_u64(key: &str) -> Option<u64> {
    env_parse::<u64>(key)
}

fn env_usize(key: &str) -> Option<usize> {
    env_parse::<usize>(key)
}
