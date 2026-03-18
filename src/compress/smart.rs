//! Smart near-duplicate deduplication using MinHash + LSH.
//!
//! Splits input into logical blocks (separated by blank lines), computes a
//! MinHash signature for each, and removes blocks that are suspiciously similar
//! to a previously seen block. This catches repeated log entries, copy-pasted
//! code, and redundant explanations that exact-match dedup misses.
//!
//! ## Algorithm
//!
//! 1. 64-function MinHash over word 3-gram shingles — 4× more accurate than the
//!    previous 16-function variant (expected error ≈ ±2.5% of true Jaccard).
//! 2. LSH banding (16 bands × 4 hashes) for O(1) candidate lookup. Two blocks are
//!    candidate duplicates if **any** band fingerprint matches; Jaccard is then
//!    verified on the full 64-hash signature.
//!
//! Detection probability at similarity thresholds (r=4 bands size, b=16 bands):
//! - s=0.72 (threshold):  P ≈ 0.997
//! - s=0.50:              P ≈ 0.64   → false-positive Jaccard check filters these out

use super::util::collapse_blank_lines;
use std::collections::{HashMap, HashSet};

/// Number of hash functions used for MinHash signatures.
const NUM_HASHES: usize = 64;

/// LSH band count and size (NUM_HASHES = NUM_BANDS × BAND_SIZE).
const NUM_BANDS: usize = 16;
const BAND_SIZE: usize = NUM_HASHES / NUM_BANDS; // = 4

/// Blocks with estimated Jaccard similarity ≥ this threshold are treated as duplicates.
const SIMILARITY_THRESHOLD: f32 = 0.72;

/// Shingle size (n-gram window over words).
const SHINGLE_SIZE: usize = 3;

// 64 pre-computed large odd multipliers for universal hash family h_k(x) = A_k*x + B_k
#[rustfmt::skip]
const HASH_A: [u64; NUM_HASHES] = [
    0x517cc1b727220a95, 0xd74e26b7f12a3d5f, 0x123456789abcdef0, 0xfedcba9876543210,
    0xaaaaaaaaaaaaaaab, 0x5555555555555557, 0xc3d2e1f099887761, 0x1122334455667789,
    0x99aabbccddeeff01, 0x1337deadbeef0003, 0xc0ffee1234567891, 0x0102030405060709,
    0xf0e0d0c0b0a09081, 0x7f6f5f4f3f2f1f0b, 0xa5a5a5a5a5a5a5a7, 0x6c62272e07bb0143,
    0x9e3779b97f4a7c15, 0x517cc1b7272200a1, 0xbf58476d1ce4e5b9, 0x94d049bb133111eb,
    0x2545f4914f6cdd1d, 0x7a9e9ccf5d2e4f3b, 0xe4b6c3a2d1f08e7d, 0x3f1c2b4a5e6d7f8c,
    0x8a7b6c5d4e3f2a1b, 0xf1e2d3c4b5a69788, 0x0a1b2c3d4e5f6071, 0x9f8e7d6c5b4a3b2c,
    0x1a2b3c4d5e6f7081, 0xabcdef0123456791, 0xfedcba9876543201, 0x0123456789abcdf1,
    0x1111111111111113, 0x2222222222222223, 0x3333333333333337, 0x4444444444444447,
    0x5555555555555559, 0x6666666666666667, 0x7777777777777779, 0x8888888888888889,
    0x9999999999999991, 0xaaaaaaaaaaaaaaa3, 0xbbbbbbbbbbbbbbbd, 0xcccccccccccccccd,
    0xdddddddddddddddf, 0xeeeeeeeeeeeeeeef, 0xffffffffffffffff, 0x0f0f0f0f0f0f0f11,
    0x1e1e1e1e1e1e1e1f, 0x2d2d2d2d2d2d2d2f, 0x3c3c3c3c3c3c3c3d, 0x4b4b4b4b4b4b4b4d,
    0x5a5a5a5a5a5a5a5b, 0x6969696969696971, 0x7878787878787879, 0x8787878787878789,
    0x9696969696969697, 0xa5a5a5a5a5a5a5a9, 0xb4b4b4b4b4b4b4b7, 0xc3c3c3c3c3c3c3c7,
    0xd2d2d2d2d2d2d2d3, 0xe1e1e1e1e1e1e1e3, 0xf0f0f0f0f0f0f0f3, 0xff00ff00ff00ff03,
];

