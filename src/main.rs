//! gavani — a focus stopwatch TUI.
//!
//! Module map (see README.md for the full architecture tour):
//!   - `config`  : user preferences loaded from config.json
//!   - `theme`   : color palettes referenced by name from config
//!   - `app`     : state machine + business logic (no rendering)
//!   - `storage` : JSON persistence for session history
//!   - `ui`      : ratatui view code, one frame at a time
//!
//! The event loop below is intentionally tiny: it only translates key events
//! into method calls on `App` and asks `ui::draw` to paint. All decisions
//! live in `App`, which is what makes the logic unit-testable.

mod app;
mod config;
mod storage;
mod theme;
mod ui;

use std::{io, time::Duration};

use app::App;
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, poll, read};

fn main() -> io::Result<()> {
    // ratatui::init/restore handle raw mode + the alternate screen buffer,
    // restoring your shell even if we return early with an error.
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

/// Main loop: draw → poll for input (max 250 ms) → dispatch keys.
///
/// The 250 ms poll doubles as our frame rate: while idle nothing happens,
/// but the timer redraws at least 4×/second so the clock stays smooth.
fn run(terminal: &mut ratatui::DefaultTerminal) -> io::Result<()> {
    let mut app = App::default();

    while !app.should_quit {
        // Snapshot the running timer every tick (~4x/sec) so a crash or
        // `q` mid-session never loses more than a fraction of a second.
        app.persist_active();

        terminal.draw(|f| ui::draw(f, &app))?;

        if poll(Duration::from_millis(250))? {
            match read()? {
                Event::Key(key) => {
                    // Only react to Press events; Windows also emits Release.
                    if key.kind == KeyEventKind::Press {
                        handle_key(&mut app, key.code, key.modifiers);
                    }
                }
                Event::Resize(_, _) => {} // next loop iteration repaints
                _ => {}
            }
        }
    }

    // Final flush of history on quit.
    storage::save(&app.store()).ok();
    Ok(())
}

/// Translate one keypress into an `App` mutation. Kept separate from `run`
/// so the mapping is readable at a glance.
fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    match code {
        // Quit (bare q, or Ctrl+C anywhere).
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => app.should_quit = true,

        // Help popup.
        KeyCode::Char('?') => app.show_help = !app.show_help,

        // Theme cycling; App persists the choice to config.json itself.
        KeyCode::Char('t') => app.next_theme(),

        // Start / stop-and-record the current focus session.
        KeyCode::Char('s') => {
            app.toggle();
            storage::save(&app.store()).ok(); // save-on-mutation: crash safe
        }

        // Pause / resume without ending the session.
        KeyCode::Char('p') => app.pause_resume(),

        // Reset: throw away the running session without recording it.
        KeyCode::Char('r') => app.reset(),

        // History navigation and deletion.
        KeyCode::Char('j') | KeyCode::Down => app.move_selection(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_selection(-1),
        KeyCode::Char('d') => {
            app.delete_selected();
            storage::save(&app.store()).ok();
        }

        _ => {}
    }
}
