//! `render(&mut Frame, &AppState)`, `Key`, `key_to_intent`.
//!
//! Must never depend on crossterm, ratzilla, tokio, or any provider crate — each host maps its
//! own key type into `Key`.

mod key;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Color as RatatuiColor;
use ratatui::text::Line;
use ratatui::widgets::{Block, List, Paragraph};
use ratatui::Frame;
use sp_core::{AppState, Theme};

pub use key::{key_to_intent, Key, Modifiers};

/// Ratatui-facing colour palette. Lives here, not in `sp-core`, because `sp-core` must not
/// depend on ratatui (see CLAUDE.md crate map).
pub struct TuiPalette {
    pub background: RatatuiColor,
    pub window: RatatuiColor,
    pub text: RatatuiColor,
    pub black: RatatuiColor,
    pub red: RatatuiColor,
    pub green: RatatuiColor,
    pub yellow: RatatuiColor,
    pub blue: RatatuiColor,
    pub magenta: RatatuiColor,
    pub cyan: RatatuiColor,
    pub white: RatatuiColor,
    pub gray: RatatuiColor,
}

impl From<&Theme> for TuiPalette {
    fn from(theme: &Theme) -> Self {
        let rgb = |c: sp_core::Color| RatatuiColor::Rgb(c.r, c.g, c.b);
        TuiPalette {
            background: rgb(theme.background),
            window: rgb(theme.window),
            text: rgb(theme.text),
            black: rgb(theme.black),
            red: rgb(theme.red),
            green: rgb(theme.green),
            yellow: rgb(theme.yellow),
            blue: rgb(theme.blue),
            magenta: rgb(theme.magenta),
            cyan: rgb(theme.cyan),
            white: rgb(theme.white),
            gray: rgb(theme.gray),
        }
    }
}

/// Glyph coverage probe for T0.2: box-drawing, powerline, and Nerd Font icons.
/// Both hosts render this identical string.
pub const GLYPH_TEST: &str = "│─┌┐└┘ \u{e0b0}\u{e0b2} \u{f07b}\u{f09b}\u{f015}";

pub fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    let block = Block::bordered().title(state.title.as_str());
    let inner = block.inner(chunks[1]);
    frame.render_widget(block, chunks[1]);

    frame.render_widget(
        Paragraph::new(Line::from(format!("{GLYPH_TEST} {}", state.typed))),
        chunks[0],
    );
    frame.render_widget(List::new(state.links.iter().map(String::as_str)), inner);
}
