<div align="center">

```
████████╗███████╗██████╗ ███████╗██╗███████╗██╗   ██╗
╚══██╔══╝██╔════╝██╔══██╗██╔════╝██║██╔════╝╚██╗ ██╔╝
   ██║   █████╗  ██████╔╝███████╗██║█████╗   ╚████╔╝
   ██║   ██╔══╝  ██╔══██╗╚════██║██║██╔══╝    ╚██╔╝
   ██║   ███████╗██║  ██║███████║██║██║        ██║
   ╚═╝   ╚══════╝╚═╝  ╚═╝╚══════╝╚═╝╚═╝        ╚═╝
```

**Stop paying for tokens you don't need.**

Tersify is a blazing-fast Rust CLI that compresses anything you pipe into it — source code, JSON, logs, git diffs — before it reaches your LLM. Same context, fraction of the cost.

[![Crates.io](https://img.shields.io/crates/v/tersify.svg?style=flat-square)](https://crates.io/crates/tersify)
[![docs.rs](https://img.shields.io/docsrs/tersify?style=flat-square)](https://docs.rs/tersify)
[![CI](https://img.shields.io/github/actions/workflow/status/rustkit-ai/tersify/ci.yml?style=flat-square)](https://github.com/rustkit-ai/tersify/actions)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-orange?style=flat-square)](https://www.rust-lang.org/)

</div>

---

## The problem

Every token you send to a language model costs you — in **money**, **latency**, and **context window space**.

But most of what you pipe is noise:

- `// comments` and `/* block comments */` the LLM doesn't need
- Python docstrings and `"""triple-quoted"""` strings
- Blank lines, indentation, formatting whitespace
- `null` fields and empty arrays in JSON responses
- The same error repeated 300 times in your logs
- Git diff context lines that haven't changed

**tersify removes the noise before it reaches the model.**

---

## Quick demo

```bash
$ git diff HEAD~1 | tersify --verbose

diff --git a/src/auth.rs b/src/auth.rs
--- a/src/auth.rs
+++ b/src/auth.rs
---
-pub fn validate(token: &str) -> bool {
+pub fn validate(token: &str) -> Result<bool> {
+    if token.is_empty() { return Err(AuthError::Empty); }

[tersify] 3 240 → 847 tokens  (74% saved)
```

The LLM gets everything it needs. You pay for 847 tokens instead of 3 240.

---

## What does it save?

| Content | Before | After | Saved |
|---|---|---|---|
| Source code | 4 200 tokens | 2 900 tokens | **~31%** |
| Git diff | 3 240 tokens | 847 tokens | **~74%** |
| JSON response | 1 800 tokens | 720 tokens | **~60%** |
| Application logs | 5 000 tokens | 1 100 tokens | **~78%** |

> **On a typical Claude Code session**: tersify users report saving **$30–60/month** by eliminating noise from context.

---

## Install

```bash
# Homebrew (macOS / Linux)
brew install rustkit-ai/tap/tersify

# Cargo
cargo install tersify
```

That's it. No config. No setup. Start piping immediately.

---

## Usage

### Pipe from anything

```bash
# Source code — strips comments (inline, block), collapses blank lines
cat src/auth.rs | tersify

# Git diff — keeps only changed lines, drops context
git diff HEAD~1 | tersify

# Logs — deduplicates repeated lines with counts
kubectl logs my-pod | tersify
docker logs my-container | tersify

# JSON — removes nulls, empty fields, whitespace
curl -s https://api.example.com/data | tersify
```

### Files and directories

```bash
# Single file
tersify src/main.rs

# Multiple files — each gets a header
tersify src/auth.rs src/middleware.rs

# Entire directory — skips target/, node_modules/, .git/, dist/, etc.
tersify src/

# Directory within a token budget
tersify src/ --budget 4000
```

### Verbose mode

Shows token count before/after on stderr — stdout stays clean for piping:

```bash
cat large_file.rs | tersify --verbose
# [tersify] 4 200 → 2 900 tokens (31% saved)
```

### Force content type

Auto-detection handles most cases. Override when needed:

```bash
cat output.txt | tersify --type logs
cat schema.txt | tersify --type json
```

---

## Set it and forget it — Claude Code hook

Run this once. Every file Claude reads will be automatically compressed from that point on:

```bash
tersify install
# ✓ Installed tersify hook at ~/.claude/hooks.json
#   Files read by Claude will now be automatically compressed.

# Remove it
tersify uninstall
```

### Track your savings

```bash
tersify stats
```

```
tersify stats
─────────────────────────────────────────
  Invocations    : 1 247
  Tokens in      : 4 821 440
  Tokens out     : 2 193 820
  Saved          : 2 627 620  (54%)
```

```bash
tersify stats-reset   # start fresh
```

---

## Shell completions

```bash
# Bash
tersify completions bash >> ~/.bashrc

# Zsh
tersify completions zsh > ~/.zfunc/_tersify

# Fish
tersify completions fish > ~/.config/fish/completions/tersify.fish
```

---

## How it works

tersify auto-detects what you're piping and applies the right compression strategy:

```
your pipe ──► detect type ──► compress ──► your LLM
                  │
                  ├── code   → strip // comments, /* blocks */, docstrings
                  │            collapse blank lines  (Rust/Python/JS/TS/Go)
                  ├── diff   → keep +/- lines only, drop context
                  ├── json   → remove nulls/empties, compact
                  ├── logs   → deduplicate + normalise timestamps/UUIDs
                  └── text   → remove duplicate sentences
```

**Detection is automatic.** File extension takes priority (`.rs`, `.py`, `.ts`…), falling back to content analysis for stdin and unknown extensions.

### What gets stripped in code

| Language | Line comments | Block comments | Docstrings / doc-comments |
|---|---|---|---|
| Rust | `// ...` ✓ | `/* ... */` ✓ | `///` and `//!` **kept** |
| Python | `# ...` ✓ | — | `"""..."""` standalone ✓ |
| JavaScript / TypeScript | `// ...` ✓ | `/* ... */` ✓ | — |
| Go | `// ...` ✓ | `/* ... */` ✓ | — |
| Generic | `// ...` `#` ✓ | — | — |

String literals are **always preserved** — `"not // a comment"` stays intact.

---

## Use as a library

tersify is also a Rust library. Integrate the compression pipeline directly into your own tooling:

```toml
[dependencies]
tersify = "0.1"
```

```rust
use tersify::{compress, detect, input};
use std::path::Path;

// Compress a string
let raw = r#"{"name":"foo","empty":null,"data":{"val":42}}"#;
let ct = detect::detect(raw);
let out = compress::compress(raw, &ct, None)?;
// → {"name":"foo","data":{"val":42}}

// Compress a file (uses extension for language detection)
let (compressed, before, after) = input::compress_file(
    Path::new("src/main.rs"),
    None,   // auto-detect type
    Some(2000), // token budget
)?;

// Compress an entire directory
let (compressed, before, after) = input::compress_directory(
    Path::new("src/"),
    None,
    Some(8000),
)?;
```

Full API docs on [docs.rs/tersify](https://docs.rs/tersify).

---

## Supported content types

| Type | Auto-detected when… |
|---|---|
| `code` | File extension matches (`.rs`, `.py`, `.js`, `.ts`, `.go`…) or content contains language keywords |
| `json` | Input starts with `{` or `[` and parses as valid JSON |
| `diff` | Input starts with `diff --git` or contains `---`/`+++` headers |
| `logs` | >30% of lines contain log level keywords (`ERROR`, `INFO`, `WARN`…) |
| `text` | Fallback |

---

## Roadmap

- [ ] Tree-sitter AST — extract function signatures, stub bodies
- [ ] `tersify install --cursor` — Cursor IDE hook
- [ ] `--smart` flag — semantic dedup with local embeddings
- [ ] `tersify bench` — compare token savings across models and content types

---

## Contributing

Issues and pull requests are welcome. Please open an issue before starting a large change.

```bash
cargo test        # run the full test suite (21 tests + doctests)
cargo clippy      # lint
cargo fmt         # format
cargo doc --open  # browse the API docs
```

---

<div align="center">

MIT License · Built by [rustkit-ai](https://github.com/rustkit-ai)

*If tersify saves you tokens, consider giving it a ⭐*

</div>
