//! AST-based compression: extract function/method signatures and stub bodies.
//!
//! Produces a skeleton view of source code — all function/method signatures are
//! preserved, all implementations replaced with `{ /* ... */ }`.
//! Ideal when passing large files as context and only the API surface matters.

use super::ast_ts;
use super::util::{brace_counts, collapse_blank_lines, leading_ws};
use crate::detect::Language;

/// Compress by stubbing all function/method bodies.
///
/// Signatures, types, imports, module declarations, and trait definitions are
/// preserved; only the implementation code inside function bodies is removed.
///
/// # Examples
///
/// ```
/// use tersify::{compress, detect};
/// use std::path::Path;
/// let src = "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n";
/// let ct = detect::detect_for_path(Path::new("add.rs"), src);
/// let out = compress::compress_with(src, &ct, &compress::CompressOptions { ast: true, ..Default::default() }).unwrap();
/// assert!(out.contains("pub fn add(a: i32, b: i32) -> i32"));
/// assert!(!out.contains("a + b"));
/// ```
pub fn stub_bodies(input: &str, lang: &Language) -> String {
    // Prefer the precise tree-sitter path; fall back to the heuristic parser.
    if let Some(result) = ast_ts::try_stub_bodies(input, lang) {
        return result;
    }
    match lang {
        Language::Python => stub_python(input),
        Language::Ruby => stub_ruby(input),
        _ => stub_cstyle(input),
    }
}

// ── C-style languages (Rust, Go, JS, TS, Generic) ───────────────────────────

fn stub_cstyle(input: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    let mut depth: i32 = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        let (opens, closes) = brace_counts(line);

        // Only stub bodies at top level (depth 0) or one level deep (impl/class methods at depth 1)
        if (depth == 0 || depth == 1) && is_fn_start(trimmed) {
            // Collect the full signature — may span multiple lines until we find `{`
            let mut sig_lines: Vec<&str> = vec![line];
            let mut total_opens = opens;
            let mut total_closes = closes;
            let mut j = i + 1;

            while total_opens == total_closes && j < lines.len() {
                sig_lines.push(lines[j]);
                let (o, c) = brace_counts(lines[j]);
                total_opens += o;
                total_closes += c;
                j += 1;
            }

            if total_opens > total_closes {
                // Body found — emit signature stub on one line
                let indent = leading_ws(line);
                let sig = build_sig(&sig_lines);
                out.push(format!("{}{} {{ /* ... */ }}", indent, sig));

                // Skip the body until depth returns to the level before the opening brace
                let target_depth = depth;
                depth += total_opens - total_closes;
                i = j;
                while i < lines.len() && depth > target_depth {
                    let (o, c) = brace_counts(lines[i]);
                    depth += o - c;
                    i += 1;
                }
            } else {
                // No body found (trait method declaration, extern fn, etc.) — keep as-is
                for &l in &sig_lines {
                    out.push(l.to_string());
                }
                depth += total_opens - total_closes;
                i = j;
            }
            continue;
        }

        depth += opens - closes;
        out.push(line.to_string());
        i += 1;
    }

    collapse_blank_lines(&out.join("\n"))
}

/// Detect whether `trimmed` is the start of a function or method declaration.
fn is_fn_start(trimmed: &str) -> bool {
    let s = strip_modifiers(trimmed);

    // Rust / generic `fn`
    if s.starts_with("fn ") {
        return true;
    }
    // Go / Swift
    if s.starts_with("func ") {
        return true;
    }
    // Kotlin
    if s.starts_with("fun ") {
        return true;
    }
    // JavaScript / TypeScript — standalone or exported
    if s.starts_with("function ") || s.starts_with("async function ") {
        return true;
    }
    if s.starts_with("export ") {
        let after = s.strip_prefix("export ").unwrap_or(s);
        let after = after.strip_prefix("default ").unwrap_or(after);
        let after = after.strip_prefix("async ").unwrap_or(after);
        if after.starts_with("function ") {
            return true;
        }
    }
    // Java methods: after visibility/static/final modifiers → `ReturnType name(`
    if is_java_method(s) {
        return true;
    }

    false
}

/// Heuristic: after stripping modifiers, does this look like a Java method declaration?
///
/// Pattern: two or more words before `(`, where the last word (method name) is a
/// lowercase-starting identifier that isn't a control-flow keyword.
fn is_java_method(s: &str) -> bool {
    if !s.contains('(') {
        return false;
    }
    let before_paren = match s.find('(') {
        Some(pos) => s[..pos].trim_end(),
        None => return false,
    };
    let parts: Vec<&str> = before_paren.split_whitespace().collect();
    if parts.len() < 2 {
        return false; // needs at least ReturnType + methodName
    }
    let name = parts.last().unwrap_or(&"");
    if !name.chars().next().is_some_and(|c| c.is_ascii_lowercase()) {
        return false; // method names start lowercase; constructors handled separately
    }
    !["if", "while", "for", "switch", "catch", "try", "new", "return", "throw", "else"]
        .contains(name)
}

