use super::util::{collapse_blank_lines, leading_ws};
use crate::detect::Language;

/// Language-aware code compression.
/// Strips comments (inline and block), collapses blank lines.
/// When `strip_docs` is true, doc comments (`///`, `//!`, `/** */`, Python docstrings) are
/// also removed.
pub fn compress(input: &str, lang: &Language, strip_docs: bool) -> String {
    match lang {
        Language::Python => collapse_blank_lines(&strip_python(input, strip_docs)),
        Language::Ruby => strip_ruby(input),
        Language::Html => strip_html(input),
        Language::Css => strip_css(input),
        Language::Sql => strip_sql(input),
        Language::Shell => strip_shell(input),
        Language::Yaml => strip_yaml(input),
        // Tsx shares TypeScript comment rules — map to TypeScript for stripping
        Language::Tsx => {
            collapse_blank_lines(&strip_cstyle(input, &Language::TypeScript, strip_docs))
        }
        _ => collapse_blank_lines(&strip_cstyle(input, lang, strip_docs)),
    }
}

// ── C-style languages (Rust, JS, TS, Go, Java, C, Swift, Kotlin, Generic) ───

/// Remove `// ...` line comments and `/* ... */` block comments.
///
/// Per-language doc-comment preservation:
/// - Rust: keeps `///` and `//!`
/// - Swift: keeps `///`
/// - Others: strips all line and block comments
fn strip_cstyle(input: &str, lang: &Language, strip_docs: bool) -> String {
    let src: Vec<char> = input.chars().collect();
    let len = src.len();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;

    while i < len {
        // ── line comment ─────────────────────────────────────────────────────
        if i + 1 < len && src[i] == '/' && src[i + 1] == '/' {
            let is_doc = match lang {
                Language::Rust => i + 2 < len && (src[i + 2] == '/' || src[i + 2] == '!'),
                Language::Swift => i + 2 < len && src[i + 2] == '/',
                _ => false,
            };
            // Keep doc comment only when strip_docs is false
            if is_doc && !strip_docs {
                while i < len && src[i] != '\n' {
                    out.push(src[i]);
                    i += 1;
                }
                continue;
            }
            while i < len && src[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // ── block comment /* ... */ ──────────────────────────────────────────
        if i + 1 < len && src[i] == '/' && src[i + 1] == '*' {
            i += 2;
            while i + 1 < len && !(src[i] == '*' && src[i + 1] == '/') {
                if src[i] == '\n' {
                    out.push('\n');
                }
                i += 1;
            }
            i += 2; // consume */
            continue;
        }

        // ── double-quoted string ─────────────────────────────────────────────
        if src[i] == '"' {
            i = copy_string(&src, i, &mut out, '"');
            continue;
        }

        // ── single-quoted char/string ────────────────────────────────────────
        if src[i] == '\'' {
            i = copy_string(&src, i, &mut out, '\'');
            continue;
        }

        // ── backtick string (JS/TS) ──────────────────────────────────────────
        if src[i] == '`' {
            i = copy_string(&src, i, &mut out, '`');
            continue;
        }

        out.push(src[i]);
        i += 1;
    }

    out
}

/// Copy a quoted string literal from `src[start]` into `out`, return next index.
fn copy_string(src: &[char], start: usize, out: &mut String, delimiter: char) -> usize {
    let mut i = start;
    out.push(src[i]); // opening quote
    i += 1;
    while i < src.len() {
        if src[i] == '\\' && i + 1 < src.len() {
            out.push(src[i]);
            out.push(src[i + 1]);
            i += 2;
            continue;
        }
        out.push(src[i]);
        if src[i] == delimiter {
            i += 1;
            break;
        }
        i += 1;
    }
    i
}

// ── Python ────────────────────────────────────────────────────────────────────

/// Remove `# ...` comments and standalone triple-quoted docstrings.
/// `strip_docs` is accepted for API consistency but has no additional effect since
/// Python docstrings are always stripped (they are indistinguishable from standalone strings).
fn strip_python(input: &str, _strip_docs: bool) -> String {
    let src: Vec<char> = input.chars().collect();
    let len = src.len();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;

    while i < len {
        // ── triple-quoted string (""" or ''') ────────────────────────────────
        if i + 2 < len
            && ((src[i] == '"' && src[i + 1] == '"' && src[i + 2] == '"')
                || (src[i] == '\'' && src[i + 1] == '\'' && src[i + 2] == '\''))
        {
            let delim = src[i];
            let is_standalone = out
                .rsplit('\n')
                .next()
                .map(|l| l.trim().is_empty())
                .unwrap_or(true);

            i += 3;
            if is_standalone {
                while i + 2 < len
                    && !(src[i] == delim && src[i + 1] == delim && src[i + 2] == delim)
                {
                    if src[i] == '\n' {
                        out.push('\n');
                    }
                    i += 1;
                }
                i += 3;
            } else {
                out.push(delim);
                out.push(delim);
                out.push(delim);
                while i + 2 < len
                    && !(src[i] == delim && src[i + 1] == delim && src[i + 2] == delim)
                {
                    out.push(src[i]);
                    i += 1;
                }
                out.push(delim);
                out.push(delim);
                out.push(delim);
                i += 3;
            }
            continue;
        }

        if src[i] == '"' || src[i] == '\'' {
            i = copy_string(&src, i, &mut out, src[i]);
            continue;
        }

        if src[i] == '#' {
            while i < len && src[i] != '\n' {
                i += 1;
            }
            continue;
        }

        out.push(src[i]);
        i += 1;
    }

    out
}

// ── Ruby ──────────────────────────────────────────────────────────────────────

/// Remove Ruby `# ...` comments and `=begin`/`=end` block comments.
fn strip_ruby(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_block_comment = false;

    for line in input.lines() {
        let trimmed = line.trim_start();

        // =begin / =end must be at the start of a line (no leading whitespace)
        if line.starts_with("=begin") {
            in_block_comment = true;
            out.push('\n');
            continue;
        }
        if line.starts_with("=end") {
            in_block_comment = false;
            out.push('\n');
            continue;
        }
        if in_block_comment {
            out.push('\n');
            continue;
        }

        // Strip `#` line comments (but keep shebang `#!/...` on first line)
        if trimmed.starts_with('#') && !line.starts_with("#!") {
            out.push('\n');
            continue;
        }

        // Inline `#` comment — find first `#` outside strings
        let clean = strip_ruby_inline_comment(line);
        out.push_str(clean);
        out.push('\n');
    }

    // Use the leading_ws utility but process via collapse_blank_lines
    let _ = leading_ws; // suppress unused warning — used in ast.rs
    collapse_blank_lines(out.trim_end())
}

/// Strip the inline `#` comment from a Ruby line, respecting string literals.
fn strip_ruby_inline_comment(line: &str) -> &str {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    let mut byte_pos = 0usize;

    while i < chars.len() {
        let c = chars[i];
        if c == '"' || c == '\'' {
            let q = c;
            byte_pos += c.len_utf8();
            i += 1;
            while i < chars.len() && chars[i] != q {
                if chars[i] == '\\' {
                    byte_pos += chars[i].len_utf8();
                    i += 1;
                }
                if i < chars.len() {
                    byte_pos += chars[i].len_utf8();
                    i += 1;
                }
            }
            if i < chars.len() {
                byte_pos += chars[i].len_utf8();
                i += 1; // closing quote
            }
            continue;
        }
        if c == '#' {
            return line[..byte_pos].trim_end();
        }
        byte_pos += c.len_utf8();
        i += 1;
    }
    line
}

// ── HTML ──────────────────────────────────────────────────────────────────────

/// Strip `<!-- ... -->` HTML comments and collapse blank lines.
fn strip_html(input: &str) -> String {
    let src: Vec<char> = input.chars().collect();
    let len = src.len();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;

    while i < len {
        // <!-- ... -->
        if i + 3 < len
            && src[i] == '<'
            && src[i + 1] == '!'
            && src[i + 2] == '-'
            && src[i + 3] == '-'
        {
            i += 4;
            while i + 2 < len && !(src[i] == '-' && src[i + 1] == '-' && src[i + 2] == '>') {
                if src[i] == '\n' {
                    out.push('\n');
                }
                i += 1;
            }
            if i + 2 < len {
                i += 3; // consume -->
            }
            continue;
        }
        out.push(src[i]);
        i += 1;
    }

    collapse_blank_lines(&out)
}

// ── CSS ───────────────────────────────────────────────────────────────────────

/// Strip `/* ... */` CSS comments and collapse blank lines.
/// Does NOT strip `//` sequences to preserve URLs like `url(//cdn.example.com/...)`.
fn strip_css(input: &str) -> String {
    let src: Vec<char> = input.chars().collect();
    let len = src.len();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;

    while i < len {
        // /* ... */
        if i + 1 < len && src[i] == '/' && src[i + 1] == '*' {
            i += 2;
            while i + 1 < len && !(src[i] == '*' && src[i + 1] == '/') {
                if src[i] == '\n' {
                    out.push('\n');
                }
                i += 1;
            }
            if i + 1 < len {
                i += 2; // consume */
            }
            continue;
        }
        out.push(src[i]);
        i += 1;
    }

    collapse_blank_lines(&out)
}

// ── SQL ───────────────────────────────────────────────────────────────────────

/// Strip SQL `--` line comments and `/* ... */` block comments; collapse blank lines.
fn strip_sql(input: &str) -> String {
    let src: Vec<char> = input.chars().collect();
    let len = src.len();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;

    while i < len {
        // -- line comment
        if i + 1 < len && src[i] == '-' && src[i + 1] == '-' {
            while i < len && src[i] != '\n' {
                i += 1;
            }
            continue;
        }
        // /* ... */ block comment
        if i + 1 < len && src[i] == '/' && src[i + 1] == '*' {
            i += 2;
            while i + 1 < len && !(src[i] == '*' && src[i + 1] == '/') {
                if src[i] == '\n' {
                    out.push('\n');
                }
                i += 1;
            }
            if i + 1 < len {
                i += 2;
            }
            continue;
        }
        // Single-quoted string literal
        if src[i] == '\'' {
            out.push(src[i]);
            i += 1;
            while i < len {
                out.push(src[i]);
                if src[i] == '\'' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        out.push(src[i]);
        i += 1;
    }

    collapse_blank_lines(&out)
}

// ── Shell ─────────────────────────────────────────────────────────────────────

/// Strip shell `#` comments (preserving shebang `#!/`), strip inline `#` comments
/// outside quoted strings, and collapse blank lines.
fn strip_shell(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut is_first_line = true;

    for line in input.lines() {
        let trimmed = line.trim_start();
        // Keep shebang on first non-empty line
        if is_first_line && line.starts_with("#!") {
            out.push_str(line);
            out.push('\n');
            is_first_line = false;
            continue;
        }
        if !trimmed.is_empty() {
            is_first_line = false;
        }
        // Full-line comment
        if trimmed.starts_with('#') {
            out.push('\n');
            continue;
        }
        // Strip inline comment outside quoted strings
        let clean = strip_shell_inline_comment(line);
        out.push_str(clean);
        out.push('\n');
    }

    collapse_blank_lines(out.trim_end())
}

/// Strip the inline `#` comment from a shell line, respecting quoted strings.
fn strip_shell_inline_comment(line: &str) -> &str {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    let mut byte_pos = 0usize;

    while i < chars.len() {
        let c = chars[i];
        // Single-quoted string: no escape sequences
        if c == '\'' {
            byte_pos += c.len_utf8();
            i += 1;
            while i < chars.len() && chars[i] != '\'' {
                byte_pos += chars[i].len_utf8();
                i += 1;
            }
            if i < chars.len() {
                byte_pos += chars[i].len_utf8();
                i += 1;
            }
            continue;
        }
        // Double-quoted string
        if c == '"' {
            byte_pos += c.len_utf8();
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    byte_pos += chars[i].len_utf8();
                    i += 1;
                }
                byte_pos += chars[i].len_utf8();
                i += 1;
            }
            if i < chars.len() {
                byte_pos += chars[i].len_utf8();
                i += 1;
            }
            continue;
        }
        if c == '#' {
            return line[..byte_pos].trim_end();
        }
        byte_pos += c.len_utf8();
        i += 1;
    }
    line
}

// ── YAML ──────────────────────────────────────────────────────────────────────

/// Strip YAML `#` comments — both full-line and inline (preceded by whitespace) —
/// respecting quoted strings; collapse blank lines.
fn strip_yaml(input: &str) -> String {
    let mut out = String::with_capacity(input.len());

    for line in input.lines() {
        let trimmed = line.trim_start();
        // Full-line comment
        if trimmed.starts_with('#') {
            out.push('\n');
            continue;
        }
        // Strip inline comment outside quoted strings
        let clean = strip_yaml_inline_comment(line);
        out.push_str(clean);
        out.push('\n');
    }

    collapse_blank_lines(out.trim_end())
}

/// Strip an inline YAML `# comment` (preceded by whitespace), respecting quoted strings.
fn strip_yaml_inline_comment(line: &str) -> &str {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    let mut byte_pos = 0usize;

    while i < chars.len() {
        let c = chars[i];
        // Single-quoted YAML string ('' escapes a single quote inside)
        if c == '\'' {
            byte_pos += c.len_utf8();
            i += 1;
            while i < chars.len() {
                if chars[i] == '\'' {
                    byte_pos += chars[i].len_utf8();
                    i += 1;
                    if i < chars.len() && chars[i] == '\'' {
                        // escaped '' — continue inside string
                        byte_pos += chars[i].len_utf8();
                        i += 1;
                    } else {
                        break; // end of string
                    }
                } else {
                    byte_pos += chars[i].len_utf8();
                    i += 1;
                }
            }
            continue;
        }
        // Double-quoted YAML string
        if c == '"' {
            byte_pos += c.len_utf8();
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    byte_pos += chars[i].len_utf8();
                    i += 1;
                }
                byte_pos += chars[i].len_utf8();
                i += 1;
            }
            if i < chars.len() {
                byte_pos += chars[i].len_utf8();
                i += 1;
            }
            continue;
        }
        // YAML inline comment: # preceded by whitespace
        if c == '#' && i > 0 && (chars[i - 1] == ' ' || chars[i - 1] == '\t') {
            return line[..byte_pos].trim_end();
        }
        byte_pos += c.len_utf8();
        i += 1;
    }
    line
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_rust_line_comments_keeps_doc() {
        let src = "// internal\n/// doc comment\nfn foo() {\n    // inline\n    let x = 1;\n}\n";
        let out = compress(src, &Language::Rust, false);
        assert!(!out.contains("// internal"));
        assert!(!out.contains("// inline"));
        assert!(out.contains("/// doc comment"));
        assert!(out.contains("fn foo()"));
    }

    #[test]
    fn strips_rust_block_comments() {
        let src = "/* header */\nfn foo() { /* inline block */ let x = 1; }\n";
        let out = compress(src, &Language::Rust, false);
        assert!(!out.contains("header"));
        assert!(!out.contains("inline block"));
        assert!(out.contains("fn foo()"));
        assert!(out.contains("let x = 1"));
    }

    #[test]
    fn preserves_string_with_comment_chars() {
        let src = r#"let s = "not // a comment";"#;
        let out = compress(src, &Language::Rust, false);
        assert!(out.contains(r#""not // a comment""#));
    }

    #[test]
    fn collapses_blank_lines() {
        let src = "fn a() {}\n\n\n\nfn b() {}";
        let out = compress(src, &Language::Rust, false);
        assert!(!out.contains("\n\n\n"));
        assert!(out.contains("fn a()"));
        assert!(out.contains("fn b()"));
    }

    #[test]
    fn strips_python_comments() {
        let src = "# module\ndef foo():\n    # inline\n    pass\n";
        let out = compress(src, &Language::Python, false);
        assert!(!out.contains("# module"));
        assert!(!out.contains("# inline"));
        assert!(out.contains("def foo():"));
    }

    #[test]
    fn strips_python_docstring() {
        let src = "def foo():\n    \"\"\"This is a docstring.\"\"\"\n    pass\n";
        let out = compress(src, &Language::Python, false);
        assert!(!out.contains("This is a docstring"));
        assert!(out.contains("def foo():"));
        assert!(out.contains("pass"));
    }

    #[test]
    fn preserves_python_string_value() {
        let src = "x = \"not # a comment\"\n";
        let out = compress(src, &Language::Python, false);
        assert!(out.contains("\"not # a comment\""));
    }

    #[test]
    fn strips_ruby_line_comment() {
        let src = "# top-level comment\ndef hello\n  puts 'hi' # inline\nend\n";
        let out = compress(src, &Language::Ruby, false);
        assert!(!out.contains("# top-level comment"));
        assert!(!out.contains("# inline"));
        assert!(out.contains("def hello"));
        assert!(out.contains("puts 'hi'"));
    }

    #[test]
    fn strips_ruby_block_comment() {
        let src = "=begin\nThis is a block comment\n=end\ndef greet\n  puts 'hello'\nend\n";
        let out = compress(src, &Language::Ruby, false);
        assert!(!out.contains("block comment"));
        assert!(out.contains("def greet"));
    }

    #[test]
    fn strips_java_comments() {
        let src = "// single line\n/* block */\npublic class Foo {\n    int x; // inline\n}\n";
        let out = compress(src, &Language::Java, false);
        assert!(!out.contains("single line"));
        assert!(!out.contains("block"));
        assert!(!out.contains("// inline"));
        assert!(out.contains("public class Foo"));
    }

    #[test]
    fn strips_c_comments() {
        let src = "#include <stdio.h>\n// comment\nint main() { /* block */ return 0; }\n";
        let out = compress(src, &Language::C, false);
        assert!(!out.contains("// comment"));
        assert!(!out.contains("block"));
        assert!(out.contains("int main()"));
    }

    #[test]
    fn swift_keeps_doc_comments() {
        let src = "/// Documented function\n// internal note\nfunc greet() {}\n";
        let out = compress(src, &Language::Swift, false);
        assert!(out.contains("/// Documented function"));
        assert!(!out.contains("// internal note"));
        assert!(out.contains("func greet()"));
    }

    #[test]
    fn strips_kotlin_comments() {
        let src = "// comment\nfun main() {\n    /* block */\n    println(\"hello\")\n}\n";
        let out = compress(src, &Language::Kotlin, false);
        assert!(!out.contains("// comment"));
        assert!(!out.contains("block"));
        assert!(out.contains("fun main()"));
    }

    #[test]
    fn strip_docs_removes_rust_doc_comments() {
        let src = "/// doc comment\n//! module doc\nfn foo() {}\n";
        let out = compress(src, &Language::Rust, true);
        assert!(!out.contains("/// doc comment"));
        assert!(!out.contains("//! module doc"));
        assert!(out.contains("fn foo()"));
    }

    #[test]
    fn strip_docs_false_keeps_rust_doc_comments() {
        let src = "/// doc comment\nfn foo() {}\n";
        let out = compress(src, &Language::Rust, false);
        assert!(out.contains("/// doc comment"));
    }
}
