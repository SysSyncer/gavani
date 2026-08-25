//! Configuration handling for gavani.
//!
//! gavani reads a small JSON config file located next to its data file:
//!
//!   Linux/BSD : ~/.config/gavani/config.json
//!   macOS     : ~/Library/Application Support/gavani/config.json
//!   Windows   : %APPDATA%\gavani\config.json
//!
//! If the file does not exist, gavani creates it with the defaults below,
//! so users always have a template they can edit.
//!
//! Future ideas that fit naturally into this file (see README):
//!   - pomodoro: { work_mins, break_mins, enabled }
//!   - notification_on_complete
//!   - confirm_before_reset / confirm_before_delete
//!   - export_format ("csv" | "json")
//!   - clock_show_seconds

use serde::{Deserialize, Serialize};

/// How timestamps are displayed in the session table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeFormat {
    /// e.g. 17:38:00
    #[serde(rename = "24h")]
    H24,
    /// e.g. 05:38 PM
    #[serde(rename = "12h")]
    H12,
}

/// Top-level application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Name of the color theme (must match a theme in theme::THEMES).
    /// One of: "tokyonight", "gruvbox", "dracula", "mono"
    pub theme: String,

    /// Clock style used for START/END columns.
    pub time_format: TimeFormat,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "tokyonight".to_string(),
            time_format: TimeFormat::H24,
        }
    }
}

impl Config {
    /// Format a chrono datetime according to the configured clock style.
    pub fn fmt_time(&self, t: &chrono::DateTime<chrono::Local>) -> String {
        match self.time_format {
            TimeFormat::H24 => t.format("%H:%M:%S").to_string(),
            TimeFormat::H12 => t.format("%I:%M:%S %p").to_string(),
        }
    }

    /// Load config from disk, writing the defaults on first launch.
    pub fn load() -> Self {
        let Some(path) = crate::storage::config_dir().map(|d| d.join("config.json")) else {
            return Config::default();
        };
        match std::fs::read_to_string(&path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
            Err(_) => {
                let cfg = Config::default();
                // Best effort: ignore write errors, app still works.
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Ok(json) = serde_json::to_string_pretty(&cfg) {
                    let _ = std::fs::write(&path, json);
                }
                cfg
            }
        }
    }

    /// Persist this config back to disk (e.g. after cycling themes).
    pub fn save(&self) {
        let Some(path) = crate::storage::config_dir().map(|d| d.join("config.json")) else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }

    /// Resolve the configured theme name to an index into theme::THEMES,
    /// falling back to the first theme when the name is unknown.
    pub fn theme_index(&self) -> usize {
        crate::theme::THEMES
            .iter()
            .position(|t| t.name == self.theme)
            .unwrap_or(0)
    }
}
