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
    Tsx,
    Go,
    Ruby,
    Java,
    C, // covers C and C++
    CSharp,
    Php,
    Swift,
    Kotlin,
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

    /// Language label for stats tracking (e.g. `"rust"`, `"json"`, `"diff"`).
    pub fn lang_str(&self) -> &'static str {
        match self {
            Self::Code(lang) => lang.as_str(),
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
            Self::Tsx => "tsx",
            Self::Go => "go",
            Self::Ruby => "ruby",
            Self::Java => "java",
            Self::C => "c",
            Self::CSharp => "csharp",
            Self::Php => "php",
            Self::Swift => "swift",
            Self::Kotlin => "kotlin",
            Self::Generic => "generic",
        }
    }
}

impl std::str::FromStr for ContentType {
    type Err = TersifyError;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            // Generic code
            "code" | "generic" => Ok(Self::Code(Language::Generic)),
            // Language-specific code
            "rust" | "rs" => Ok(Self::Code(Language::Rust)),
            "python" | "py" => Ok(Self::Code(Language::Python)),
            "javascript" | "js" => Ok(Self::Code(Language::JavaScript)),
            "typescript" | "ts" => Ok(Self::Code(Language::TypeScript)),
            "tsx" => Ok(Self::Code(Language::Tsx)),
            "go" => Ok(Self::Code(Language::Go)),
            "ruby" | "rb" => Ok(Self::Code(Language::Ruby)),
            "java" => Ok(Self::Code(Language::Java)),
            "c" | "cpp" | "c++" => Ok(Self::Code(Language::C)),
            "csharp" | "cs" | "c#" => Ok(Self::Code(Language::CSharp)),
            "php" => Ok(Self::Code(Language::Php)),
            "swift" => Ok(Self::Code(Language::Swift)),
            "kotlin" | "kt" => Ok(Self::Code(Language::Kotlin)),
            // Other types
            "json" => Ok(Self::Json),
            "logs" | "log" => Ok(Self::Logs),
            "diff" | "patch" => Ok(Self::Diff),
            "text" | "txt" => Ok(Self::Text),
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
        Some("ts") => ContentType::Code(Language::TypeScript),
        Some("tsx") => ContentType::Code(Language::Tsx),
        Some("go") => ContentType::Code(Language::Go),
        Some("rb" | "rake" | "gemspec") => ContentType::Code(Language::Ruby),
        Some("java") => ContentType::Code(Language::Java),
        Some("c" | "cpp" | "cc" | "cxx" | "h" | "hpp" | "hxx") => ContentType::Code(Language::C),
        Some("cs") => ContentType::Code(Language::CSharp),
        Some("php" | "phtml" | "php5") => ContentType::Code(Language::Php),
        Some("swift") => ContentType::Code(Language::Swift),
        Some("kt" | "kts") => ContentType::Code(Language::Kotlin),
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
    // C/C++: distinctive preprocessor directives
    if s.contains("#include") || s.contains("#define") || s.contains("#pragma") {
        return Some(Language::C);
    }
    // Rust signals
    if s.contains("fn ") && (s.contains("let ") || s.contains("impl ") || s.contains("use ")) {
        return Some(Language::Rust);
    }
    // TypeScript (must come before JS — more specific)
    if s.contains("interface ") || s.contains(": string") || s.contains(": number") {
        return Some(Language::TypeScript);
    }
    // Kotlin
    if s.contains("fun ") && (s.contains("val ") || s.contains("var ") || s.contains("data class"))
    {
        return Some(Language::Kotlin);
    }
    // Swift
    if s.contains("import Foundation") || s.contains("import UIKit") || s.contains("import SwiftUI")
    {
        return Some(Language::Swift);
    }
    // Java
    if s.contains("public class ")
        || s.contains("import java.")
        || s.contains("@Override")
        || s.contains("System.out.println")
    {
        return Some(Language::Java);
    }
    // Ruby
    if s.contains("def ") && s.contains("end") && (s.contains("require ") || s.contains("attr_")) {
        return Some(Language::Ruby);
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
    fn detects_c_by_extension() {
        let ct = detect_for_path(Path::new("main.cpp"), "int main() {}");
        assert_eq!(ct, ContentType::Code(Language::C));
    }

    #[test]
    fn detects_ruby_by_extension() {
        let ct = detect_for_path(Path::new("app.rb"), "def hello; end");
        assert_eq!(ct, ContentType::Code(Language::Ruby));
    }

    #[test]
    fn detects_kotlin_by_extension() {
        let ct = detect_for_path(Path::new("Main.kt"), "fun main() {}");
        assert_eq!(ct, ContentType::Code(Language::Kotlin));
    }

    #[test]
    fn detects_java_by_extension() {
        let ct = detect_for_path(Path::new("Main.java"), "public class Main {}");
        assert_eq!(ct, ContentType::Code(Language::Java));
    }

    #[test]
    fn detects_swift_by_extension() {
        let ct = detect_for_path(Path::new("App.swift"), "func greet() {}");
        assert_eq!(ct, ContentType::Code(Language::Swift));
    }

    #[test]
    fn parses_language_type_flags() {
        assert_eq!(
            "rust".parse::<ContentType>().unwrap(),
            ContentType::Code(Language::Rust)
        );
        assert_eq!(
            "ruby".parse::<ContentType>().unwrap(),
            ContentType::Code(Language::Ruby)
        );
        assert_eq!(
            "kotlin".parse::<ContentType>().unwrap(),
            ContentType::Code(Language::Kotlin)
        );
        assert_eq!(
            "c".parse::<ContentType>().unwrap(),
            ContentType::Code(Language::C)
        );
        assert_eq!(
            "swift".parse::<ContentType>().unwrap(),
            ContentType::Code(Language::Swift)
        );
    }

    #[test]
    fn unknown_type_returns_err() {
        let result = "unknown".parse::<ContentType>();
        assert!(result.is_err());
    }
}
