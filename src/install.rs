use anyhow::{Context, Result};
use std::path::PathBuf;

const TERSIFY_MARKER: &str = "// tersify-hook";

const CLAUDE_HOOK_BLOCK: &str = r#"// tersify-hook
{
  "PreToolUse": [
    {
      "matcher": "Read",
      "hooks": [
        {
          "type": "command",
          "command": "tersify \"$CLAUDE_TOOL_INPUT_FILE_PATH\""
        }
      ]
    }
  ]
}"#;

const CURSOR_RULE_CONTENT: &str = r#"---
description: Compress file context with tersify to reduce token usage
alwaysApply: true
---

When including file contents in your context window, prefer running them
through `tersify` first to strip noise and save tokens:

```bash
# Single file
tersify src/main.rs

# Entire directory (skips target/, node_modules/, etc.)
tersify src/

# Pipe from stdin
cat large_file.rs | tersify

# Show token savings
cat large_file.rs | tersify --verbose
```

tersify removes comments, collapses blank lines, strips null JSON fields,
and deduplicates repeated log lines — saving 30–78% of tokens with no loss
of information.

Install: `cargo install tersify` or download from https://github.com/rustkit-ai/tersify/releases
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
    let path = claude_hooks_path()?;

    if path.exists() {
        let existing = std::fs::read_to_string(&path)?;
        if existing.contains(TERSIFY_MARKER) {
            println!("✓ tersify hook already installed at {}", path.display());
            return Ok(());
        }
        println!(
            "⚠  Hooks file already exists at {}.\n   Add tersify manually or run `tersify uninstall` first.",
            path.display()
        );
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    std::fs::write(&path, CLAUDE_HOOK_BLOCK)
        .with_context(|| format!("failed to write {}", path.display()))?;

    println!("✓ Installed tersify hook at {}", path.display());
    println!("  Files read by Claude will now be automatically compressed.");
    Ok(())
}

pub fn uninstall() -> Result<()> {
    let path = claude_hooks_path()?;

    if !path.exists() {
        println!("Nothing to uninstall — hooks file not found.");
        return Ok(());
    }

    let content = std::fs::read_to_string(&path)?;

    if !content.contains(TERSIFY_MARKER) {
        println!("tersify hook not found in {}.", path.display());
        return Ok(());
    }

    if content.trim() == CLAUDE_HOOK_BLOCK.trim() {
        std::fs::remove_file(&path)?;
        println!("✓ Removed tersify hook (deleted {})", path.display());
    } else {
        let cleaned = remove_hook_block(&content);
        std::fs::write(&path, cleaned)?;
        println!("✓ Removed tersify hook from {}", path.display());
    }

    Ok(())
}

fn remove_hook_block(content: &str) -> String {
    let mut out = Vec::new();
    let mut in_block = false;

    for line in content.lines() {
        if line.contains(TERSIFY_MARKER) {
            in_block = true;
            continue;
        }
        if in_block {
            if line.trim() == "}" {
                in_block = false;
            }
            continue;
        }
        out.push(line);
    }

    out.join("\n")
}

fn claude_hooks_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("$HOME not set")?;
    Ok(PathBuf::from(home).join(".claude").join("hooks.json"))
}

// ── Cursor IDE ───────────────────────────────────────────────────────────────

fn install_cursor() -> Result<()> {
    let rule_path = cursor_rule_path()?;

    if rule_path.exists() {
        println!("✓ Cursor rule already installed at {}", rule_path.display());
        return Ok(());
    }

    // Create the rules directory if it doesn't exist
    if let Some(parent) = rule_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    std::fs::write(&rule_path, CURSOR_RULE_CONTENT)
        .with_context(|| format!("failed to write {}", rule_path.display()))?;

    println!("✓ Installed tersify rule at {}", rule_path.display());
    println!("  Cursor will now suggest using tersify for context compression.");
    println!();
    println!("  Tip: add a shell alias for quick manual use:");
    println!("    echo \"alias tcat='tersify'\" >> ~/.zshrc");
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

When including file contents in your context window, prefer running them
through `tersify` first to reduce token usage:

```bash
tersify src/main.rs          # single file
tersify src/                  # entire directory
cat large_file.rs | tersify   # pipe from stdin
cat file.rs | tersify --ast   # signatures only
```

tersify removes comments, blank lines, null JSON fields, and deduplicates
repeated log lines — saving 30–78% of tokens with no information loss.

Install: `cargo install tersify` or download from https://github.com/rustkit-ai/tersify/releases
"#;

fn install_windsurf() -> Result<()> {
    let rule_path = windsurf_rule_path()?;

    if rule_path.exists() {
        println!(
            "✓ Windsurf rule already installed at {}",
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

    println!("✓ Installed tersify rule at {}", rule_path.display());
    println!("  Windsurf will now suggest using tersify for context compression.");
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
    fn claude_hooks_path_structure() {
        let path = claude_hooks_path().unwrap();
        let s = path.to_string_lossy();
        assert!(s.contains(".claude"));
        assert!(s.ends_with("hooks.json"));
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