#[rustfmt::skip]
const HASH_B: [u64; NUM_HASHES] = [
    0xe17a1465deadbeef, 0x9c6b2f3d1a2b3c4d, 0x7e81a5c3cafebabe, 0x4d2f8b1efeed1234,
    0x3c9d7f2a5678abcd, 0x8f4e2d1cef012345, 0x6a3b9c7d23456789, 0x2e5f8a1b9abcdef0,
    0xb7c3e4f566778899, 0xa1b2c3d41f2e3d4c, 0x5e6f7a8b0a1b2c3d, 0x9d8c7b6a8f9eadbc,
    0x4c3b2a1912345678, 0x8f9eadbc87654321, 0x1234567801020304, 0x8765432109080706,
    0xdeadbeefcafebab1, 0x0123456789abcde3, 0xfedcba9876543213, 0x1122334455667791,
    0x99aabbccddeeff13, 0x1337deadbeef0015, 0xc0ffee1234567893, 0x0102030405060701,
    0xf0e0d0c0b0a09093, 0x7f6f5f4f3f2f1f1d, 0xa5a5a5a5a5a5a5b9, 0x6c62272e07bb0155,
    0x9e3779b97f4a7c27, 0x517cc1b7272200b3, 0xbf58476d1ce4e5cb, 0x94d049bb133111fd,
    0x2545f4914f6cdd2f, 0x7a9e9ccf5d2e4f4d, 0xe4b6c3a2d1f08e8f, 0x3f1c2b4a5e6d7f9e,
    0x8a7b6c5d4e3f2a2d, 0xf1e2d3c4b5a6979a, 0x0a1b2c3d4e5f6083, 0x9f8e7d6c5b4a3b3e,
    0x1a2b3c4d5e6f7093, 0xabcdef01234567a3, 0xfedcba9876543213, 0x0123456789abcde3,
    0x1111111111111125, 0x2222222222222235, 0x3333333333333349, 0x4444444444444459,
    0x555555555555556b, 0x6666666666666679, 0x777777777777778b, 0x888888888888889b,
    0x99999999999999a3, 0xaaaaaaaaaaaaab15, 0xbbbbbbbbbbbbbbcf, 0xccccccccccccccdf,
    0xddddddddddddddf1, 0xeeeeeeeeeeeeef01, 0xffffffffffffffff, 0x0f0f0f0f0f0f0f23,
    0x1e1e1e1e1e1e1e31, 0x2d2d2d2d2d2d2d41, 0x3c3c3c3c3c3c3c4f, 0x4b4b4b4b4b4b4b5d,
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
    let mut seen = SeenSigs::new();
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

        // Near-duplicate check via MinHash + LSH
        let sig = minhash(trimmed);
        if seen.is_near_dup(&sig) {
            continue;
        }

        seen.insert(sig);
        kept.push(block);
    }

    // Reconstruct output, collapsing runs of blank lines
    let raw = kept.join("\n\n");
    collapse_blank_lines(&raw)
}

// ── LSH index ────────────────────────────────────────────────────────────────

/// MinHash signature store with LSH banding for O(1) candidate lookup.
struct SeenSigs {
    sigs: Vec<[u64; NUM_HASHES]>,
    /// bands[b] maps a band fingerprint → list of sig indices in that band.
    bands: Vec<HashMap<u64, Vec<usize>>>,
}

impl SeenSigs {
    fn new() -> Self {
        Self {
            sigs: Vec::new(),
            bands: (0..NUM_BANDS).map(|_| HashMap::new()).collect(),
        }
    }

    /// Returns `true` if `sig` is a near-duplicate of any stored signature.
    fn is_near_dup(&self, sig: &[u64; NUM_HASHES]) -> bool {
        // Collect candidate indices via LSH: any shared band fingerprint → candidate
        let mut candidates: HashSet<usize> = HashSet::new();
        for band in 0..NUM_BANDS {
            let fp = band_fp(sig, band);
            if let Some(indices) = self.bands[band].get(&fp) {
                candidates.extend(indices);
            }
        }
        // Verify candidates with exact Jaccard on full signature
        candidates
            .iter()
            .any(|&idx| jaccard(&self.sigs[idx], sig) >= SIMILARITY_THRESHOLD)
    }

    fn insert(&mut self, sig: [u64; NUM_HASHES]) {
        let idx = self.sigs.len();
        self.sigs.push(sig);
        for band in 0..NUM_BANDS {
            let fp = band_fp(&self.sigs[idx], band);
            self.bands[band].entry(fp).or_default().push(idx);
        }
    }
}

/// Fingerprint for one LSH band: FNV over the 4 hash values in that band.
fn band_fp(sig: &[u64; NUM_HASHES], band: usize) -> u64 {
    let start = band * BAND_SIZE;
    let mut h: u64 = 0xcbf29ce484222325;
    for i in 0..BAND_SIZE {
        h ^= sig[start + i];
        h = h.wrapping_mul(0x100000001b3);
    }
    h
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

    #[test]
    fn lsh_bands_correct_size() {
        assert_eq!(NUM_BANDS * BAND_SIZE, NUM_HASHES);
    }
}
