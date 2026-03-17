# tersify — Documentation

**Token compression for LLM context windows.** No network calls. No configuration. Deterministic output.

---

## Navigation

| Document | Description |
|---|---|
| [Getting Started](getting-started.md) | Install tersify in 60 seconds |
| [Automatic Mode](automatic-mode.md) | **Zero-effort integration** — hook into Claude Code, Cursor, Windsurf |
| [CLI Reference](cli-reference.md) | Every command, flag, and option |
| [Languages](languages.md) | All 16 supported content types |
| [Library](library.md) | Use tersify as a Rust crate |
| [MCP Server](mcp-server.md) | Expose tersify tools to any AI agent |

---

## Why tersify?

Every file you send to an LLM is **30–50% noise**: comments, blank lines, `null` JSON fields, duplicate log lines. tersify strips all of it before the token counter starts.

```
$ tersify src/ --verbose
[tersify] 5 439 → 3 559 tokens  (35% saved, 1 880 tokens freed)
```

**Two modes:**

- **Standard** — strips comments, blank lines, JSON nulls, duplicate logs. Saves ~35%.
- **AST (`--ast`)** — powered by tree-sitter: replaces every function body with `{ /* ... */ }`. Saves ~54%. Perfect for understanding a codebase's API surface.

---

## Quick numbers

| Mode | Before | After | Saved |
|---|---:|---:|---:|
| Standard (all types) | 5 439 | 3 559 | **35%** |
| AST (code only) | 3 070 | 1 417 | **54%** |

Run `tersify bench` to reproduce locally.
