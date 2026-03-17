use crate::cli::Cli;
use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{Shell, generate};

/// Write shell completion script to stdout.
pub fn run(shell: Shell) -> Result<()> {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "tersify", &mut std::io::stdout());
    Ok(())
}
