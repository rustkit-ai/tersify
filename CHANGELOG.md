# Changelog

All notable changes to tersify are documented here.

---

## [0.3.0] — 2026-03-17

### Added

- **Tree-sitter AST engine** — `--ast` mode is now powered by [tree-sitter](https://tree-sitter.github.io/)
  for 10 languages (Rust, Go, Python, Java, JavaScript, TypeScript, Ruby, C/C++, C#, PHP). Uses the
  exact parse tree to locate function bodies — no heuristics, no edge cases. Falls back to the
  line-based engine for unsupported languages. Saves **54% on average** across supported languages
  (up from ~45% with the previous heuristic).

- **5 new languages: C#, PHP, Ruby, Java, C/C++** — All support both standard compression and
  AST mode (`--ast`). `--type` aliases added: `csharp`/`cs`, `php`, `ruby`/`rb`, `java`, `c`/`cpp`/`c++`.

- **`tersify install --windsurf`** — Install a global Windsurf IDE rule at
  `~/.windsurf/rules/tersify.md`. `tersify uninstall --windsurf` removes it.

- **`tersify token-cost`** — Estimate LLM API cost before and after compression. Shows a formatted
  table across 10 models (Claude, GPT, Gemini, DeepSeek) with per-call and projected monthly savings.
  Accepts files, directories, or stdin. Supports `--model` to filter.

  ```bash
  tersify token-cost src/
  tersify token-cost --model claude src/main.rs
  cat large.json | tersify token-cost
  ```

- **`tersify mcp`** — MCP server (JSON-RPC 2.0 over stdio, protocol `2024-11-05`).
  Register with Claude Code in one command:

  ```bash
  claude mcp add tersify -- tersify mcp
  ```

  Exposes three tools: `compress`, `count_tokens`, `estimate_cost`.

- **`tersify bench` — AST section** — `tersify bench` now prints a second table showing
  AST mode savings for all tree-sitter-backed languages alongside the standard mode results.

### Changed

- `compress::util` module extracted — `brace_counts`, `collapse_blank_lines`, and `leading_ws`
  shared across `code.rs`, `ast.rs`, and `smart.rs` (no user-facing change).

- AST mode now uses tree-sitter instead of the heuristic line-parser for all 10 supported languages.
  Output is more accurate, especially for multi-line signatures and nested functions.

---

## [0.2.0] — 2026-03-17

### Added

- **`--ast` flag** — Extract function/method signatures only; replace all bodies with `{ /* ... */ }`.
  Supports Rust, Go, JavaScript, TypeScript, and Python. Saves ~71% on large implementation files
  when only the API surface matters.

- **`--smart` flag** — MinHash-based near-duplicate deduplication. Splits input into logical blocks
  and removes blocks whose Jaccard similarity exceeds 80%. No ML model or network call required —
  pure Rust with 16-function MinHash signatures over word 3-gram shingles.

- **`tersify bench`** — New subcommand that runs compression across embedded representative samples
  for all content types and prints a formatted token-savings table.

- **`tersify install --cursor`** — Install a global Cursor IDE rule at `~/.cursor/rules/tersify.mdc`
  that instructs Cursor to use tersify for context compression.

- **`tersify uninstall --cursor`** — Remove the Cursor IDE rule.

- **`compress::CompressOptions`** — New public struct for library users to configure AST mode,
  smart dedup, and token budget in one place. `compress()` remains unchanged for backward
  compatibility; `compress_with()` accepts the new options.

- **`input::compress_file_with`** and **`input::compress_directory_with`** — Library functions
  accepting `CompressOptions` for full pipeline control.

### Fixed

- Broken intra-doc link `TersifyError::InvalidJson` in `compress::compress` doc comment
  (caused `cargo doc` to fail in CI with `-D warnings`).

### Changed

- Version bumped to `0.2.0`.

---

## [0.1.0] — 2026-03-10

### Added

- Initial release.
- Content-type auto-detection: Code (Rust/Python/JS/TS/Go/Generic), JSON, Diff, Logs, Text.
- Language-aware comment stripping via char-level state machine.
- JSON compression: remove nulls, empty strings, empty arrays/objects.
- Git diff compression: keep only `+`/`-` lines, drop context.
- Log deduplication: normalise timestamps/UUIDs, collapse repeated lines with `[×N]`.
- Text deduplication: remove duplicate sentences.
- Token budget (`--budget`): hard-truncate with notice.
- Multi-file and directory compression with path headers.
- `tersify install` / `tersify uninstall` — Claude Code PreToolUse hook.
- `tersify stats` / `tersify stats-reset` — cumulative token savings.
- `tersify completions` — shell completions for Bash, Zsh, Fish.
- Public library API: `compress`, `detect`, `input`, `tokens`.
