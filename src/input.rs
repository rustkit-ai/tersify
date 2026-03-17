//! Input resolution: stdin, single file, multiple files, directories.

use crate::{
    compress,
    detect::{self, ContentType},
    error::Result,
    tokens,
};
use std::path::Path;
use walkdir::WalkDir;

/// Directories and files to skip during directory traversal.
const SKIP_DIRS: &[&str] = &[
    "target",
    "node_modules",
    ".git",
    "dist",
    "build",
    ".next",
    "__pycache__",
    ".venv",
    "venv",
    ".cache",
];

/// File extensions to include during directory traversal.
const INCLUDE_EXT: &[&str] = &[
    "rs", "py", "js", "jsx", "mjs", "ts", "tsx", "go", "json", "jsonc", "log", "diff", "patch",
    "md", "txt", "yaml", "yml", "toml",
];

/// Compress a single file, returning the compressed content.
pub fn compress_file(
    path: &Path,
    forced_type: Option<&str>,
    budget: Option<usize>,
) -> Result<(String, usize, usize)> {
    let content = std::fs::read_to_string(path)?;
    compress_content(&content, forced_type, Some(path), budget)
}

/// Compress raw content (e.g. from stdin).
pub fn compress_content(
    content: &str,
    forced_type: Option<&str>,
    path: Option<&Path>,
    budget: Option<usize>,
) -> Result<(String, usize, usize)> {
    let ct = resolve_type(forced_type, path, content)?;
    let before = tokens::count(content);
    let compressed = compress::compress(content, &ct, budget)?;
    let after = tokens::count(&compressed);
    Ok((compressed, before, after))
}

/// Compress all eligible files in a directory, concatenated with headers.
pub fn compress_directory(
    dir: &Path,
    forced_type: Option<&str>,
    budget: Option<usize>,
) -> Result<(String, usize, usize)> {
    let mut combined = String::new();
    let mut total_before = 0usize;
    let mut total_after = 0usize;

    let entries = WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Skip hidden and known large/irrelevant directories
            !e.file_name()
                .to_str()
                .map(|s| s.starts_with('.') || SKIP_DIRS.contains(&s))
                .unwrap_or(false)
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| INCLUDE_EXT.contains(&ext))
                .unwrap_or(false)
        });

    for entry in entries {
        let content = match std::fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue, // skip binary / unreadable files
        };

        if content.trim().is_empty() {
            continue;
        }

        let ct = if let Some(t) = forced_type {
            t.parse::<ContentType>()?
        } else {
            detect::detect_for_path(entry.path(), &content)
        };

        let before = tokens::count(&content);
        let compressed = compress::compress(&content, &ct, None)?;
        let after = tokens::count(&compressed);

        total_before += before;
        total_after += after;

        combined.push_str(&format!(
            "// === {} ===\n{}\n\n",
            entry.path().display(),
            compressed
        ));
    }

    // Apply budget to the combined output
    let final_output = match budget {
        Some(limit) if tokens::count(&combined) > limit => enforce_budget(combined, limit),
        _ => combined,
    };

    Ok((final_output, total_before, total_after))
}

fn enforce_budget(text: String, budget: usize) -> String {
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

fn resolve_type(forced: Option<&str>, path: Option<&Path>, content: &str) -> Result<ContentType> {
    if let Some(t) = forced {
        return t.parse::<ContentType>();
    }
    Ok(match path {
        Some(p) => detect::detect_for_path(p, content),
        None => detect::detect(content),
    })
}