/// Strip language visibility/async/unsafe modifiers to expose the core keyword.
fn strip_modifiers(s: &str) -> &str {
    let mut cur = s;
    loop {
        let prev = cur;
        for prefix in &[
            // Rust
            "pub(crate) ",
            "pub(super) ",
            "pub ",
            "async ",
            "unsafe ",
            "const ",
            "extern ",
            // Java
            "public ",
            "private ",
            "protected ",
            "static ",
            "final ",
            "abstract ",
            "synchronized ",
            "native ",
            "strictfp ",
            // Kotlin
            "override ",
            "open ",
            "inline ",
            "tailrec ",
            "suspend ",
            "operator ",
            "infix ",
            "data ",
            "sealed ",
            // Swift
            "internal ",
            "fileprivate ",
            "mutating ",
        ] {
            if let Some(rest) = cur.strip_prefix(prefix) {
                cur = rest;
                break;
            }
        }
        // Handle `extern "C" fn` — skip the ABI string
        if cur.starts_with('"')
            && let Some(end) = cur.find("\" ")
        {
            cur = &cur[end + 2..];
            continue;
        }
        if cur == prev {
            break;
        }
    }
    cur
}

/// Build a clean single-line signature from possibly multi-line input.
fn build_sig(lines: &[&str]) -> String {
    // Join and normalise whitespace
    let joined = lines.join(" ");
    let normalized: String = joined.split_whitespace().collect::<Vec<_>>().join(" ");
    // Strip everything from the first unquoted `{` onwards
    if let Some(pos) = first_brace_pos(&normalized) {
        normalized[..pos].trim_end().to_string()
    } else {
        normalized.trim().to_string()
    }
}

/// Find the byte position of the first `{` not inside a string.
fn first_brace_pos(s: &str) -> Option<usize> {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut byte_pos = 0;

    while i < chars.len() {
        let c = chars[i];
        if c == '"' || c == '\'' || c == '`' {
            let q = c;
            let char_len = c.len_utf8();
            byte_pos += char_len;
            i += 1;
            while i < chars.len() && chars[i] != q {
                if chars[i] == '\\' {
                    byte_pos += chars[i].len_utf8();
                    i += 1;
                }
                byte_pos += chars[i].len_utf8();
                i += 1;
            }
            if i < chars.len() {
                byte_pos += chars[i].len_utf8();
                i += 1; // closing quote
            }
            continue;
        }
        if c == '{' {
            return Some(byte_pos);
        }
        byte_pos += c.len_utf8();
        i += 1;
    }
    None
}

// ── Ruby ─────────────────────────────────────────────────────────────────────

fn stub_ruby(input: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.starts_with("def ") {
            let indent = leading_ws(line);

            // Collect multi-line signature until parentheses are balanced
            let mut sig_lines: Vec<&str> = vec![line];
            let mut j = i + 1;
            while j < lines.len() && ruby_sig_continues(&sig_lines) {
                sig_lines.push(lines[j]);
                j += 1;
            }

            // Emit compact signature + stub
            let sig_joined = sig_lines.join(" ");
            let sig_clean: String = sig_joined.split_whitespace().collect::<Vec<_>>().join(" ");
            out.push(format!("{}{}", indent, sig_clean.trim_start()));
            out.push(format!("{}  # ...", indent));

            // Skip body until the matching `end` (track keyword depth)
            let mut depth = 1i32;
            i = j;
            while i < lines.len() {
                let inner = lines[i].trim();
                if ruby_opens_block(inner) {
                    depth += 1;
                }
                if ruby_closes_block(inner) {
                    depth -= 1;
                    if depth == 0 {
                        out.push(lines[i].to_string());
                        i += 1;
                        break;
                    }
                }
                i += 1;
            }
            continue;
        }

        out.push(line.to_string());
        i += 1;
    }

    collapse_blank_lines(&out.join("\n"))
}

/// Returns true if the last signature line leaves parentheses unbalanced.
fn ruby_sig_continues(sig_lines: &[&str]) -> bool {
    let joined = sig_lines.join(" ");
    let opens: i32 = joined.chars().filter(|&c| c == '(').count() as i32;
    let closes: i32 = joined.chars().filter(|&c| c == ')').count() as i32;
    opens > closes
}

/// Keywords that open a Ruby block requiring a matching `end`.
fn ruby_opens_block(trimmed: &str) -> bool {
    for kw in &[
        "def ", "class ", "module ", "if ", "unless ", "while ", "until ", "for ", "case ",
    ] {
        if trimmed.starts_with(kw) {
            return true;
        }
    }
    matches!(trimmed, "begin" | "do") || trimmed.starts_with("begin ")
}

/// A line that closes a Ruby block.
fn ruby_closes_block(trimmed: &str) -> bool {
    trimmed == "end"
        || trimmed.starts_with("end ")
        || trimmed.starts_with("end#")
        || trimmed.starts_with("end;")
}

// ── Python ───────────────────────────────────────────────────────────────────

