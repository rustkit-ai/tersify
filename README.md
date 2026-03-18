<div align="center">

# tersify

**Strip the noise from any file before it hits your LLM context window.**

[![Crates.io](https://img.shields.io/crates/v/tersify)](https://crates.io/crates/tersify)
[![CI](https://img.shields.io/github/actions/workflow/status/rustkit-ai/tersify/ci.yml?label=tests)](https://github.com/rustkit-ai/tersify/actions)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

</div>

---

Every file you send to Claude or GPT is **30–50% noise**: comments, blank lines, `null` JSON fields, repeated log lines. tersify removes all of it — automatically, in milliseconds, with zero configuration.

```
$ tersify src/ --verbose
[tersify] 5,439 → 3,559 tokens  (35% saved)
```

Works as a **silent automatic hook** in Claude Code (fires on every file read), and as an **AI-guided rule** in Cursor and Windsurf.

---

## Install

**Homebrew**
```bash
brew tap rustkit-ai/tap
brew install tersify
```

**One-liner** (macOS / Linux)
```bash
curl -fsSL https://raw.githubusercontent.com/rustkit-ai/tersify/main/install.sh | bash
```

**Cargo**
```bash
cargo install tersify
tersify install --all
```

Both methods end with `tersify install --all` — auto-detects Claude Code, Cursor, and Windsurf and hooks into all of them.

---

## What it does

```rust
// BEFORE — 384 tokens
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
// Returns an error if the token is expired or malformed.
pub fn validate_token(token: &str, secret: &[u8]) -> Result<Claims> {
    // Decode the header first to get the algorithm
    let header = decode_header(token)
        .context("failed to decode JWT header")?;
    // Build a validation config matching issuer requirements
    let mut validation = Validation::new(header.alg);
    validation.validate_exp = true; // always enforce expiry
    let key = DecodingKey::from_secret(secret);
    let data = decode::<Claims>(token, &key, &validation)
        .context("JWT validation failed")?;
    if data.claims.sub.is_empty() {
        anyhow::bail!("token subject is empty");
    }
    Ok(data.claims)
}
```

```rust
// AFTER — 228 tokens  ↓ 41% smaller
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

All logic is preserved. Only noise is removed.

---

## Automatic mode — Claude Code

After `tersify install`, every file Claude reads is silently compressed before it enters the context window. Nothing changes in your workflow.

```bash
tersify install          # hook into Claude Code
tersify stats            # see what you've saved
```

```
  tersify — token savings
  ─────────────────────────────────────────
  Compressions : 1,247
  Tokens in    : 4,821,334
  Tokens out   : 3,094,452
  Tokens saved : 1,726,882  (36% smaller)

  Cost saved (what you didn't pay for):
    claude-sonnet-4.6      $3.00/M   → $5.18 saved
    claude-opus-4.6        $15.00/M  → $25.90 saved
    gpt-4o                 $5.00/M   → $8.63 saved
    gemini-2.5-pro         $1.25/M   → $2.16 saved

  By language:
    rust             2,841,012 → 1,738,014  (39%)   $3.31 saved
    typescript         498,234 →   348,764  (30%)   $0.45 saved
    python             391,018 →   215,512  (45%)   $0.53 saved
    json               284,912 →   169,004  (41%)   $0.35 saved
```

---

## AST mode — signatures only

Pass `--ast` to go further: tersify uses [tree-sitter](https://tree-sitter.github.io/) to parse the full syntax tree and stub every function body. The output is a precise API surface.

```rust
// tersify --ast src/auth.rs  →  209 tokens  ↓ 46% vs standard, ↓ 54% total

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims { pub sub: String, pub exp: usize, pub roles: Vec<String> }

pub fn validate_token(token: &str, secret: &[u8]) -> Result<Claims> { /* ... */ }
pub fn bearer_header(token: &str) -> String { /* ... */ }
pub fn refresh_token(claims: &Claims, secret: &[u8]) -> Result<String> { /* ... */ }
```

Use `--ast` when you want Claude to understand a project's shape without reading every implementation.

---

## Quick start

```bash
tersify src/main.rs            # single file → stdout
tersify src/                   # entire directory (parallel, cached)
cat file.rs | tersify          # pipe stdin
git diff | tersify             # compress diffs

tersify src/ --verbose         # show token savings
tersify src/ --ast             # signatures only
tersify src/ --strip-docs      # also remove doc comments (///, /** */)
tersify src/ --budget 4000     # hard limit: truncate to 4 000 tokens

tersify token-cost src/        # estimate LLM API cost before vs after
tersify bench                  # benchmark all content types
tersify stats                  # cumulative savings since install
```

---

## Editors

| Editor | Integration | How it works |
|---|---|---|
| **Claude Code** | Automatic hook | Compresses every file read silently via PostToolUse hook |
| **Cursor** | AI-guided rule | Cursor's AI uses tersify before reading files into context |
| **Windsurf** | AI-guided rule | Windsurf's AI uses tersify before reading files into context |

```bash
tersify install                # Claude Code
tersify install --cursor       # Cursor
tersify install --windsurf     # Windsurf
tersify install --all          # all detected editors at once

tersify uninstall --all        # remove all hooks
```

---

## Benchmarks

Run `tersify bench` to reproduce locally.

**Standard mode**

| Content type | Before | After | Saved |
|---|---:|---:|---:|
| Rust | 384 | 228 | **41%** |
| Python | 524 | 289 | **45%** |
| TypeScript | 528 | 369 | **30%** |
| Ruby | 447 | 285 | **36%** |
| Java | 608 | 435 | **28%** |
| C / C++ | 579 | 342 | **41%** |
| Kotlin | 604 | 336 | **44%** |
| JSON | 181 | 103 | **43%** |
| Git diff | 275 | 213 | **23%** |
| Logs | 340 | 173 | **49%** |
| **Total** | **5,439** | **3,559** | **35%** |

**AST mode (`--ast`)**

| Language | Before | After | Saved |
|---|---:|---:|---:|
| Python | 524 | 162 | **69%** |
| Java | 608 | 265 | **56%** |
| Ruby | 447 | 212 | **53%** |
| TypeScript | 528 | 265 | **50%** |
| C / C++ | 579 | 304 | **47%** |
| Rust | 384 | 209 | **46%** |
| **Total** | **3,070** | **1,417** | **54%** |

---

## Supported languages

| Language | Standard | AST | Extensions |
|---|:---:|:---:|---|
| Rust | ✓ | ✓ | `.rs` |
| Python | ✓ | ✓ | `.py` |
| TypeScript / TSX | ✓ | ✓ | `.ts` `.tsx` |
| JavaScript | ✓ | ✓ | `.js` `.jsx` `.mjs` |
| Go | ✓ | ✓ | `.go` |
| Java | ✓ | ✓ | `.java` |
| Ruby | ✓ | ✓ | `.rb` |
| C / C++ | ✓ | ✓ | `.c` `.cpp` `.h` `.hpp` |
| C# | ✓ | ✓ | `.cs` |
| PHP | ✓ | ✓ | `.php` |
| Swift | ✓ | — | `.swift` |
| Kotlin | ✓ | — | `.kt` |
| HTML | ✓ | — | `.html` `.htm` |
| CSS | ✓ | — | `.css` |
| SQL | ✓ | — | `.sql` |
| Shell | ✓ | — | `.sh` `.bash` |
| YAML | ✓ | — | `.yaml` `.yml` |
| JSON / JSONC | ✓ | — | `.json` |
| Logs | ✓ | — | `.log` |
| Git diffs | ✓ | — | `.diff` `.patch` |

---

## What gets removed

| Content | Stripped |
|---|---|
| **Code** | Comments, consecutive blank lines |
| **+ `--strip-docs`** | Also `///`, `//!`, `/** */`, Python docstrings |
| **JSON** | `null` fields, empty `[]` and `{}` |
| **Logs** | Repeated lines → first occurrence + `[×N]` count |
| **Diffs** | Context lines — keeps only `+` / `-` and file headers |
| **`--ast`** | Function bodies → `{ /* ... */ }` (full syntax tree parse) |

---

## Token cost estimator

```bash
tersify token-cost src/
tersify token-cost src/ --model claude-sonnet
```

```
  5,439 → 3,559 tokens  (35% saved, 1,880 tokens freed)

  Model                  Provider     $/M tokens     Raw cost   Compressed   Saved/call
  ─────────────────────────────────────────────────────────────────────────────────────
  claude-opus-4.6        Anthropic       $15.00       $0.0816      $0.0534      -$0.0282
  claude-sonnet-4.6      Anthropic        $3.00       $0.0163      $0.0107      -$0.0056
  gpt-4o                 OpenAI           $5.00       $0.0272      $0.0178      -$0.0094
  gemini-2.5-pro         Google           $1.25       $0.0068      $0.0044      -$0.0023
  ─────────────────────────────────────────────────────────────────────────────────────

  At 100 calls/day with claude-opus-4.6: saves $2.82/day → $84.60/month
```

---

## Use as a library

```toml
[dependencies]
tersify = "0.4"
```

```rust
use tersify::{compress::{compress_with, CompressOptions}, detect};
use std::path::Path;

let src = std::fs::read_to_string("src/main.rs")?;
let ct  = detect::detect_for_path(Path::new("src/main.rs"), &src);

// Standard compression
let out = compress_with(&src, &ct, &CompressOptions::default())?;

// AST mode — signatures only
let out = compress_with(&src, &ct, &CompressOptions {
    ast: true,
    ..Default::default()
})?;
```

---

## Custom strip rules

Create `.tersify.toml` at the root of your project to strip project-specific noise — debug logs, TODO comments, test scaffolding:

```toml
[strip]
patterns = [
  'console\.log\([^)]*\);?',   # JS/TS debug logs
  '\bdebugger;',                # JS debugger statements
  'print\(f?"[Dd]ebug.*"\)',   # Python debug prints
  '# TODO.*',                  # TODO comments
  '// FIXME.*',                # FIXME comments
]
```

Patterns use [regex-lite](https://docs.rs/regex-lite) syntax. Each match is removed inline — lines that become empty are dropped entirely.

One-off via CLI flag (no config needed):
```bash
tersify src/ --pattern 'console\.log\([^)]*\);?'
tersify src/ -p '\bdebugger;' -p '# TODO.*'
```

The hook picks up `.tersify.toml` automatically — patterns apply on every file Claude reads.

---

## MCP server

tersify ships a built-in [MCP](https://modelcontextprotocol.io/) server for agent pipelines.

```bash
claude mcp add tersify -- tersify mcp
```

Tools: `compress`, `count_tokens`, `estimate_cost`.

---

## CLI reference

```
tersify [FILES|DIRS]           Compress to stdout (stdin if omitted)
  -t, --type <lang>            Force content type
  -b, --budget <N>             Hard token limit — truncate at N tokens
  -v, --verbose                Print token savings to stderr
  -a, --ast                    Signatures only (tree-sitter)
  -s, --smart                  Semantic deduplication (MinHash)
      --strip-docs             Remove doc comments too (///, /** */)
  -p, --pattern <REGEX>        Strip matching text (repeatable)

tersify install [--cursor|--windsurf|--all]    Hook into AI editors
tersify uninstall [--cursor|--windsurf|--all]  Remove hooks
tersify stats                  Show cumulative token savings
tersify stats-reset            Reset stats
tersify bench                  Benchmark all content types
tersify token-cost [FILES]     Estimate LLM API cost
  -m, --model <filter>         Filter models by name
tersify mcp                    Start MCP server (stdio)
tersify completions <shell>    Shell completions (bash|zsh|fish)
```

`--type` values: `rust` `python` `javascript` `typescript` `tsx` `go` `ruby` `java` `c` `cpp` `csharp` `php` `swift` `kotlin` `html` `css` `sql` `shell` `yaml` `json` `logs` `diff` `text`

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

MIT — [rustkit-ai](https://github.com/rustkit-ai) · [Contributing](CONTRIBUTING.md)

</div>
