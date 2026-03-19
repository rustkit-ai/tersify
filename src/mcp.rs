//! `tersify mcp` — minimal MCP server over stdio (JSON-RPC 2.0).
//!
//! Exposes two tools that any MCP-compatible client (Claude Code, Cursor, etc.)
//! can call without shelling out manually:
//!
//! - `compress`      — compress text with auto-detection or forced type
//! - `count_tokens`  — count tokens in a string
//!
//! Run with: `tersify mcp`
//! Then register in Claude Code: `claude mcp add tersify -- tersify mcp`

use anyhow::Result;
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use tersify::{compress, detect, tokens};

const PROTOCOL_VERSION: &str = "2024-11-05";
const SERVER_NAME: &str = "tersify";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run() -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();

    eprintln!(
        "[tersify mcp] server started (protocol {})",
        PROTOCOL_VERSION
    );

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[tersify mcp] parse error: {e}");
                continue;
            }
        };

        let id = msg.get("id").cloned();
        let method = msg
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("")
            .to_owned();

        // Notifications have no "id" — no response required
        if id.is_none() {
            continue;
        }

        let response = match method.as_str() {
            "initialize" => handle_initialize(id.clone()),
            "tools/list" => handle_tools_list(id.clone()),
            "tools/call" => handle_tools_call(id.clone(), &msg),
            _ => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": "Method not found" }
            }),
        };

        let mut out = stdout.lock();
        writeln!(out, "{}", response)?;
        out.flush()?;
    }

    Ok(())
}

// ── Request handlers ──────────────────────────────────────────────────────────

fn handle_initialize(id: Option<Value>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": SERVER_NAME,
                "version": SERVER_VERSION
            }
        }
    })
}

fn handle_tools_list(id: Option<Value>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "tools": [
                {
                    "name": "compress",
                    "description": "Compress text to reduce LLM token usage. Strips comments, blank lines, null JSON fields, and deduplicates logs. Auto-detects content type from content, or use content_type to override.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "text": {
                                "type": "string",
                                "description": "The text to compress."
                            },
                            "content_type": {
                                "type": "string",
                                "description": "Force content type: code | rust | python | js | ts | go | ruby | java | c | swift | kotlin | json | logs | diff | text. Omit for auto-detection.",
                                "enum": ["code","rust","python","js","ts","go","ruby","java","c","swift","kotlin","json","logs","diff","text"]
                            },
                            "ast": {
                                "type": "boolean",
                                "description": "If true and content_type is code, extract function signatures only and stub bodies. Default false."
                            },
                            "smart": {
                                "type": "boolean",
                                "description": "If true, apply MinHash-based near-duplicate deduplication. Default false."
                            },
                            "budget": {
                                "type": "integer",
                                "description": "Maximum token budget; output is hard-truncated if exceeded."
                            }
                        },
                        "required": ["text"]
                    }
                },
                {
                    "name": "count_tokens",
                    "description": "Count the approximate number of LLM tokens in a string (using the ~4 chars/token heuristic).",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "text": {
                                "type": "string",
                                "description": "The text to count tokens for."
                            }
                        },
                        "required": ["text"]
                    }
                },
                {
                    "name": "estimate_cost",
                    "description": "Estimate LLM API cost for a text before and after tersify compression. Returns a per-model cost table with savings.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "text": {
                                "type": "string",
                                "description": "The text to estimate cost for."
                            },
                            "content_type": {
                                "type": "string",
                                "description": "Force content type (same values as compress). Omit for auto-detection.",
                                "enum": ["code","rust","python","js","ts","go","ruby","java","c","swift","kotlin","json","logs","diff","text"]
                            },
                            "model": {
                                "type": "string",
                                "description": "Filter to models whose name contains this string (e.g. \"claude\", \"gpt-4o\"). Omit for all models."
                            }
                        },
                        "required": ["text"]
                    }
                }
            ]
        }
    })
}

