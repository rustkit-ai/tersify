//! `tersify bench` — benchmark compression savings across all content types.
//!
//! Runs tersify over representative embedded samples for each content type
//! and prints a formatted table of token counts and savings percentages.

use anyhow::Result;
use tersify::{compress, compress::CompressOptions, detect, tokens};

pub fn run() -> Result<()> {
    const SAMPLES: &[(&str, &str)] = &[
        ("Rust code", RUST_SAMPLE),
        ("Python code", PYTHON_SAMPLE),
        ("TypeScript code", TS_SAMPLE),
        ("Ruby code", RUBY_SAMPLE),
        ("Java code", JAVA_SAMPLE),
        ("C code", C_SAMPLE),
        ("Swift code", SWIFT_SAMPLE),
        ("Kotlin code", KOTLIN_SAMPLE),
        ("JSON", JSON_SAMPLE),
        ("Git diff", DIFF_SAMPLE),
        ("Application logs", LOGS_SAMPLE),
        ("Plain text", TEXT_SAMPLE),
    ];

    let col_label = 20usize;
    let col_num = 9usize;

    println!(
        "{:<col_label$}  {:>col_num$}  {:>col_num$}  {:>8}",
        "Content type", "Before", "After", "Saved"
    );
    println!("{}", "─".repeat(col_label + col_num * 2 + 14));

    let mut total_before = 0usize;
    let mut total_after = 0usize;

    for (label, sample) in SAMPLES {
        let ct = detect::detect(sample);
        let compressed = compress::compress(sample, &ct, None)?;
        let before = tokens::count(sample);
        let after = tokens::count(&compressed);
        let saved_pct = tokens::savings_pct(before, after);

        total_before += before;
        total_after += after;

        println!(
            "{:<col_label$}  {:>col_num$}  {:>col_num$}  {:>7.0}%",
            label, before, after, saved_pct
        );
    }

    println!("{}", "─".repeat(col_label + col_num * 2 + 14));
    let total_pct = tokens::savings_pct(total_before, total_after);
    println!(
        "{:<col_label$}  {:>col_num$}  {:>col_num$}  {:>7.0}%",
        "TOTAL", total_before, total_after, total_pct
    );

    // ── AST mode (--ast) ──────────────────────────────────────────────────────

    const AST_SAMPLES: &[(&str, &str)] = &[
        ("Rust", RUST_SAMPLE),
        ("Python", PYTHON_SAMPLE),
        ("TypeScript", TS_SAMPLE),
        ("Ruby", RUBY_SAMPLE),
        ("Java", JAVA_SAMPLE),
        ("C / C++", C_SAMPLE),
    ];

    println!();
    println!(
        "{:<col_label$}  {:>col_num$}  {:>col_num$}  {:>8}",
        "AST mode (--ast)", "Before", "After", "Saved"
    );
    println!("{}", "─".repeat(col_label + col_num * 2 + 14));

    let ast_opts = CompressOptions {
        ast: true,
        ..Default::default()
    };
    let mut ast_before = 0usize;
    let mut ast_after = 0usize;

    for (label, sample) in AST_SAMPLES {
        let ct = detect::detect(sample);
        let compressed = compress::compress_with(sample, &ct, &ast_opts)?;
        let before = tokens::count(sample);
        let after = tokens::count(&compressed);
        let saved_pct = tokens::savings_pct(before, after);

        ast_before += before;
        ast_after += after;

        println!(
            "{:<col_label$}  {:>col_num$}  {:>col_num$}  {:>7.0}%",
            label, before, after, saved_pct
        );
    }

    println!("{}", "─".repeat(col_label + col_num * 2 + 14));
    let ast_total_pct = tokens::savings_pct(ast_before, ast_after);
    println!(
        "{:<col_label$}  {:>col_num$}  {:>col_num$}  {:>7.0}%",
        "TOTAL", ast_before, ast_after, ast_total_pct
    );

    Ok(())
}

// ── Embedded samples ─────────────────────────────────────────────────────────

const RUST_SAMPLE: &str = r#"
// Authentication middleware for the REST API.
// Validates JWT tokens issued by our identity provider.
use anyhow::{Context, Result};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};

