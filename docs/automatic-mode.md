# Automatic Mode

The most powerful way to use tersify: install it once, forget about it, and let it silently compress every file your AI reads.

---

## Claude Code

### Install (one command)

```bash
tersify install
```

That's it. tersify writes a `PreToolUse` hook to `~/.claude/hooks.json`:

```json
{
  "PreToolUse": [
    {
      "matcher": "Read",
      "hooks": [
        {
          "type": "command",
          "command": "tersify \"$CLAUDE_TOOL_INPUT_FILE_PATH\""
        }
      ]
    }
  ]
}
```

**What this does:** every time Claude Code reads a file (via its `Read` tool), tersify intercepts the content and compresses it before it enters the context window. Claude sees a clean, token-efficient version. You see no difference in the workflow.

### Verify it's working

```bash
tersify stats
```

After a few coding sessions, you'll see cumulative savings.

### Remove the hook

```bash
tersify uninstall
```

---

## Cursor

### Install

```bash
tersify install --cursor
```

This writes a global Cursor rule to `~/.cursor/rules/tersify.mdc`. The rule instructs Cursor to compress file contents through tersify before including them in context.

### Remove

```bash
tersify uninstall --cursor
```

---

## Windsurf

### Install

```bash
tersify install --windsurf
```

This writes a global Windsurf rule to `~/.windsurf/rules/tersify.md`.

### Remove

```bash
tersify uninstall --windsurf
```

---

## How automatic compression works

```
Your file (384 tokens)
       │
       ▼
  tersify hook
  ─ strip comments
  ─ collapse blank lines
  ─ remove JSON nulls
  ─ deduplicate logs
       │
       ▼
Compressed file (228 tokens)
       │
       ▼
   AI context window
```

The AI receives fewer tokens but the **same information**. Comments, blank lines, and formatting noise carry no semantic content for an LLM.

---

## AST mode in automatic compression

Automatic mode uses standard compression by default. For AST mode (signatures only) you call tersify manually:

```bash
tersify src/ --ast | pbcopy   # copy to clipboard
tersify src/ --ast > context.txt
```

Or pipe a directory to Claude Code directly:

```bash
tersify src/ --ast | claude "explain this codebase's architecture"
```

---

## What gets compressed automatically

The hook fires on every `Read` tool call. tersify auto-detects the content type from the file extension and content:

| File type | What happens |
|---|---|
| `.rs` `.go` `.py` `.ts` `.js` `.java` `.cs` `.php` `.rb` `.c` `.cpp` | Comments stripped, blank lines collapsed |
| `.json` | Null fields and empty arrays/objects removed |
| `.log` | Duplicate lines collapsed |
| `.diff` `.patch` | Context lines removed |
| `.md` `.txt` | Duplicate sentences removed |
| Everything else | Passed through unchanged |

---

## Track savings over time

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

At $3/M tokens (claude-sonnet-4.6), 1.7M tokens saved ≈ **$5 saved**.

---

## MCP server (alternative integration)

If you prefer tool-based integration over a hook, run tersify as an MCP server — see [MCP Server](mcp-server.md).
