use axum::{
    Router,
    extract::Request,
    http::{HeaderMap, HeaderValue},
    middleware::Next,
    response::Response,
};
use logrs::Instrument;
use uuid::Uuid;

// Re-export RequestIdConfig from configrs
pub use configrs::RequestIdConfig;

/// Apply request ID middleware to the router
pub fn apply_request_id_layer<S>(config: &RequestIdConfig, router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    if !config.enabled {
        logrs::info!("Request ID middleware disabled");
        return router;
    }

    logrs::info!(
        "Applying request ID middleware with header: {}",
        config.header_name
    );

    let header_name = config.header_name.clone();
    let generate_if_missing = config.generate_if_missing;

    router.layer(axum::middleware::from_fn(
        move |request: Request, next: Next| {
            let header_name = header_name.clone();
            async move {
                let request_id =
                    extract_or_generate_request_id(&request, &header_name, generate_if_missing);

                // Add request ID to logrs span
                let span = logrs::info_span!(
                    "request",
                    request_id = %request_id,
                );

                async move {
                    // Process the request
                    let mut response = next.run(request).await;

                    // Add request ID to response headers
                    response.headers_mut().insert(
                        header_name.parse::<axum::http::HeaderName>().unwrap(),
                        HeaderValue::from_str(&request_id).unwrap(),
                    );

                    response
                }
                .instrument(span)
                .await
            }
        },
    ))
}

fn extract_or_generate_request_id(
    request: &Request,
    header_name: &str,
    generate_if_missing: bool,
) -> String {
    // Try to extract existing request ID from headers
    if let Some(header_value) = request.headers().get(header_name)
        && let Ok(request_id) = header_value.to_str()
        && !request_id.is_empty()
    {
        logrs::debug!("Found existing request ID: {}", request_id);
        return request_id.to_string();
    }

    // Generate new request ID if configured to do so
    if generate_if_missing {
        let request_id = Uuid::new_v4().to_string();
        logrs::debug!("Generated new request ID: {}", request_id);
        request_id
    } else {
        logrs::debug!("No request ID found and generation disabled");
        "unknown".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };

    #[test]
    fn test_extract_existing_request_id() {
        let mut request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        request
            .headers_mut()
            .insert("X-Request-Id", HeaderValue::from_static("test-request-id"));

        let request_id = extract_or_generate_request_id(&request, "X-Request-Id", true);
        assert_eq!(request_id, "test-request-id");
    }

    #[test]
    fn test_generate_new_request_id() {
        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let request_id = extract_or_generate_request_id(&request, "X-Request-Id", true);
        assert!(!request_id.is_empty());
        assert_ne!(request_id, "unknown");
        // UUID format validation
        assert_eq!(request_id.len(), 36); // Standard UUID length
        assert_eq!(request_id.chars().filter(|&c| c == '-').count(), 4); // UUID has 4 hyphens
    }

    #[test]
    fn test_no_generation_when_disabled() {
        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let request_id = extract_or_generate_request_id(&request, "X-Request-Id", false);
        assert_eq!(request_id, "unknown");
    }
}
