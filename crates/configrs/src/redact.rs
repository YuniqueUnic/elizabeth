//! Helpers for secret-safe `Debug` formatting.

/// Placeholder used when a secret value is present but must not be logged.
pub const REDACTED: &str = "[REDACTED]";

/// Format a secret string for `Debug` without revealing its contents.
pub fn secret_for_debug(value: &str) -> &'static str {
    if value.is_empty() {
        "<empty>"
    } else {
        REDACTED
    }
}

/// Format an optional secret for `Debug`.
pub fn optional_secret_for_debug(value: Option<&str>) -> Option<&'static str> {
    value.map(secret_for_debug)
}

/// Redact credentials embedded in a database URL userinfo section.
///
/// Examples:
/// - `postgresql://user:pass@host/db` -> `postgresql://[REDACTED]@host/db`
/// - `sqlite://app.db?mode=rwc` stays unchanged (no userinfo)
pub fn database_url_for_debug(url: &str) -> String {
    let Some(scheme_sep) = url.find("://") else {
        return url.to_string();
    };
    let scheme = &url[..scheme_sep];
    let rest = &url[scheme_sep + 3..];
    let Some(at) = rest.find('@') else {
        return url.to_string();
    };
    format!("{scheme}://{REDACTED}@{}", &rest[at + 1..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_for_debug_hides_non_empty_values() {
        assert_eq!(secret_for_debug("super-secret-value"), REDACTED);
        assert_eq!(secret_for_debug(""), "<empty>");
    }

    #[test]
    fn optional_secret_for_debug_preserves_none() {
        assert_eq!(optional_secret_for_debug(None), None);
        assert_eq!(optional_secret_for_debug(Some("room-pass")), Some(REDACTED));
    }

    #[test]
    fn database_url_for_debug_redacts_userinfo() {
        assert_eq!(
            database_url_for_debug("postgresql://alice:s3cret@db.example:5432/app"),
            format!("postgresql://{REDACTED}@db.example:5432/app")
        );
        assert_eq!(
            database_url_for_debug("sqlite://app.db?mode=rwc"),
            "sqlite://app.db?mode=rwc"
        );
        assert_eq!(database_url_for_debug("not-a-url"), "not-a-url");
    }
}
