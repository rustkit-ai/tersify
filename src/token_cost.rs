//! `tersify token-cost` — estimate LLM API cost before and after compression.

use anyhow::Result;
use std::{
    io::{self, Read},
    path::Path,
};
use tersify::{compress::CompressOptions, input, tokens};

use tersify::MODEL_PRICING;

pub fn run(inputs: &[String], forced_type: Option<&str>, model_filter: Option<&str>) -> Result<()> {
    let (before, after) = collect_tokens(inputs, forced_type)?;

    let saved_tokens = before.saturating_sub(after);
    let saved_pct = tokens::savings_pct(before, after);

    // Header
    println!(
        "\n  {} → {} tokens  ({:.0}% saved, {} tokens freed)\n",
        fmt_tokens(before),
        fmt_tokens(after),
        saved_pct,
        fmt_tokens(saved_tokens),
    );

    let models: Vec<_> = MODEL_PRICING
        .iter()
        .filter(|(name, _, _)| {
            model_filter
                .map(|f| name.to_lowercase().contains(&f.to_lowercase()))
                .unwrap_or(true)
        })
        .collect();

    if models.is_empty() {
        eprintln!("No model matched \"{}\".", model_filter.unwrap_or(""));
        return Ok(());
    }

    let col_model = 20usize;
    let col_prov = 12usize;
    let col_rate = 12usize;
    let col_cost = 14usize;
    let width = col_model + col_prov + col_rate + col_cost * 2 + col_cost + 6;

    println!(
        "  {:<col_model$}  {:<col_prov$}  {:>col_rate$}  {:>col_cost$}  {:>col_cost$}  {:>col_cost$}",
        "Model", "Provider", "$/M tokens", "Raw cost", "Compressed", "Saved/call"
    );
    println!("  {}", "─".repeat(width));

    let mut best_saving: f64 = 0.0;
    let mut best_model = "";

    for (name, provider, price_per_m) in &models {
        let cost_raw = cost_usd(before, *price_per_m);
        let cost_compressed = cost_usd(after, *price_per_m);
        let cost_saved = cost_raw - cost_compressed;

        if cost_saved > best_saving {
            best_saving = cost_saved;
            best_model = name;
        }

        println!(
            "  {:<col_model$}  {:<col_prov$}  {:>col_rate$}  {:>col_cost$}  {:>col_cost$}  {:>col_cost$}",
            name,
            provider,
            format!("${:.2}", price_per_m),
            format_cost(cost_raw),
            format_cost(cost_compressed),
            format!("-{}", format_cost(cost_saved)),
        );
    }

    println!("  {}", "─".repeat(width));

    // Monthly projection at 100 calls/day
    if model_filter.is_none() && best_saving > 0.0 {
        let daily = best_saving * 100.0;
        let monthly = daily * 30.0;
        println!(
            "\n  At 100 calls/day with {}: saves ${:.2}/day → ${:.2}/month",
            best_model, daily, monthly
        );
    }

    println!();
    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn collect_tokens(inputs: &[String], forced_type: Option<&str>) -> Result<(usize, usize)> {
    let opts = CompressOptions::default();

    if inputs.is_empty() {
        // Read from stdin
        if is_terminal_stdin() {
            anyhow::bail!(
                "tersify token-cost: no input.\n\n  cat file.rs | tersify token-cost\n  tersify token-cost src/"
            );
        }
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        let (_, before, after) = input::compress_content_with(&buf, forced_type, None, &opts)?;
        return Ok((before, after));
    }

    let mut total_before = 0usize;
    let mut total_after = 0usize;

    for path_str in inputs {
        let path = Path::new(path_str);
        if !path.exists() {
            anyhow::bail!("path not found: {}", path.display());
        }
        let (before, after) = if path.is_dir() {
            let (_, b, a) = input::compress_directory_with(path, forced_type, &opts)?;
            (b, a)
        } else {
            let (_, b, a) = input::compress_file_with(path, forced_type, &opts)?;
            (b, a)
        };
        total_before += before;
        total_after += after;
    }

    Ok((total_before, total_after))
}

fn cost_usd(token_count: usize, price_per_million: f64) -> f64 {
    token_count as f64 / 1_000_000.0 * price_per_million
}

fn format_cost(usd: f64) -> String {
    if usd < 0.0001 {
        format!("${:.6}", usd)
    } else {
        format!("${:.4}", usd)
    }
}

fn fmt_tokens(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        // Non-breaking thin space grouping — zero-pad the hundreds group
        format!("{} {:03}", n / 1_000, n % 1_000)
    } else {
        n.to_string()
    }
}

fn is_terminal_stdin() -> bool {
    use is_terminal::IsTerminal;
    std::io::stdin().is_terminal()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_usd_one_million_tokens() {
        let cost = cost_usd(1_000_000, 3.0);
        assert!((cost - 3.0).abs() < 1e-9);
    }

    #[test]
    fn cost_usd_half_million_at_15() {
        let cost = cost_usd(500_000, 15.0);
        assert!((cost - 7.5).abs() < 1e-9);
    }

    #[test]
    fn cost_usd_zero_tokens() {
        assert_eq!(cost_usd(0, 15.0), 0.0);
    }

    #[test]
    fn fmt_tokens_small() {
        assert_eq!(fmt_tokens(0), "0");
        assert_eq!(fmt_tokens(999), "999");
    }

    #[test]
    fn fmt_tokens_thousands() {
        assert_eq!(fmt_tokens(1_000), "1 000");
        assert_eq!(fmt_tokens(12_345), "12 345");
    }

    #[test]
    fn fmt_tokens_millions() {
        let s = fmt_tokens(1_500_000);
        assert!(s.ends_with('M'), "expected M suffix, got: {s}");
        assert!(s.starts_with("1.5"), "expected 1.5, got: {s}");
    }

    #[test]
    fn format_cost_large_uses_4_decimals() {
        let s = format_cost(0.1234);
        assert!(s.starts_with('$'));
        // Should have exactly 4 decimal digits
        let digits_after_dot = s.trim_start_matches('$').split('.').nth(1).unwrap_or("");
        assert_eq!(digits_after_dot.len(), 4);
    }

    #[test]
    fn format_cost_tiny_uses_6_decimals() {
        let s = format_cost(0.000001);
        assert!(s.starts_with('$'));
        let digits_after_dot = s.trim_start_matches('$').split('.').nth(1).unwrap_or("");
        assert_eq!(digits_after_dot.len(), 6);
    }
}
