//! # tersify
//!
//! Universal LLM context compressor. Pipe anything — code, JSON, logs, git diffs — and
//! get a token-optimized version ready to send to any language model.
//!
//! ## As a CLI
//!
//! ```bash
//! cat src/main.rs | tersify --verbose
//! cat src/main.rs | tersify --ast          # signatures only, bodies stubbed
//! git diff HEAD~1 | tersify
//! tersify src/                             # entire directory
//! tersify install                          # set up Claude Code hooks
//! tersify install --cursor                 # set up Cursor IDE rule
//! tersify bench                            # benchmark savings across content types
//! tersify stats                            # show cumulative savings
//! ```
//!
//! ## As a library
//!
//! ```rust
//! use tersify::{compress::{self, CompressOptions}, detect};
//!
//! // Basic compression
//! let input = r#"{"name":"foo","empty":null}"#;
//! let content_type = detect::detect(input);
//! let compressed = compress::compress(input, &content_type, None).unwrap();
//! assert_eq!(compressed, r#"{"name":"foo"}"#);
//!
//! // AST mode: extract function signatures (use detect_for_path for Code type)
//! let src = "fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n";
//! let ct = detect::detect_for_path(std::path::Path::new("add.rs"), src);
//! let opts = CompressOptions { ast: true, ..Default::default() };
//! let out = compress::compress_with(src, &ct, &opts).unwrap();
//! assert!(out.contains("fn add(a: i32, b: i32) -> i32"));
//! assert!(!out.contains("a + b"));
//! ```

pub mod cache;
pub mod compress;
pub mod detect;
pub mod error;
pub mod input;
pub mod tokens;