/// Claims embedded in the JWT token.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,   // subject — user id
    pub exp: usize,    // expiration timestamp
    pub roles: Vec<String>, // authorisation roles
}

// Validates a bearer token and returns the embedded claims.
// Returns an error if the token is expired, malformed, or signed with the wrong key.
pub fn validate_token(token: &str, secret: &[u8]) -> Result<Claims> {
    // Decode the header first to get the algorithm
    let header = decode_header(token)
        .context("failed to decode JWT header")?;

    // Build a validation config matching the issuer requirements
    let mut validation = Validation::new(header.alg);
    validation.validate_exp = true; // always enforce expiry

    let key = DecodingKey::from_secret(secret);

    let data = decode::<Claims>(token, &key, &validation)
        .context("JWT validation failed")?;

    // Extra check: ensure the subject is non-empty
    if data.claims.sub.is_empty() {
        anyhow::bail!("token subject is empty");
    }

    Ok(data.claims)
}

// Build an authorization header value from a raw token string.
pub fn bearer_header(token: &str) -> String {
    // Standard HTTP Authorization header format
    format!("Bearer {}", token)
}
"#;

const PYTHON_SAMPLE: &str = r#"
"""
Data processing pipeline for the analytics service.
Handles ingestion, transformation, and aggregation of raw events.
"""

import json
import logging
from datetime import datetime, timezone
from typing import Any

# Module-level logger — use structured fields for Datadog
logger = logging.getLogger(__name__)

# Default batch size used when the caller does not specify one.
# Chosen to stay within the 1 MB Lambda payload limit.
DEFAULT_BATCH_SIZE = 256


def process_events(raw_events: list[dict], batch_size: int = DEFAULT_BATCH_SIZE) -> list[dict]:
    """
    Process a list of raw event dicts into normalised analytics records.

    Args:
        raw_events: Unsanitised event payloads from the ingestion queue.
        batch_size: Number of events to process in each internal batch.

    Returns:
        List of normalised event dicts ready for downstream storage.
    """
    results = []

    # Work in batches to bound peak memory usage
    for offset in range(0, len(raw_events), batch_size):
        batch = raw_events[offset : offset + batch_size]
        logger.info("processing batch", extra={"offset": offset, "size": len(batch)})

        for event in batch:
            try:
                normalised = _normalise(event)
                results.append(normalised)
            except (KeyError, ValueError) as exc:
                # Log and skip malformed events — never let one bad record
                # poison the whole batch.
                logger.warning("skipping malformed event", extra={"error": str(exc)})

    return results


def _normalise(event: dict[str, Any]) -> dict[str, Any]:
    """Convert a raw event dict into the canonical analytics schema."""
    # Timestamps arrive as Unix milliseconds; convert to ISO-8601
    ts_ms = event["timestamp"]
    dt = datetime.fromtimestamp(ts_ms / 1000, tz=timezone.utc)

    return {
        "event_id": event["id"],
        "event_type": event["type"].lower().strip(),
        "user_id": event.get("user_id"),
        "occurred_at": dt.isoformat(),
        "payload": json.dumps(event.get("data", {})),
    }
"#;

const TS_SAMPLE: &str = r#"
// HTTP client wrapper with automatic retry logic and structured logging.
// Uses exponential back-off with jitter to avoid thundering-herd problems.

import { logger } from './logger';
import { sleep } from './utils';

/** Configuration for a single HTTP request. */
export interface RequestOptions {
  method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
  url: string;
  body?: unknown;
  /** Maximum number of retry attempts (default: 3). */
  maxRetries?: number;
  /** Base delay in ms for exponential back-off (default: 200). */
  baseDelay?: number;
}

/** Typed response envelope returned by every call. */
export interface ApiResponse<T = unknown> {
  status: number;
  data: T;
  requestId: string;
}

/**
 * Send an HTTP request with automatic retry on transient errors.
 * Retries on 429 (rate limit) and 5xx responses.
 */
