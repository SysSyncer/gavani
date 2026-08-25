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

/// Load all stored sessions, synchronising the data file with disk state:
///
/// - **Missing file** → an empty store is created on disk, so
///   `~/.config/gavani/sessions.json` always exists after first run.
/// - **Corrupt file** (invalid JSON, truncated by a crash mid-write…) →
///   the file is **overwritten** with an empty store. We never panic or
///   refuse to start; the corrupt content is unrecoverable JSON anyway.
///
/// Any other IO failure (permissions, disk) yields an empty in-memory
/// store — the app must always start.
pub fn load() -> Store {
    let Some(path) = data_path() else {
        return Store::default();
    };
    match std::fs::read_to_string(&path) {
        Ok(data) => match serde_json::from_str(&data) {
            Ok(store) => store,
            Err(_) => {
                // Corrupt: overwrite with a fresh empty store.
                let fresh = Store::default();
                let _ = save(&fresh);
                fresh
            }
        },
        Err(_) => {
            // Missing: create the file so the data layout is materialised.
            let fresh = Store::default();
            let _ = save(&fresh);
            fresh
        }
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

// ---------------------------------------------------------------------------
// Active session persistence (crash / quit recovery)
// ---------------------------------------------------------------------------

/// Snapshot of an in-progress session, saved continuously so that if the
/// app is killed (or quit with `q`) mid-session, the next launch can
/// restore the timer in a paused state with the elapsed time intact.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ActiveSession {
    /// Wall-clock start of the session (for display).
    pub start: DateTime<Local>,
    /// Seconds focused so far (pause-adjusted, frozen at save time).
    pub elapsed: u64,
}

/// Path of the active-session file (…/gavani/active.json).
fn active_path() -> Option<std::path::PathBuf> {
    config_dir().map(|d| d.join("active.json"))
}

/// Load a saved in-progress session, if the last run left one behind.
pub fn load_active() -> Option<ActiveSession> {
    let path = active_path()?;
    std::fs::read_to_string(path)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
}

/// Persist the in-progress session (called every UI tick while running).
pub fn save_active(active: &ActiveSession) {
    let Some(path) = active_path() else { return };
    if let Ok(json) = serde_json::to_string(active) {
        let _ = std::fs::write(path, json);
    }
}

/// Remove the active-session file. Called when a session is recorded or
/// reset, so a stale snapshot is never restored later.
pub fn clear_active() {
    if let Some(path) = active_path() {
        let _ = std::fs::remove_file(path);
    }
}
