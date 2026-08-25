//! Application state and business logic.
//!
//! This module is deliberately free of any rendering code: it only models
//! *what* the app knows (current timer state, session history, selected row,
//! theme, config). The `ui` module decides *how* that knowledge is drawn.
//! That separation (state vs. view) makes the logic unit-testable without a
//! terminal.
//!
//! # Timing model
//!
//! Durations come from `std::time::Instant`, which is monotonic — it can
//! never go backwards even if the user changes their system clock. Wall-clock
//! timestamps (`chrono::Local`) are stored purely for display.
//!
//! A session accumulates time across pause/resume cycles:
//!
//!   Focusing { accumulated } --pause--> Paused { elapsed }
//!        ^                                    |
//!        +----------resume--------------------+
//!
//! On stop, total = accumulated + (Instant::now() - instant).

use std::time::Instant;

use chrono::{DateTime, Local};

use crate::config::Config;
use crate::storage::{self, Session};
use crate::theme::Theme;

/// The focus timer's finite state machine.
#[derive(Debug)]
pub enum State {
    /// No active session.
    Idle,
    /// Actively counting. `accumulated` holds seconds earned before the most
    /// recent resume; `instant` was reset at every resume.
    Focusing {
        started_at: DateTime<Local>,
        instant: Instant,
        accumulated: u64,
    },
    /// Timer frozen, holding the seconds accumulated so far.
    Paused {
        started_at: DateTime<Local>,
        elapsed: u64,
    },
}

impl State {
    /// Wall-clock start of the current (or paused) session, if any.
    pub fn started_at(&self) -> Option<DateTime<Local>> {
        match self {
            State::Focusing { started_at, .. } | State::Paused { started_at, .. } => {
                Some(*started_at)
            }
            State::Idle => None,
        }
    }
}

/// Everything the UI needs to know. One instance lives for the app's lifetime.
pub struct App {
    /// Current position in the timer state machine.
    pub state: State,

    /// Focus history, newest first.
    pub sessions: Vec<Session>,

    /// Highlighted row in the sessions table (index into `sessions`).
    pub selected: Option<usize>,

    /// Set to true when the event loop should exit.
    pub should_quit: bool,

    /// Whether the help popup is visible.
    pub show_help: bool,

    /// Index into `theme::THEMES`.
    pub theme_idx: usize,

    /// User configuration loaded from config.json at startup.
    pub config: Config,
}

impl Default for App {
    fn default() -> Self {
        let config = Config::load();
        // Crash/quit recovery: if the previous run closed while a session
        // was in progress (running OR paused), restore it as paused so no
        // focus time is silently lost.
        let state = match crate::storage::load_active() {
            Some(active) => State::Paused {
                started_at: active.start,
                elapsed: active.elapsed,
            },
            None => State::Idle,
        };
        Self {
            state,
            sessions: crate::storage::load().sessions,
            selected: None,
            should_quit: false,
            show_help: false,
            theme_idx: config.theme_index(),
            config,
        }
    }
}

