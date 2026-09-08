//! `render(&mut Frame, &AppState)`, `Key`, `key_to_intent`.
//!
//! Must never depend on crossterm, ratzilla, tokio, or any provider crate — each host maps its
//! own key type into `Key`.

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::Line;
use ratatui::widgets::{Block, List, Paragraph};
use ratatui::Frame;
use sp_core::{AppState, Intent};

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

pub enum Key {}

pub fn key_to_intent(_key: Key) -> Option<Intent> {
    None
}
