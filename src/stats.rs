use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use tersify::error::{Result, TersifyError};

use tersify::MODEL_PRICING;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LangStat {
    pub tokens_before: u64,
    pub tokens_after: u64,
}

impl LangStat {
    fn saved(&self) -> u64 {
        self.tokens_before.saturating_sub(self.tokens_after)
    }

    fn pct(&self) -> f64 {
        if self.tokens_before == 0 {
            return 0.0;
        }
        self.saved() as f64 / self.tokens_before as f64 * 100.0
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Stats {
    pub total_invocations: u64,
    pub total_tokens_before: u64,
    pub total_tokens_after: u64,
    #[serde(default)]
    pub by_language: HashMap<String, LangStat>,
}

impl Stats {
    pub fn tokens_saved(&self) -> u64 {
        self.total_tokens_before
            .saturating_sub(self.total_tokens_after)
    }

    pub fn savings_pct(&self) -> f64 {
        if self.total_tokens_before == 0 {
            return 0.0;
        }
        self.tokens_saved() as f64 / self.total_tokens_before as f64 * 100.0
    }
}

/// Record a single compression event (totals only, no language breakdown).
#[allow(dead_code)]
pub fn record(tokens_before: usize, tokens_after: usize) -> Result<()> {
    record_with_lang(tokens_before, tokens_after, None)
}

/// Record a compression event with language info.
pub fn record_with_lang(
    tokens_before: usize,
    tokens_after: usize,
    lang: Option<&str>,
) -> Result<()> {
    let path = stats_path()?;
    let mut stats = load(&path).unwrap_or_default();
    stats.total_invocations += 1;
    stats.total_tokens_before += tokens_before as u64;
    stats.total_tokens_after += tokens_after as u64;
    if let Some(l) = lang {
        let entry = stats.by_language.entry(l.to_string()).or_default();
        entry.tokens_before += tokens_before as u64;
        entry.tokens_after += tokens_after as u64;
    }
    save(&path, &stats)
}

pub fn reset() -> anyhow::Result<()> {
    let path = stats_path()?;
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    println!("✓ Stats reset.");
    Ok(())
}

pub fn run_json() -> anyhow::Result<()> {
    let path = stats_path()?;
    let stats = load(&path).unwrap_or_default();
    println!("{}", serde_json::to_string_pretty(&stats)?);
    Ok(())
}

pub fn run() -> anyhow::Result<()> {
    let path = stats_path()?;
    let stats = load(&path).unwrap_or_default();

    if stats.total_invocations == 0 {
        println!("No data yet — compress some files first:");
        println!("  cat file.rs | tersify");
        println!("  tersify src/");
        return Ok(());
    }

    let saved = stats.tokens_saved();
    let pct = stats.savings_pct();

    println!();
    println!("  tersify — token savings");
    println!("  ─────────────────────────────────────────");
    println!("  Compressions : {}", fmt_num(stats.total_invocations));
    println!("  Tokens in    : {}", fmt_num(stats.total_tokens_before));
    println!("  Tokens out   : {}", fmt_num(stats.total_tokens_after));
    println!("  Tokens saved : {}  ({:.0}% smaller)", fmt_num(saved), pct);

    // Dollar cost savings
    println!();
    println!("  Cost saved (what you didn't pay for):");
    for (model, _provider, price_per_m) in MODEL_PRICING {
        let saved_usd = saved as f64 / 1_000_000.0 * price_per_m;
        println!(
            "    {:<22} {:>8}   → {:>16}",
            model,
            format!("${:.2}/M", price_per_m),
            fmt_usd(saved_usd)
        );
    }

    // By language
    if !stats.by_language.is_empty() {
        println!();
        println!("  By language:");
        let mut langs: Vec<(&String, &LangStat)> = stats.by_language.iter().collect();
        langs.sort_by(|a, b| b.1.saved().cmp(&a.1.saved()));
        for (lang, ls) in langs {
            let lang_usd = ls.saved() as f64 / 1_000_000.0 * MODEL_PRICING[1].2; // sonnet pricing
            println!(
                "    {:<16} {:>9} → {:>9}  ({:.0}%)   {:>16}",
                lang,
                fmt_num(ls.tokens_before),
                fmt_num(ls.tokens_after),
                ls.pct(),
                fmt_usd(lang_usd),
            );
        }
        println!();
        println!("  * cost column uses claude-sonnet-4.6 ($3.00/M) as reference");
    }

    println!();
    Ok(())
}

// ── Formatting helpers ────────────────────────────────────────────────────────

fn fmt_num(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

fn fmt_usd(usd: f64) -> String {
    if usd >= 1.0 {
        format!("${:.2} saved", usd)
    } else if usd >= 0.001 {
        format!("${:.4} saved", usd)
    } else {
        format!("${:.6} saved", usd)
    }
}

// ── Persistence ───────────────────────────────────────────────────────────────

fn stats_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").map_err(|_| TersifyError::Stats("$HOME not set".into()))?;
    let dir = PathBuf::from(home).join(".tersify");
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("stats.json"))
}

fn load(path: &PathBuf) -> Option<Stats> {
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

fn save(path: &PathBuf, stats: &Stats) -> Result<()> {
    let data = serde_json::to_string_pretty(stats).map_err(TersifyError::InvalidJson)?;
    std::fs::write(path, data)?;
    Ok(())
}
