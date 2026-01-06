use crate::cmd;

use anyhow::{Result, bail};
use std::path::Path;

pub fn init(args: &cmd::CliArgs) -> Result<configrs::Config> {
    if let Some(config_file) = &args.config_file
        && !Path::new(config_file).exists()
    {
        bail!("Config file '{}' does not exist", config_file);
    }

    // Load config from specified file or default file
    let cfg_mgr = if let Some(config_file) = &args.config_file {
        configrs::ConfigManager::new_with_file(config_file)
    } else {
        configrs::ConfigManager::new()
    };

    let should_persist_default = args.config_file.is_none() && !cfg_mgr.file_exists();
    let cfg = merge_config_with_cli_args(args, cfg_mgr, should_persist_default)?;

    Ok(cfg)
}

macro_rules! merge_cli_arg {
    // Direct assignment for Option<T>
    ($cfg_field:expr, $cli_field:expr) => {
        if let Some(value) = $cli_field {
            $cfg_field = value;
        }
    };

    // Assignment with inline transformation
    ($cfg_field:expr, $cli_field:expr => |$value:ident| $transform:expr) => {
        if let Some($value) = $cli_field {
            $cfg_field = $transform;
        }
    };

    // Assignment with function call
    ($cfg_field:expr, $cli_field:expr, $func:expr) => {
        if let Some(value) = $cli_field {
            $cfg_field = $func(value);
        }
    };
}

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

fn merge_config_with_cli_args(
    args: &cmd::CliArgs,
    cfg_mgr: configrs::ConfigManager,
    persist_default_file: bool,
) -> Result<configrs::Config> {
    let mut cfg: configrs::Config = cfg_mgr.source()?;
    apply_program_env_overrides(&mut cfg);

    let match_log_level = |verbose: u8| {
        let res = match verbose {
            0 => "off",
            1 => "info",
            2 => "debug",
            _ => "trace",
        };
        res.to_string()
    };

    merge_cli_arg!(cfg.app.server.port, args.port);
    merge_cli_arg!(cfg.app.server.addr, args.listen_addr.clone());

    if let Some(verbose) = args.verbose.filter(|value| *value > 0) {
        cfg.app.logging.level = match_log_level(verbose);
    }

    // If logging level is empty or invalid after merging CLI/env, fall back to info
    if cfg.app.logging.level.trim().is_empty() {
        cfg.app.logging.level = "info".to_string();
    }

    merge_cli_arg!(cfg.app.jwt.secret, args.jwt_secret.clone());
    merge_cli_arg!(cfg.app.database.url, args.db_url.clone());

    if persist_default_file && let Err(e) = cfg_mgr.save(&cfg) {
        log::warn!(
            "Failed to persist merged configuration ({}). Continuing with in-memory config.",
            e
        );
    }

    Ok(cfg)
}