fn handle_tools_call(id: Option<Value>, msg: &Value) -> Value {
    let params = match msg.get("params") {
        Some(p) => p,
        None => {
            return json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32602, "message": "Missing params" }
            });
        }
    };

    let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");

    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    match tool_name {
        "compress" => call_compress(id, &args),
        "count_tokens" => call_count_tokens(id, &args),
        "estimate_cost" => call_estimate_cost(id, &args),
        other => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32602, "message": format!("Unknown tool: {other}") }
        }),
    }
}

// ── Tool implementations ──────────────────────────────────────────────────────

fn call_compress(id: Option<Value>, args: &Value) -> Value {
    let text = match args.get("text").and_then(|t| t.as_str()) {
        Some(t) => t,
        None => {
            return error_response(id, -32602, "Missing required argument: text");
        }
    };

    let content_type_str = args.get("content_type").and_then(|c| c.as_str());
    let ast = args.get("ast").and_then(|v| v.as_bool()).unwrap_or(false);
    let smart = args.get("smart").and_then(|v| v.as_bool()).unwrap_or(false);
    let budget = args
        .get("budget")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize);

    let ct = match content_type_str {
        Some(s) => match s.parse::<tersify::detect::ContentType>() {
            Ok(ct) => ct,
            Err(e) => return error_response(id, -32602, &e.to_string()),
        },
        None => detect::detect(text),
    };

    let opts = compress::CompressOptions {
        budget,
        ast,
        smart,
        strip_docs: false,
        custom_patterns: vec![],
    };

    let compressed = match compress::compress_with(text, &ct, &opts) {
        Ok(c) => c,
        Err(e) => return error_response(id, -32603, &e.to_string()),
    };

    let before = tokens::count(text);
    let after = tokens::count(&compressed);
    let saved_pct = tokens::savings_pct(before, after);

    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [{ "type": "text", "text": compressed }],
            "meta": {
                "tokens_before": before,
                "tokens_after": after,
                "saved_pct": format!("{:.0}%", saved_pct),
                "content_type": ct.to_string()
            }
        }
    })
}

fn call_count_tokens(id: Option<Value>, args: &Value) -> Value {
    let text = match args.get("text").and_then(|t| t.as_str()) {
        Some(t) => t,
        None => return error_response(id, -32602, "Missing required argument: text"),
    };

    let count = tokens::count(text);

    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [{
                "type": "text",
                "text": format!("{} tokens", count)
            }],
            "meta": { "count": count }
        }
    })
}

fn call_estimate_cost(id: Option<Value>, args: &Value) -> Value {
    let text = match args.get("text").and_then(|t| t.as_str()) {
        Some(t) => t,
        None => return error_response(id, -32602, "Missing required argument: text"),
    };

    let content_type_str = args.get("content_type").and_then(|c| c.as_str());
    let model_filter = args.get("model").and_then(|m| m.as_str());

    let ct = match content_type_str {
        Some(s) => match s.parse::<tersify::detect::ContentType>() {
            Ok(ct) => ct,
            Err(e) => return error_response(id, -32602, &e.to_string()),
        },
        None => detect::detect(text),
    };

    let opts = compress::CompressOptions::default();
    let compressed = match compress::compress_with(text, &ct, &opts) {
        Ok(c) => c,
        Err(e) => return error_response(id, -32603, &e.to_string()),
    };

    let before = tokens::count(text);
    let after = tokens::count(&compressed);
    let saved_pct = tokens::savings_pct(before, after);

    let models_table = tersify::MODEL_PRICING;

    let models: Vec<_> = models_table
        .iter()
        .filter(|(name, _, _)| {
            model_filter
                .map(|f| name.to_lowercase().contains(&f.to_lowercase()))
                .unwrap_or(true)
        })
        .collect();

    if models.is_empty() {
        return error_response(
            id,
            -32602,
            &format!("No model matched \"{}\"", model_filter.unwrap_or("")),
        );
    }

    let mut lines = vec![
        format!("{} → {} tokens ({:.0}% saved)", before, after, saved_pct),
        String::new(),
        format!(
            "{:<22} {:>12} {:>12} {:>12}",
            "Model", "Raw", "Compressed", "Saved"
        ),
        "─".repeat(60),
    ];

    let mut cost_data: Vec<Value> = Vec::new();

    for (name, _provider, price) in &models {
        let raw = before as f64 / 1_000_000.0 * price;
        let comp = after as f64 / 1_000_000.0 * price;
        let saved = raw - comp;
        lines.push(format!(
            "{:<22} {:>12} {:>12} {:>12}",
            name,
            format!("${:.4}", raw),
            format!("${:.4}", comp),
            format!("-${:.4}", saved),
        ));
        cost_data.push(json!({
            "model": name,
            "cost_raw_usd": raw,
            "cost_compressed_usd": comp,
            "cost_saved_usd": saved,
        }));
    }

    let summary = lines.join("\n");

    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [{ "type": "text", "text": summary }],
            "meta": {
                "tokens_before": before,
                "tokens_after": after,
                "saved_pct": format!("{:.0}%", saved_pct),
                "content_type": ct.to_string(),
                "models": cost_data,
            }
        }
    })
}

