use clap::Parser;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Default, Parser)]
pub struct CliArgs {
    /// The path to the configuration file, defaults = ~/.config/elife/config.yaml
    #[clap(short = 'c', long, env = "CONFIG_FILE")]
    pub config_file: Option<String>,

    /// The port to run the server on, defaults = 4092
    #[clap(short = 'p', long, env = "PORT")]
    pub port: Option<u16>,

    /// The address to bind the server to, default_value = "127.0.0.1"
    #[clap(short = 'l', long, env = "LISTEN_ADDR")]
    pub listen_addr: Option<String>,

    /// The JWT secret for signing tokens, default_value = "secret"
    #[clap(short = 'j', long, env = "JWT_SECRET")]
    pub jwt_secret: Option<String>,

    /// The database URL, default_value = "sqlite://lifit.db",
    #[clap(short = 'd', long, env = "DATABASE_URL")]
    pub db_url: Option<String>,

    /// The log level to use, -v for info, -vv for debug, -vvv/v for trace, default_value = "off"
    #[clap(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: Option<u8>,
}

impl std::fmt::Debug for CliArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Keep this free of crate deps: build.rs includes this file for completions.
        f.debug_struct("CliArgs")
            .field("config_file", &self.config_file)
            .field("port", &self.port)
            .field("listen_addr", &self.listen_addr)
            .field(
                "jwt_secret",
                &self.jwt_secret.as_ref().map(|_| "[REDACTED]"),
            )
            .field(
                "db_url",
                &self.db_url.as_deref().map(redact_db_url_for_debug),
            )
            .field("verbose", &self.verbose)
            .finish()
    }
}

fn redact_db_url_for_debug(url: &str) -> String {
    let Some(scheme_sep) = url.find("://") else {
        return url.to_string();
    };
    let scheme = &url[..scheme_sep];
    let rest = &url[scheme_sep + 3..];
    let Some(at) = rest.find('@') else {
        return url.to_string();
    };
    format!("{scheme}://[REDACTED]@{}", &rest[at + 1..])
}

#[derive(Debug, Parser)]
#[allow(clippy::large_enum_variant)]
#[clap(author, version = VERSION, about)]
pub enum Cli {
    /// Start the server with the specified arguments
    #[command(alias = "run", alias = "serve")]
    Start(CliArgs),

    #[cfg(feature = "completions")]
    /// Generate shell completions
    #[command(alias = "complete", alias = "comp", alias = "completion")]
    Completions {
        /// The shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

/// Shells for which to generate completions
#[derive(Debug, Copy, Clone, clap::ValueEnum)]
pub enum Shell {
    Bash,
    Elvish,
    Fish,
    #[allow(clippy::enum_variant_names)]
    PowerShell,
    Zsh,
}

impl std::fmt::Display for Shell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Shell::Bash => write!(f, "bash"),
            Shell::Elvish => write!(f, "elvish"),
            Shell::Fish => write!(f, "fish"),
            Shell::PowerShell => write!(f, "powershell"),
            Shell::Zsh => write!(f, "zsh"),
        }
    }
}
