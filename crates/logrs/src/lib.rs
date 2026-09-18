mod logger;

pub use logger::init_logger;
pub use tracing::{Instrument, Level, info_span};
pub use tracing_subscriber::filter::LevelFilter;

#[macro_export]
macro_rules! log_println {
    ($level:expr, $($xx:tt)*) => {{
        #[cfg(feature = "logging")]
        {
            log::log!($level, $($xx)*);
        }

        #[cfg(not(feature = "logging"))]
        {
            println!($($xx)*);
        }
    }};
}

#[macro_export]
macro_rules! trace {
    ($($xx:tt)*) => {
        $crate::log_println!(log::Level::Trace, $($xx)*);
    };
}

#[macro_export]
macro_rules! debug {
    ($($xx:tt)*) => {
        $crate::log_println!(log::Level::Debug, $($xx)*);
    };
}

#[macro_export]
macro_rules! info {
    ($($xx:tt)*) => {
        $crate::log_println!(log::Level::Info, $($xx)*);
    };
}

#[macro_export]
macro_rules! warn {
    ($($xx:tt)*) => {
        $crate::log_println!(log::Level::Warn, $($xx)*);
    };
}

#[macro_export]
macro_rules! error {
    ($($xx:tt)*) => {
        $crate::log_println!(log::Level::Error, $($xx)*);
    };
}
