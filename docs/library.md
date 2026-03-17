# Using tersify as a Library

tersify is available as a Rust crate for use in your own tools, pipelines, and agents.

## Add to Cargo.toml

```toml
[dependencies]
tersify = "0.3"
```

---

## Auto-detect and compress

```rust
use tersify::{compress, detect};
use std::path::Path;

let src = std::fs::read_to_string("src/main.rs")?;

// Detect content type from path + content
let ct = detect::detect_for_path(Path::new("src/main.rs"), &src);

// Compress with default options
let out = compress::compress(&src, &ct, None)?;

println!("{out}");
```

---

## Compress with options

```rust
use tersify::compress::{compress_with, CompressOptions};
use tersify::detect;

let src = std::fs::read_to_string("src/lib.rs")?;
let ct = detect::detect_for_path(Path::new("src/lib.rs"), &src);

let out = compress_with(&src, &ct, &CompressOptions {
    ast: true,              // signatures only (tree-sitter)
    strip_docs: false,      // keep /// doc comments
    smart: false,           // no near-duplicate dedup
    budget: Some(2000),     // hard cap at 2000 tokens
})?;
```

---

## Compress a file

```rust
use tersify::input;
use tersify::compress::CompressOptions;
use std::path::Path;

let opts = CompressOptions::default();

// Returns (compressed_text, tokens_before, tokens_after)
let (output, before, after) = input::compress_file_with(
    Path::new("src/main.rs"),
    None,    // force type — None = auto-detect
    &opts,
)?;

println!("Saved {} tokens ({:.0}%)",
    before - after,
    (before - after) as f64 / before as f64 * 100.0,
);
```

---

## Compress a directory

```rust
use tersify::input;
use tersify::compress::CompressOptions;
use std::path::Path;

let opts = CompressOptions { ast: true, ..Default::default() };

// Returns (combined_output, total_tokens_before, total_tokens_after)
let (output, before, after) = input::compress_directory_with(
    Path::new("src/"),
    None,    // force type — None = auto-detect per file
    &opts,
)?;
```

---

## Count tokens

```rust
use tersify::tokens;

let text = "fn add(a: i32, b: i32) -> i32 { a + b }";
let count = tokens::count(text);   // ~4 chars/token heuristic
let pct = tokens::savings_pct(100, count);
```

---

## Detect content type

```rust
use tersify::detect::{ContentType, detect, detect_for_path};
use std::path::Path;

// From content only
let ct = detect("{ \"key\": null }");
assert_eq!(ct, ContentType::Json);

// From path + content (preferred — uses file extension)
let ct = detect_for_path(Path::new("main.rs"), "fn main() {}");
assert_eq!(ct, ContentType::Code(Language::Rust));
```

---

## CompressOptions reference

```rust
pub struct CompressOptions {
    /// Replace function bodies with { /* ... */ } using tree-sitter.
    /// Supported: Rust, Go, Python, JS, TS, TSX, Java, Ruby, C/C++, C#, PHP.
    pub ast: bool,

    /// Also strip doc comments (///, //!, /** */, Python docstrings).
    pub strip_docs: bool,

    /// Apply MinHash near-duplicate deduplication (experimental).
    pub smart: bool,

    /// Hard token budget — output is truncated if exceeded.
    pub budget: Option<usize>,
}
```

All fields implement `Default` (all false / None).

---

## Error handling

```rust
use tersify::error::TersifyError;

match compress::compress(src, &ct, None) {
    Ok(out) => { /* use out */ }
    Err(TersifyError::InvalidJson(e)) => eprintln!("JSON parse error: {e}"),
    Err(e) => eprintln!("Compression error: {e}"),
}
```

---

## API docs

Full rustdoc: [docs.rs/tersify](https://docs.rs/tersify)
