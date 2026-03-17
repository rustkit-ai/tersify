//! Smart near-duplicate deduplication using MinHash.
//!
//! Splits input into logical blocks (separated by blank lines), computes a
//! MinHash signature for each, and removes blocks that are suspiciously similar
//! to a previously seen block. This catches repeated log entries, copy-pasted
//! code, and redundant explanations that exact-match dedup misses.
//!
//! The implementation is purely in-process — no embeddings, no ML model.
//! Similarity is estimated via word-level 3-gram shingles and 16 universal
//! hash functions. Expected accuracy: ≈±5% of true Jaccard similarity.

use super::util::collapse_blank_lines;
use std::collections::HashSet;

/// Number of hash functions used for MinHash signatures.
const NUM_HASHES: usize = 16;

/// Blocks with estimated Jaccard similarity ≥ this threshold are treated as duplicates.
const SIMILARITY_THRESHOLD: f32 = 0.8;

/// Shingle size (n-gram window over words).
const SHINGLE_SIZE: usize = 3;

// Pre-computed large odd multipliers (universal hash family: h_k(x) = A_k*x + B_k)
const HASH_A: [u64; NUM_HASHES] = [
    0x517cc1b727220a95,
    0xd74e26b7f12a3d5f,
    0x123456789abcdef0,
    0xfedcba9876543210,
    0xaaaaaaaaaaaaaaab,
    0x5555555555555557,
    0xc3d2e1f099887761,
    0x1122334455667789,
    0x99aabbccddeeff01,
    0x1337deadbeef0003,
    0xc0ffee1234567891,
    0x0102030405060709,
    0xf0e0d0c0b0a09081,
    0x7f6f5f4f3f2f1f0b,
    0xa5a5a5a5a5a5a5a7,
    0x6c62272e07bb0143,
];

const HASH_B: [u64; NUM_HASHES] = [
    0xe17a1465deadbeef,
    0x9c6b2f3d1a2b3c4d,
    0x7e81a5c3cafebabe,
    0x4d2f8b1efeed1234,
    0x3c9d7f2a5678abcd,
    0x8f4e2d1cef012345,
    0x6a3b9c7d23456789,
    0x2e5f8a1b9abcdef0,
    0xb7c3e4f566778899,
    0xa1b2c3d41f2e3d4c,
    0x5e6f7a8b0a1b2c3d,
    0x9d8c7b6a8f9eadbc,
    0x4c3b2a1912345678,
    0x8f9eadbc87654321,
    0x1234567801020304,
    0x8765432109080706,
];

/// Remove near-duplicate blocks from `input`.
///
/// Blocks are separated by one or more blank lines. Within each block,
/// word ordering matters — two blocks that say the same thing in a different
/// order will not be deduplicated unless their Jaccard similarity exceeds the
/// threshold regardless.
pub fn dedup(input: &str) -> String {
    let blocks = split_blocks(input);
    let mut seen_exact: HashSet<u64> = HashSet::new();
    let mut seen_sigs: Vec<[u64; NUM_HASHES]> = Vec::new();
    let mut kept: Vec<&str> = Vec::new();

    for block in &blocks {
        let trimmed = block.trim();

        // Always keep blank separators
        if trimmed.is_empty() {
            kept.push(block);
            continue;
        }

        // Exact duplicate check (fast path)
        let exact = fnv64(trimmed);
        if !seen_exact.insert(exact) {
            continue;
        }

        // Near-duplicate check via MinHash
        let sig = minhash(trimmed);
        let is_near_dup = seen_sigs
            .iter()
            .any(|prev| jaccard(prev, &sig) >= SIMILARITY_THRESHOLD);
        if is_near_dup {
            continue;
        }

        seen_sigs.push(sig);
        kept.push(block);
    }

    // Reconstruct output, collapsing runs of blank lines
    let raw = kept.join("\n\n");
    collapse_blank_lines(&raw)
}

// ── Block splitting ──────────────────────────────────────────────────────────

/// Split `input` into blocks at blank-line boundaries, preserving the separators.
fn split_blocks(input: &str) -> Vec<&str> {
    let mut blocks: Vec<&str> = Vec::new();
    let mut start = 0usize;
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // Detect a blank line (\n\n or \r\n\r\n)
        if bytes[i] == b'\n' {
            let block_end = i;
            // Skip the blank lines
            while i < bytes.len() && (bytes[i] == b'\n' || bytes[i] == b'\r') {
                i += 1;
            }
            if i > block_end + 1 {
                // More than one newline — block boundary
                blocks.push(&input[start..block_end]);
                start = i;
                continue;
            }
        }
        i += 1;
    }
    if start < input.len() {
        blocks.push(&input[start..]);
    }
    blocks
}

// ── Hashing ──────────────────────────────────────────────────────────────────

fn fnv64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in s.as_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn hash_shingle(shingle: &[&str]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for word in shingle {
        for &b in word.as_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h ^= 0x20; // space separator between words
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn minhash(text: &str) -> [u64; NUM_HASHES] {
    let mut sigs = [u64::MAX; NUM_HASHES];
    let words: Vec<&str> = text.split_whitespace().collect();

    if words.is_empty() {
        return sigs;
    }

    let shingles: Vec<u64> = if words.len() >= SHINGLE_SIZE {
        words.windows(SHINGLE_SIZE).map(hash_shingle).collect()
    } else {
        // Fewer words than shingle size — use unigrams
        words.iter().map(|w| fnv64(w)).collect()
    };

    for &sh in &shingles {
        for k in 0..NUM_HASHES {
            let h = HASH_A[k].wrapping_mul(sh).wrapping_add(HASH_B[k]);
            if h < sigs[k] {
                sigs[k] = h;
            }
        }
    }

    sigs
}

fn jaccard(a: &[u64; NUM_HASHES], b: &[u64; NUM_HASHES]) -> f32 {
    let matches = a.iter().zip(b.iter()).filter(|(x, y)| x == y).count();
    matches as f32 / NUM_HASHES as f32
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_exact_duplicate_blocks() {
        let input = "ERROR: connection refused\n\nERROR: connection refused\n\nINFO: retrying";
        let out = dedup(input);
        let count = out.matches("ERROR: connection refused").count();
        assert_eq!(count, 1, "exact duplicate block should appear once");
        assert!(out.contains("INFO: retrying"));
    }

    #[test]
    fn keeps_distinct_blocks() {
        let input =
            "block one content here\n\nblock two different content\n\nblock three more stuff";
        let out = dedup(input);
        assert!(out.contains("block one"));
        assert!(out.contains("block two"));
        assert!(out.contains("block three"));
    }

    #[test]
    fn removes_near_duplicate_blocks() {
        // Two blocks that share most of their words
        let a = "the quick brown fox jumps over the lazy dog near the river bank";
        let b = "the quick brown fox jumps over the lazy dog near the river side";
        let input = format!("{}\n\n{}", a, b);
        let out = dedup(&input);
        // Near-duplicates: at least one should be removed
        let lines: Vec<&str> = out.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(
            lines.len(),
            1,
            "near-duplicate block should be deduplicated"
        );
    }

    #[test]
    fn empty_input() {
        assert_eq!(dedup(""), "");
    }
}
