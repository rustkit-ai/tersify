<div align="center">

```
 ████████╗███████╗██████╗ ███████╗██╗███████╗██╗   ██╗
 ╚══██╔══╝██╔════╝██╔══██╗██╔════╝██║██╔════╝╚██╗ ██╔╝
    ██║   █████╗  ██████╔╝███████╗██║█████╗   ╚████╔╝
    ██║   ██╔══╝  ██╔══██╗╚════██║██║██╔══╝    ╚██╔╝
    ██║   ███████╗██║  ██║███████║██║██║        ██║
    ╚═╝   ╚══════╝╚═╝  ╚═╝╚══════╝╚═╝╚═╝        ╚═╝
```

**Token compression for LLM context windows**

[![Crates.io](https://img.shields.io/crates/v/tersify)](https://crates.io/crates/tersify)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![CI](https://img.shields.io/github/actions/workflow/status/rustkit-ai/tersify/ci.yml?label=tests)](https://github.com/rustkit-ai/tersify/actions)

</div>

---

Every file you send to an LLM is **30–50% noise**: comments, blank lines, `null` JSON fields, duplicate log lines. tersify strips all of it before the token counter starts.

```
$ tersify src/ --verbose
[tersify] 5 439 → 3 559 tokens  (35% saved, 1 880 tokens freed)
```

**35% fewer tokens** in standard mode. **54% fewer** with `--ast` (signatures only).
No network calls. No configuration. Deterministic output.

---

## Install

```bash
# cargo
cargo install tersify

```

---

## How it works

### Standard mode — remove the noise

tersify applies language-aware rules to strip everything that carries no information for an LLM:

```rust
// Before — 384 tokens
// Authentication middleware for the REST API.
// Validates JWT tokens issued by our identity provider.
use anyhow::{Context, Result};

/// Claims embedded in the JWT token.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,   // subject — user id
    pub exp: usize,    // expiration timestamp
    pub roles: Vec<String>, // authorisation roles
}

// Validates a bearer token and returns the embedded claims.
// Returns an error if the token is expired, malformed, or signed with the wrong key.
pub fn validate_token(token: &str, secret: &[u8]) -> Result<Claims> {
    // Decode the header first to get the algorithm
    let header = decode_header(token)
        .context("failed to decode JWT header")?;

    // Build a validation config matching the issuer requirements
    let mut validation = Validation::new(header.alg);
    validation.validate_exp = true; // always enforce expiry

    let key = DecodingKey::from_secret(secret);

    let data = decode::<Claims>(token, &key, &validation)
        .context("JWT validation failed")?;

    // Extra check: ensure the subject is non-empty
    if data.claims.sub.is_empty() {
        anyhow::bail!("token subject is empty");
    }

    Ok(data.claims)
}
```

```rust
// After — 228 tokens  (41% saved)
use anyhow::{Context, Result};

/// Claims embedded in the JWT token.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub roles: Vec<String>,
}

pub fn validate_token(token: &str, secret: &[u8]) -> Result<Claims> {
    let header = decode_header(token)
        .context("failed to decode JWT header")?;
    let mut validation = Validation::new(header.alg);
    validation.validate_exp = true;
    let key = DecodingKey::from_secret(secret);
    let data = decode::<Claims>(token, &key, &validation)
        .context("JWT validation failed")?;
    if data.claims.sub.is_empty() {
        anyhow::bail!("token subject is empty");
    }
    Ok(data.claims)
}
```

### AST mode — signatures only

Pass `--ast` to go further: powered by [tree-sitter](https://tree-sitter.github.io/), tersify parses the full syntax tree and replaces every function body with a stub. The result is a precise API surface.

```rust
// tersify src/auth.rs --ast  →  209 tokens  (46% saved)
use anyhow::{Context, Result};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub roles: Vec<String>,
}

pub fn validate_token(token: &str, secret: &[u8]) -> Result<Claims> { /* ... */ }
pub fn bearer_header(token: &str) -> String { /* ... */ }
```

Use AST mode when you want Claude to understand a project's structure without reading every implementation.

---

## Quick start

```bash
tersify src/main.rs            # single file → stdout
tersify src/                   # entire directory
cat file.rs | tersify          # stdin
tersify src/ --verbose         # show token savings
tersify src/ --ast             # signatures only
tersify src/ --strip-docs      # also remove doc comments (///, /** */)
tersify token-cost src/        # estimate API cost
```

---

## Integrate with your AI editor

### Claude Code

```bash
tersify install
```

One command. Every file Claude reads is now automatically compressed before it enters the context window — no workflow changes required.

```bash
tersify uninstall              # remove the hook
```

### Cursor

```bash
tersify install --cursor
```

### Windsurf

```bash
tersify install --windsurf
```

---

## Benchmark

Real numbers — run `tersify bench` to reproduce locally.

### Standard mode

| Content type | Before | After | Saved |
|---|---:|---:|---:|
| Rust | 384 | 228 | **41%** |
| Python | 524 | 289 | **45%** |
| TypeScript | 528 | 369 | **30%** |
| Ruby | 447 | 285 | **36%** |
| Java | 608 | 435 | **28%** |
| C / C++ | 579 | 342 | **41%** |
| Swift | 699 | 597 | **15%** |
| Kotlin | 604 | 336 | **44%** |
| JSON | 181 | 103 | **43%** |
| Git diff | 275 | 213 | **23%** |
| Logs | 340 | 173 | **49%** |
| Plain text | 270 | 189 | **30%** |
| **Total** | **5 439** | **3 559** | **35%** |

### AST mode (`--ast`)

| Language | Before | After | Saved |
|---|---:|---:|---:|
| Python | 524 | 162 | **69%** |
| Java | 608 | 265 | **56%** |
| Ruby | 447 | 212 | **53%** |
| TypeScript | 528 | 265 | **50%** |
| C / C++ | 579 | 304 | **47%** |
| Rust | 384 | 209 | **46%** |
| **Total** | **3 070** | **1 417** | **54%** |

---

## What gets removed

| Content | What's stripped |
|---|---|
| **Code** | Inline and block comments, consecutive blank lines |
| **Code + `--strip-docs`** | Also removes `///`, `//!`, `/** */`, Python docstrings |
| **JSON** | `null` fields, empty `[]` and `{}` |
| **Logs** | Repeated identical lines → `[×47]` on the first |
| **Diffs** | Context lines — keeps only `+` / `-` and headers |
| **AST** | Function bodies → `{ /* ... */ }` (exact parse tree, tree-sitter) |

Doc comments (`///`, `//!`, `/** */`) are preserved by default — pass `--strip-docs` to remove them too.

---

## Supported languages

| Language | Standard | AST | Extensions |
|---|:---:|:---:|---|
| Rust | ✓ | ✓ | `.rs` |
| Python | ✓ | ✓ | `.py` |
| TypeScript | ✓ | ✓ | `.ts` |
| TSX | ✓ | ✓ | `.tsx` |
| JavaScript | ✓ | ✓ | `.js` `.jsx` `.mjs` |
| Go | ✓ | ✓ | `.go` |
| Java | ✓ | ✓ | `.java` |
| Ruby | ✓ | ✓ | `.rb` |
| C / C++ | ✓ | ✓ | `.c` `.cpp` `.h` `.hpp` |
| C# | ✓ | ✓ | `.cs` |
| PHP | ✓ | ✓ | `.php` |
| Swift | ✓ | — | `.swift` |
| Kotlin | ✓ | — | `.kt` |
| JSON | ✓ | — | `.json` |
| Logs | ✓ | — | `.log` |
| Git diffs | ✓ | — | `.diff` `.patch` |

---

## Cost estimator

```bash
tersify token-cost src/
tersify token-cost src/ --model claude
```

```
  5 439 → 3 559 tokens  (35% saved, 1 880 tokens freed)

  Model                   Provider      $/M tokens        Raw cost    Compressed    Saved/call
  ────────────────────────────────────────────────────────────────────────────────────────────
  claude-opus-4.6         Anthropic         $15.00         $0.0816      $0.0534      -$0.0282
  claude-sonnet-4.6       Anthropic          $3.00         $0.0163      $0.0107      -$0.0056
  claude-haiku-4.5        Anthropic          $0.80         $0.0044      $0.0028      -$0.0014
  gpt-4o                  OpenAI             $5.00         $0.0272      $0.0178      -$0.0094
  gemini-2.5-pro          Google             $1.25         $0.0068      $0.0044      -$0.0023
  deepseek-v3             DeepSeek           $0.27         $0.0015      $0.0010      -$0.0005
  ────────────────────────────────────────────────────────────────────────────────────────────

  At 100 calls/day with claude-opus-4.6: saves $2.82/day → $84.60/month
```

---

## MCP server

tersify ships a built-in [MCP](https://modelcontextprotocol.io/) server for agent pipelines.

```bash
# Add to Claude Code
claude mcp add tersify -- tersify mcp
```

Three tools exposed: `compress`, `count_tokens`, `estimate_cost`.

---

## Use as a library

```toml
[dependencies]
tersify = "0.3"
```

```rust
use tersify::{compress, detect};
use std::path::Path;

// Auto-detect and compress
let src = std::fs::read_to_string("src/main.rs")?;
let ct  = detect::detect_for_path(Path::new("src/main.rs"), &src);
let out = compress::compress(&src, &ct, None)?;

// AST mode
use tersify::compress::{compress_with, CompressOptions};
let out = compress_with(&src, &ct, &CompressOptions { ast: true, ..Default::default() })?;
```

---

## CLI reference

```
tersify [FILES|DIRS]           Compress to stdout
  --ast                        Signatures only (tree-sitter)
  --strip-docs                 Also remove doc comments (///, //!, /** */)
  --type <lang>                Force content type
  --verbose                    Show token savings

tersify bench                  Benchmark all content types
tersify token-cost [FILES]     Estimate LLM API cost
  --model <filter>             Filter models by name

tersify mcp                    Start MCP server

tersify install                Hook into Claude Code
tersify install --cursor       Hook into Cursor
tersify install --windsurf     Hook into Windsurf
tersify uninstall              Remove the hook

tersify completions <shell>    Generate shell completions
```

`--type` values: `rust` `python` `javascript` `typescript` `tsx` `go` `ruby` `java` `c` `cpp` `csharp` `php` `swift` `kotlin` `json` `logs` `diff` `text`

---

## Development

```bash
git clone https://github.com/rustkit-ai/tersify
cd tersify
cargo test           # 90 tests
cargo run -- bench   # live benchmark
```

---

<div align="center">

MIT — made by [rustkit-ai](https://github.com/rustkit-ai)

[Contributing](CONTRIBUTING.md)

</div>