export async function request<T>(opts: RequestOptions): Promise<ApiResponse<T>> {
  const { method, url, body, maxRetries = 3, baseDelay = 200 } = opts;

  let attempt = 0;

  while (true) {
    try {
      // Attach a unique request ID so we can correlate logs end-to-end
      const requestId = crypto.randomUUID();
      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
        'X-Request-ID': requestId,
      };

      const res = await fetch(url, {
        method,
        headers,
        body: body !== undefined ? JSON.stringify(body) : undefined,
      });

      if (!res.ok && shouldRetry(res.status) && attempt < maxRetries) {
        const delay = baseDelay * 2 ** attempt + Math.random() * 50;
        logger.warn('retrying request', { url, status: res.status, attempt, delay });
        await sleep(delay);
        attempt++;
        continue;
      }

      const data = (await res.json()) as T;
      return { status: res.status, data, requestId };
    } catch (err) {
      if (attempt >= maxRetries) throw err;
      attempt++;
    }
  }
}

/** Returns true for HTTP status codes that warrant a retry. */
function shouldRetry(status: number): boolean {
  return status === 429 || (status >= 500 && status < 600);
}
"#;

const RUBY_SAMPLE: &str = r#"
# HTTP client with retry logic for the internal services mesh.
require 'faraday'
require 'json'
require 'logger'

# Shared logger for all service calls — structured fields for Datadog.
LOGGER = Logger.new($stdout)

# Default retry configuration.
DEFAULT_RETRIES = 3
BASE_DELAY_MS   = 200

=begin
This module wraps Faraday with structured logging and exponential back-off.
All responses are logged with a request ID for end-to-end traceability.
=end
module ServiceClient
  # Send a GET request with automatic retry on 5xx.
  def self.get(base_url, path, headers: {}, retries: DEFAULT_RETRIES)
    # Attach request ID to every outgoing call
    request_id = SecureRandom.uuid
    merged = headers.merge('X-Request-Id' => request_id)
    attempt = 0
    begin
      conn = Faraday.new(url: base_url)
      resp = conn.get(path) { |req| req.headers.merge!(merged) }
      # Retry on 5xx with exponential back-off
      if resp.status >= 500 && attempt < retries
        delay = BASE_DELAY_MS * (2 ** attempt) / 1000.0
        LOGGER.warn("retrying", { status: resp.status, attempt: attempt })
        sleep(delay)
        attempt += 1
        retry
      end
      JSON.parse(resp.body)
    rescue Faraday::ConnectionFailed => e
      # Network-level failure — re-raise so the caller can decide
      LOGGER.error("connection failed", { error: e.message })
      raise
    end
  end

  # POST a JSON body and return the parsed response.
  def self.post(base_url, path, body:, headers: {})
    # Serialise to JSON and set the Content-Type header
    conn = Faraday.new(url: base_url)
    resp = conn.post(path) do |req|
      req.headers['Content-Type'] = 'application/json'
      req.headers.merge!(headers)
      req.body = JSON.dump(body)
    end
    JSON.parse(resp.body)
  end
end
"#;

const JAVA_SAMPLE: &str = r#"
// JWT authentication service — validates tokens from the identity provider.

package com.example.auth;

import io.jsonwebtoken.Claims;
import io.jsonwebtoken.Jwts;
import io.jsonwebtoken.SignatureAlgorithm;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import java.util.Date;

/**
 * Stateless JWT validation service.
 * Thread-safe — a single instance can be shared across the application.
 */
public class JwtService {

    private static final Logger log = LoggerFactory.getLogger(JwtService.class);

    // Secret used to sign tokens — injected from environment at startup
    private final byte[] secret;

    public JwtService(byte[] secret) {
        // Validate that the secret meets minimum length requirements
        if (secret == null || secret.length < 32) {
            throw new IllegalArgumentException("JWT secret must be at least 32 bytes");
        }
        this.secret = secret;
    }

    /**
     * Validate a bearer token and return its embedded claims.
     * @throws AuthException if expired, malformed, or has an invalid signature
     */
    public Claims validateToken(String token) {
        // Strip "Bearer " prefix if the caller includes it
        String raw = token.startsWith("Bearer ") ? token.substring(7) : token;
        try {
            Claims claims = Jwts.parser()
                    .setSigningKey(secret)
                    .parseClaimsJws(raw)
                    .getBody();
            // Reject tokens with an empty subject
            if (claims.getSubject() == null || claims.getSubject().isEmpty()) {
                throw new AuthException("Token subject is empty");
            }
            log.debug("Token validated for subject={}", claims.getSubject());
            return claims;
        } catch (Exception e) {
            log.warn("Token validation failed: {}", e.getMessage());
            throw new AuthException("Invalid token: " + e.getMessage(), e);
        }
    }

