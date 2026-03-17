# Contributing to tersify

Thanks for taking the time to contribute.

## Ways to contribute

- **Bug reports** — open an issue with a minimal reproducer
- **Feature requests** — open an issue describing the use-case before writing code
- **Pull requests** — welcome for bug fixes, new language support, and performance improvements

## Development setup

```bash
git clone https://github.com/rustkit-ai/tersify
cd tersify
cargo build
cargo test
```

Run the benchmark to make sure you haven't regressed compression ratios:

```bash
cargo run -- bench
```

## Adding a language

tersify has two compression layers. When adding a language, implement both:

### 1. Detection (`src/detect.rs`)

Add a `Language::YourLang` variant, map file extensions in `detect_for_path()`, add a
`--type yourlang` alias in `ContentType::from_str()`, and optionally add content-based
heuristics in `detect_language()`.

### 2. Comment stripping (`src/compress/code.rs`)

If the language uses `// ...` and `/* ... */` comments, `strip_cstyle()` will work out of the
box — just add the variant to the `_ =>` fallback branch. For languages with different comment
syntax (Python `#`, Ruby `# / =begin/=end`), add a dedicated `strip_X()` function and dispatch
from `compress()`.

### 3. AST mode (`src/compress/ast_ts.rs`)

If a tree-sitter grammar exists for the language:

1. Add the crate to `Cargo.toml` (`tree-sitter-X = "..."`)
2. Add a `Language::YourLang` arm to `lang_config()` with the correct `fn_kinds` and `body_field`
3. Run `cargo test` — the `no_functions_returns_none` and `unknown_language_returns_none` tests
   serve as regression guards

Check [crates.io](https://crates.io/search?q=tree-sitter-) for available grammar crates.
ABI version 15 is required — use `tree-sitter = "0.25"` as the core dependency.

## Tests

Every change needs tests. For a new language, add at least:

- A `compress::code` test for comment stripping
- A `compress::ast_ts` test for AST body stubbing (if tree-sitter is added)
- A `detect` test for extension-based detection

Run the full suite before opening a PR:

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

## Commit style

Plain English, imperative mood: `add PHP AST support`, `fix fmt_tokens zero-padding`.
No conventional commits required.

## Pull request checklist

- [ ] `cargo test` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] New behaviour is covered by tests
- [ ] CHANGELOG.md updated under `## [Unreleased]`

## Questions

Open an issue or start a discussion on GitHub.