fn error_response(id: Option<Value>, code: i32, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_returns_protocol_version() {
        let resp = handle_initialize(Some(json!(1)));
        assert_eq!(resp["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(resp["result"]["serverInfo"]["name"], SERVER_NAME);
        assert_eq!(resp["id"], 1);
    }

    #[test]
    fn tools_list_includes_all_three_tools() {
        let resp = handle_tools_list(Some(json!(1)));
        let tools = resp["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"compress"));
        assert!(names.contains(&"count_tokens"));
        assert!(names.contains(&"estimate_cost"));
    }

    #[test]
    fn compress_tool_strips_comments() {
        let msg = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {
                "name": "compress",
                "arguments": { "text": "// comment\nfn foo() {}", "content_type": "rust" }
            }
        });
        let resp = handle_tools_call(Some(json!(1)), &msg);
        assert!(resp.get("error").is_none(), "unexpected error: {resp}");
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(!text.contains("// comment"));
        assert!(text.contains("fn foo()"));
    }

    #[test]
    fn compress_tool_missing_text_returns_error() {
        let msg = json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": { "name": "compress", "arguments": {} }
        });
        let resp = handle_tools_call(Some(json!(2)), &msg);
        assert!(resp.get("error").is_some());
    }

    #[test]
    fn count_tokens_returns_count() {
        let msg = json!({
            "jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": {
                "name": "count_tokens",
                "arguments": { "text": "hello world" }
            }
        });
        let resp = handle_tools_call(Some(json!(3)), &msg);
        assert!(resp.get("error").is_none());
        let count = resp["result"]["meta"]["count"].as_u64().unwrap();
        assert!(count > 0);
    }

    #[test]
    fn estimate_cost_returns_model_table() {
        let msg = json!({
            "jsonrpc": "2.0", "id": 4, "method": "tools/call",
            "params": {
                "name": "estimate_cost",
                "arguments": { "text": "// comment\nfn foo() { let x = 1; }", "model": "claude-sonnet" }
            }
        });
        let resp = handle_tools_call(Some(json!(4)), &msg);
        assert!(resp.get("error").is_none(), "unexpected error: {resp}");
        let meta = &resp["result"]["meta"];
        assert!(meta["tokens_before"].as_u64().unwrap() > 0);
        let models = meta["models"].as_array().unwrap();
        assert!(!models.is_empty());
        assert!(
            models[0]["model"]
                .as_str()
                .unwrap()
                .contains("claude-sonnet")
        );
    }

    #[test]
    fn unknown_method_returns_method_not_found() {
        let msg = json!({ "jsonrpc": "2.0", "id": 5, "method": "unknown/method", "params": {} });
        let id = msg.get("id").cloned();
        let resp = match msg["method"].as_str().unwrap() {
            "initialize" => handle_initialize(id.clone()),
            "tools/list" => handle_tools_list(id.clone()),
            "tools/call" => handle_tools_call(id.clone(), &msg),
            _ => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": "Method not found" }
            }),
        };
        assert_eq!(resp["error"]["code"], -32601);
    }
}
