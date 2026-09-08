//! `sp-ui`'s own key representation. Hosts map their native key type (crossterm, ratzilla) into
//! `Key`/`Modifiers` before calling `key_to_intent`. No host key types may appear in this file
//! (CLAUDE.md invariant 2).

use sp_core::Intent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Backspace,
    Enter,
    /// Shift+Tab arrives as a separate crossterm variant (`BackTab`) but as `Tab` + shift on the
    /// web; hosts normalize both into `Tab` and set `Modifiers::shift` accordingly.
    Tab,
    Up,
    Down,
    Esc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Modifiers {
    pub const NONE: Self = Self {
        ctrl: false,
        alt: false,
        shift: false,
    };

    pub const CTRL: Self = Self {
        ctrl: true,
        alt: false,
        shift: false,
    };

    pub const SHIFT: Self = Self {
        ctrl: false,
        alt: false,
        shift: true,
    };
}

/// Pure: no I/O, no state. Same `(Key, Modifiers)` always yields the same `Intent`.
pub fn key_to_intent(key: Key, modifiers: Modifiers) -> Option<Intent> {
    match key {
        Key::Char('c') if modifiers.ctrl => Some(Intent::Quit),
        Key::Char(c) => Some(Intent::Char(c)),
        Key::Backspace => Some(Intent::Backspace),
        Key::Enter if modifiers.ctrl => Some(Intent::ForceSearch),
        Key::Enter => Some(Intent::Activate),
        Key::Tab if modifiers.shift => Some(Intent::Prev),
        // Tab accepts the ghost suggestion (T1.4), zsh/fish style. Next/Prev are
        // Up/Down; Tab is not a second binding for them.
        Key::Tab => Some(Intent::AcceptSuggestion),
        Key::Up => Some(Intent::Prev),
        Key::Down => Some(Intent::Next),
        Key::Esc => Some(Intent::Quit),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table() {
        let cases: &[(Key, Modifiers, Option<Intent>)] = &[
            (Key::Char('a'), Modifiers::NONE, Some(Intent::Char('a'))),
            (Key::Char('c'), Modifiers::CTRL, Some(Intent::Quit)),
            (Key::Backspace, Modifiers::NONE, Some(Intent::Backspace)),
            (Key::Enter, Modifiers::NONE, Some(Intent::Activate)),
            (Key::Enter, Modifiers::CTRL, Some(Intent::ForceSearch)),
            (Key::Tab, Modifiers::NONE, Some(Intent::AcceptSuggestion)),
            (Key::Tab, Modifiers::SHIFT, Some(Intent::Prev)),
            (Key::Up, Modifiers::NONE, Some(Intent::Prev)),
            (Key::Down, Modifiers::NONE, Some(Intent::Next)),
            (Key::Esc, Modifiers::NONE, Some(Intent::Quit)),
        ];

        for (key, modifiers, expected) in cases {
            assert_eq!(
                key_to_intent(*key, *modifiers),
                *expected,
                "key {key:?} modifiers {modifiers:?}"
            );
        }
    }
}
