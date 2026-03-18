# CLI Reference

## Compression

```
tersify [FILES|DIRS|STDIN]
```

Compress one or more files or directories to stdout.

| Flag | Description |
|---|---|
| `--ast` | Signatures only — replace function bodies with `{ /* ... */ }` using tree-sitter |
| `--strip-docs` | Also remove doc comments (`///`, `//!`, `/** */`, Python docstrings) |
| `--type <lang>` | Force content type (see values below) |
| `--verbose` | Print token savings to stderr |
| `--budget <N>` | Hard-truncate output at N tokens |

**`--type` values:**
`rust` `python` `javascript` `typescript` `tsx` `go` `ruby` `java` `c` `cpp` `csharp` `php` `swift` `kotlin` `json` `logs` `diff` `text`

**Examples:**

```bash
tersify src/main.rs                        # single file
tersify src/                               # directory (recursive)
cat file.rs | tersify                      # stdin
tersify src/ --verbose                     # show savings
tersify src/ --ast                         # signatures only
tersify src/ --strip-docs                  # strip doc comments too
tersify src/ --type rust                   # force type
tersify src/ --budget 2000                 # cap at 2000 tokens
```

---

## Install / Uninstall

```bash
tersify install --all            # auto-detect and hook into all present editors
tersify install                  # hook into Claude Code only
tersify install --cursor         # hook into Cursor only
tersify install --windsurf       # hook into Windsurf only

tersify uninstall --all          # remove hooks from all detected editors
tersify uninstall                # remove Claude Code hook
tersify uninstall --cursor       # remove Cursor rule
tersify uninstall --windsurf     # remove Windsurf rule
```

See [Automatic Mode](automatic-mode.md) for details.

---

## Stats

```bash
tersify stats               # show cumulative token savings since install
tersify stats-reset         # reset the counter
```

Stats are stored in `~/.tersify/stats.json`.

---

## Token cost estimator

```bash
tersify token-cost [FILES|DIRS]
tersify token-cost src/ --model claude     # filter models by name
cat large.json | tersify token-cost        # from stdin
```

Shows a table of before/after cost for 10 models (Claude, GPT, Gemini, DeepSeek).

---

## Benchmark

```bash
tersify bench
```

Runs compression on embedded representative samples for all content types and prints token savings tables — both standard and AST mode.

---

## MCP Server

```bash
tersify mcp
```

Starts a JSON-RPC 2.0 MCP server on stdio. Register with Claude Code:

```bash
claude mcp add tersify -- tersify mcp
```

See [MCP Server](mcp-server.md) for the tool schema.

---

## Shell completions

```bash
tersify completions bash   >> ~/.bash_completion
tersify completions zsh    >> ~/.zshrc
tersify completions fish   > ~/.config/fish/completions/tersify.fish
```

---

## Version

```bash
tersify --version
tersify -V
```
