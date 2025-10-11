mod cli;
#[cfg(feature = "completions")]
mod completions;
pub use cli::{Cli, CliArgs};
#[cfg(feature = "completions")]
pub(crate) use completions::output_completions;
