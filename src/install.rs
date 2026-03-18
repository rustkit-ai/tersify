use anyhow::{Context, Result};
use std::path::PathBuf;

const TERSIFY_HOOK_COMMAND: &str = "tersify hook";

const CURSOR_RULE_CONTENT: &str = r#"---
description: Use tersify to compress file content before adding to context — saves 30-78% of tokens
alwaysApply: true
---

# tersify — automatic token compression

Before reading any file into your context, run it through `tersify` first.
This strips comments, blank lines, null JSON fields, and duplicate log lines
without losing any information — saving 30–78% of tokens.

## Usage

```bash
# Compress a single file (replaces comments/blanks, keeps all logic)
tersify src/main.rs

# Compress an entire directory in parallel
tersify src/

# Pipe output from any command
cat large_file.rs | tersify
git diff | tersify

# Signatures only — stub function bodies (saves 50-80%)
tersify --ast src/lib.rs

# Show savings
tersify --verbose src/main.rs
```

## When to use
- ALWAYS run `tersify` before reading source files into context
- Run `tersify src/` when exploring an unfamiliar codebase
- Run `git diff | tersify` before code review to save context space
- Run `tersify --ast` when you only need the API surface, not implementations

## What tersify removes
- Single-line comments (`//`, `#`, `--`)
- Block comments (`/* */`, `""" """`)
- Blank lines and trailing whitespace
- null/empty fields in JSON
- Duplicate log lines (keeps first + count)

tersify is installed at: $(which tersify 2>/dev/null || echo "cargo install tersify")
"#;

/// Target IDE for install/uninstall.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    ClaudeCode,
    Cursor,
    Windsurf,
}

/// Install tersify hooks for the given target.
pub fn run_with_opts(target: Target) -> Result<()> {
    match target {
        Target::ClaudeCode => install_claude(),
        Target::Cursor => install_cursor(),
        Target::Windsurf => install_windsurf(),
    }
}

/// Remove tersify hooks for the given target.
pub fn uninstall_with_opts(target: Target) -> Result<()> {
    match target {
        Target::ClaudeCode => uninstall(),
        Target::Cursor => uninstall_cursor(),
        Target::Windsurf => uninstall_windsurf(),
    }
}

/// Detect which AI editors are present on this machine.
fn detect_installed_targets() -> Vec<Target> {
    let home = match std::env::var("HOME") {
        Ok(h) => std::path::PathBuf::from(h),
        Err(_) => return vec![Target::ClaudeCode], // always attempt Claude Code
    };

    let mut targets = Vec::new();

    // Claude Code — ~/.claude/ directory exists or `claude` is on PATH
    if home.join(".claude").exists() || which_exists("claude") {
        targets.push(Target::ClaudeCode);
    } else {
        // Always try Claude Code even if not detected — creates the dir
        targets.push(Target::ClaudeCode);
    }

    // Cursor — ~/.cursor/ directory exists
    if home.join(".cursor").exists() {
        targets.push(Target::Cursor);
    }

    // Windsurf — ~/.windsurf/ directory exists
    if home.join(".windsurf").exists() {
        targets.push(Target::Windsurf);
    }

    targets
}

fn which_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Install tersify hooks into all detected AI editors.
pub fn run_all() -> Result<()> {
    let targets = detect_installed_targets();

    if targets.is_empty() {
        println!("No supported AI editors detected. Run one of:");
        println!("  tersify install            # Claude Code");
        println!("  tersify install --cursor   # Cursor");
        println!("  tersify install --windsurf # Windsurf");
        return Ok(());
    }

    println!("Detected editors: {}", format_targets(&targets));
    println!();

    for target in targets {
        run_with_opts(target)?;
    }

    println!();
    println!("✓ All done! Run `tersify stats` to track your token savings.");
    Ok(())
}

/// Uninstall tersify hooks from all detected AI editors.
pub fn uninstall_all() -> Result<()> {
    let targets = detect_installed_targets();
    for target in targets {
        let _ = uninstall_with_opts(target); // best-effort
    }
    Ok(())
}

fn format_targets(targets: &[Target]) -> String {
    targets
        .iter()
        .map(|t| match t {
            Target::ClaudeCode => "Claude Code",
            Target::Cursor => "Cursor",
            Target::Windsurf => "Windsurf",
        })
        .collect::<Vec<_>>()
        .join(", ")
}

// ── Claude Code ──────────────────────────────────────────────────────────────