fn stub_python(input: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.starts_with("def ") || trimmed.starts_with("async def ") {
            let def_indent = leading_ws(line).len();

            // Collect multi-line signature until we see the `:` that ends the header
            let mut sig_lines: Vec<&str> = vec![line];
            let mut j = i + 1;

            // A `def` header ends at the `:` that isn't inside parens or brackets
            while !sig_ends(&sig_lines) && j < lines.len() {
                sig_lines.push(lines[j]);
                j += 1;
            }

            // Emit the signature as-is (joining multi-line into one)
            let sig_raw = sig_lines.join(" ");
            let sig_clean: String = sig_raw.split_whitespace().collect::<Vec<_>>().join(" ");
            let sig_final = if sig_clean.ends_with(':') {
                sig_clean
            } else {
                format!("{}:", sig_clean)
            };
            let indent = leading_ws(line);
            out.push(format!("{}{}", indent, sig_final.trim_start()));
            out.push(format!("{}    ...", indent));

            // Skip the body — all lines more indented than def_indent
            i = j;
            while i < lines.len() {
                let next = lines[i];
                if next.trim().is_empty() {
                    i += 1;
                    continue;
                }
                if leading_ws(next).len() > def_indent {
                    i += 1;
                } else {
                    break;
                }
            }
            continue;
        }

        out.push(line.to_string());
        i += 1;
    }

    out.join("\n")
}

/// Rough check: has the `def` header line ended (found a `:` outside parens)?
fn sig_ends(sig_lines: &[&str]) -> bool {
    let joined = sig_lines.join(" ");
    let mut depth = 0i32;
    for c in joined.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            ':' if depth == 0 => return true,
            _ => {}
        }
    }
    false
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stubs_rust_function_body() {
        let src = "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n";
        let out = stub_bodies(src, &Language::Rust);
        assert!(out.contains("pub fn add(a: i32, b: i32) -> i32"));
        assert!(!out.contains("a + b"));
        assert!(out.contains("/* ... */"));
    }

    #[test]
    fn keeps_struct_and_impl_header() {
        let src = "struct Foo {\n    x: i32,\n}\n\nimpl Foo {\n    fn bar(&self) -> i32 {\n        self.x\n    }\n}\n";
        let out = stub_bodies(src, &Language::Rust);
        assert!(out.contains("struct Foo {"));
        assert!(out.contains("x: i32,"));
        assert!(out.contains("impl Foo {"));
        assert!(out.contains("fn bar(&self) -> i32"));
        assert!(!out.contains("self.x"));
    }

    #[test]
    fn stubs_go_function() {
        let src = "func Add(a, b int) int {\n\treturn a + b\n}\n";
        let out = stub_bodies(src, &Language::Go);
        assert!(out.contains("func Add(a, b int) int"));
        assert!(!out.contains("return a + b"));
    }

    #[test]
    fn stubs_python_function() {
        let src = "def process(data):\n    result = transform(data)\n    return result\n\ndef helper():\n    pass\n";
        let out = stub_python(src);
        assert!(out.contains("def process(data):"));
        assert!(!out.contains("result = transform"));
        assert!(out.contains("def helper():"));
    }

    #[test]
    fn keeps_trait_method_declaration() {
        // Trait methods with semicolons (no body) should be kept
        let src = "trait Validator {\n    fn validate(&self, input: &str) -> bool;\n}\n";
        let out = stub_bodies(src, &Language::Rust);
        assert!(out.contains("fn validate(&self, input: &str) -> bool;"));
    }

    #[test]
    fn stubs_ruby_method_body() {
        let src = "def greet(name)\n  puts \"Hello, #{name}\"\n  42\nend\n";
        let out = stub_bodies(src, &Language::Ruby);
        assert!(out.contains("def greet(name)"));
        assert!(!out.contains("puts"));
        assert!(out.contains("# ..."));
        assert!(out.contains("end"));
    }

    #[test]
    fn stubs_ruby_nested_blocks_correctly() {
        let src = "def process(items)\n  items.each do |i|\n    puts i\n  end\nend\n";
        let out = stub_bodies(src, &Language::Ruby);
        assert!(out.contains("def process(items)"));
        assert!(!out.contains("each"));
        assert!(!out.contains("puts"));
        assert!(out.contains("end"));
    }

    #[test]
    fn stubs_kotlin_function() {
        let src = "fun add(a: Int, b: Int): Int {\n    return a + b\n}\n";
        let out = stub_bodies(src, &Language::Kotlin);
        assert!(out.contains("fun add(a: Int, b: Int): Int"));
        assert!(!out.contains("return a + b"));
        assert!(out.contains("/* ... */"));
    }

    #[test]
    fn stubs_swift_function() {
        let src = "func greet(name: String) -> String {\n    return \"Hello \" + name\n}\n";
        let out = stub_bodies(src, &Language::Swift);
        assert!(out.contains("func greet(name: String) -> String"));
        assert!(!out.contains("Hello"));
        assert!(out.contains("/* ... */"));
    }

    #[test]
    fn stubs_java_method() {
        let src =
            "public class Foo {\n    public int add(int a, int b) {\n        return a + b;\n    }\n}\n";
        let out = stub_bodies(src, &Language::Java);
        assert!(out.contains("public class Foo {"));
        assert!(out.contains("public int add(int a, int b)"));
        assert!(!out.contains("return a + b"));
        assert!(out.contains("/* ... */"));
    }
}
