//! # tersify
//!
//! Universal LLM context compressor. Pipe anything — code, JSON, logs, git diffs — and
//! get a token-optimized version ready to send to any language model.
//!
//! ## As a CLI
//!
//! ```bash
//! cat src/main.rs | tersify --verbose
//! git diff HEAD~1 | tersify
//! tersify install   # set up Claude Code hooks
//! tersify stats     # show cumulative savings
//! ```
//!
//! ## As a library
//!
//! ```rust
//! use tersify::{compress, detect};
//!
//! let input = r#"{"name":"foo","empty":null}"#;
//! let content_type = detect::detect(input);
//! let compressed = compress::compress(input, &content_type, None).unwrap();
//! assert_eq!(compressed, r#"{"name":"foo"}"#);
//! ```

pub mod compress;
pub mod detect;
pub mod error;
pub mod input;
pub mod tokens;
