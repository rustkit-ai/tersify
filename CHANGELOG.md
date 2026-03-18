# Changelog

All notable changes to tersify are documented here.

---

## [0.5.0] — 2026-03-18

### Added

- **GitHub Copilot integration** — `tersify install --copilot` writes
  `.github/copilot-instructions.md` in the current project directory, telling Copilot to run
  tersify before reading files. Idempotent: if the file already exists, the tersify section is
  appended. `tersify uninstall --copilot` removes it cleanly.

- **PostToolUse Bash hook** — The Claude Code hook now also fires on `Bash` tool outputs.
  When Claude runs shell commands whose output contains file content (e.g. `cat`, `grep -r`),
  the output is compressed before entering the context window.

- **PreToolUse Write/Edit hook** — Before Claude writes or edits a file, the current on-disk
  version is read, compressed, and injected as `additionalContext`. Claude gets a compact
  reference of what it's about to change without requiring a separate Read call.

- **MinHash LSH upgrade** — `--smart` deduplication upgraded from 16 to 64 hash functions
  (4× more accurate, expected error ≈ ±2.5%) with LSH banding (16 bands × 4 hashes) for
  O(1) candidate lookup instead of O(n) linear scan. Detection probability at the threshold
  (0.72 Jaccard): ~99.7%.

---

## [0.4.1] — 2026-03-18

### Fixed

- Stats cost columns now right-aligned with `{:>16}` so all rows line up regardless of dollar amount width.

---

## [0.4.0] — 2026-03-18

### Added

- **5 new languages: HTML, CSS, SQL, Shell, YAML** — Standard compression for all 5. HTML strips
  `<!-- -->` comments; CSS strips `/* */` only (preserves `//` inside `url()`); SQL strips `--` and
  `/* */` (respects single-quoted strings); Shell strips `#` comments while preserving shebangs
  (`#!/`); YAML strips full-line and inline `#` comments.

- **Precise BPE token counting** — Replaced the `÷4` heuristic with
  [tiktoken-rs](https://crates.io/crates/tiktoken-rs) `cl100k_base` (GPT-4 / Claude tokenizer).
  Token counts are now accurate to ±1%.

- **Incremental file cache** — Compressed results are cached in `~/.tersify/cache/` keyed by
  content hash + option flags. Repeated compressions of unchanged files return instantly with zero
  re-processing.

- **Custom strip rules** — Define project-specific patterns to strip in `.tersify.toml`:

  ```toml
  [strip]
  patterns = [
    'console\.log\([^)]*\);?',
    '# TODO.*',
  ]
  ```

  Or pass them ad-hoc: `tersify src/ -p 'console\.log\(.*?\)'`. CLI flags and config patterns are
  merged and deduplicated. The hook picks up `.tersify.toml` automatically.

### Removed

- **npm package** — Removed the npm wrapper entirely. Install via Homebrew, cargo, or the one-liner
  install script instead.

---

## [0.3.4] — 2026-03-17

### Added

- **`tersify stats` rewrite** — Stats now record per-language token counts, show a cost savings
  table across 4 models (claude-sonnet-4.6, claude-opus-4.6, gpt-4o, gemini-2.5-pro), and break
  down savings by language.

- **Homebrew tap** — `brew tap rustkit-ai/tap && brew install tersify`. Formula is auto-updated
  on every tagged release via GitHub Actions.

- **`tersify install --all`** — Auto-detect all present AI editors (Claude Code + Cursor + Windsurf)
  and install tersify hooks into all of them in one command.

### Changed

- Improved README with cleaner install section and benchmark tables.
- Claude Code hook migrated from legacy `hooks.json` to `settings.json` (PostToolUse).

---

## [0.3.3] — 2026-03-17

### Added

- **`.tersifyignore`** — Place a `.tersifyignore` in any directory; tersify skips matched paths
  during directory traversal. Supports `*` wildcards and relative path patterns.

- **Parallel directory compression** — `tersify src/` now uses `rayon` to process files in
  parallel across all available CPU cores.

- **Per-language stats** — `tersify stats` shows a breakdown by language alongside the cumulative
  totals.

- **`.tersify.toml` config** — Project-level config file for persistent `--ast`, `--smart`,
  `--strip-docs`, and `--budget` defaults.

- **GitHub Actions workflow** — Release CI builds binaries for
  `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-musl`,
  `aarch64-unknown-linux-musl`, and `x86_64-pc-windows-msvc`; creates a GitHub Release with
  archives + SHA256 checksums; auto-updates the Homebrew formula.

---

## [0.3.1] — 2026-03-17

### Added

- **One-liner install script** — `curl -fsSL .../install.sh | bash` downloads the right binary,
  places it in `~/.local/bin`, and runs `tersify install --all`.

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
