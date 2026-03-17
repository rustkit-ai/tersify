use crate::detect::Language;

/// Language-aware code compression.
/// Strips comments (inline and block), collapses blank lines.
pub fn compress(input: &str, lang: &Language) -> String {
    let stripped = match lang {
        Language::Python => strip_python(input),
        _ => strip_cstyle(input, lang),
    };
    collapse_blank_lines(&stripped)
}

// ── C-style languages (Rust, JS, TS, Go, Generic) ─────────────────────────────

/// Remove `// ...` line comments and `/* ... */` block comments.
/// Preserves string literals and Rust doc comments (`///`, `//!`).
fn strip_cstyle(input: &str, lang: &Language) -> String {
    let src: Vec<char> = input.chars().collect();
    let len = src.len();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;

    while i < len {
        // ── line comment ────────────────────────────────────────────────────
        if i + 1 < len && src[i] == '/' && src[i + 1] == '/' {
            // Rust: preserve /// and //! doc comments
            if matches!(lang, Language::Rust)
                && i + 2 < len
                && (src[i + 2] == '/' || src[i + 2] == '!')
            {
                while i < len && src[i] != '\n' {
                    out.push(src[i]);
                    i += 1;
                }
                continue;
            }
            // Strip to end of line (keep the newline itself)
            while i < len && src[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // ── block comment /* ... */ ──────────────────────────────────────────
        if i + 1 < len && src[i] == '/' && src[i + 1] == '*' {
            i += 2;
            // Replace block comment content with whitespace to preserve line count
            while i + 1 < len && !(src[i] == '*' && src[i + 1] == '/') {
                if src[i] == '\n' {
                    out.push('\n');
                }
                i += 1;
            }
            i += 2; // consume */
            continue;
        }

        // ── double-quoted string literal ─────────────────────────────────────
        if src[i] == '"' {
            i = copy_string(&src, i, &mut out, '"');
            continue;
        }

        // ── single-quoted char/string literal ────────────────────────────────
        if src[i] == '\'' {
            i = copy_string(&src, i, &mut out, '\'');
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
            // escaped character
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
fn strip_python(input: &str) -> String {
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
            // Check if this triple-quote is at the start of a logical line
            // (only whitespace before it on this line) → it's a docstring → strip
            let is_standalone = out
                .rsplit('\n')
                .next()
                .map(|l| l.trim().is_empty())
                .unwrap_or(true);

            i += 3; // consume opening """
            if is_standalone {
                // Skip until closing triple quote, preserving newlines
                while i + 2 < len
                    && !(src[i] == delim && src[i + 1] == delim && src[i + 2] == delim)
                {
                    if src[i] == '\n' {
                        out.push('\n');
                    }
                    i += 1;
                }
                i += 3; // consume closing """
            } else {
                // Inline triple-quoted value (e.g. assigned to variable) — keep it
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

        // ── single/double quoted string literal ──────────────────────────────
        if src[i] == '"' || src[i] == '\'' {
            i = copy_string(&src, i, &mut out, src[i]);
            continue;
        }

        // ── # line comment ───────────────────────────────────────────────────
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

// ── Shared post-processing ─────────────────────────────────────────────────────

/// Collapse runs of >1 blank line to exactly one, and strip leading/trailing blank lines.
fn collapse_blank_lines(input: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    let mut blank_run = 0usize;

    for line in input.lines() {
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run == 1 {
                out.push("");
            }
        } else {
            blank_run = 0;
            out.push(line);
        }
    }

    while out.first().is_some_and(|l| l.is_empty()) {
        out.remove(0);
    }
    while out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }

    out.join("\n")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_rust_line_comments_keeps_doc() {
        let src = "// internal\n/// doc comment\nfn foo() {\n    // inline\n    let x = 1;\n}\n";
        let out = compress(src, &Language::Rust);
        assert!(!out.contains("// internal"));
        assert!(!out.contains("// inline"));
        assert!(out.contains("/// doc comment"));
        assert!(out.contains("fn foo()"));
    }

    #[test]
    fn strips_rust_block_comments() {
        let src = "/* header */\nfn foo() { /* inline block */ let x = 1; }\n";
        let out = compress(src, &Language::Rust);
        assert!(!out.contains("header"));
        assert!(!out.contains("inline block"));
        assert!(out.contains("fn foo()"));
        assert!(out.contains("let x = 1"));
    }

    #[test]
    fn preserves_string_with_comment_chars() {
        let src = r#"let s = "not // a comment";"#;
        let out = compress(src, &Language::Rust);
        assert!(out.contains(r#""not // a comment""#));
    }

    #[test]
    fn collapses_blank_lines() {
        let src = "fn a() {}\n\n\n\nfn b() {}";
        let out = compress(src, &Language::Rust);
        assert!(!out.contains("\n\n\n"));
        assert!(out.contains("fn a()"));
        assert!(out.contains("fn b()"));
    }

    #[test]
    fn strips_python_comments() {
        let src = "# module\ndef foo():\n    # inline\n    pass\n";
        let out = compress(src, &Language::Python);
        assert!(!out.contains("# module"));
        assert!(!out.contains("# inline"));
        assert!(out.contains("def foo():"));
    }

    #[test]
    fn strips_python_docstring() {
        let src = "def foo():\n    \"\"\"This is a docstring.\"\"\"\n    pass\n";
        let out = compress(src, &Language::Python);
        assert!(!out.contains("This is a docstring"));
        assert!(out.contains("def foo():"));
        assert!(out.contains("pass"));
    }

    #[test]
    fn preserves_python_string_value() {
        let src = "x = \"not # a comment\"\n";
        let out = compress(src, &Language::Python);
        assert!(out.contains("\"not # a comment\""));
    }
}
