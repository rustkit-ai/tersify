//! `tersify __hook` — Claude Code PostToolUse hook handler.
//!
//! Called by Claude Code after every `Read` tool invocation.
//! Reads the hook JSON payload from stdin, compresses the file content,
//! and outputs the Claude Code PostToolUse response JSON to stdout.
//!
//! Claude Code injects the compressed content as `additionalContext`
//! and suppresses the original (verbose) file content.

use anyhow::Result;
use serde_json::json;
use std::io::Read;
use tersify::{compress::CompressOptions, detect, tokens};

pub fn run() -> Result<()> {
    // ── Read hook payload from stdin ─────────────────────────────────────
    let mut raw = String::new();
    std::io::stdin().read_to_string(&mut raw)?;

    let input: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        // Malformed JSON — silently pass through (don't break Claude)
        Err(_) => return Ok(()),
    };

    // ── Extract file content from tool_response ───────────────────────────
    let content = extract_content(&input);
    let content = match content {
        Some(c) if !c.trim().is_empty() => c,
        _ => return Ok(()), // nothing to compress
    };

    // ── Detect language from file path ────────────────────────────────────
    let file_path = input
        .get("tool_input")
        .and_then(|i| i.get("file_path"))
        .and_then(|f| f.as_str());

    let ct = match file_path {
        Some(p) => detect::detect_for_path(std::path::Path::new(p), content),
        None => detect::detect(content),
    };

    // ── Compress ──────────────────────────────────────────────────────────
    let opts = CompressOptions::default();
    let compressed = match tersify::compress::compress_with(content, &ct, &opts) {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };

    let before = tokens::count(content);
    let after = tokens::count(&compressed);

    // Skip if compression made no difference
    if after >= before {
        return Ok(());
    }

    // ── Record stats ──────────────────────────────────────────────────────
    let _ = crate::stats::record_with_lang(before, after, Some(ct.lang_str()));

    // ── Output Claude Code PostToolUse hook response ──────────────────────
    // suppressOutput: hide the raw file content from Claude's context.
    // additionalContext: inject the compressed version instead.
    let pct = (1.0 - after as f64 / before as f64) * 100.0;
    let response = json!({
        "hookSpecificOutput": {
            "hookEventName": "PostToolUse",
            "suppressOutput": true,
            "additionalContext": format!(
                "[tersify: {before}→{after} tokens, {pct:.0}% saved]\n\n{compressed}",
            )
        }
    });

    println!("{response}");
    Ok(())
}

/// Extract the text content from a Claude Code tool_response payload.
///
/// Handles both array-of-content-blocks and plain string formats.
fn extract_content(input: &serde_json::Value) -> Option<&str> {
    let response = input.get("tool_response")?;

    // Array format: {"content": [{"type": "text", "text": "..."}]}
    if let Some(arr) = response.get("content").and_then(|c| c.as_array()) {
        for item in arr {
            if item.get("type").and_then(|t| t.as_str()) == Some("text")
                && let Some(text) = item.get("text").and_then(|t| t.as_str())
            {
                return Some(text);
            }
        }
    }

    // Plain string format
    response.as_str()
}
