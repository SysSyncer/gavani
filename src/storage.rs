//! Persistence layer for focus sessions.
//!
//! Sessions are stored as pretty-printed JSON in the user's platform config
//! directory (via the `dirs` crate), which resolves to:
//!
//!   Linux/BSD : ~/.config/gavani/
//!   macOS     : ~/Library/Application Support/gavani/
//!   Windows   : %APPDATA%\gavani\
//!
//! Design notes:
//! - We save the whole store on every mutation (start/stop/delete). Session
//!   counts are small, so simplicity beats incremental writes here.
//! - All IO errors are swallowed with `.ok()` on purpose: a broken disk or
//!   read-only home should never crash the stopwatch itself.

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

/// One continuous focus session ("lap"): when it started, when it ended,
/// and how long it lasted in seconds.
///
/// `duration_secs` is measured with a monotonic clock (`std::time::Instant`)
/// inside the app, so it stays correct even if the system wall-clock is
/// changed mid-session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Wall-clock time the session started (for display).
    pub start: DateTime<Local>,
    /// Wall-clock time the session ended (for display).
    pub end: DateTime<Local>,
    /// Total focused seconds (monotonic, pause-adjusted).
    #[serde(default)]
    pub duration_secs: u64,
}

/// The serializable root of the data file: just a list of sessions.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Store {
    pub sessions: Vec<Session>,
}

impl Session {
    /// Build a session from its start/end timestamps and measured duration.
    pub fn new(start: DateTime<Local>, end: DateTime<Local>, duration_secs: u64) -> Self {
        Self {
            start,
            end,
            duration_secs,
        }
    }
}

/// Platform-appropriate base directory for gavani's files (…/gavani).
pub fn config_dir() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("gavani"))
}

/// Full path of the sessions data file.
pub fn data_path() -> Option<std::path::PathBuf> {
    config_dir().map(|d| d.join("sessions.json"))
}

/// Load all stored sessions. Any failure (missing/corrupt file) yields an
/// empty store rather than an error — the app must always start.
pub fn load() -> Store {
    let Some(path) = data_path() else {
        return Store::default();
    };
    match std::fs::read_to_string(&path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => Store::default(),
    }
}

/// Write the whole store to disk, creating parent directories as needed.
pub fn save(store: &Store) -> std::io::Result<()> {
    let Some(path) = data_path() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(store)?;
    std::fs::write(path, data)
}
