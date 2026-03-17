use crate::error::{Result, TersifyError};
use std::path::Path;

/// The kind of content, used to select the compression strategy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentType {
    Code(Language),
    Json,
    Logs,
    Diff,
    Text,
}

/// Programming language, for language-aware code compression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Generic,
}

impl ContentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Code(_) => "code",
            Self::Json => "json",
            Self::Logs => "logs",
            Self::Diff => "diff",
            Self::Text => "text",
        }
    }
}

impl std::fmt::Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Code(lang) => write!(f, "code({})", lang.as_str()),
            other => f.write_str(other.as_str()),
        }
    }
}

impl Language {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Go => "go",
            Self::Generic => "generic",
        }
    }
}

impl std::str::FromStr for ContentType {
    type Err = TersifyError;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "code" => Ok(Self::Code(Language::Generic)),
            "json" => Ok(Self::Json),
            "logs" => Ok(Self::Logs),
            "diff" => Ok(Self::Diff),
            "text" => Ok(Self::Text),
            other => Err(TersifyError::UnknownContentType(other.to_string())),
        }
    }
}

/// Detect content type using file extension as primary signal, falling back to content analysis.
pub fn detect_for_path(path: &Path, content: &str) -> ContentType {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => ContentType::Code(Language::Rust),
        Some("py") => ContentType::Code(Language::Python),
        Some("js" | "jsx" | "mjs" | "cjs") => ContentType::Code(Language::JavaScript),
        Some("ts" | "tsx") => ContentType::Code(Language::TypeScript),
        Some("go") => ContentType::Code(Language::Go),
        Some("json" | "jsonc") => ContentType::Json,
        Some("log") => ContentType::Logs,
        Some("diff" | "patch") => ContentType::Diff,
        _ => detect(content),
    }
}

/// Auto-detect content type from raw input.
pub fn detect(input: &str) -> ContentType {
    let trimmed = input.trim_start();

    if is_diff(input) {
        return ContentType::Diff;
    }
    if is_json(trimmed) {
        return ContentType::Json;
    }
    if is_logs(input) {
        return ContentType::Logs;
    }
    if let Some(lang) = detect_language(input) {
        return ContentType::Code(lang);
    }
    ContentType::Text
}

/// Infer programming language from content patterns.
pub fn detect_language(s: &str) -> Option<Language> {
    // Rust signals
    if s.contains("fn ") && (s.contains("let ") || s.contains("impl ") || s.contains("use ")) {
        return Some(Language::Rust);
    }
    // TypeScript (must come before JS — more specific)
    if s.contains("interface ") || s.contains(": string") || s.contains(": number") {
        return Some(Language::TypeScript);
    }
    // Python
    if s.contains("def ") && (s.contains("self") || s.contains("import ")) {
        return Some(Language::Python);
    }
    // Go
    if s.contains("func ") && s.contains("package ") {
        return Some(Language::Go);
    }
    // JavaScript
    if s.contains("const ") || s.contains("function ") || s.contains("export ") {
        return Some(Language::JavaScript);
    }

    let code_signals: &[&str] = &[
        "fn ", "impl ", "struct ", "enum ", "trait ", "def ", "class ", "import ", "#include",
    ];
    let matches = code_signals.iter().filter(|&&sig| s.contains(sig)).count();
    if matches >= 2 {
        return Some(Language::Generic);
    }
    None
}

fn is_json(s: &str) -> bool {
    (s.starts_with('{') || s.starts_with('['))
        && serde_json::from_str::<serde_json::Value>(s).is_ok()
}

fn is_diff(s: &str) -> bool {
    s.starts_with("diff --git")
        || s.starts_with("--- ")
        || (s.contains("\n+++ ") && s.contains("\n--- "))
}

fn is_logs(s: &str) -> bool {
    let log_signals: &[&str] = &[
        "ERROR",
        "WARN",
        "INFO",
        "DEBUG",
        "TRACE",
        "[error]",
        "[warn]",
        "[info]",
        "[debug]",
        "level=",
        "msg=",
        "timestamp=",
    ];
    let lines: Vec<&str> = s.lines().collect();
    let total = lines.len();
    if total < 3 {
        return false;
    }
    let matching = lines
        .iter()
        .filter(|l| log_signals.iter().any(|sig| l.contains(sig)))
        .count();
    matching as f64 / total as f64 > 0.3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_json() {
        assert_eq!(detect(r#"{"key": "value"}"#), ContentType::Json);
        assert_eq!(detect(r#"[1, 2, 3]"#), ContentType::Json);
    }

    #[test]
    fn detects_rust() {
        let src = "use std::io;\nfn main() {\n    let x = 1;\n}";
        assert_eq!(detect(src), ContentType::Code(Language::Rust));
    }

    #[test]
    fn detects_diff() {
        let d = "diff --git a/foo.rs b/foo.rs\n--- a/foo.rs\n+++ b/foo.rs\n@@ -1,3 +1,3 @@\n";
        assert_eq!(detect(d), ContentType::Diff);
    }

    #[test]
    fn detects_logs() {
        let logs = "2024-01-01 INFO server started\n2024-01-01 ERROR connection refused\n2024-01-01 WARN retry attempt\n2024-01-01 INFO done\n";
        assert_eq!(detect(logs), ContentType::Logs);
    }

    #[test]
    fn unknown_type_returns_err() {
        let result = "unknown".parse::<ContentType>();
        assert!(result.is_err());
    }
}