    /** Build a signed token for the given subject with a TTL in seconds. */
    public String issueToken(String subject, long ttlSeconds) {
        Date now = new Date();
        Date expiry = new Date(now.getTime() + ttlSeconds * 1000L);
        return Jwts.builder()
                .setSubject(subject)
                .setIssuedAt(now)
                .setExpiration(expiry)
                .signWith(SignatureAlgorithm.HS256, secret)
                .compact();
    }
}
"#;

const C_SAMPLE: &str = r#"
/* HTTP request parser — zero-allocation, fixed-size output structs.
 * Designed for embedded systems with no dynamic memory. */

#include <stdio.h>
#include <string.h>
#include <ctype.h>

/* Maximum number of headers parsed per request */
#define MAX_HEADERS 32

typedef struct {
    char key[64];    /* header name */
    char value[256]; /* header value */
} HttpHeader;

typedef struct {
    char method[8];          /* HTTP method: GET, POST, etc. */
    char path[512];          /* request path */
    char version[16];        /* HTTP version string */
    HttpHeader headers[MAX_HEADERS];
    int header_count;        /* number of headers parsed */
    const char *body;        /* pointer into original buffer */
    size_t body_len;         /* body length in bytes */
} HttpRequest;

/*
 * parse_request — parse a raw HTTP/1.1 request buffer.
 * Returns 0 on success, -1 on parse error.
 * The buffer must remain valid for the lifetime of the HttpRequest.
 */
int parse_request(const char *buf, size_t len, HttpRequest *out) {
    /* Validate inputs — never trust caller to pass valid pointers */
    if (buf == NULL || out == NULL || len == 0) {
        return -1;
    }
    memset(out, 0, sizeof(*out));

    const char *p = buf;
    const char *end = buf + len;

    /* Method: read until first space */
    int i = 0;
    while (p < end && *p != ' ' && i < 7) {
        out->method[i++] = *p++;
    }
    out->method[i] = '\0';
    if (*p != ' ') return -1; /* malformed — no space after method */
    p++;

    /* Path: read until next space */
    i = 0;
    while (p < end && *p != ' ' && i < 511) {
        out->path[i++] = *p++;
    }
    out->path[i] = '\0';

    /* Version: read until CRLF */
    if (*p != ' ') return -1;
    p++;
    i = 0;
    while (p < end && *p != '\r' && i < 15) {
        out->version[i++] = *p++;
    }
    out->version[i] = '\0';

    return 0;
}

/* Look up a header value by name (case-insensitive). Returns NULL if not found. */
const char *get_header(const HttpRequest *req, const char *name) {
    /* Linear scan — fine for the small header count */
    for (int i = 0; i < req->header_count; i++) {
        if (strcasecmp(req->headers[i].key, name) == 0) {
            return req->headers[i].value;
        }
    }
    return NULL; /* not found */
}
"#;

const SWIFT_SAMPLE: &str = r#"
// NetworkManager — lightweight async wrapper around URLSession.
// Uses Swift concurrency (async/await) for clean call sites in SwiftUI.

import Foundation

/// Errors thrown by NetworkManager.
enum NetworkError: Error {
    case invalidURL
    case httpError(statusCode: Int)
    case decodingFailed(Error)
}

/// Central HTTP client; inject as an environment object or a singleton.
final class NetworkManager {

    // Shared URL session configured with caller-supplied timeout
    private let session: URLSession

    /// Base URL for all requests — injected for testability.
    let baseURL: URL

    init(baseURL: URL, timeoutInterval: TimeInterval = 30) {
        // Build a custom config with the specified timeout
        let config = URLSessionConfiguration.default
        config.timeoutIntervalForRequest = timeoutInterval
        self.session = URLSession(configuration: config)
        self.baseURL = baseURL
    }

