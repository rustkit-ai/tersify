# MCP Server

tersify ships a built-in [MCP](https://modelcontextprotocol.io/) server for agent pipelines. It exposes three tools over JSON-RPC 2.0 (stdio transport, protocol `2024-11-05`).

---

## Start the server

```bash
tersify mcp
```

The server reads JSON-RPC requests from stdin and writes responses to stdout.

---

## Register with Claude Code

```bash
claude mcp add tersify -- tersify mcp
```

After this, Claude Code can call `compress`, `count_tokens`, and `estimate_cost` as native tools — no more shelling out manually.

---

## Tools

### `compress`

Compress text to reduce LLM token usage.

**Input:**

| Parameter | Type | Required | Description |
|---|---|:---:|---|
| `text` | string | ✓ | The text to compress |
| `content_type` | string | — | Force type: `rust` `python` `js` `ts` `go` `ruby` `java` `c` `json` `logs` `diff` `text` — omit for auto-detection |
| `ast` | boolean | — | Extract signatures only (default: false) |
| `smart` | boolean | — | Near-duplicate deduplication (default: false) |
| `budget` | integer | — | Hard token cap |

**Output:**

```json
{
  "content": [{ "type": "text", "text": "<compressed output>" }],
  "meta": {
    "tokens_before": 384,
    "tokens_after": 228,
    "saved_pct": "41%",
    "content_type": "rust"
  }
}
```

---

### `count_tokens`

Count the approximate number of LLM tokens in a string (~4 chars/token).

**Input:**

| Parameter | Type | Required | Description |
|---|---|:---:|---|
| `text` | string | ✓ | The text to count |

**Output:**

```json
{
  "content": [{ "type": "text", "text": "96 tokens" }],
  "meta": { "count": 96 }
}
```

---

### `estimate_cost`

Show API cost before and after compression across all major LLM providers.

**Input:**

| Parameter | Type | Required | Description |
|---|---|:---:|---|
| `text` | string | ✓ | The text to estimate |
| `content_type` | string | — | Force type (same values as compress) |
| `model` | string | — | Filter models by name (e.g. `"claude"`, `"gpt-4o"`) |

**Output:**

```json
{
  "content": [{ "type": "text", "text": "384 → 228 tokens (41% saved)\n\nModel  ..." }],
  "meta": {
    "tokens_before": 384,
    "tokens_after": 228,
    "saved_pct": "41%",
    "models": [
      {
        "model": "claude-sonnet-4.6",
        "cost_raw_usd": 0.001152,
        "cost_compressed_usd": 0.000684,
        "cost_saved_usd": 0.000468
      }
    ]
  }
}
```

---

## Protocol details

- Transport: **stdio** (JSON-RPC 2.0, one message per line)
- Protocol version: `2024-11-05`
- Supported methods: `initialize`, `tools/list`, `tools/call`
- Notifications (no `id`): silently ignored

---

## Use from another MCP client

Any client that speaks MCP stdio works. Spawn the process and communicate over its stdin/stdout:

```
Process: tersify mcp
  stdin  ← your JSON-RPC requests
  stdout → tersify responses
  stderr → tersify logs (prefix: [tersify mcp])
```

Example initialize handshake:

```json
→ {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{}}}
← {"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"tersify","version":"0.3.0"}}}
```