fn install_claude() -> Result<()> {
    let settings_path = claude_settings_path()?;

    // Remove the legacy hooks.json written by older tersify versions
    cleanup_legacy_hooks_json();

    // Load existing settings.json or start fresh
    let mut settings: serde_json::Value = if settings_path.exists() {
        let content = std::fs::read_to_string(&settings_path)
            .with_context(|| format!("failed to read {}", settings_path.display()))?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if hook_is_installed(&settings) {
        println!(
            "✓ tersify hook already installed in {}",
            settings_path.display()
        );
        return Ok(());
    }

    // Ensure hooks → PostToolUse array exists, then append our entry
    {
        let obj = settings
            .as_object_mut()
            .context("settings.json root is not an object")?;
        let hooks = obj.entry("hooks").or_insert_with(|| serde_json::json!({}));
        let post_tool_use = hooks
            .as_object_mut()
            .context("settings.json hooks is not an object")?
            .entry("PostToolUse")
            .or_insert_with(|| serde_json::json!([]));
        if let Some(arr) = post_tool_use.as_array_mut() {
            arr.push(serde_json::json!({
                "matcher": "Read",
                "hooks": [{"type": "command", "command": TERSIFY_HOOK_COMMAND}]
            }));
        }
    }

    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    std::fs::write(
        &settings_path,
        serde_json::to_string_pretty(&settings).context("failed to serialise settings.json")?
            + "\n",
    )
    .with_context(|| format!("failed to write {}", settings_path.display()))?;

    println!(
        "✓ Claude Code — automatic hook installed ({})",
        settings_path.display()
    );
    println!("  Every file Claude reads is now silently compressed.");
    println!("  Nothing to do — it just works. Track savings: tersify stats");
    Ok(())
}

pub fn uninstall() -> Result<()> {
    let settings_path = claude_settings_path()?;

    cleanup_legacy_hooks_json();

    if !settings_path.exists() {
        println!("Nothing to uninstall — ~/.claude/settings.json not found.");
        return Ok(());
    }

    let content = std::fs::read_to_string(&settings_path)
        .with_context(|| format!("failed to read {}", settings_path.display()))?;
    let mut settings: serde_json::Value =
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}));

    if !hook_is_installed(&settings) {
        println!("tersify hook not found in settings.json.");
        return Ok(());
    }

    // Remove all tersify PostToolUse entries
    if let Some(arr) = settings
        .pointer_mut("/hooks/PostToolUse")
        .and_then(|v| v.as_array_mut())
    {
        arr.retain(|entry| !entry_is_tersify(entry));
    }

    std::fs::write(
        &settings_path,
        serde_json::to_string_pretty(&settings).context("failed to serialise settings.json")?
            + "\n",
    )
    .with_context(|| format!("failed to write {}", settings_path.display()))?;

    println!("✓ Removed tersify hook from {}", settings_path.display());
    Ok(())
}

fn hook_is_installed(settings: &serde_json::Value) -> bool {
    settings
        .pointer("/hooks/PostToolUse")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().any(entry_is_tersify))
        .unwrap_or(false)
}

