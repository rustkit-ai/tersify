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

    /// Force content type: code | rust | python | js | ts | go | ruby | java | c | swift | kotlin | json | logs | diff | text
    #[arg(long, short = 't')]
    pub r#type: Option<String>,

    /// Max token budget for output
    #[arg(long, short = 'b')]
    pub budget: Option<usize>,

    /// Show token count before/after on stderr
    #[arg(long, short = 'v')]
    pub verbose: bool,

    /// Extract function signatures only — stub all function bodies with `{ /* ... */ }`
    #[arg(long, short = 'a')]
    pub ast: bool,

    /// Enable semantic near-duplicate deduplication (MinHash-based)
    #[arg(long, short = 's')]
    pub smart: bool,

    /// Also strip doc comments (///, //!, /** */, Python docstrings)
    #[arg(long)]
    pub strip_docs: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Install tersify hooks into your AI coding environment
    Install {
        /// Install for Cursor IDE (~/.cursor/rules/tersify.mdc)
        #[arg(long, conflicts_with_all = ["windsurf", "all"])]
        cursor: bool,
        /// Install for Windsurf IDE (~/.windsurf/rules/tersify.md)
        #[arg(long, conflicts_with_all = ["cursor", "all"])]
        windsurf: bool,
        /// Auto-detect and install for all present editors (Claude Code + Cursor + Windsurf)
        #[arg(long, conflicts_with_all = ["cursor", "windsurf"])]
        all: bool,
    },
    /// Remove tersify hooks
    Uninstall {
        /// Remove Cursor IDE rule
        #[arg(long, conflicts_with_all = ["windsurf", "all"])]
        cursor: bool,
        /// Remove Windsurf IDE rule
        #[arg(long, conflicts_with_all = ["cursor", "all"])]
        windsurf: bool,
        /// Remove hooks from all detected editors
        #[arg(long, conflicts_with_all = ["cursor", "windsurf"])]
        all: bool,
    },
    /// Show token savings statistics
    Stats,
    /// Reset saved statistics
    StatsReset,
    /// Benchmark compression savings across all content types
    Bench,
    /// Estimate LLM API cost before and after compression
    TokenCost {
        /// Files or directories to analyse (defaults to stdin)
        inputs: Vec<String>,
        /// Filter to a specific model (e.g. claude-sonnet, gpt-4o)
        #[arg(long, short = 'm')]
        model: Option<String>,
        /// Force content type
        #[arg(long, short = 't')]
        r#type: Option<String>,
    },
    /// Start an MCP server over stdio (register with: claude mcp add tersify -- tersify mcp)
    Mcp,
    /// Print shell completion script
    Completions {
        /// Target shell
        #[arg(value_enum)]
        shell: Shell,
    },
}
