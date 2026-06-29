pub mod theme;

use error::log_to_file;
use std::{fs, path::PathBuf};
use theme::Theme;

#[derive(Debug, Clone)]
pub struct Config {
    pub theme: Theme,
    pub background: bool,
    pub seek_seconds: u8,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            theme: Theme::catppuccin_mocha(),
            background: true,
            seek_seconds: 5,
        }
    }
}

const VALID_KEYS: &[&str] = &["theme", "background", "seek_seconds"];

impl Config {
    fn path() -> PathBuf {
        dirs::config_dir()
            .map(|p| p.join("gytm/config.toml"))
            .unwrap_or_else(|| PathBuf::from("config.toml"))
    }

    pub fn load() -> Self {
        let path = Self::path();
        if !path.exists() {
            return Config::default();
        }

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                log_to_file(format!("Cannot read config: {}", e));
                return Config::default();
            }
        };

        let table: serde_json::Value = match basic_toml::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                log_to_file(format!("Config parse error at {}: {}", path.display(), e));
                return Config::default();
            }
        };

        let table = match table.as_object() {
            Some(t) => t,
            None => return Config::default(),
        };

        for key in table.keys() {
            if !VALID_KEYS.contains(&key.as_str()) {
                log_to_file(format!("Unknown config key: {}", key));
            }
        }

        Config {
            theme: Theme::from_name(
                table
                    .get("theme")
                    .and_then(|v| v.as_str())
                    .unwrap_or("catppuccin_mocha"),
            ),

            background: table
                .get("background")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),

            seek_seconds: table
                .get("seek_seconds")
                .and_then(|v| v.as_i64())
                .map(|n| n as u8)
                .filter(|&n| (1..=60).contains(&n))
                .unwrap_or(5),
        }
    }
}