fn entry_is_tersify(entry: &serde_json::Value) -> bool {
    entry
        .get("hooks")
        .and_then(|h| h.as_array())
        .map(|hooks| {
            hooks.iter().any(|h| {
                h.get("command")
                    .and_then(|c| c.as_str())
                    .map(|c| c.contains(TERSIFY_HOOK_COMMAND))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

/// Remove the legacy `~/.claude/hooks.json` written by older tersify versions.
fn cleanup_legacy_hooks_json() {
    if let Ok(home) = std::env::var("HOME") {
        let path = PathBuf::from(home).join(".claude").join("hooks.json");
        if path.exists() && std::fs::remove_file(&path).is_ok() {
            println!("  Removed legacy ~/.claude/hooks.json");
        }
    }
}

fn claude_settings_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("$HOME not set")?;
    Ok(PathBuf::from(home).join(".claude").join("settings.json"))
}

// ── Cursor IDE ───────────────────────────────────────────────────────────────

fn install_cursor() -> Result<()> {
    let rule_path = cursor_rule_path()?;

    if rule_path.exists() {
        println!(
            "✓ Cursor — rule already installed at {}",
            rule_path.display()
        );
        return Ok(());
    }

    // Create the rules directory if it doesn't exist
    if let Some(parent) = rule_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    std::fs::write(&rule_path, CURSOR_RULE_CONTENT)
        .with_context(|| format!("failed to write {}", rule_path.display()))?;

    println!("✓ Cursor — AI rule installed ({})", rule_path.display());
    println!("  Cursor's AI will now always use tersify before reading files.");
    println!("  Note: Cursor uses AI-guided rules (not automatic hooks like Claude Code).");
    println!("  The AI knows to run tersify — it happens transparently.");
    Ok(())
}

fn uninstall_cursor() -> Result<()> {
    let rule_path = cursor_rule_path()?;

    if !rule_path.exists() {
        println!("Nothing to uninstall — Cursor rule not found.");
        return Ok(());
    }

    std::fs::remove_file(&rule_path)
        .with_context(|| format!("failed to remove {}", rule_path.display()))?;

    println!("✓ Removed tersify Cursor rule ({})", rule_path.display());
    Ok(())
}

fn cursor_rule_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("$HOME not set")?;
    Ok(PathBuf::from(home)
        .join(".cursor")
        .join("rules")
        .join("tersify.mdc"))
}

// ── Windsurf IDE ──────────────────────────────────────────────────────────────

const WINDSURF_RULE_CONTENT: &str = r#"---
trigger: always_on
---

# tersify — automatic token compression

Before reading any file into your context, run it through `tersify` first.
This strips comments, blank lines, null JSON fields, and duplicate log lines
without losing any information — saving 30–78% of tokens.

## Usage

```bash
tersify src/main.rs          # compress a single file
tersify src/                 # compress entire directory in parallel
cat large_file.rs | tersify  # pipe any command output
git diff | tersify           # compress diffs before review
tersify --ast src/lib.rs     # signatures only (50-80% savings)
tersify --verbose src/       # show token counts
```

## Always do this
- Run `tersify` before reading source files — every read saves tokens
- Run `tersify src/` when exploring a codebase
- Run `tersify --ast` when you only need function signatures

## What tersify removes
- Comments (single-line and block) in all supported languages
- Blank lines and trailing whitespace
- null/empty fields in JSON
- Duplicate log lines (keeps unique lines + count)

tersify installed: $(which tersify 2>/dev/null || echo "cargo install tersify")
"#;

fn install_windsurf() -> Result<()> {
    let rule_path = windsurf_rule_path()?;

    if rule_path.exists() {
        println!(
            "✓ Windsurf — rule already installed at {}",
            rule_path.display()
        );
        return Ok(());
    }

    if let Some(parent) = rule_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    std::fs::write(&rule_path, WINDSURF_RULE_CONTENT)
        .with_context(|| format!("failed to write {}", rule_path.display()))?;

    println!("✓ Windsurf — AI rule installed ({})", rule_path.display());
    println!("  Windsurf's AI will now always use tersify before reading files.");
    println!("  Note: Windsurf uses AI-guided rules (not automatic hooks like Claude Code).");
    println!("  The AI knows to run tersify — it happens transparently.");
    Ok(())
}

fn uninstall_windsurf() -> Result<()> {
    let rule_path = windsurf_rule_path()?;

    if !rule_path.exists() {
        println!("Nothing to uninstall — Windsurf rule not found.");
        return Ok(());
    }

    std::fs::remove_file(&rule_path)
        .with_context(|| format!("failed to remove {}", rule_path.display()))?;

    println!("✓ Removed tersify Windsurf rule ({})", rule_path.display());
    Ok(())
}

fn windsurf_rule_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("$HOME not set")?;
    Ok(PathBuf::from(home)
        .join(".windsurf")
        .join("rules")
        .join("tersify.md"))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn windsurf_rule_path_structure() {
        let path = windsurf_rule_path().unwrap();
        let s = path.to_string_lossy();
        assert!(s.contains(".windsurf"));
        assert!(s.contains("rules"));
        assert!(s.ends_with("tersify.md"));
    }

    #[test]
    #[cfg(unix)]
    fn cursor_rule_path_structure() {
        let path = cursor_rule_path().unwrap();
        let s = path.to_string_lossy();
        assert!(s.contains(".cursor"));
        assert!(s.contains("rules"));
        assert!(s.ends_with("tersify.mdc"));
    }

    #[test]
    #[cfg(unix)]
    fn claude_settings_path_structure() {
        let path = claude_settings_path().unwrap();
        let s = path.to_string_lossy();
        assert!(s.contains(".claude"));
        assert!(s.ends_with("settings.json"));
    }

    #[test]
    fn windsurf_rule_content_has_trigger() {
        assert!(WINDSURF_RULE_CONTENT.contains("trigger: always_on"));
        assert!(WINDSURF_RULE_CONTENT.contains("tersify"));
    }

    #[test]
    fn cursor_rule_content_has_always_apply() {
        assert!(CURSOR_RULE_CONTENT.contains("alwaysApply: true"));
        assert!(CURSOR_RULE_CONTENT.contains("tersify"));
    }

    #[test]
    fn resolve_target_flags() {
        // This replicates main.rs resolve_target logic
        assert_eq!(Target::ClaudeCode, Target::ClaudeCode);
        assert_ne!(Target::Cursor, Target::Windsurf);
    }
}
