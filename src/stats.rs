use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use tersify::error::{Result, TersifyError};

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

pub fn run() -> anyhow::Result<()> {
    let path = stats_path()?;
    let stats = load(&path).unwrap_or_default();

    if stats.total_invocations == 0 {
        println!("No data yet. Run tersify on some files first.");
        return Ok(());
    }

    println!("tersify stats");
    println!("─────────────────────────────────────────────────");
    println!("  Invocations  : {}", stats.total_invocations);
    println!("  Tokens in    : {}", stats.total_tokens_before);
    println!("  Tokens out   : {}", stats.total_tokens_after);
    println!(
        "  Saved        : {} ({:.0}%)",
        stats.tokens_saved(),
        stats.savings_pct()
    );

    if !stats.by_language.is_empty() {
        println!();
        println!("  By language:");
        // Sort by tokens saved descending
        let mut langs: Vec<(&String, &LangStat)> = stats.by_language.iter().collect();
        langs.sort_by(|a, b| b.1.saved().cmp(&a.1.saved()));
        for (lang, ls) in langs {
            println!(
                "    {:<14} {:>8} → {:>8}  ({:.0}%)",
                lang,
                ls.tokens_before,
                ls.tokens_after,
                ls.pct()
            );
        }
    }

    Ok(())
}

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
