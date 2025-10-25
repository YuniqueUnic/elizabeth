# Middleware System

This directory contains a comprehensive middleware management system for the
Elizabeth board application.

## Architecture

The middleware system is designed to be modular and configurable, allowing users
to enable/disable specific middleware components based on their needs.

### Available Middleware

1. **Tracing Middleware** (`tracing.rs`)
   - Provides request/response logging with structured tracing
   - Configurable log levels for different events
   - Integration with the tracing ecosystem

2. **Request ID Middleware** (`request_id.rs`)
   - Generates unique request IDs for tracing
   - Extracts existing request IDs from headers
   - Adds request IDs to response headers
   - Creates tracing spans with request context

3. **Compression Middleware** (`compression.rs`)
   - Brotli compression for response bodies
   - Configurable compression levels
   - Automatic content negotiation

4. **CORS Middleware** (`cors.rs`)
   - Cross-Origin Resource Sharing support
   - Configurable allowed origins, methods, and headers
   - Support for credentials and private network access

5. **Security Headers Middleware** (`security.rs`)
   - XSS Protection headers
   - Content Type Options
   - Frame Options
   - Referrer Policy
   - Content Security Policy

6. **Rate Limiting Middleware** (`rate_limit.rs`)
   - Request rate limiting based on IP
   - Configurable limits and windows
   - Distributed storage support

## Configuration

The middleware system is controlled by the `MiddlewareConfig` structure:

```rust
#[derive(Debug, Clone, serde::Deserialize)]
pub struct MiddlewareConfig {
    pub tracing: bool,
    pub request_id: bool,
    pub compression: bool,
    pub cors: bool,
    pub security: bool,
    pub rate_limit: bool,
}
```

### Default Configuration

```rust
impl Default for MiddlewareConfig {
    fn default() -> Self {
        Self {
            tracing: true,
            request_id: true,
            compression: false,  // Disabled for development
            cors: false,        // Disabled for development
            security: true,
            rate_limit: false,  // Disabled for development
        }
    }
}
```

## Usage

### Adding New Middleware

1. Create a new module file (e.g., `new_middleware.rs`)
2. Implement the middleware function with the signature:
   ```rust
   pub fn apply_new_middleware_layer<S>(enabled: bool, router: Router<S>) -> Router<S>
   where
       S: Clone + Send + Sync + 'static,
   ```
3. Add the module to `mod.rs`
4. Add the configuration field to `MiddlewareConfig`
5. Apply the middleware in the `apply` function

### Example Custom Middleware

```rust
use axum::Router;

pub fn apply_custom_layer<S>(enabled: bool, router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    if !enabled {
        tracing::info!("Custom middleware disabled");
        return router;
    }

    tracing::info!("Applying custom middleware");
    router.layer(axum::middleware::from_fn(custom_handler))
}

async fn custom_handler(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    // Custom logic here
    next.run(request).await
}
```

## Integration with Application

The middleware system is integrated into the main application in `lib.rs`:

```rust
fn build_api_router(app_state: Arc<AppState>, cfg: &configrs::Config) -> (String, axum::Router) {
    // ... router setup ...

    // Apply middleware configuration
    let middleware_config = crate::middleware::from_app_config(cfg);
    let router = crate::middleware::apply(&middleware_config, router);

    (scalar_path, router)
}
```

## Development vs Production

### Development Settings

- Compression: Disabled (for easier debugging)
- CORS: Disabled (to avoid cross-origin issues during development)
- Rate Limiting: Disabled (to avoid blocking development traffic)

### Production Settings

- Compression: Enabled (for better performance)
- CORS: Enabled (as needed for frontend integration)
- Rate Limiting: Enabled (for protection against abuse)

## Testing

Each middleware module includes comprehensive tests covering:

- Basic functionality
- Configuration enable/disable
- Error handling
- Integration scenarios

Run tests with:

```bash
cargo test middleware
```

## Dependencies

The middleware system relies on several external crates:

- `tower`: Core middleware abstractions
- `tower-http`: HTTP-specific middleware implementations
- `tower_governor`: Rate limiting functionality
- `tracing`: Structured logging and tracing
- `uuid`: Request ID generation
- `axum`: Web framework integration

## Performance Considerations

1. **Order of Operations**: Middleware is applied in the order defined in the
   `apply` function
2. **Memory Usage**: Each middleware adds some overhead to request processing
3. **CPU Impact**: Compression and rate limiting have the highest CPU impact
4. **Network Impact**: CORS headers and security headers add minimal overhead

## Security Notes

1. **CORS Configuration**: Be careful with wildcard origins in production
2. **Rate Limiting**: Ensure rate limits are appropriate for your traffic
   patterns
3. **Security Headers**: Review and customize security headers for your specific
   requirements
4. **Request IDs**: Ensure request IDs don't leak sensitive information

## Future Enhancements

Potential areas for future improvement:

1. **Dynamic Configuration**: Runtime configuration updates
2. **Metrics Integration**: Prometheus metrics for middleware performance
3. **Advanced Rate Limiting**: User-based and API-key-based rate limiting
4. **Circuit Breaker**: Integration with external service calls
5. **Request/Response Transformation**: Middleware for API versioning or format
   conversion
