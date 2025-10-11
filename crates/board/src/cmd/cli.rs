use clap::Parser;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Default, Parser)]
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
