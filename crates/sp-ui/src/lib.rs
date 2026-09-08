//! `render(&mut Frame, &AppState)`, `Key`, `key_to_intent`.
//!
//! Must never depend on crossterm, ratzilla, tokio, or any provider crate — each host maps its
//! own key type into `Key`.

mod key;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color as RatatuiColor, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, Paragraph};
use ratatui::Frame;
use sp_core::{AppState, Theme};
use unicode_truncate::UnicodeTruncateStr;
use unicode_width::UnicodeWidthStr;

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

/// Truncates `s` to at most `max_width` display columns, appending `…` when it doesn't fit.
/// Unicode-width-aware (CJK, zero-width joiners) via `unicode-truncate`/`unicode-width`, the
/// same crates ratatui-core itself uses for grapheme width — a naive `char`/byte count would
/// split wide glyphs and corrupt the grid.
fn ellipsize(s: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if s.width() <= max_width {
        return s.to_string();
    }
    if max_width == 1 {
        return "…".to_string();
    }
    let (truncated, _) = s.unicode_truncate(max_width - 1);
    format!("{truncated}…")
}

/// Groups `filtered` link indices by category for the link grid. `AppState` has no category
/// field yet (T1.3 constraint: sp-core is owned by another agent this session), so this always
/// returns a single unlabelled group. Once `AppState`/`sp-config` grow categories, replace the
/// body with a real grouping and `render_links` below needs no change — it already iterates
/// `Vec<(Option<&str>, &[usize])>`.
fn link_groups(state: &AppState) -> Vec<(Option<&str>, &[usize])> {
    vec![(None, &state.filtered[..])]
}

fn render_prompt(frame: &mut Frame, area: ratatui::layout::Rect, palette: &TuiPalette, state: &AppState) {
    let typed_style = Style::default().fg(palette.text);
    let ghost_style = Style::default().fg(palette.gray).add_modifier(Modifier::DIM);

    let mut spans = vec![Span::styled("> ", Style::default().fg(palette.blue)), Span::styled(state.typed.as_str(), typed_style)];
    if let Some(suggestion) = &state.suggestion
        && let Some(rest) = suggestion.strip_prefix(state.typed.as_str())
    {
        spans.push(Span::styled(rest, ghost_style));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)).style(Style::default().bg(palette.background)), area);
}

fn render_links(frame: &mut Frame, area: ratatui::layout::Rect, palette: &TuiPalette, state: &AppState) {
    let block = Block::bordered().title(state.title.as_str()).border_style(Style::default().fg(palette.window)).style(Style::default().bg(palette.background).fg(palette.text));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let list_width = inner.width as usize;
    let normal_style = Style::default().fg(palette.text);
    let selected_style = Style::default().fg(palette.black).bg(palette.cyan);

    // Flattened here because there is exactly one group today; a header `ListItem` per named
    // group is the extension point once `link_groups` returns real categories.
    let items: Vec<ListItem> = link_groups(state)
        .into_iter()
        .flat_map(|(_heading, indices)| indices.iter().copied())
        .filter_map(|idx| {
            state.links.get(idx).map(|link| {
                let style = if idx == state.selected { selected_style } else { normal_style };
                ListItem::new(ellipsize(link, list_width)).style(style)
            })
        })
        .collect();

    frame.render_widget(List::new(items), inner);
}

fn render_status(frame: &mut Frame, area: ratatui::layout::Rect, palette: &TuiPalette, state: &AppState) {
    let text = format!("{}/{} links", state.filtered.len(), state.links.len());
    let style = Style::default().fg(palette.gray).bg(palette.background).add_modifier(Modifier::DIM);
    frame.render_widget(Paragraph::new(Line::styled(text, style)), area);
}

pub fn render(frame: &mut Frame, state: &AppState) {
    let palette = TuiPalette::from(&Theme::default());
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    render_prompt(frame, chunks[0], &palette, state);
    render_links(frame, chunks[1], &palette, state);
    render_status(frame, chunks[2], &palette, state);
}
