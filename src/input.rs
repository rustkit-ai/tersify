//! Input resolution: stdin, single file, multiple files, directories.

use crate::{
    compress::{self, CompressOptions},
    detect::{self, ContentType},
    error::Result,
    tokens,
};
use rayon::prelude::*;
use std::path::Path;
use walkdir::WalkDir;

/// Directories to always skip during directory traversal.
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
    "go",  // Ruby
    "rb", "rake", "gemspec", // Java
    "java",    // C / C++
    "c", "cpp", "cc", "cxx", "h", "hpp", "hxx",   // Swift
    "swift", // Kotlin
    "kt", "kts", // Web
    "html", "htm", "css", // SQL
    "sql", // Shell
    "sh", "bash", // Data / config
    "json", "jsonc", "yaml", "yml", "toml", // Logs / diffs
    "log", "diff", "patch", // Docs
    "md", "txt",
];

// ── .tersifyignore ────────────────────────────────────────────────────────────

/// Load ignore patterns from `.tersifyignore` in `dir`.
///
/// Lines starting with `#` are comments. Trailing `/` is stripped (treated
/// as a directory name). `*` is supported as a wildcard within a segment.
fn load_ignore_patterns(dir: &Path) -> Vec<String> {
    let path = dir.join(".tersifyignore");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.trim_end_matches('/').to_string())
        .collect()
}

/// Returns `true` if `path` matches any ignore pattern.
///
/// Matching rules:
/// - A pattern with no `/` is matched against every path component (basename).
/// - A pattern with `/` is matched against the path relative to the root dir.
/// - `*` within a segment matches any sequence of non-`/` characters.
fn is_ignored(path: &Path, root: &Path, patterns: &[String]) -> bool {
    let rel = path.strip_prefix(root).unwrap_or(path);
    let rel_str = rel.to_string_lossy();

    for pattern in patterns {
        if pattern.contains('/') {
            // Match against relative path
            if glob_match(pattern, &rel_str) {
                return true;
            }
        } else {
            // Match against each path component
            for component in rel.components() {
                if let Some(s) = component.as_os_str().to_str()
                    && glob_match(pattern, s)
                {
                    return true;
                }
            }
        }
    }
    false
}

/// Minimal glob: supports `*` wildcard (matches any sequence within a segment).
fn glob_match(pattern: &str, text: &str) -> bool {
    glob_match_bytes(pattern.as_bytes(), text.as_bytes())
}

fn glob_match_bytes(p: &[u8], t: &[u8]) -> bool {
    match p.first() {
        None => t.is_empty(),
        Some(&b'*') => {
            // * matches zero or more characters
            for skip in 0..=t.len() {
                if glob_match_bytes(&p[1..], &t[skip..]) {
                    return true;
                }
            }
            false
        }
        Some(&pc) => match t.first() {
            Some(&tc) if tc == pc => glob_match_bytes(&p[1..], &t[1..]),
            _ => false,
        },
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

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
///
/// Results are cached by content + option hash in `~/.tersify/cache/`.
pub fn compress_file_with(
    path: &Path,
    forced_type: Option<&str>,
    opts: &CompressOptions,
) -> Result<(String, usize, usize)> {
    let content = std::fs::read_to_string(path)?;
    let opts_key =
        (opts.ast as u8) | ((opts.strip_docs as u8) << 1) | ((opts.smart as u8) << 2);

    if let Some(cached) = crate::cache::get(&content, opts_key) {
        let before = tokens::count(&content);
        let after = tokens::count(&cached);
        return Ok((cached, before, after));
    }

    let result = compress_content_with(&content, forced_type, Some(path), opts)?;
    crate::cache::set(&content, opts_key, &result.0);
    Ok(result)
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
///
/// Files are processed in parallel (rayon). `.tersifyignore` in `dir` is
/// respected. Output is deterministically ordered by file path.
pub fn compress_directory_with(
    dir: &Path,
    forced_type: Option<&str>,
    opts: &CompressOptions,
) -> Result<(String, usize, usize)> {
    let ignore_patterns = load_ignore_patterns(dir);

    // ── Collect + sort all eligible file paths ────────────────────────────
    let mut paths: Vec<std::path::PathBuf> = WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_str().unwrap_or("");
            // Skip hardcoded dirs
            if e.file_type().is_dir() && (name.starts_with('.') || SKIP_DIRS.contains(&name)) {
                return false;
            }
            // Skip .tersifyignore-matched entries
            if !ignore_patterns.is_empty() && is_ignored(e.path(), dir, &ignore_patterns) {
                return false;
            }
            true
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| INCLUDE_EXT.contains(&ext))
                .unwrap_or(false)
        })
        .map(|e| e.into_path())
        .collect();

    paths.sort();

    // ── Process files in parallel ─────────────────────────────────────────
    let file_opts = CompressOptions {
        budget: None, // budget applied to combined output below
        ast: opts.ast,
        smart: opts.smart,
        strip_docs: opts.strip_docs,
        custom_patterns: opts.custom_patterns.clone(),
    };

    let opts_key =
        (file_opts.ast as u8) | ((file_opts.strip_docs as u8) << 1) | ((file_opts.smart as u8) << 2);

    let results: Vec<(String, usize, usize)> = paths
        .par_iter()
        .filter_map(|path| {
            let content = std::fs::read_to_string(path).ok()?;
            if content.trim().is_empty() {
                return None;
            }
            // Try cache first
            if let Some(cached) = crate::cache::get(&content, opts_key) {
                let before = tokens::count(&content);
                let after = tokens::count(&cached);
                let chunk = format!("// === {} ===\n{}\n\n", path.display(), cached);
                return Some((chunk, before, after));
            }
            let ct = if let Some(t) = forced_type {
                t.parse::<ContentType>().ok()?
            } else {
                detect::detect_for_path(path, &content)
            };
            let before = tokens::count(&content);
            let compressed = compress::compress_with(&content, &ct, &file_opts).ok()?;
            crate::cache::set(&content, opts_key, &compressed);
            let after = tokens::count(&compressed);
            let chunk = format!("// === {} ===\n{}\n\n", path.display(), compressed);
            Some((chunk, before, after))
        })
        .collect();

    // ── Merge results (in path-sorted order from par_iter) ────────────────
    let mut combined = String::new();
    let mut total_before = 0usize;
    let mut total_after = 0usize;

    for (chunk, before, after) in results {
        combined.push_str(&chunk);
        total_before += before;
        total_after += after;
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
