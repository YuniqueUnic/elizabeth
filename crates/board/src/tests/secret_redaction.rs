use crate::cmd::CliArgs;
use crate::config::{AuthConfig, DatabaseConfig, RoomCreationDefaults};

#[test]
fn cli_debug_redacts_jwt_secret_and_db_credentials() {
    let args = CliArgs {
        config_file: Some("/tmp/config.yaml".into()),
        port: Some(4092),
        listen_addr: Some("0.0.0.0".into()),
        jwt_secret: Some("cli-jwt-secret-must-not-appear".into()),
        db_url: Some("postgresql://alice:cli-db-pass@db.internal/app".into()),
        verbose: Some(1),
    };

    let debug = format!("{args:?}");
    assert!(!debug.contains("cli-jwt-secret-must-not-appear"));
    assert!(!debug.contains("cli-db-pass"));
    assert!(debug.contains("[REDACTED]"));
    assert!(debug.contains("0.0.0.0"));
}

#[test]
fn auth_config_debug_redacts_jwt_secret() {
    let auth = AuthConfig::new("unit-test-jwt-secret-with-enough-length".into())
        .expect("secret length is valid");
    let debug = format!("{auth:?}");
    assert!(!debug.contains("unit-test-jwt-secret-with-enough-length"));
    assert!(debug.contains("[REDACTED]"));
}

#[test]
fn database_and_room_debug_redact_sensitive_fields() {
    let database = DatabaseConfig {
        url: "postgresql://user:board-db-pass@localhost:5432/elizabeth".into(),
        ..Default::default()
    };
    let room = RoomCreationDefaults {
        password: Some("default-room-pass".into()),
        ..Default::default()
    };

    let db_debug = format!("{database:?}");
    let room_debug = format!("{room:?}");
    assert!(!db_debug.contains("board-db-pass"));
    assert!(db_debug.contains("[REDACTED]"));
    assert!(!room_debug.contains("default-room-pass"));
    assert!(room_debug.contains("[REDACTED]"));
}
