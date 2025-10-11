use crate::cmd::cli::Shell;

use anyhow::anyhow;

include!(concat!(env!("OUT_DIR"), "/completions.rs"));

/// Get completion script for the specified shell
fn get_completion_script(shell: &str) -> Option<&'static str> {
    match shell {
        "bash" => Some(BASH_COMPLETION),
        "elvish" => Some(ELVISH_COMPLETION),
        "fish" => Some(FISH_COMPLETION),
        "powershell" => Some(POWERSHELL_COMPLETION),
        "zsh" => Some(ZSH_COMPLETION),
        _ => None,
    }
}

/// Output completion script for the specified shell
pub fn output_completions(shell: Shell) -> anyhow::Result<()> {
    let completion_script = get_completion_script(&shell.to_string())
        .ok_or_else(|| anyhow!(format!("Unsupported shell: {}", shell)))?;
    println!("{}", completion_script);
    Ok(())
}