impl App {
    /// Currently active color theme.
    pub fn theme(&self) -> &'static Theme {
        &crate::theme::THEMES[self.theme_idx]
    }

    /// Switch to the next theme and remember it in config.json.
    pub fn next_theme(&mut self) {
        self.theme_idx = (self.theme_idx + 1) % crate::theme::THEMES.len();
        self.config.theme = self.theme().name.to_string();
        self.config.save();
    }

    /// Start a new session if idle, otherwise stop (record) the current one.
    ///
    /// This single entry point backs the `s` keybinding.
    pub fn toggle(&mut self) {
        match &self.state {
            State::Idle => {
                self.state = State::Focusing {
                    started_at: Local::now(),
                    instant: Instant::now(),
                    accumulated: 0,
                };
            }
            _ => self.stop(),
        }
    }

    /// Freeze the running timer, or unfreeze a paused one. No-op when idle.
    pub fn pause_resume(&mut self) {
        match std::mem::replace(&mut self.state, State::Idle) {
            // Focusing -> Paused: bank the seconds counted since last resume.
            State::Focusing {
                started_at,
                instant,
                accumulated,
            } => {
                let elapsed = accumulated + instant.elapsed().as_secs();
                self.state = State::Paused {
                    started_at,
                    elapsed,
                };
            }
            // Paused -> Focusing: restart the monotonic clock from zero but
            // carry over everything banked so far.
            State::Paused {
                started_at,
                elapsed,
            } => {
                self.state = State::Focusing {
                    started_at,
                    instant: Instant::now(),
                    accumulated: elapsed,
                };
            }
            State::Idle => {}
        }
    }

    /// Abandon the current session entirely: nothing gets recorded.
    /// No-op when idle. Backs the `r` keybinding.
    pub fn reset(&mut self) {
        if !matches!(self.state, State::Idle) {
            self.state = State::Idle;
        }
    }

    /// Record the current session into history and return to Idle.
    fn stop(&mut self) {
        // Pull (total focused secs, started_at) out of whichever state we're in.
        let (started_at, total) = match &self.state {
            State::Focusing {
                started_at,
                instant,
                accumulated,
            } => (*started_at, accumulated + instant.elapsed().as_secs()),
            State::Paused {
                started_at,
                elapsed,
            } => (*started_at, *elapsed),
            State::Idle => return,
        };
        let session = Session::new(started_at, Local::now(), total);
        self.sessions.insert(0, session); // newest first
        self.selected = Some(0);
        self.state = State::Idle;
        storage::clear_active(); // snapshot recorded; don't restore it later
    }

    /// Save a snapshot of the in-progress session (if any) so the timer
    /// survives a crash or `q` while running/paused. Called every UI tick:
    /// a hard kill loses at most ~250 ms of counted time.
    ///
    /// A *running* session is saved as elapsed-frozen; on restore it comes
    /// back paused (we can't keep counting while the process is dead).
    pub fn persist_active(&self) {
        match self.timer() {
            Some((elapsed, _paused)) => {
                if let Some(start) = self.state.started_at() {
                    storage::save_active(&storage::ActiveSession { start, elapsed });
                }
            }
            // Idle: make sure no stale snapshot survives a stop/reset.
            None => storage::clear_active(),
        }
    }

    /// `(seconds on the clock, is_paused)` while a session exists.
    pub fn timer(&self) -> Option<(u64, bool)> {
        match &self.state {
            State::Focusing {
                instant,
                accumulated,
                ..
            } => Some((*accumulated + instant.elapsed().as_secs(), false)),
            State::Paused { elapsed, .. } => Some((*elapsed, true)),
            State::Idle => None,
        }
    }

    /// Remove the highlighted session from history.
    pub fn delete_selected(&mut self) {
        if let Some(i) = self.selected
            && i < self.sessions.len()
        {
            self.sessions.remove(i);
            if self.sessions.is_empty() {
                self.selected = None;
            } else {
                // Keep selection in bounds after removal.
                self.selected = Some(i.min(self.sessions.len() - 1));
            }
        }
    }

    /// Move the table highlight up (-1) or down (+1), clamped to the list.
    pub fn move_selection(&mut self, delta: i64) {
        if self.sessions.is_empty() {
            return;
        }
        let len = self.sessions.len() as i64;
        let cur = self.selected.map(|s| s as i64).unwrap_or(0);
        let next = (cur + delta).clamp(0, len - 1) as usize;
        self.selected = Some(next);
    }

    /// Snapshot the current sessions for persistence.
    pub fn store(&self) -> crate::storage::Store {
        crate::storage::Store {
            sessions: self.sessions.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_app() -> App {
        App {
            sessions: vec![],
            ..App::default()
        }
    }

    #[test]
    fn pause_resume_stop_flow() {
        let mut app = fresh_app();
        assert!(matches!(app.state, State::Idle));

        app.toggle();
        assert!(matches!(app.state, State::Focusing { .. }));

        app.pause_resume();
        let State::Paused { elapsed, .. } = &app.state else {
            panic!("expected paused");
        };
        assert_eq!(*elapsed, 0);

        app.pause_resume();
        assert!(matches!(app.state, State::Focusing { .. }));

        app.toggle();
        assert_eq!(app.sessions.len(), 1);
        assert!(matches!(app.state, State::Idle));
    }

    #[test]
    fn zero_duration_session_recorded() {
        let mut app = fresh_app();
        app.toggle();
        app.pause_resume(); // paused at 0s
        app.toggle();
        assert_eq!(app.sessions.len(), 1);
        assert!(matches!(app.state, State::Idle));
    }

    #[test]
    fn reset_discards_running_session() {
        let mut app = fresh_app();
        app.toggle();
        app.reset();
        assert!(matches!(app.state, State::Idle));
        assert!(app.sessions.is_empty());
        // Resetting again while idle must be a safe no-op.
        app.reset();
        assert!(matches!(app.state, State::Idle));
    }

    #[test]
    fn theme_cycles_and_persists() {
        let mut app = fresh_app();
        let start = app.theme_idx;
        app.next_theme();
        assert_eq!(app.theme_idx, (start + 1) % crate::theme::THEMES.len());
        assert_eq!(app.config.theme, app.theme().name);
    }

    #[test]
    fn delete_keeps_selection_in_bounds() {
        use chrono::TimeZone;
        let t = Local.with_ymd_and_hms(2026, 8, 25, 10, 0, 0).unwrap();
        let mk = || Session::new(t, t, 60);
        let mut app = App {
            sessions: vec![mk(), mk(), mk()],
            selected: Some(2),
            ..App::default()
        };
        app.delete_selected(); // remove last; selection clamps to index 1
        assert_eq!(app.selected, Some(1));
        app.delete_selected();
        app.delete_selected();
        assert_eq!(app.selected, None);
    }

    #[test]
    fn active_session_survives_restart_as_paused() {
        // Simulate: app quits while a session is running.
        let mut app = fresh_app();
        app.toggle();
        app.persist_active();

        // A brand-new App instance (like relaunching the binary) must
        // restore the session in the paused state with elapsed time kept.
        let restored = App {
            sessions: vec![],
            ..App::default()
        };
        let State::Paused { elapsed, .. } = restored.state else {
            panic!("expected restored session to be paused");
        };
        assert_eq!(elapsed, 0);

        // Stopping the restored session records it and clears the snapshot.
        let mut restored = restored;
        restored.toggle();
        assert_eq!(restored.sessions.len(), 1);
        assert!(crate::storage::load_active().is_none());
    }
}
