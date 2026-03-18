//! Token counting utilities.
//!
//! Uses the cl100k_base BPE tokenizer (GPT-4 / Claude-compatible) for accurate
//! token counts. The encoder is initialised once and cached for the process lifetime.

use std::sync::OnceLock;
use tiktoken_rs::CoreBPE;

fn bpe() -> &'static CoreBPE {
    static ENCODER: OnceLock<CoreBPE> = OnceLock::new();
    ENCODER.get_or_init(|| tiktoken_rs::cl100k_base().expect("cl100k_base tokenizer"))
}

/// Count the number of LLM tokens in `text` using the cl100k_base BPE tokenizer.
///
/// Accurate for GPT-4 and within ~5% for Claude and other modern models.
pub fn count(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    bpe().encode_ordinary(text).len()
}

/// Compute the percentage of tokens saved after compression.
///
/// Returns `0.0` if `before` is zero.
pub fn savings_pct(before: usize, after: usize) -> f64 {
    if before == 0 {
        return 0.0;
    }
    ((before.saturating_sub(after)) as f64 / before as f64) * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_empty() {
        assert_eq!(count(""), 0);
    }

    #[test]
    fn count_simple() {
        // "hello world" is 2 tokens in cl100k_base
        assert_eq!(count("hello world"), 2);
    }

    #[test]
    fn savings_zero_before() {
        assert_eq!(savings_pct(0, 0), 0.0);
    }

    #[test]
    fn savings_half() {
        assert_eq!(savings_pct(100, 50), 50.0);
    }
}
