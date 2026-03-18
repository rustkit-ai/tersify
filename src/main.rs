mod bench;
mod cli;
mod completions;
mod config;
mod hook;
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
use tersify::{compress::CompressOptions, detect, input, tokens};

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load config — CLI flags take precedence over config values.
    let cfg = config::Config::load();

    match cli.command {
        Some(Command::Install {
            cursor,
            windsurf,
            copilot,
            all,
        }) => {
            if all {
                install::run_all()?;
            } else {
                let target = resolve_target(cursor, windsurf, copilot);
                install::run_with_opts(target)?;
            }
        }
        Some(Command::Uninstall {
            cursor,
            windsurf,
            copilot,
            all,
        }) => {
            if all {
                install::uninstall_all()?;
            } else {
                let target = resolve_target(cursor, windsurf, copilot);
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
        Some(Command::HookRun) => hook::run()?,
        None => run_compress(cli, &cfg)?,
    }

    Ok(())
}

fn resolve_target(cursor: bool, windsurf: bool, copilot: bool) -> Target {
    if cursor {
        Target::Cursor
    } else if windsurf {
        Target::Windsurf
    } else if copilot {
        Target::Copilot
    } else {
        Target::ClaudeCode
    }
}

fn run_compress(cli: Cli, cfg: &config::Config) -> Result<()> {
    if cli.inputs.is_empty() {
        if io::stdin().is_terminal() {
            eprintln!(
                "tersify: no input provided.\n\nUsage:\n  cat file.rs | tersify\n  git diff   | tersify\n  tersify src/\n  tersify --help"
            );
            std::process::exit(1);
        }
        return compress_stdin(&cli, cfg);
    }

    let opts = build_opts(&cli, cfg, None);
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
            let (compressed, before, after) = input::compress_file_with(path, forced, &opts)?;
            // Re-detect language for stats (cheap, content already cached)
            let content = std::fs::read_to_string(path)?;
            let ct = if let Some(t) = forced {
                t.parse::<tersify::detect::ContentType>()?
            } else {
                detect::detect_for_path(path, &content)
            };
            let lang = ct.lang_str();

            if inputs_count > 1 {
                println!("// === {} ===", path.display());
            }
            print!("{}", compressed);
            total_before += before;
            total_after += after;

            // Record per-language stats for single file
            let _ = stats::record_with_lang(before, after, Some(lang));
        }
    }

    // For directory compression, record totals only (language breakdown happens inside)
    if cli.inputs.iter().any(|p| Path::new(p).is_dir()) {
        report(cli.verbose, total_before, total_after, None)?;
    } else {
        report_verbose_only(cli.verbose, total_before, total_after)?;
    }

    Ok(())
}

fn compress_stdin(cli: &Cli, cfg: &config::Config) -> Result<()> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;

    if buf.trim().is_empty() {
        return Ok(());
    }

    let opts = build_opts(cli, cfg, None);
    let ct = if let Some(t) = &cli.r#type {
        t.parse::<tersify::detect::ContentType>()?
    } else {
        detect::detect(&buf)
    };
    let lang = ct.lang_str();
    let before = tokens::count(&buf);
    let compressed = tersify::compress::compress_with(&buf, &ct, &opts)?;
    let after = tokens::count(&compressed);

    print!("{}", compressed);
    report(cli.verbose, before, after, Some(lang))?;
    Ok(())
}

fn build_opts(cli: &Cli, cfg: &config::Config, budget_override: Option<usize>) -> CompressOptions {
    // Merge CLI patterns and config patterns (dedup by value)
    let mut patterns = cfg.strip.patterns.clone();
    for p in &cli.patterns {
        if !patterns.contains(p) {
            patterns.push(p.clone());
        }
    }
    CompressOptions {
        ast: cli.ast || cfg.defaults.ast,
        smart: cli.smart || cfg.defaults.smart,
        strip_docs: cli.strip_docs || cfg.defaults.strip_docs,
        budget: budget_override.or(cli.budget).or(cfg.defaults.budget),
        custom_patterns: patterns,
    }
}

fn report(verbose: bool, before: usize, after: usize, lang: Option<&str>) -> Result<()> {
    if verbose {
        let pct = tokens::savings_pct(before, after);
        eprintln!(
            "\n[tersify] {} → {} tokens  ({:.0}% saved)",
            before, after, pct
        );
    }
    let _ = stats::record_with_lang(before, after, lang);
    Ok(())
}

fn report_verbose_only(verbose: bool, before: usize, after: usize) -> Result<()> {
    if verbose {
        let pct = tokens::savings_pct(before, after);
        eprintln!(
            "\n[tersify] {} → {} tokens  ({:.0}% saved)",
            before, after, pct
        );
    }
    Ok(())
}
