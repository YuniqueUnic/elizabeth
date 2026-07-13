use crate::cmd::CliArgs;
use crate::config::{AuthConfig, DatabaseConfig, RoomCreationDefaults};

#[test]
fn cli_debug_redacts_jwt_secret_and_db_credentials() {
    let args = CliArgs {
        config_file: Some("/tmp/config.yaml".into()),
        port: Some(4092),
        listen_addr: Some("0.0.0.0".into()),
        jwt_secret: Some("cli-jwt-secret-must-not-appear".into()), // pragma: allowlist secret
        db_url: Some("postgresql://alice:cli-db-pass@db.internal/app".into()), // pragma: allowlist secret
        verbose: Some(1),
    };

    let debug = format!("{args:?}");
    assert!(!debug.contains("cli-jwt-secret-must-not-appear")); // pragma: allowlist secret
    assert!(!debug.contains("cli-db-pass")); // pragma: allowlist secret
    assert!(debug.contains("[REDACTED]"));
    assert!(debug.contains("0.0.0.0"));
}

#[test]
fn auth_config_debug_redacts_jwt_secret() {
    let auth = AuthConfig::new("unit-test-jwt-secret-with-enough-length".into()) // pragma: allowlist secret
        .expect("secret length is valid");
    let debug = format!("{auth:?}");
    assert!(!debug.contains("unit-test-jwt-secret-with-enough-length")); // pragma: allowlist secret
    assert!(debug.contains("[REDACTED]"));
}

#[test]
fn database_and_room_debug_redact_sensitive_fields() {
    let database = DatabaseConfig {
        url: "postgresql://user:board-db-pass@localhost:5432/elizabeth".into(), // pragma: allowlist secret
        ..Default::default()
    };
    let room = RoomCreationDefaults {
        password: Some("default-room-pass".into()), // pragma: allowlist secret
        ..Default::default()
    };

    let db_debug = format!("{database:?}");
    let room_debug = format!("{room:?}");
    assert!(!db_debug.contains("board-db-pass")); // pragma: allowlist secret
    assert!(db_debug.contains("[REDACTED]"));
    assert!(!room_debug.contains("default-room-pass")); // pragma: allowlist secret
    assert!(room_debug.contains("[REDACTED]"));
}
