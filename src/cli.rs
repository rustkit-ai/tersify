use clap::{Parser, Subcommand};
use clap_complete::Shell;

#[derive(Parser)]
#[command(name = "tersify")]
#[command(
    version,
    about = "Universal LLM context compressor — pipe anything, get token-optimized output",
    long_about = None
)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Files or directories to compress (defaults to stdin)
    pub inputs: Vec<String>,

    /// Force content type: code | json | logs | diff | text
    #[arg(long, short = 't')]
    pub r#type: Option<String>,

    /// Max token budget for output
    #[arg(long, short = 'b')]
    pub budget: Option<usize>,

    /// Show token count before/after on stderr
    #[arg(long, short = 'v')]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Install tersify hooks into Claude Code
    Install,
    /// Remove tersify hooks from Claude Code
    Uninstall,
    /// Show token savings statistics
    Stats,
    /// Reset saved statistics
    StatsReset,
    /// Print shell completion script
    Completions {
        /// Target shell
        #[arg(value_enum)]
        shell: Shell,
    },
}
