# Getting Started

## Install

**Homebrew (recommended)**

```bash
brew install rustkit-ai/tap/tersify
```

**cargo**

```bash
cargo install tersify
```

**Pre-built binaries**

Download from [github.com/rustkit-ai/tersify/releases](https://github.com/rustkit-ai/tersify/releases) — no Rust toolchain required.

Verify the install:

```bash
tersify --version   # tersify 0.3.0
```

---

## Your first compression

```bash
# Compress a single file → stdout
tersify src/main.rs

# Compress an entire directory
tersify src/

# Read from stdin
cat auth.rs | tersify

# Show exactly how many tokens you saved
tersify src/ --verbose
```

Example output with `--verbose`:

```
[tersify] 384 → 228 tokens  (41% saved, 156 tokens freed)
```

---

## Standard mode — what gets removed

tersify applies language-aware rules:

| Content | What's stripped |
|---|---|
| Code | Inline comments (`//`, `#`, `--`), block comments (`/* */`), consecutive blank lines |
| Code + `--strip-docs` | Also removes doc comments (`///`, `//!`, `/** */`, Python docstrings) |
| JSON | `null` fields, empty arrays `[]`, empty objects `{}` |
| Logs | Repeated identical lines → collapsed to `[×47]` on the first occurrence |
| Git diffs | Context lines — keeps only `+` / `-` lines and headers |

---

## AST mode — signatures only

Pass `--ast` to go further. tersify parses the full syntax tree with [tree-sitter](https://tree-sitter.github.io/) and replaces every function body with a stub. The result is a precise API surface — without any implementation noise.

```bash
tersify src/auth.rs --ast
```

Before (`--ast`, 384 tokens → 209 tokens, **46% saved**):

```rust
pub fn validate_token(token: &str, secret: &[u8]) -> Result<Claims> { /* ... */ }
pub fn bearer_header(token: &str) -> String { /* ... */ }
```

Use AST mode when you want an AI to understand a project's structure without reading every implementation detail.

---

## Estimate cost before sending

```bash
tersify token-cost src/

# Filter to a specific model
tersify token-cost src/ --model claude
```

Output:

```
  5 439 → 3 559 tokens  (35% saved, 1 880 tokens freed)

  Model                   Provider      $/M tokens        Raw cost    Compressed    Saved/call
  ────────────────────────────────────────────────────────────────────────────────────────────
  claude-opus-4.6         Anthropic         $15.00         $0.0816      $0.0534      -$0.0282
  claude-sonnet-4.6       Anthropic          $3.00         $0.0163      $0.0107      -$0.0056
  ...

  At 100 calls/day with claude-opus-4.6: saves $2.82/day → $84.60/month
```

---

## Track your cumulative savings

Once tersify is hooked into your editor (see [Automatic Mode](automatic-mode.md)), it records every compression:

```bash
tersify stats
```

```
tersify stats
─────────────────────────────────────
  Invocations  : 1 247
  Tokens in    : 4 821 033
  Tokens out   : 3 134 672
  Saved        : 1 686 361 (35%)
```

Reset the counter:

```bash
tersify stats-reset
```

---

## Next steps

- **[Automatic Mode](automatic-mode.md)** — make every file your AI reads automatically compressed
- **[CLI Reference](cli-reference.md)** — full list of commands and flags
- **[Languages](languages.md)** — what tersify understands
