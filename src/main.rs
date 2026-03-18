mod bench;
mod cli;
mod completions;
mod install;
mod mcp;
mod stats;
mod token_cost;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use install::Target;
use is_terminal::IsTerminal;
use std::{
    io::{self, Read},
    path::Path,
};
use tersify::{compress::CompressOptions, input, tokens};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Install {
            cursor,
            windsurf,
            all,
        }) => {
            if all {
                install::run_all()?;
            } else {
                let target = resolve_target(cursor, windsurf);
                install::run_with_opts(target)?;
            }
        }
        Some(Command::Uninstall {
            cursor,
            windsurf,
            all,
        }) => {
            if all {
                install::uninstall_all()?;
            } else {
                let target = resolve_target(cursor, windsurf);
                install::uninstall_with_opts(target)?;
            }
        }
        Some(Command::Stats) => stats::run()?,
        Some(Command::StatsReset) => stats::reset()?,
        Some(Command::Bench) => bench::run()?,
        Some(Command::TokenCost {
            inputs,
            model,
            r#type,
        }) => {
            token_cost::run(&inputs, r#type.as_deref(), model.as_deref())?;
        }
        Some(Command::Mcp) => mcp::run()?,
        Some(Command::Completions { shell }) => completions::run(shell)?,
        None => run_compress(cli)?,
    }

    Ok(())
}

fn resolve_target(cursor: bool, windsurf: bool) -> Target {
    if cursor {
        Target::Cursor
    } else if windsurf {
        Target::Windsurf
    } else {
        Target::ClaudeCode
    }
}

fn run_compress(cli: Cli) -> Result<()> {
    if cli.inputs.is_empty() {
        if io::stdin().is_terminal() {
            eprintln!(
                "tersify: no input provided.\n\nUsage:\n  cat file.rs | tersify\n  git diff   | tersify\n  tersify src/\n  tersify --help"
            );
            std::process::exit(1);
        }
        return compress_stdin(&cli);
    }

    let opts = build_opts(&cli, None);
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
            let (out, before, after) = input::compress_directory_with(path, forced, &opts)?;
            print!("{}", out);
            total_before += before;
            total_after += after;
        } else {
            let (out, before, after) = input::compress_file_with(path, forced, &opts)?;
            if inputs_count > 1 {
                println!("// === {} ===", path.display());
            }
            print!("{}", out);
            total_before += before;
            total_after += after;
        }
    }

    report(cli.verbose, total_before, total_after)?;
    Ok(())
}

fn compress_stdin(cli: &Cli) -> Result<()> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;

    if buf.trim().is_empty() {
        return Ok(());
    }

    let opts = build_opts(cli, None);
    let (out, before, after) =
        input::compress_content_with(&buf, cli.r#type.as_deref(), None, &opts)?;

    print!("{}", out);
    report(cli.verbose, before, after)?;
    Ok(())
}

fn build_opts(cli: &Cli, budget_override: Option<usize>) -> CompressOptions {
    CompressOptions {
        budget: budget_override.or(cli.budget),
        ast: cli.ast,
        smart: cli.smart,
        strip_docs: cli.strip_docs,
    }
}

fn report(verbose: bool, before: usize, after: usize) -> Result<()> {
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
