//! crossterm driver.

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use sp_core::AppState;

/// Runs the terminal UI event loop until the user quits (q, Esc, Ctrl-C).
pub fn run() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let state = AppState::default();

    let result = loop {
        terminal.draw(|frame| sp_ui::render(frame, &state))?;

        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break Ok(()),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    break Ok(());
                }
                _ => {}
            },
            _ => {}
        }
    };

    ratatui::restore();
    result
}
