use crate::{
    cmd,
    config::{AppConfig as RuntimeAppConfig, RoomConfig as RuntimeRoomConfig},
    init::cfg_service,
};
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

fn load_custom_config(config_path: &std::path::Path) -> anyhow::Result<configrs::Config> {
    cfg_service::init(&cmd::CliArgs {
        config_file: Some(config_path.to_string_lossy().into_owned()),
        ..Default::default()
    })
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
fn room_creation_defaults_load_as_one_typed_yaml_policy() {
    let temp = tempdir().expect("tempdir");
    let config_path = temp.path().join("custom.yaml");
    fs::write(
        &config_path,
        "app:\n  room:\n    defaults:\n      password: null\n      max_times_entered: 100\n      max_size: 50MiB\n      permissions:\n        read: true\n        edit: true\n        share: true\n        delete: true\n    expiry:\n      allowed_ages: [1m, 2h]\n      default_age: 2h\n",
    )
    .expect("write config");

    let cfg = load_custom_config(&config_path).expect("config loaded");
    let runtime = RuntimeRoomConfig::try_from(&cfg.app.room).expect("runtime room config");

    assert_eq!(runtime.defaults.password, None);
    assert_eq!(runtime.defaults.max_times_entered, 100);
    assert_eq!(runtime.defaults.max_content_size, 50 * 1024 * 1024);
    assert_eq!(runtime.defaults.permission.bits(), 15);
    assert_eq!(runtime.expiry.default_age_seconds(), 7200);
}

#[test]
#[serial]
fn room_creation_permissions_preserve_four_independent_bits() {
    let temp = tempdir().expect("tempdir");
    let config_path = temp.path().join("custom.yaml");
    fs::write(
        &config_path,
        "app:\n  room:\n    defaults:\n      permissions:\n        read: true\n        edit: false\n        share: true\n        delete: false\n",
    )
    .expect("write config");

    let cfg = load_custom_config(&config_path).expect("typed config loaded");
    let runtime = RuntimeRoomConfig::try_from(&cfg.app.room).expect("runtime room config");
    assert_eq!(runtime.defaults.permission.bits(), 1 | 4);
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

#[test]
#[serial]
fn human_readable_room_limits_load_from_yaml() {
    let temp = tempdir().expect("tempdir");
    let config_path = temp.path().join("custom.yaml");
    fs::write(
        &config_path,
        "app:\n  room:\n    defaults:\n      max_size: 50M\n    share_disabled_lock_duration: 30m\n",
    )
    .expect("write config");

    let cfg = load_custom_config(&config_path).expect("config loaded");

    assert_eq!(cfg.app.room.defaults.max_size.as_u64(), 50_000_000);
    assert_eq!(cfg.app.room.share_disabled_lock_duration.as_secs(), 30 * 60);
}

#[test]
#[serial]
fn integer_room_limits_remain_supported() {
    let temp = tempdir().expect("tempdir");
    let config_path = temp.path().join("custom.yaml");
    fs::write(
        &config_path,
        "app:\n  room:\n    defaults:\n      max_size: 52428800\n    share_disabled_lock_duration: 3600\n",
    )
    .expect("write config");

    let cfg = load_custom_config(&config_path).expect("config loaded");

    assert_eq!(cfg.app.room.defaults.max_size.as_u64(), 52_428_800);
    assert_eq!(cfg.app.room.share_disabled_lock_duration.as_secs(), 3600);
}

#[test]
#[serial]
fn negative_integer_room_lock_duration_is_rejected() {
    let temp = tempdir().expect("tempdir");
    let config_path = temp.path().join("custom.yaml");
    fs::write(
        &config_path,
        "app:\n  room:\n    share_disabled_lock_duration: -1\n",
    )
    .expect("write config");

    let err = load_custom_config(&config_path).expect_err("negative duration must fail");

    assert!(
        err.to_string().contains("share_disabled_lock_duration")
            || err.to_string().contains("non-negative"),
        "unexpected error: {err:#}"
    );
}

#[test]
#[serial]
fn nested_env_overrides_human_readable_room_limits() {
    let temp = tempdir().expect("tempdir");
    let config_path = temp.path().join("custom.yaml");
    fs::write(&config_path, "app: {}\n").expect("write config");
    let _env_guard = EnvGuard::set(vec![
        (
            "ELIZABETH__APP__ROOM__DEFAULTS__MAX_SIZE",
            Some("100MiB".into()),
        ),
        (
            "ELIZABETH__APP__ROOM__SHARE_DISABLED_LOCK_DURATION",
            Some("1h".into()),
        ),
    ]);

    let cfg = load_custom_config(&config_path).expect("config loaded");

    assert_eq!(cfg.app.room.defaults.max_size.as_u64(), 100 * 1024 * 1024);
    assert_eq!(cfg.app.room.share_disabled_lock_duration.as_secs(), 3600);
}

#[test]
#[serial]
fn program_env_aliases_use_human_readable_room_limit_types() {
    let (_temp_dir, _home_guard) = with_temp_home();
    let _env_guard = EnvGuard::set(vec![
        ("ROOM_MAX_SIZE", Some("1G".into())),
        ("ROOM_SHARE_DISABLED_LOCK_DURATION", Some("30m".into())),
    ]);

    let cfg = cfg_service::init(&cmd::CliArgs::default()).expect("config loaded");

    assert_eq!(cfg.app.room.defaults.max_size.as_u64(), 1_000_000_000);
    assert_eq!(cfg.app.room.share_disabled_lock_duration.as_secs(), 1800);
}

#[test]
#[serial]
fn room_limits_save_and_load_roundtrip() {
    let temp = tempdir().expect("tempdir");
    let config_path = temp.path().join("custom.yaml");
    fs::write(&config_path, "app: {}\n").expect("write config");
    let manager =
        configrs::ConfigManager::new_with_file(config_path.to_str().expect("UTF-8 config path"));
    let mut expected = configrs::Config::default();
    expected.app.room.defaults.max_size = bytesize::ByteSize::gib(1);
    expected.app.room.share_disabled_lock_duration = std::time::Duration::from_secs(30 * 60).into();

    manager.save(&expected).expect("save config");
    let loaded: configrs::Config = manager.load().expect("load config");

    assert_eq!(
        loaded.app.room.defaults.max_size.as_u64(),
        1024 * 1024 * 1024
    );
    assert_eq!(loaded.app.room.share_disabled_lock_duration.as_secs(), 1800);
    let persisted = read_config_file(&config_path);
    assert!(persisted.contains("1.0 GiB"));
    assert!(persisted.contains("30m"));
}

#[test]
#[serial]
fn invalid_human_readable_room_limit_units_are_rejected() {
    let temp = tempdir().expect("tempdir");
    let config_path = temp.path().join("custom.yaml");
    fs::write(
        &config_path,
        "app:\n  room:\n    defaults:\n      max_size: 50XB\n    share_disabled_lock_duration: tomorrow\n",
    )
    .expect("write config");

    let err = load_custom_config(&config_path).expect_err("invalid units must fail");

    assert!(
        err.to_string().contains("max_size") || err.to_string().contains("parsable string"),
        "unexpected error: {err:#}"
    );
}

#[test]
#[serial]
fn nested_env_rejects_invalid_human_readable_room_limit_unit() {
    let temp = tempdir().expect("tempdir");
    let config_path = temp.path().join("custom.yaml");
    fs::write(&config_path, "app: {}\n").expect("write config");
    let _env_guard = EnvGuard::set(vec![(
        "ELIZABETH__APP__ROOM__DEFAULTS__MAX_SIZE",
        Some("50XB".into()),
    )]);

    let err = load_custom_config(&config_path).expect_err("invalid nested env must fail");

    assert!(
        err.to_string().contains("max_size") || err.to_string().contains("parsable string"),
        "unexpected error: {err:#}"
    );
}

#[test]
#[serial]
fn nested_env_rejects_invalid_human_readable_room_duration_unit() {
    let temp = tempdir().expect("tempdir");
    let config_path = temp.path().join("custom.yaml");
    fs::write(&config_path, "app: {}\n").expect("write config");
    let _env_guard = EnvGuard::set(vec![(
        "ELIZABETH__APP__ROOM__SHARE_DISABLED_LOCK_DURATION",
        Some("tomorrow".into()),
    )]);

    let err = load_custom_config(&config_path).expect_err("invalid nested env must fail");

    assert!(
        err.to_string().contains("share_disabled_lock_duration")
            || err.to_string().contains("duration"),
        "unexpected error: {err:#}"
    );
}

#[test]
#[serial]
fn room_limit_conversion_preserves_units_and_rejects_zero() {
    let temp = tempdir().expect("tempdir");
    let config_path = temp.path().join("custom.yaml");
    fs::write(
        &config_path,
        "app:\n  room:\n    defaults:\n      max_size: 50MiB\n    share_disabled_lock_duration: 1h\n",
    )
    .expect("write config");
    let cfg = load_custom_config(&config_path).expect("config loaded");

    let runtime_room = RuntimeRoomConfig::try_from(&cfg.app.room).expect("convert room config");
    assert_eq!(runtime_room.defaults.max_content_size, 52_428_800);
    assert_eq!(runtime_room.share_disabled_lock_duration, 3600);

    fs::write(
        &config_path,
        "app:\n  room:\n    defaults:\n      max_size: 0B\n    share_disabled_lock_duration: 0s\n",
    )
    .expect("write zero config");
    let zero_cfg = load_custom_config(&config_path).expect("typed config accepts zero values");
    let zero_room =
        RuntimeRoomConfig::try_from(&zero_cfg.app.room).expect("convert zero room config");
    let mut runtime = RuntimeAppConfig {
        room: zero_room,
        ..Default::default()
    };
    assert!(runtime.validate().is_err());
    runtime.room.defaults.max_content_size = 1;
    assert!(runtime.validate().is_err());
    runtime.room.share_disabled_lock_duration = -1;
    assert!(runtime.validate().is_err());
}

#[test]
#[serial]
fn room_limit_conversion_rejects_i64_overflow() {
    let temp = tempdir().expect("tempdir");
    let config_path = temp.path().join("custom.yaml");
    fs::write(
        &config_path,
        "app:\n  room:\n    defaults:\n      max_size: 9223372036854775808B\n    share_disabled_lock_duration: 1s\n",
    )
    .expect("write config");
    let cfg = load_custom_config(&config_path).expect("typed config accepts u64 range");

    let err = RuntimeRoomConfig::try_from(&cfg.app.room).expect_err("i64 overflow must fail");

    assert!(err.to_string().contains("exceeds the supported range"));

    fs::write(
        &config_path,
        "app:\n  room:\n    defaults:\n      max_size: 1B\n    share_disabled_lock_duration: 9223372036854775808s\n",
    )
    .expect("write duration overflow config");
    let cfg = load_custom_config(&config_path).expect("typed config accepts u64 duration range");

    let err = RuntimeRoomConfig::try_from(&cfg.app.room).expect_err("duration overflow must fail");

    assert!(
        err.to_string()
            .contains("Share-disabled lock duration exceeds the supported range")
    );
}
