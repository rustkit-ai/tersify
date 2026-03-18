//! Project and global configuration via `.tersify.toml`.
//!
//! Resolution order (first match wins):
//! 1. `.tersify.toml` in the current working directory
//! 2. `~/.tersify/config.toml` (global)
//!
//! Any value can be overridden by an explicit CLI flag.

use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub defaults: Defaults,
}

/// Default CLI flag values.
#[derive(Debug, Default, Deserialize)]
pub struct Defaults {
    /// Same as `--ast`
    #[serde(default)]
    pub ast: bool,

    /// Same as `--strip-docs`
    #[serde(default)]
    pub strip_docs: bool,

    /// Same as `--smart`
    #[serde(default)]
    pub smart: bool,

    /// Same as `--budget`
    pub budget: Option<usize>,
}

impl Config {
    /// Load config, preferring local over global, returning default if neither exists.
    pub fn load() -> Self {
        if let Some(cfg) = Self::load_from(Path::new(".tersify.toml")) {
            return cfg;
        }
        if let Ok(home) = std::env::var("HOME") {
            let global = PathBuf::from(home).join(".tersify").join("config.toml");
            if let Some(cfg) = Self::load_from(&global) {
                return cfg;
            }
        }
        Self::default()
    }

    fn load_from(path: &Path) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        toml::from_str(&content).ok()
    }
}
