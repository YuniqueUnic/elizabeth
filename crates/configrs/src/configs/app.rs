use derive_more::From;
use smart_default::SmartDefault;

use crate::merge::{Merge, overwrite, overwrite_not_empty_string};

#[derive(From, Merge, SmartDefault, Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct AppConfig {
    #[default = "127.0.0.1"]
    #[merge(strategy = overwrite_not_empty_string)]
    pub addr: String,
    #[default = 4092]
    #[merge(strategy = overwrite)]
    pub port: u16,
    #[default = "info"]
    #[merge(strategy = overwrite_not_empty_string)]
    pub log_level: String,
    #[default = "sqlite:app.db"]
    #[merge(strategy = overwrite_not_empty_string)]
    pub db_url: String,
    #[default = "secret"]
    #[merge(strategy = overwrite_not_empty_string)]
    pub jwt_secret: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_merge() {
        let mut left = AppConfig::default();
        assert_eq!(left.addr, "127.0.0.1");
        assert_eq!(left.port, 4092);
        assert_eq!(left.log_level, "info");
        let right = AppConfig {
            addr: "128.0.0.1".to_string(),
            port: 0,
            log_level: "debug".to_string(),
            db_url: "sqlite://test.db".to_string(),
            jwt_secret: "foobar".into(), // pragma: allowlist secret
        };

        left.merge(right);
        assert_eq!(left.addr, "128.0.0.1");
        assert_eq!(left.port, 0);
        assert_eq!(left.log_level, "debug");
        assert_eq!(left.db_url, "sqlite://test.db");
        assert_eq!(left.jwt_secret, "foobar");
    }
}
