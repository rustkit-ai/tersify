//! Content-type-aware compression pipeline.
//!
//! The top-level [`compress`] function selects the right strategy based on
//! [`ContentType`] and enforces an optional token budget.
//!
//! Use [`compress_with`] and [`CompressOptions`] to enable advanced modes
//! such as AST-based body stubbing (`ast: true`) or semantic near-duplicate
//! deduplication (`smart: true`).

mod ast;
mod ast_ts;
mod code;
mod diff;
mod json;
mod logs;
mod smart;
mod text;
mod util;

use crate::detect::{ContentType, Language};
use crate::error::Result;
use crate::tokens;
use regex_lite::Regex;

/// Options controlling the compression pipeline.
///
/// Construct with `Default::default()` for standard compression, then set
/// individual fields as needed.
///
/// # Examples
///
/// ```
/// use tersify::compress::CompressOptions;
///
/// // Standard compression
/// let opts = CompressOptions::default();
///
/// // AST mode: extract signatures, stub bodies
/// let opts = CompressOptions { ast: true, ..Default::default() };
///
/// // Smart mode: near-duplicate deduplication
/// let opts = CompressOptions { smart: true, ..Default::default() };
/// ```
#[derive(Debug, Default, Clone)]
pub struct CompressOptions {
    /// Maximum token budget; output is hard-truncated when set.
    pub budget: Option<usize>,
    /// Extract function/method signatures only; stub all bodies.
    pub ast: bool,
    /// Remove near-duplicate blocks (MinHash-based semantic dedup).
    pub smart: bool,
    /// Also strip doc comments (`///`, `//!`, `/** */`, Python docstrings).
    pub strip_docs: bool,
    /// Custom regex patterns (regex-lite syntax). Each matching portion is
    /// removed from the line; lines that become empty are dropped entirely.
    pub custom_patterns: Vec<String>,
}

/// Compress `input` using the strategy appropriate for `content_type`.
///
/// This is the zero-configuration entry point. For advanced modes, use
/// [`compress_with`] with a [`CompressOptions`].
///
/// If `budget` is `Some(n)`, the output is hard-truncated to fit within
/// `n` tokens, with a notice appended so the LLM knows context was trimmed.
///
/// # Errors
///
/// Returns [`crate::error::TersifyError::InvalidJson`] if `content_type` is `Json` and the
/// input cannot be parsed.
pub fn compress(input: &str, content_type: &ContentType, budget: Option<usize>) -> Result<String> {
    compress_with(
        input,
        content_type,
        &CompressOptions {
            budget,
            ..Default::default()
        },
    )
}

/// Compress `input` with full control over compression options.
///
/// # Errors
///
/// Returns [`crate::error::TersifyError::InvalidJson`] if `content_type` is `Json` and the
/// input cannot be parsed.
pub fn compress_with(
    input: &str,
    content_type: &ContentType,
    opts: &CompressOptions,
) -> Result<String> {
    let compressed = match content_type {
        ContentType::Code(lang) => {
            if opts.ast {
                // AST stubbing not meaningful for markup/data/shell — fall back to standard compress
                match lang {
                    Language::Html
                    | Language::Css
                    | Language::Sql
                    | Language::Shell
                    | Language::Yaml => code::compress(input, lang, opts.strip_docs),
                    _ => ast::stub_bodies(input, lang),
                }
            } else {
                code::compress(input, lang, opts.strip_docs)
            }
        }
        ContentType::Json => json::compress(input)?,
        ContentType::Logs => logs::compress(input),
        ContentType::Diff => diff::compress(input),
        ContentType::Text => text::compress(input),
    };

    let after_smart = if opts.smart {
        smart::dedup(&compressed)
    } else {
        compressed
    };

    let after_patterns = if opts.custom_patterns.is_empty() {
        after_smart
    } else {
        apply_custom_patterns(&after_smart, &opts.custom_patterns)
    };

    Ok(match opts.budget {
        Some(limit) => enforce_budget(after_patterns, limit),
        None => after_patterns,
    })
}

/// Apply custom regex patterns: remove matched portions from each line.
/// Lines that become blank after all substitutions are dropped entirely.
///
/// Invalid patterns are silently ignored.
fn apply_custom_patterns(text: &str, patterns: &[String]) -> String {
    let regexes: Vec<Regex> = patterns
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect();

    if regexes.is_empty() {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        // Apply all regexes sequentially
        let mut result = line.to_string();
        for re in &regexes {
            result = re.replace_all(&result, "").to_string();
        }
        // Preserve indentation; drop line if it became empty
        let trimmed = result.trim();
        if !trimmed.is_empty() {
            let indent = &line[..line.len() - line.trim_start().len()];
            out.push_str(indent);
            out.push_str(trimmed);
            out.push('\n');
        }
    }
    out.trim_end().to_string()
}

/// Truncate `text` to stay within `budget` tokens, preserving whole lines.
fn enforce_budget(text: String, budget: usize) -> String {
    if tokens::count(&text) <= budget {
        return text;
    }
    let mut out = String::new();
    for line in text.lines() {
        let candidate = format!("{out}{line}\n");
        if tokens::count(&candidate) > budget {
            break;
        }
        out = candidate;
    }
    out.push_str("// [tersify: truncated to fit token budget]\n");
    out
}
