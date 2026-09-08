//! crossterm driver.

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use sp_core::{reduce, AppState};
use sp_ui::{key_to_intent, Key, Modifiers};

/// Maps a crossterm key press into `sp-ui`'s own `Key`/`Modifiers`. `None` for keys the app
/// doesn't handle (function keys, page up/down, etc).
pub fn map_key(code: KeyCode, modifiers: KeyModifiers) -> Option<(Key, Modifiers)> {
    let mods = Modifiers {
        ctrl: modifiers.contains(KeyModifiers::CONTROL),
        alt: modifiers.contains(KeyModifiers::ALT),
        shift: modifiers.contains(KeyModifiers::SHIFT),
    };
    let key = match code {
        KeyCode::Char(c) => Key::Char(c),
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Enter => Key::Enter,
        // Terminals report shift+tab as a distinct BackTab keycode; normalize to Tab+shift so
        // both hosts converge on the same sp_ui::Key.
        KeyCode::BackTab => {
            return Some((
                Key::Tab,
                Modifiers {
                    shift: true,
                    ..mods
                },
            ))
        }
        KeyCode::Tab => Key::Tab,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Esc => Key::Esc,
        _ => return None,
    };
    Some((key, mods))
}

/// Runs the terminal UI event loop until the user quits (Esc, Ctrl-C).
pub fn run() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let mut state = AppState::default();

    let result = loop {
        terminal.draw(|frame| sp_ui::render(frame, &state))?;

        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
            && let Some((mapped_key, modifiers)) = map_key(key.code, key.modifiers)
            && let Some(intent) = key_to_intent(mapped_key, modifiers)
        {
            reduce(&mut state, intent);
            if state.should_quit {
                break Ok(());
            }
        }
    };

    ratatui::restore();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_backtab_to_shift_tab() {
        let (key, modifiers) = map_key(KeyCode::BackTab, KeyModifiers::NONE).unwrap();
        assert_eq!(key, Key::Tab);
        assert!(modifiers.shift);
    }

    #[test]
    fn maps_ctrl_c_quit() {
        let (key, modifiers) = map_key(KeyCode::Char('c'), KeyModifiers::CONTROL).unwrap();
        assert_eq!(key_to_intent(key, modifiers), Some(sp_core::Intent::Quit));
    }

    #[test]
    fn unhandled_key_is_none() {
        assert_eq!(map_key(KeyCode::PageUp, KeyModifiers::NONE), None);
    }
}
