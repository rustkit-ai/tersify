use anyhow::{Context, Result};
use std::path::PathBuf;

const TERSIFY_MARKER: &str = "// tersify-hook";

const HOOK_BLOCK: &str = r#"// tersify-hook
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

pub fn run() -> Result<()> {
    let path = hooks_file_path()?;

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

    std::fs::write(&path, HOOK_BLOCK)
        .with_context(|| format!("failed to write {}", path.display()))?;

    println!("✓ Installed tersify hook at {}", path.display());
    println!("  Files read by Claude will now be automatically compressed.");
    Ok(())
}

pub fn uninstall() -> Result<()> {
    let path = hooks_file_path()?;

    if !path.exists() {
        println!("Nothing to uninstall — hooks file not found.");
        return Ok(());
    }

    let content = std::fs::read_to_string(&path)?;

    if !content.contains(TERSIFY_MARKER) {
        println!("tersify hook not found in {}.", path.display());
        return Ok(());
    }

    // If the whole file is our hook block, delete the file
    if content.trim() == HOOK_BLOCK.trim() {
        std::fs::remove_file(&path)?;
        println!("✓ Removed tersify hook (deleted {})", path.display());
    } else {
        // Remove only our block — strip lines between markers
        let cleaned = remove_hook_block(&content);
        std::fs::write(&path, cleaned)?;
        println!("✓ Removed tersify hook from {}", path.display());
    }

    Ok(())
}

/// Remove the tersify hook block from a hooks file that may contain other hooks.
fn remove_hook_block(content: &str) -> String {
    let mut out = Vec::new();
    let mut in_block = false;

    for line in content.lines() {
        if line.contains(TERSIFY_MARKER) {
            in_block = true;
            continue;
        }
        if in_block {
            // End of our JSON block when we hit the closing brace at root level
            if line.trim() == "}" {
                in_block = false;
                continue;
            }
            continue;
        }
        out.push(line);
    }

    out.join("\n")
}

fn hooks_file_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("$HOME not set")?;
    Ok(PathBuf::from(home).join(".claude").join("hooks.json"))
}