fn apply_program_env_overrides(cfg: &mut configrs::Config) {
    apply_env!(env_string, "LOG_LEVEL", cfg.app.logging.level);

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
        "JWT_MAX_REFRESH_COUNT",
        cfg.app.jwt.max_refresh_count
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

    apply_env!(env_i64, "ROOM_MAX_SIZE", cfg.app.room.max_size);
    apply_env!(
        env_i64,
        "ROOM_MAX_TIMES_ENTERED",
        cfg.app.room.max_times_entered
    );

    apply_env!(
        env_i64,
        "UPLOAD_RESERVATION_TTL_SECONDS",
        cfg.app.upload.reservation_ttl_seconds
    );

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

fn env_u32(key: &str) -> Option<u32> {
    env_parse::<u32>(key)
}

fn env_u64(key: &str) -> Option<u64> {
    env_parse::<u64>(key)
}

fn env_usize(key: &str) -> Option<usize> {
    env_parse::<usize>(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::{fs, path::PathBuf};
    use tempfile::tempdir;

    struct EnvGuard(Vec<(String, Option<String>)>);

    impl EnvGuard {
        fn set(vars: Vec<(&str, Option<String>)>) -> Self {
            let mut previous = Vec::new();
            for (key, value) in vars {
                let key = key.to_string();
                let prev = std::env::var(&key).ok();
                match value {
                    Some(val) => unsafe { std::env::set_var(&key, val) },
                    None => unsafe { std::env::remove_var(&key) },
                }
                previous.push((key, prev));
            }
            Self(previous)
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, value) in self.0.drain(..) {
                if let Some(val) = value {
                    unsafe { std::env::set_var(&key, val) };
                } else {
                    unsafe { std::env::remove_var(&key) };
                }
            }
        }
    }

    fn with_temp_home() -> (tempfile::TempDir, EnvGuard) {
        let temp = tempdir().expect("tempdir");
        let guard = EnvGuard::set(vec![
            ("HOME", Some(temp.path().to_string_lossy().into_owned())),
            ("CONFIG_DIR", None),
            ("USERPROFILE", None),
            ("LOCALAPPDATA", None),
            ("APPDATA", None),
        ]);
        (temp, guard)
    }

    fn read_config_file(path: &PathBuf) -> String {
        fs::read_to_string(path).expect("read config file")
    }

    #[test]
    #[serial]
    fn logging_defaults_to_info_without_verbose() {
        let (_temp_dir, _guard) = with_temp_home();
        let args = cmd::CliArgs::default();
        let cfg = init(&args).expect("config loaded");
        assert_eq!(cfg.app.logging.level, "info");

        let default_path = configrs::ConfigManager::default_config_path();
        assert!(
            default_path.exists(),
            "default config file should be created"
        );
    }

    #[test]
    #[serial]
    fn verbose_flag_sets_logging_level() {
        let (_temp_dir, _guard) = with_temp_home();
        let args = cmd::CliArgs {
            verbose: Some(2),
            ..Default::default()
        };
        let cfg = init(&args).expect("config loaded");
        assert_eq!(cfg.app.logging.level, "debug");
    }

    #[test]
    #[serial]
    fn env_overrides_config_when_cli_absent() {
        let temp = tempdir().expect("tempdir");
        let config_path = temp.path().join("custom.yaml");
        fs::write(&config_path, "app:\n  server:\n    port: 1111\n").unwrap();

        let _env_guard = EnvGuard::set(vec![("ELIZABETH__APP__SERVER__PORT", Some("2222".into()))]);

        let args = cmd::CliArgs {
            config_file: Some(config_path.to_string_lossy().into_owned()),
            ..Default::default()
        };

        let cfg = init(&args).expect("config loaded");
        assert_eq!(cfg.app.server.port, 2222);

        let persisted = read_config_file(&config_path);
        assert!(persisted.contains("port: 1111"));
    }

    #[test]
    #[serial]
    fn cli_overrides_env_and_config_without_mutating_file() {
        let temp = tempdir().expect("tempdir");
        let config_path = temp.path().join("custom.yaml");
        fs::write(&config_path, "app:\n  server:\n    port: 1111\n").unwrap();

        let _env_guard = EnvGuard::set(vec![("ELIZABETH__APP__SERVER__PORT", Some("2222".into()))]);

        let args = cmd::CliArgs {
            config_file: Some(config_path.to_string_lossy().into_owned()),
            port: Some(3333),
            ..Default::default()
        };

        let cfg = init(&args).expect("config loaded");
        assert_eq!(cfg.app.server.port, 3333);

        let persisted = read_config_file(&config_path);
        assert!(persisted.contains("port: 1111"));
    }

    #[test]
    #[serial]
    fn env_log_level_overrides_defaults() {
        let (_temp_dir, _guard) = with_temp_home();
        let _env_guard = EnvGuard::set(vec![("LOG_LEVEL", Some("debug".into()))]);

        let args = cmd::CliArgs::default();
        let cfg = init(&args).expect("config loaded");

        assert_eq!(cfg.app.logging.level.to_lowercase(), "debug");
    }

    #[test]
    #[serial]
    fn init_errors_when_custom_config_missing() {
        let temp = tempdir().expect("tempdir");
        let missing = temp.path().join("missing.yaml");
        let args = cmd::CliArgs {
            config_file: Some(missing.to_string_lossy().into_owned()),
            ..Default::default()
        };

        let err = init(&args).expect_err("expected error for missing config");
        assert!(
            err.to_string().contains("does not exist"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    #[serial]
    fn program_env_overrides_apply_before_cli() {
        let (_temp_dir, _guard) = with_temp_home();
        let _env_guard = EnvGuard::set(vec![
            ("LOG_LEVEL", Some("debug".into())),
            ("JWT_SECRET", Some("env-secret".into())),
            ("MIDDLEWARE_SECURITY_ENABLED", Some("false".into())),
        ]);
        let args = cmd::CliArgs::default();
        let cfg = init(&args).expect("config loaded");
        assert_eq!(cfg.app.logging.level, "debug");
        assert_eq!(cfg.app.jwt.secret, "env-secret");
        assert!(!cfg.app.middleware.security.enabled);
    }

    #[test]
    #[serial]
    fn cli_env_file_priority_respected() {
        let (temp_dir, _guard) = with_temp_home();
        let config_path = temp_dir.path().join("custom.yaml");
        fs::write(&config_path, "app:\n  logging:\n    level: warn\n").unwrap();

        let _env_guard = EnvGuard::set(vec![("LOG_LEVEL", Some("info".into()))]);

        let args = cmd::CliArgs {
            config_file: Some(config_path.to_string_lossy().into_owned()),
            verbose: Some(2),
            ..Default::default()
        };

        let cfg = init(&args).expect("config loaded");
        assert_eq!(cfg.app.logging.level, "debug");
    }
}
