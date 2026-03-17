# Supported Languages

## Overview

| Language | Standard | AST (`--ast`) | Extensions | `--type` value |
|---|:---:|:---:|---|---|
| Rust | ✓ | ✓ | `.rs` | `rust` |
| Go | ✓ | ✓ | `.go` | `go` |
| Python | ✓ | ✓ | `.py` | `python` |
| JavaScript | ✓ | ✓ | `.js` `.jsx` `.mjs` | `javascript` |
| TypeScript | ✓ | ✓ | `.ts` | `typescript` |
| TSX | ✓ | ✓ | `.tsx` | `tsx` |
| Java | ✓ | ✓ | `.java` | `java` |
| Ruby | ✓ | ✓ | `.rb` | `ruby` |
| C / C++ | ✓ | ✓ | `.c` `.cpp` `.h` `.hpp` | `c` / `cpp` |
| C# | ✓ | ✓ | `.cs` | `csharp` |
| PHP | ✓ | ✓ | `.php` | `php` |
| Swift | ✓ | — | `.swift` | `swift` |
| Kotlin | ✓ | — | `.kt` | `kotlin` |
| JSON | ✓ | — | `.json` | `json` |
| Logs | ✓ | — | `.log` | `logs` |
| Git diffs | ✓ | — | `.diff` `.patch` | `diff` |
| Plain text | ✓ | — | `.md` `.txt` | `text` |

---

## Standard mode — per language

### Code languages (Rust, Go, Python, JS, TS, TSX, Java, Ruby, C/C++, C#, PHP, Swift, Kotlin)

- Strips inline comments (`//`, `#`, `--`)
- Strips block comments (`/* */`, `"""`, `'''`)
- Collapses consecutive blank lines to one
- Optionally strips doc comments with `--strip-docs` (`///`, `//!`, `/** */`, Python docstrings)

### JSON

- Removes `null` value fields
- Removes empty arrays `[]`
- Removes empty objects `{}`

### Logs

- Groups consecutive duplicate lines: `[×47] <line>` with a count on the first occurrence
- Normalises timestamps and UUIDs before comparison so near-identical lines are caught

### Git diffs

- Keeps only `+` / `-` lines (the actual changes)
- Keeps diff headers (`@@`, `---`, `+++`)
- Removes all context lines (the `space`-prefixed lines)

---

## AST mode — how it works

AST mode is powered by [tree-sitter](https://tree-sitter.github.io/). tersify parses the full syntax tree and replaces the **body** of every function/method with a stub:

```
{ /* ... */ }        ← Rust, Go, Java, JS, TS, TSX, C/C++, C#, PHP
:
    pass             ← Python
  # ...              ← Ruby
```

**What's preserved:**
- Function signatures (name, parameters, return type, generics, visibility)
- Everything outside functions (imports, structs, classes, type definitions, constants)

**What's removed:**
- Function bodies (the implementation)
- Abstract/interface methods are skipped (body is just a semicolon)

This is ideal when you want an AI to understand a module's API without reading every implementation detail.

### AST savings benchmark

| Language | Before | After | Saved |
|---|---:|---:|---:|
| Python | 524 | 162 | **69%** |
| Java | 608 | 265 | **56%** |
| Ruby | 447 | 212 | **53%** |
| TypeScript | 528 | 265 | **50%** |
| C / C++ | 579 | 304 | **47%** |
| Rust | 384 | 209 | **46%** |
| **Total** | **3 070** | **1 417** | **54%** |

Run `tersify bench` to reproduce.

---

## Auto-detection

tersify detects content type from the file extension first, then from content heuristics (e.g., a `{` / `}` heavy file without a `.json` extension is still detected as JSON).

Force a specific type with `--type`:

```bash
tersify --type rust   my_file.txt   # treat as Rust
cat some_output | tersify --type logs
```
