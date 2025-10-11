mod cmd;
mod init;

use configrs::AppConfig;
use shadow_rs::shadow;

shadow!(build);

pub async fn run() -> anyhow::Result<()> {
    const_service::init();

    let cli = cmd::Cli::parse();
    println!("Parsed CLI arguments: {cli:?}");
    match cli {
        cmd::Cli::Start(args) => {
            let cfg = config_service::init(&args)?;
            inner_run(&cfg).await?
        }
        #[cfg(feature = "completions")]
        cmd::Cli::Completions { shell } => cmd::output_completions(shell)?,
    }
    Ok(())
}

async fn inner_run(cfg: &AppConfig) -> anyhow::Result<()> {
    Ok(())
}
