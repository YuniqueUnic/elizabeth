use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{EnvFilter, fmt, prelude::*, util::SubscriberInitExt};

/// Initialize the logger
pub fn init_logger_with_level(level: LevelFilter) {
    let subscriber = tracing_subscriber::registry();

    let console_layer = Some({
        #[cfg(feature = "structured-log")]
        {
            fmt::layer().json().with_writer(std::io::stderr)
        }
        #[cfg(not(feature = "structured-log"))]
        {
            #[allow(unused_mut)]
            let mut layer = fmt::layer();
            #[cfg(debug_assertions)]
            {
                layer = layer.with_file(true).with_line_number(true);
            }
            layer.with_writer(std::io::stderr)
        }
    });

    subscriber
        .with(
            // 使用 RUST_LOG 环境变量，默认 INFO
            EnvFilter::builder()
                .with_default_directive(level.into())
                .from_env_lossy(),
        )
        .with(console_layer)
        .init()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;
    use tracing::{debug, error, info, trace, warn};

    static INIT: Once = Once::new();

    fn init_test_logger() {
        INIT.call_once(|| {
            init_logger_with_level(LevelFilter::TRACE);
        });
    }

    #[test]
    fn test_custom_log_format() {
        init_test_logger();

        // 测试不同级别的日志输出
        info!("这是一个 INFO 级别的日志");
        warn!("这是一个 WARNING 级别的日志");
        error!("这是一个 ERROR 级别的日志");
        debug!("这是一个 DEBUG 级别的日志");
        trace!("这是一个 TRACE 级别的日志");
    }

    #[test]
    fn test_log_with_structured_data() {
        init_test_logger();

        info!(user_id = 12345, action = "login", "用户登录成功");

        error!(
            error_code = 500,
            error_msg = "database connection failed",
            "数据库连接失败"
        );
    }

    #[test]
    fn test_log_format_matches_requirements() {
        init_test_logger();

        info!(
            "private_key: 2b81d1d0993e6bd02258f150ca75c166cb691d0544e62d96fa6f4cd9cf01a69b, public_key: 97d316b1622e2625ef5d1131831a7c3c374ff687ea2b48da23242b2c3c6aae38, shared_secret: d64c9e8430ac2422c89a34eeba40a3caa5d8269e49fd42d75d1306ec1013b26a"
        );
        warn!("send buffer size is too large: 1544");
        error!("connection timeout occurred");
    }
}
