//! crossterm driver.

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use sp_core::{reduce, AppState, Effect};
use sp_ui::{key_to_intent, Key, Modifiers};

/// Performs a side effect returned by `reduce`. Exhaustive on purpose (no `_` arm): a new
/// `Effect` variant must fail this match at compile time rather than being silently dropped.
fn handle_effect(effect: Effect) {
    match effect {
        Effect::OpenUrl(url) => {
            if let Err(err) = open::that(&url) {
                eprintln!("failed to open {url}: {err}");
            }
        }
    }
}

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
///
/// Panic safety: `ratatui::init()` installs a panic hook that calls `ratatui::restore()` before
/// the previous hook runs (see `ratatui::init::set_panic_hook`), so a panic anywhere in this
/// process restores the terminal for free — no extra hook needed here.
///
/// Real SIGINT (`kill -INT <pid>`, as opposed to a Ctrl+C keypress): raw mode clears the tty's
/// ISIG flag, so a Ctrl+C keypress never reaches the OS as a signal — it arrives as a normal key
/// event and is handled by `key_to_intent` -> `Intent::Quit` below. An externally sent SIGINT
/// bypasses the tty entirely and, uncaught, would kill the process mid-raw-mode. `ctrlc` installs
/// a real signal handler for exactly that case.
pub fn run() -> std::io::Result<()> {
    ctrlc::set_handler(|| {
        ratatui::restore();
        std::process::exit(130);
    })
    .expect("failed to install SIGINT handler");

    let mut terminal = ratatui::init();
    let mut state = AppState::default();

    let result = loop {
        terminal.draw(|frame| sp_ui::render(frame, &state))?;

        match event::read()? {
            Event::Key(key)
                if key.kind == KeyEventKind::Press
                    && let Some((mapped_key, modifiers)) = map_key(key.code, key.modifiers)
                    && let Some(intent) = key_to_intent(mapped_key, modifiers) =>
            {
                for effect in reduce(&mut state, intent) {
                    handle_effect(effect);
                }
                if state.should_quit {
                    break Ok(());
                }
            }
            // Redundant with the unconditional `terminal.draw` at the top of the loop (ratatui
            // re-queries terminal size and autoresizes every frame), but explicit so the next
            // iteration's redraw is clearly a response to this event, not a coincidence.
            Event::Resize(_, _) => {}
            _ => {}
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
