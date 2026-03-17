//! Tree-sitter–backed AST body stubbing.
//!
//! Uses the `tree-sitter` parse-tree to locate function/method bodies precisely,
//! then replaces each body with `{ /* ... */ }` (C-style) or `\n    pass` (Python).
//! Falls back gracefully: if parsing fails or no functions are found, returns `None`
//! so the caller can use the heuristic path.

use crate::detect::Language;
use tree_sitter::{Language as TsLanguage, Parser};

// ── Language registry ─────────────────────────────────────────────────────────

struct LangConfig {
    ts_language: TsLanguage,
    fn_kinds: &'static [&'static str],
    body_field: &'static str,
    stub_style: StubStyle,
}

#[derive(Clone, Copy)]
enum StubStyle {
    /// Replace body with `{ /* ... */ }`
    Braces,
    /// Replace body with `:\n    pass` (Python)
    Python,
    /// Replace body_statement contents (Ruby)
    Ruby,
}

fn lang_config(lang: &Language) -> Option<LangConfig> {
    Some(match lang {
        Language::Rust => LangConfig {
            ts_language: TsLanguage::new(tree_sitter_rust::LANGUAGE),
            fn_kinds: &["function_item"],
            body_field: "body",
            stub_style: StubStyle::Braces,
        },
        Language::Go => LangConfig {
            ts_language: TsLanguage::new(tree_sitter_go::LANGUAGE),
            fn_kinds: &["function_declaration", "method_declaration"],
            body_field: "body",
            stub_style: StubStyle::Braces,
        },
        Language::Java => LangConfig {
            ts_language: TsLanguage::new(tree_sitter_java::LANGUAGE),
            fn_kinds: &["method_declaration", "constructor_declaration"],
            body_field: "body",
            stub_style: StubStyle::Braces,
        },
        // Use the C++ grammar for Language::C — it is a superset that parses plain C correctly.
        Language::C => LangConfig {
            ts_language: TsLanguage::new(tree_sitter_cpp::LANGUAGE),
            fn_kinds: &["function_definition"],
            body_field: "body",
            stub_style: StubStyle::Braces,
        },
        Language::JavaScript => LangConfig {
            ts_language: TsLanguage::new(tree_sitter_javascript::LANGUAGE),
            fn_kinds: &[
                "function_declaration",
                "function",
                "method_definition",
                "arrow_function",
            ],
            body_field: "body",
            stub_style: StubStyle::Braces,
        },
        Language::TypeScript => LangConfig {
            ts_language: TsLanguage::new(tree_sitter_typescript::LANGUAGE_TYPESCRIPT),
            fn_kinds: &[
                "function_declaration",
                "function",
                "method_definition",
                "arrow_function",
            ],
            body_field: "body",
            stub_style: StubStyle::Braces,
        },
        Language::Tsx => LangConfig {
            ts_language: TsLanguage::new(tree_sitter_typescript::LANGUAGE_TSX),
            fn_kinds: &[
                "function_declaration",
                "function",
                "method_definition",
                "arrow_function",
            ],
            body_field: "body",
            stub_style: StubStyle::Braces,
        },
        Language::Python => LangConfig {
            ts_language: TsLanguage::new(tree_sitter_python::LANGUAGE),
            fn_kinds: &["function_definition"],
            body_field: "body",
            stub_style: StubStyle::Python,
        },
        Language::Ruby => LangConfig {
            ts_language: TsLanguage::new(tree_sitter_ruby::LANGUAGE),
            fn_kinds: &["method", "singleton_method"],
            body_field: "body",
            stub_style: StubStyle::Ruby,
        },
        Language::CSharp => LangConfig {
            ts_language: TsLanguage::new(tree_sitter_c_sharp::LANGUAGE),
            fn_kinds: &[
                "method_declaration",
                "constructor_declaration",
                "local_function_statement",
            ],
            body_field: "body",
            stub_style: StubStyle::Braces,
        },
        Language::Php => LangConfig {
            ts_language: TsLanguage::new(tree_sitter_php::LANGUAGE_PHP),
            fn_kinds: &["function_definition", "method_declaration"],
            body_field: "body",
            stub_style: StubStyle::Braces,
        },
        // No tree-sitter grammar configured for Swift, Kotlin, Generic
        _ => return None,
    })
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Try to stub function bodies using tree-sitter.
///
/// Returns `Some(stubbed)` on success, `None` if the language has no
/// tree-sitter grammar configured or if parsing produces no edits.
pub fn try_stub_bodies(input: &str, lang: &Language) -> Option<String> {
    let config = lang_config(lang)?;

    let mut parser = Parser::new();
    parser.set_language(&config.ts_language).ok()?;

    let tree = parser.parse(input, None)?;
    let root = tree.root_node();

    let bytes = input.as_bytes();
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    collect_body_ranges(root, config.fn_kinds, config.body_field, &mut ranges);

    if ranges.is_empty() {
        return None;
    }

    Some(apply_stubs(input, bytes, &ranges, config.stub_style))
}

// ── AST traversal ─────────────────────────────────────────────────────────────

fn collect_body_ranges(
    node: tree_sitter::Node<'_>,
    fn_kinds: &[&str],
    body_field: &str,
    out: &mut Vec<(usize, usize)>,
) {
    if fn_kinds.contains(&node.kind())
        && let Some(body) = node.child_by_field_name(body_field)
    {
        // Skip abstract / interface methods that have no real body
        // (body node exists but is just a semicolon → byte length ≤ 1)
        if body.end_byte() - body.start_byte() > 1 {
            out.push((body.start_byte(), body.end_byte()));
            // Don't recurse into the body — nested fns are intentionally omitted
            return;
        }
    }

    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            collect_body_ranges(cursor.node(), fn_kinds, body_field, out);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

// ── Stub application ──────────────────────────────────────────────────────────

fn apply_stubs(src: &str, bytes: &[u8], ranges: &[(usize, usize)], style: StubStyle) -> String {
    let mut sorted = ranges.to_vec();
    sorted.sort_unstable_by_key(|r| r.0);

    let mut result = String::with_capacity(src.len());
    let mut cursor = 0usize;

    for (start, end) in sorted {
        if start < cursor {
            continue; // overlapping range — skip
        }
        result.push_str(std::str::from_utf8(&bytes[cursor..start]).unwrap_or(""));
        result.push_str(stub_replacement(style));
        cursor = end;
    }

    result.push_str(std::str::from_utf8(&bytes[cursor..]).unwrap_or(""));
    result
}

fn stub_replacement(style: StubStyle) -> &'static str {
    match style {
        StubStyle::Braces => "{ /* ... */ }",
        StubStyle::Python => ":\n    pass",
        StubStyle::Ruby => "\n  # ...",
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Rust ──────────────────────────────────────────────────────────────────

    #[test]
    fn rust_simple_function() {
        let src = "fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n";
        let out = try_stub_bodies(src, &Language::Rust).unwrap();
        assert!(
            out.contains("fn add(a: i32, b: i32) -> i32"),
            "sig preserved"
        );
        assert!(!out.contains("a + b"), "body removed");
        assert!(out.contains("/* ... */"), "stub marker present");
    }

    #[test]
    fn rust_multiline_sig() {
        let src = "pub async fn complex<T: Clone>(\n    x: T,\n    y: u32,\n) -> Option<T> {\n    Some(x)\n}\n";
        let out = try_stub_bodies(src, &Language::Rust).unwrap();
        assert!(out.contains("pub async fn complex"), "sig preserved");
        assert!(!out.contains("Some(x)"), "body removed");
    }

    #[test]
    fn rust_impl_methods() {
        let src = "impl Foo {\n    pub fn bar(&self) -> u32 {\n        42\n    }\n}\n";
        let out = try_stub_bodies(src, &Language::Rust).unwrap();
        assert!(out.contains("pub fn bar"), "method sig preserved");
        assert!(!out.contains("42"), "body removed");
    }

    // ── Go ────────────────────────────────────────────────────────────────────

    #[test]
    fn go_function() {
        let src = "package main\n\nfunc Add(a, b int) int {\n\treturn a + b\n}\n";
        let out = try_stub_bodies(src, &Language::Go).unwrap();
        assert!(out.contains("func Add(a, b int) int"), "sig preserved");
        assert!(!out.contains("return a + b"), "body removed");
    }

    #[test]
    fn go_method() {
        let src = "package main\n\ntype Rect struct{ W, H float64 }\n\nfunc (r Rect) Area() float64 {\n\treturn r.W * r.H\n}\n";
        let out = try_stub_bodies(src, &Language::Go).unwrap();
        assert!(out.contains("func (r Rect) Area()"), "method sig preserved");
        assert!(!out.contains("r.W * r.H"), "body removed");
    }

    // ── Python ────────────────────────────────────────────────────────────────

    #[test]
    fn python_function() {
        let src = "def greet(name):\n    print(f'Hello, {name}')\n    return name\n";
        let out = try_stub_bodies(src, &Language::Python).unwrap();
        assert!(out.contains("def greet(name)"), "sig preserved");
        assert!(!out.contains("print"), "body removed");
        assert!(out.contains("pass"), "python stub present");
    }

    // ── Java ──────────────────────────────────────────────────────────────────

    #[test]
    fn java_method() {
        let src =
            "class Calc {\n    public int add(int a, int b) {\n        return a + b;\n    }\n}\n";
        let out = try_stub_bodies(src, &Language::Java).unwrap();
        assert!(
            out.contains("public int add(int a, int b)"),
            "sig preserved"
        );
        assert!(!out.contains("return a + b"), "body removed");
    }

    // ── JavaScript ────────────────────────────────────────────────────────────

    #[test]
    fn javascript_function() {
        let src = "function greet(name) {\n  console.log('hi', name);\n  return name;\n}\n";
        let out = try_stub_bodies(src, &Language::JavaScript).unwrap();
        assert!(out.contains("function greet(name)"), "sig preserved");
        assert!(!out.contains("console.log"), "body removed");
    }

    #[test]
    fn javascript_class_method() {
        let src = "class Svc {\n  async process(req) {\n    return req.body;\n  }\n}\n";
        let out = try_stub_bodies(src, &Language::JavaScript).unwrap();
        assert!(out.contains("async process(req)"), "method sig preserved");
        assert!(!out.contains("return req.body"), "body removed");
    }

    // ── TypeScript ────────────────────────────────────────────────────────────

    #[test]
    fn typescript_function() {
        let src = "function identity<T>(x: T): T {\n  return x;\n}\n";
        let out = try_stub_bodies(src, &Language::TypeScript).unwrap();
        assert!(out.contains("function identity"), "sig preserved");
        assert!(!out.contains("return x"), "body removed");
    }

    // ── C / C++ ───────────────────────────────────────────────────────────────

    #[test]
    fn c_function() {
        let src = "int add(int a, int b) {\n    return a + b;\n}\n";
        let out = try_stub_bodies(src, &Language::C).unwrap();
        assert!(out.contains("int add(int a, int b)"), "sig preserved");
        assert!(!out.contains("return a + b"), "body removed");
    }

    #[test]
    fn cpp_class_method() {
        let src = "class Calc {\npublic:\n    int multiply(int a, int b) {\n        return a * b;\n    }\n};\n";
        let out = try_stub_bodies(src, &Language::C).unwrap();
        assert!(out.contains("int multiply(int a, int b)"), "sig preserved");
        assert!(!out.contains("return a * b"), "body removed");
    }

    // ── Ruby ──────────────────────────────────────────────────────────────────

    #[test]
    fn ruby_method() {
        let src = "class Calculator\n  def add(a, b)\n    a + b\n  end\nend\n";
        let out = try_stub_bodies(src, &Language::Ruby).unwrap();
        assert!(out.contains("def add(a, b)"), "sig preserved");
        assert!(!out.contains("a + b"), "body removed");
    }

    // ── C# ────────────────────────────────────────────────────────────────────

    #[test]
    fn csharp_method() {
        let src =
            "class Calc {\n    public int Add(int a, int b) {\n        return a + b;\n    }\n}\n";
        let out = try_stub_bodies(src, &Language::CSharp).unwrap();
        assert!(
            out.contains("public int Add(int a, int b)"),
            "sig preserved"
        );
        assert!(!out.contains("return a + b"), "body removed");
    }

    // ── PHP ───────────────────────────────────────────────────────────────────

    #[test]
    fn php_function() {
        let src = "<?php\nfunction add($a, $b) {\n    return $a + $b;\n}\n";
        let out = try_stub_bodies(src, &Language::Php).unwrap();
        assert!(out.contains("function add($a, $b)"), "sig preserved");
        assert!(!out.contains("return $a + $b"), "body removed");
    }

    // ── Edge cases ────────────────────────────────────────────────────────────

    #[test]
    fn unknown_language_returns_none() {
        let src = "some content";
        assert!(try_stub_bodies(src, &Language::Generic).is_none());
    }

    #[test]
    fn no_functions_returns_none() {
        // Valid Rust but no function bodies — should not panic
        let _ = try_stub_bodies("struct Foo { x: i32 }\n", &Language::Rust);
    }
}
