//! Content-type-aware compression pipeline.
//!
//! The top-level [`compress`] function selects the right strategy based on
//! [`ContentType`] and enforces an optional token budget.

mod code;
mod diff;
mod json;
mod logs;
mod text;

use crate::detect::ContentType;
use crate::error::Result;
use crate::tokens;

/// Compress `input` using the strategy appropriate for `content_type`.
///
/// If `budget` is `Some(n)`, the output is hard-truncated to fit within
/// `n` tokens, with a notice appended so the LLM knows context was trimmed.
///
/// # Errors
///
/// Returns [`TersifyError::InvalidJson`] if `content_type` is `Json` and the
/// input cannot be parsed.
pub fn compress(input: &str, content_type: &ContentType, budget: Option<usize>) -> Result<String> {
    let compressed = match content_type {
        ContentType::Code(lang) => code::compress(input, lang),
        ContentType::Json => json::compress(input)?,
        ContentType::Logs => logs::compress(input),
        ContentType::Diff => diff::compress(input),
        ContentType::Text => text::compress(input),
    };

    Ok(match budget {
        Some(limit) => enforce_budget(compressed, limit),
        None => compressed,
    })
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
