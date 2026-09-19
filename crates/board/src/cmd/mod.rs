mod cli;
#[cfg(feature = "completions")]
mod completions;
pub(crate) mod health;
pub use cli::{Cli, CliArgs};
#[cfg(feature = "completions")]
pub(crate) use completions::output_completions;
