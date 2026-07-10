use crate::{cmd, init::cfg_service};
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
    let cfg = cfg_service::init(&args).expect("config loaded");
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
    let cfg = cfg_service::init(&args).expect("config loaded");
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

    let cfg = cfg_service::init(&args).expect("config loaded");
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

    let cfg = cfg_service::init(&args).expect("config loaded");
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
    let cfg = cfg_service::init(&args).expect("config loaded");

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

    let err = cfg_service::init(&args).expect_err("expected error for missing config");
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
    let cfg = cfg_service::init(&args).expect("config loaded");
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

    let cfg = cfg_service::init(&args).expect("config loaded");
    assert_eq!(cfg.app.logging.level, "debug");
}

#[test]
#[serial]
fn room_expiry_durations_load_from_yaml() {
    let temp = tempdir().expect("tempdir");
    let config_path = temp.path().join("custom.yaml");
    fs::write(
        &config_path,
        "app:\n  room:\n    expiry:\n      allowed_ages: [1m, 2h, 365d]\n      default_age: 2h\n",
    )
    .expect("write config");

    let args = cmd::CliArgs {
        config_file: Some(config_path.to_string_lossy().into_owned()),
        ..Default::default()
    };
    let cfg = cfg_service::init(&args).expect("config loaded");

    assert_eq!(
        cfg.app
            .room
            .expiry
            .allowed_ages
            .iter()
            .map(|age| age.as_secs())
            .collect::<Vec<_>>(),
        vec![60, 7200, 31_536_000]
    );
    assert_eq!(cfg.app.room.expiry.default_age.as_secs(), 7200);
}

#[test]
#[serial]
fn nested_env_overrides_room_expiry_age_list() {
    let temp = tempdir().expect("tempdir");
    let config_path = temp.path().join("custom.yaml");
    fs::write(
        &config_path,
        "app:\n  room:\n    expiry:\n      allowed_ages: [1m, 2h]\n      default_age: 2h\n",
    )
    .expect("write config");
    let _env_guard = EnvGuard::set(vec![
        (
            "ELIZABETH__APP__ROOM__EXPIRY__ALLOWED_AGES",
            Some("30m,12h".into()),
        ),
        (
            "ELIZABETH__APP__ROOM__EXPIRY__DEFAULT_AGE",
            Some("12h".into()),
        ),
    ]);

    let args = cmd::CliArgs {
        config_file: Some(config_path.to_string_lossy().into_owned()),
        ..Default::default()
    };
    let cfg = cfg_service::init(&args).expect("config loaded");

    assert_eq!(
        cfg.app
            .room
            .expiry
            .allowed_ages
            .iter()
            .map(|age| age.as_secs())
            .collect::<Vec<_>>(),
        vec![1800, 43_200]
    );
    assert_eq!(cfg.app.room.expiry.default_age.as_secs(), 43_200);
}
