use crate::cmd;

use anyhow::Result;

pub fn init(args: &cmd::CliArgs) -> Result<configrs::Config> {
    // Load config from specified file or default file
    let cfg_mgr = if let Some(config_file) = &args.config_file {
        configrs::ConfigManager::new_with_file(config_file)
    } else {
        configrs::ConfigManager::new()
    };

    let cfg = merge_config_with_cli_args(args, cfg_mgr)?;

    Ok(cfg)
}

macro_rules! merge_cli_arg {
    // Direct assignment for Option<T>
    ($cfg_field:expr, $cli_field:expr) => {
        if let Some(value) = $cli_field {
            $cfg_field = value;
        }
    };

    // Assignment with inline transformation
    ($cfg_field:expr, $cli_field:expr => |$value:ident| $transform:expr) => {
        if let Some($value) = $cli_field {
            $cfg_field = $transform;
        }
    };

    // Assignment with function call
    ($cfg_field:expr, $cli_field:expr, $func:expr) => {
        if let Some(value) = $cli_field {
            $cfg_field = $func(value);
        }
    };
}

fn merge_config_with_cli_args(
    args: &cmd::CliArgs,
    cfg_mgr: configrs::ConfigManager,
) -> Result<configrs::Config> {
    let mut cfg: configrs::Config = cfg_mgr.source()?;

    let match_log_level = |verbose: u8| {
        let res = match verbose {
            0 => "off",
            1 => "info",
            2 => "debug",
            _ => "trace",
        };
        res.to_string()
    };

    merge_cli_arg!(cfg.app.port, args.port);
    merge_cli_arg!(cfg.app.addr, args.listen_addr.clone());
    merge_cli_arg!(cfg.app.log_level, args.verbose, match_log_level);
    merge_cli_arg!(cfg.app.jwt_secret, args.jwt_secret.clone());

    cfg_mgr.save(&cfg)?;

    Ok(cfg)
}
