use axum::{
    Router,
    http::{HeaderValue, header},
};
use tower_http::set_header::SetResponseHeaderLayer;

// Re-export SecurityConfig from configrs
pub use configrs::SecurityConfig;

/// Apply security headers to the router
pub fn apply_security_layer<S>(config: &SecurityConfig, router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    if !config.enabled {
        tracing::info!("Security headers middleware disabled");
        return router;
    }

    tracing::info!("Applying security headers middleware");

    let mut router = router;

    // X-Content-Type-Options
    if config.content_type_options {
        router = router.layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ));
        tracing::debug!("Security: X-Content-Type-Options enabled");
    }

    // X-Frame-Options
    if !config.frame_options.is_empty() {
        router = router.layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_str(&config.frame_options).unwrap_or_else(|_| {
                tracing::warn!(
                    "Invalid frame_options value: {}, using DENY",
                    config.frame_options
                );
                HeaderValue::from_static("DENY")
            }),
        ));
        tracing::debug!("Security: X-Frame-Options set to {}", config.frame_options);
    }

    // X-XSS-Protection
    if !config.xss_protection.is_empty() {
        router = router.layer(SetResponseHeaderLayer::overriding(
            header::X_XSS_PROTECTION,
            HeaderValue::from_str(&config.xss_protection).unwrap_or_else(|_| {
                tracing::warn!(
                    "Invalid xss_protection value: {}, using default",
                    config.xss_protection
                );
                HeaderValue::from_static("1; mode=block")
            }),
        ));
        tracing::debug!(
            "Security: X-XSS-Protection set to {}",
            config.xss_protection
        );
    }

    // Strict-Transport-Security (HSTS)
    if !config.strict_transport_security.is_empty() {
        router = router.layer(SetResponseHeaderLayer::overriding(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_str(&config.strict_transport_security).unwrap_or_else(|_| {
                tracing::warn!("Invalid HSTS value, using default");
                HeaderValue::from_static("max-age=31536000; includeSubDomains")
            }),
        ));
        tracing::debug!("Security: HSTS enabled");
    }

    // Referrer-Policy
    if !config.referrer_policy.is_empty() {
        router = router.layer(SetResponseHeaderLayer::overriding(
            header::REFERRER_POLICY,
            HeaderValue::from_str(&config.referrer_policy).unwrap_or_else(|_| {
                tracing::warn!("Invalid referrer_policy value, using default");
                HeaderValue::from_static("strict-origin-when-cross-origin")
            }),
        ));
        tracing::debug!(
            "Security: Referrer-Policy set to {}",
            config.referrer_policy
        );
    }

    // Additional recommended security headers
    router = router
        .layer(SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static("default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' data:; connect-src 'self'"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_DNS_PREFETCH_CONTROL,
            HeaderValue::from_static("off"),
        ));

    tracing::debug!("Security: Additional CSP and DNS prefetch control headers applied");

    router
}
