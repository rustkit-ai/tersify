//! `tersify hook` — Claude Code hook handler.
//!
//! Handles three hook events:
//!
//! - **PostToolUse Read** — compresses file content after Claude reads it, injecting
//!   the tersified version as `additionalContext` and suppressing the original.
//! - **PostToolUse Bash** — compresses bash command output using the same pipeline.
//! - **PreToolUse Write/Edit** — reads the current file before Claude writes or edits
//!   it, compresses it, and injects the compact version as context so Claude has a
//!   fresh reference without a separate read.

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
        Err(_) => return Ok(()), // Malformed JSON — silently pass through
    };

    let event = input
        .get("hookEventName")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let tool = input
        .get("tool_name")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    match (event, tool) {
        ("PostToolUse", "Read") => handle_post_read(&input),
        ("PostToolUse", "Bash") => handle_post_bash(&input),
        ("PreToolUse", "Write") | ("PreToolUse", "Edit") => handle_pre_write_edit(&input),
        // Unknown event — pass through silently
        _ => Ok(()),
    }
}

// ── PostToolUse Read ──────────────────────────────────────────────────────────

fn handle_post_read(input: &serde_json::Value) -> Result<()> {
    let content = match extract_response_content(input) {
        Some(c) if !c.trim().is_empty() => c,
        _ => return Ok(()),
    };

    let file_path = input
        .get("tool_input")
        .and_then(|i| i.get("file_path"))
        .and_then(|f| f.as_str());

    let ct = match file_path {
        Some(p) => detect::detect_for_path(std::path::Path::new(p), content),
        None => detect::detect(content),
    };

    compress_and_respond(content, &ct, true)
}

// ── PostToolUse Bash ──────────────────────────────────────────────────────────

fn handle_post_bash(input: &serde_json::Value) -> Result<()> {
    let content = match extract_response_content(input) {
        Some(c) if !c.trim().is_empty() => c,
        _ => return Ok(()),
    };

    // Detect content type from the text itself (no file path for bash output)
    let ct = detect::detect(content);
    compress_and_respond(content, &ct, true)
}

// ── PreToolUse Write / Edit ───────────────────────────────────────────────────

fn handle_pre_write_edit(input: &serde_json::Value) -> Result<()> {
    let file_path = match input
        .get("tool_input")
        .and_then(|i| i.get("file_path"))
        .and_then(|f| f.as_str())
    {
        Some(p) => p,
        None => return Ok(()),
    };

    let path = std::path::Path::new(file_path);

    // Nothing useful to do if the file doesn't exist yet
    let content = match std::fs::read_to_string(path) {
        Ok(c) if !c.trim().is_empty() => c,
        _ => return Ok(()),
    };

    let ct = detect::detect_for_path(path, &content);
    let opts = CompressOptions::default();

    let compressed = match tersify::compress::compress_with(&content, &ct, &opts) {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };

    let before = tokens::count(&content);
    let after = tokens::count(&compressed);

    if after >= before {
        return Ok(());
    }

    let pct = (1.0 - after as f64 / before as f64) * 100.0;

    // PreToolUse response: allow the tool to proceed, inject compressed file as context
    let response = json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "additionalContext": format!(
                "[tersify: current file {before}→{after} tokens ({pct:.0}% smaller)]\n\n{compressed}"
            )
        }
    });

    println!("{response}");
    Ok(())
}

// ── Shared helpers ────────────────────────────────────────────────────────────

fn compress_and_respond(content: &str, ct: &detect::ContentType, suppress: bool) -> Result<()> {
    let opts = CompressOptions::default();
    let compressed = match tersify::compress::compress_with(content, ct, &opts) {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };

    let before = tokens::count(content);
    let after = tokens::count(&compressed);

    if after >= before {
        return Ok(());
    }

    let _ = crate::stats::record_with_lang(before, after, Some(ct.lang_str()));

    let pct = (1.0 - after as f64 / before as f64) * 100.0;
    let response = json!({
        "hookSpecificOutput": {
            "hookEventName": "PostToolUse",
            "suppressOutput": suppress,
            "additionalContext": format!(
                "[tersify: {before}→{after} tokens, {pct:.0}% saved]\n\n{compressed}"
            )
        }
    });

    println!("{response}");
    Ok(())
}

/// Extract the text content from a Claude Code tool_response payload.
///
/// Handles both array-of-content-blocks and plain string formats.
fn extract_response_content(input: &serde_json::Value) -> Option<&str> {
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
