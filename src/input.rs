//! Input resolution: stdin, single file, multiple files, directories.

use crate::{
    compress::{self, CompressOptions},
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
    // Rust ecosystem
    "rs", // Python
    "py", // JavaScript / TypeScript
    "js", "jsx", "mjs", "cjs", "ts", "tsx", // Go
    "go", // Ruby
    "rb", "rake", "gemspec", // Java
    "java", // C / C++
    "c", "cpp", "cc", "cxx", "h", "hpp", "hxx", // Swift
    "swift", // Kotlin
    "kt", "kts", // Data / config
    "json", "jsonc", "yaml", "yml", "toml", // Logs / diffs
    "log", "diff", "patch", // Docs
    "md", "txt",
];

/// Compress a single file, returning `(compressed, tokens_before, tokens_after)`.
pub fn compress_file(
    path: &Path,
    forced_type: Option<&str>,
    budget: Option<usize>,
) -> Result<(String, usize, usize)> {
    let content = std::fs::read_to_string(path)?;
    compress_content(&content, forced_type, Some(path), budget)
}

/// Compress a single file with full [`CompressOptions`].
pub fn compress_file_with(
    path: &Path,
    forced_type: Option<&str>,
    opts: &CompressOptions,
) -> Result<(String, usize, usize)> {
    let content = std::fs::read_to_string(path)?;
    compress_content_with(&content, forced_type, Some(path), opts)
}

/// Compress raw content (e.g. from stdin).
pub fn compress_content(
    content: &str,
    forced_type: Option<&str>,
    path: Option<&Path>,
    budget: Option<usize>,
) -> Result<(String, usize, usize)> {
    compress_content_with(
        content,
        forced_type,
        path,
        &CompressOptions {
            budget,
            ..Default::default()
        },
    )
}

/// Compress raw content with full [`CompressOptions`].
pub fn compress_content_with(
    content: &str,
    forced_type: Option<&str>,
    path: Option<&Path>,
    opts: &CompressOptions,
) -> Result<(String, usize, usize)> {
    let ct = resolve_type(forced_type, path, content)?;
    let before = tokens::count(content);
    let compressed = compress::compress_with(content, &ct, opts)?;
    let after = tokens::count(&compressed);
    Ok((compressed, before, after))
}

/// Compress all eligible files in a directory, concatenated with headers.
pub fn compress_directory(
    dir: &Path,
    forced_type: Option<&str>,
    budget: Option<usize>,
) -> Result<(String, usize, usize)> {
    compress_directory_with(
        dir,
        forced_type,
        &CompressOptions {
            budget,
            ..Default::default()
        },
    )
}

/// Compress all eligible files in a directory with full [`CompressOptions`].
pub fn compress_directory_with(
    dir: &Path,
    forced_type: Option<&str>,
    opts: &CompressOptions,
) -> Result<(String, usize, usize)> {
    let mut combined = String::new();
    let mut total_before = 0usize;
    let mut total_after = 0usize;

    let entries = WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
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

        let file_opts = CompressOptions {
            budget: None, // budget is applied to combined output below
            ast: opts.ast,
            smart: opts.smart,
            strip_docs: opts.strip_docs,
        };

        let before = tokens::count(&content);
        let compressed = compress::compress_with(&content, &ct, &file_opts)?;
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
    let final_output = match opts.budget {
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
