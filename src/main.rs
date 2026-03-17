mod cli;
mod completions;
mod install;
mod stats;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use is_terminal::IsTerminal;
use std::{
    io::{self, Read},
    path::Path,
};
use tersify::{input, tokens};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Install) => install::run()?,
        Some(Command::Uninstall) => install::uninstall()?,
        Some(Command::Stats) => stats::run()?,
        Some(Command::StatsReset) => stats::reset()?,
        Some(Command::Completions { shell }) => completions::run(shell)?,
        None => run_compress(cli)?,
    }

    Ok(())
}

fn run_compress(cli: Cli) -> Result<()> {
    // ── Resolve inputs ───────────────────────────────────────────────────────
    if cli.inputs.is_empty() {
        // No paths given — require piped stdin
        if io::stdin().is_terminal() {
            eprintln!(
                "tersify: no input provided.\n\nUsage:\n  cat file.rs | tersify\n  git diff   | tersify\n  tersify src/\n  tersify --help"
            );
            std::process::exit(1);
        }
        return compress_stdin(&cli);
    }

    let forced = cli.r#type.as_deref();
    let mut total_before = 0usize;
    let mut total_after = 0usize;
    let inputs_count = cli.inputs.len();

    for path_str in &cli.inputs {
        let path = Path::new(path_str);

        if !path.exists() {
            anyhow::bail!("path not found: {}", path.display());
        }

        if path.is_dir() {
            let (out, before, after) = input::compress_directory(path, forced, cli.budget)?;
            print!("{}", out);
            total_before += before;
            total_after += after;
        } else {
            let (out, before, after) = input::compress_file(path, forced, cli.budget)?;
            // Print a header when processing multiple files
            if inputs_count > 1 {
                println!("// === {} ===", path.display());
            }
            print!("{}", out);
            total_before += before;
            total_after += after;
        }
    }

    report(cli.verbose, total_before, total_after, forced.is_some())?;
    Ok(())
}

fn compress_stdin(cli: &Cli) -> Result<()> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;

    if buf.trim().is_empty() {
        return Ok(());
    }

    let (out, before, after) =
        input::compress_content(&buf, cli.r#type.as_deref(), None, cli.budget)?;

    print!("{}", out);
    report(cli.verbose, before, after, false)?;
    Ok(())
}

fn report(verbose: bool, before: usize, after: usize, _forced: bool) -> Result<()> {
    if verbose {
        let pct = tokens::savings_pct(before, after);
        eprintln!(
            "\n[tersify] {} → {} tokens  ({:.0}% saved)",
            before, after, pct
        );
    }
    let _ = stats::record(before, after);
    Ok(())
}
