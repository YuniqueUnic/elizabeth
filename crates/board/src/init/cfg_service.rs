use crate::cmd;

use anyhow::{Result, bail};
use std::path::Path;

mod env_overrides;
use env_overrides::apply_program_env_overrides;

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

    let legacy_normalized_db_url = normalize_database_url_compat(&cfg.app.database.url);
    if legacy_normalized_db_url != cfg.app.database.url {
        log::warn!(
            "Detected legacy sqlite URL format. Normalized database url from '{}' to '{}'. Consider updating your config file.",
            cfg.app.database.url,
            legacy_normalized_db_url
        );
        cfg.app.database.url = legacy_normalized_db_url;
    }

    let create_mode_db_url = ensure_sqlite_create_mode(&cfg.app.database.url);
    if create_mode_db_url != cfg.app.database.url {
        log::info!(
            "SQLite database url did not specify mode; appended mode=rwc to allow auto-create: '{}' -> '{}'",
            cfg.app.database.url,
            create_mode_db_url
        );
        cfg.app.database.url = create_mode_db_url;
    }

    if persist_default_file && let Err(e) = cfg_mgr.save(&cfg) {
        log::warn!(
            "Failed to persist merged configuration ({}). Continuing with in-memory config.",
            e
        );
    }

    Ok(cfg)
}

fn normalize_database_url_compat(value: &str) -> String {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();

    // Keep SQLx-native URLs untouched.
    if lower.starts_with("sqlite://") || lower.starts_with("sqlite::") {
        return trimmed.to_string();
    }

    // Backward compatibility: historically we used `sqlite:app.db` which SQLx does not accept.
    // Also handle the ambiguous `sqlite:/path` form by normalizing it to a relative path (matching
    // our historical semantics).
    if lower.starts_with("sqlite:") {
        let mut rest = trimmed["sqlite:".len()..].to_string();
        while rest.starts_with('/') {
            rest.remove(0);
        }
        return format!("sqlite://{rest}");
    }

    trimmed.to_string()
}

fn ensure_sqlite_create_mode(value: &str) -> String {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();

    // Only apply to file-based sqlite urls; skip `sqlite::memory:` and friends.
    if !lower.starts_with("sqlite:") || lower.starts_with("sqlite::") {
        return trimmed.to_string();
    }

    if lower.contains("mode=") {
        return trimmed.to_string();
    }

    let joiner = if trimmed.contains('?') { '&' } else { '?' };
    format!("{trimmed}{joiner}mode=rwc")
}