    /// Fetch and decode a `Decodable` resource from the given path.
    /// Throws `NetworkError` for HTTP errors or decoding failures.
    func fetch<T: Decodable>(_ type: T.Type, path: String) async throws -> T {
        // Validate the URL before making any network call
        guard let url = URL(string: path, relativeTo: baseURL) else {
            throw NetworkError.invalidURL
        }
        let (data, response) = try await session.data(from: url)
        // Treat anything outside 2xx as an error — caller decides how to handle
        if let http = response as? HTTPURLResponse, !(200..<300).contains(http.statusCode) {
            throw NetworkError.httpError(statusCode: http.statusCode)
        }
        do {
            return try JSONDecoder().decode(T.self, from: data)
        } catch {
            throw NetworkError.decodingFailed(error)
        }
    }

    /// POST an `Encodable` body and decode the response.
    func post<Body: Encodable, Response: Decodable>(
        _ body: Body,
        path: String,
        responseType: Response.Type
    ) async throws -> Response {
        guard let url = URL(string: path, relativeTo: baseURL) else {
            throw NetworkError.invalidURL
        }
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONEncoder().encode(body)
        let (data, response) = try await session.data(for: request)
        if let http = response as? HTTPURLResponse, !(200..<300).contains(http.statusCode) {
            throw NetworkError.httpError(statusCode: http.statusCode)
        }
        do {
            return try JSONDecoder().decode(Response.self, from: data)
        } catch {
            throw NetworkError.decodingFailed(error)
        }
    }
}
"#;

const KOTLIN_SAMPLE: &str = r#"
// Repository layer for the user profile feature.
// Follows the offline-first pattern: read from cache, sync in the background.

package com.example.profile

import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.withContext
import kotlinx.coroutines.Dispatchers

/**
 * Single source of truth for user profile data.
 */
class UserProfileRepository(
    private val localDao: UserProfileDao,
    private val remoteApi: UserProfileApi,
) {

    /**
     * Observe the local profile for [userId] as a reactive Flow.
     * Emits an update whenever the cached data changes.
     */
    fun observeProfile(userId: String): Flow<UserProfile?> {
        // Map the database entity to the domain model — keeps callers decoupled
        return localDao.observeById(userId).map { entity ->
            entity?.toDomain()
        }
    }

    /**
     * Fetch the latest profile from the remote API and update the local cache.
     * Returns the refreshed profile or throws on network failure.
     */
    suspend fun refreshProfile(userId: String): UserProfile {
        // All network I/O must stay off the main thread
        return withContext(Dispatchers.IO) {
            val dto = remoteApi.getProfile(userId)
            val entity = dto.toEntity()
            // Upsert preserves local-only fields (e.g. last viewed timestamp)
            localDao.upsert(entity)
            dto.toDomain()
        }
    }

    /**
     * Update the display name with an optimistic local write + remote sync.
     * Rolls back the local write if the remote call fails.
     */
    suspend fun updateDisplayName(userId: String, newName: String) {
        // Fetch current entity before mutating — needed for potential rollback
        val current = withContext(Dispatchers.IO) { localDao.findById(userId) }
            ?: error("User $userId not found in local cache")
        // Optimistic write: update locally before calling the API
        withContext(Dispatchers.IO) {
            localDao.upsert(current.copy(displayName = newName))
        }
        try {
            withContext(Dispatchers.IO) {
                remoteApi.updateDisplayName(userId, newName)
            }
        } catch (e: Exception) {
            // Rollback the optimistic update on remote failure
            withContext(Dispatchers.IO) { localDao.upsert(current) }
            throw e
        }
    }
}
"#;

const JSON_SAMPLE: &str = r#"{
  "id": "usr_7f2a91bc",
  "email": "alice@example.com",
  "display_name": "Alice Dupont",
  "avatar_url": null,
  "bio": null,
  "phone": null,
  "created_at": "2024-11-03T14:22:00Z",
  "updated_at": "2025-01-17T09:05:33Z",
  "last_login": "2025-03-12T08:47:11Z",
  "is_active": true,
  "is_verified": true,
  "is_deleted": false,
  "metadata": {},
  "tags": [],
  "preferences": {
    "theme": "dark",
    "locale": "fr-FR",
    "notifications": {
      "email": true,
      "sms": false,
      "push": null
    },
    "dashboard": {
      "widgets": [],
      "layout": null
    }
  },
  "roles": ["user", "admin"],
  "quota": {
    "used": 1482,
    "limit": 10000,
    "unit": "requests",
    "reset_at": null
  }
}"#;

