//! Token counting utilities.
//!
//! Uses the ~4 chars/token heuristic — accurate enough for budget enforcement
//! without pulling in heavy tokenizer dependencies.

/// Estimate the number of LLM tokens in `text`.
///
/// Uses the standard ~4 characters per token approximation, which is
/// accurate within ±10% for most English and code content.
pub fn count(text: &str) -> usize {
    (text.len() as f64 / 4.0).ceil() as usize
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
    fn savings_zero_before() {
        assert_eq!(savings_pct(0, 0), 0.0);
    }

    #[test]
    fn savings_half() {
        assert_eq!(savings_pct(100, 50), 50.0);
    }
}