const DIFF_SAMPLE: &str = r#"diff --git a/src/auth/middleware.rs b/src/auth/middleware.rs
index 3a5f8c2..9b1e4d7 100644
--- a/src/auth/middleware.rs
+++ b/src/auth/middleware.rs
@@ -12,18 +12,22 @@ use crate::error::AppError;

 pub struct AuthMiddleware {
     secret: Vec<u8>,
+    /// Allow requests with no token when set (useful for public endpoints).
+    allow_anonymous: bool,
 }

 impl AuthMiddleware {
-    pub fn new(secret: Vec<u8>) -> Self {
-        Self { secret }
+    pub fn new(secret: Vec<u8>, allow_anonymous: bool) -> Self {
+        Self { secret, allow_anonymous }
     }

     pub async fn validate(&self, req: &Request) -> Result<Claims, AppError> {
-        let token = extract_bearer(req).ok_or(AppError::Unauthorized)?;
-        validate_token(&token, &self.secret).map_err(|_| AppError::Unauthorized)
+        let token = extract_bearer(req);
+        match token {
+            None if self.allow_anonymous => Ok(Claims::anonymous()),
+            None => Err(AppError::Unauthorized),
+            Some(t) => validate_token(&t, &self.secret).map_err(|_| AppError::Unauthorized),
+        }
     }
 }
"#;

const LOGS_SAMPLE: &str = r#"2025-03-14T08:00:01.234Z INFO  [worker-1] Starting processing cycle id=7f2a91bc-0001
2025-03-14T08:00:01.235Z INFO  [worker-1] Starting processing cycle id=7f2a91bc-0002
2025-03-14T08:00:01.236Z INFO  [worker-1] Starting processing cycle id=7f2a91bc-0003
2025-03-14T08:00:02.100Z ERROR [worker-1] Connection timeout host=db-primary:5432 attempt=1/3
2025-03-14T08:00:02.700Z ERROR [worker-1] Connection timeout host=db-primary:5432 attempt=1/3
2025-03-14T08:00:03.300Z ERROR [worker-1] Connection timeout host=db-primary:5432 attempt=1/3
2025-03-14T08:00:03.900Z ERROR [worker-1] Connection timeout host=db-primary:5432 attempt=1/3
2025-03-14T08:00:04.200Z WARN  [worker-1] Falling back to replica host=db-replica-1:5432
2025-03-14T08:00:04.250Z INFO  [worker-1] Connected to replica in 50ms
2025-03-14T08:00:04.300Z INFO  [worker-1] Processed 128 events duration_ms=47
2025-03-14T08:00:04.350Z INFO  [worker-1] Processed 128 events duration_ms=51
2025-03-14T08:00:04.400Z INFO  [worker-1] Processed 128 events duration_ms=44
2025-03-14T08:00:05.000Z INFO  [worker-1] Processed 128 events duration_ms=49
2025-03-14T08:00:05.050Z INFO  [worker-1] Processed 128 events duration_ms=52
2025-03-14T08:00:05.100Z INFO  [worker-1] Processed 128 events duration_ms=48
2025-03-14T08:00:06.000Z INFO  [scheduler] Cycle complete total_events=768 errors=0 duration_ms=4766
"#;

const TEXT_SAMPLE: &str = r#"
Tersify is a token compression tool designed for use with large language models.
It reduces the number of tokens in your context window before sending to the model.
Tersify strips comments, blank lines, null JSON fields, and repeated log entries.

Tersify is a token compression tool designed for use with large language models.
It reduces the number of tokens in your context window before sending to the model.

When you use tersify, you can save between 30% and 78% of your tokens depending
on the content type. Code files typically save around 31% by removing comments.
Git diffs save the most — around 74% — by keeping only the changed lines.
Log files save around 78% by collapsing repeated identical lines into one.

When you use tersify, you can save between 30% and 78% of your tokens depending
on the content type. Code files typically save around 31% by removing comments.

Install tersify from crates.io: cargo install tersify
Run `tersify install` once to automatically compress files as Claude reads them.
"#;
