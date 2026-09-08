//! `render(&mut Frame, &AppState)`, `Key`, `key_to_intent`.
//!
//! Must never depend on crossterm, ratzilla, tokio, or any provider crate — each host maps its
//! own key type into `Key`.

mod key;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color as RatatuiColor, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
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

/// Maps Excalith icon identifiers to Nerd Font glyphs with clean fallbacks.
pub fn icon_glyph(icon: Option<&str>) -> &'static str {
    match icon {
        Some("mdi:web") => "\u{f0ac}",             // 
        Some("fa-brands:keybase") => "\u{f084}",   // 
        Some("simple-icons:openai") => "\u{f085}", // 
        Some("simple-icons:oracle") => "\u{f0c2}", // 
        Some("mdi:github") => "\u{f09b}",          // 
        Some("ph:gitlab-logo-simple-fill") => "\u{f296}", // 
        Some("material-symbols:logo-dev") => "\u{f121}",  // 
        Some("mdi:stack-overflow") => "\u{f16c}",  // 
        Some("mdi:twitter") => "\u{f099}",         // 
        Some("ri:mastodon-fill") => "\u{f4fe}",    // 
        Some("mdi:reddit") => "\u{f281}",          // 
        Some("simple-icons:polywork") => "\u{f0b1}", // 
        Some("uil:polygon") => "\u{f219}",         // 
        Some("mdi:currency-sign") => "\u{f11b}",   // 
        Some("ph:toilet-paper-bold") => "\u{f1ea}", // 
        Some("tabler:hand-rock") => "\u{f255}",    // 
        Some("material-symbols:science") => "\u{f0c3}", // 
        Some("fa6-solid:user-astronaut") => "\u{f135}", // 
        Some("simple-icons:nasa") => "\u{f135}",   // 
        Some("mdi:black-mesa") => "\u{f12e}",      // 
        Some("game-icons:techno-heart") => "\u{f004}", // 
        Some("arcticons:verge") => "\u{f1ea}",     // 
        Some("uil:linux") => "\u{f17c}",           // 
        Some(s) if s.contains("github") => "\u{f09b}",
        Some(s) if s.contains("gitlab") => "\u{f296}",
        Some(s) if s.contains("twitter") => "\u{f099}",
        Some(s) if s.contains("reddit") => "\u{f281}",
        Some(s) if s.contains("linux") => "\u{f17c}",
        Some(s) if s.contains("code") || s.contains("dev") => "\u{f121}",
        Some(s) if s.contains("globe") || s.contains("web") => "\u{f0ac}",
        Some(s) if s.contains("rocket") || s.contains("space") => "\u{f135}",
        _ => "\u{f0c1}",                           //  fallback
    }
}

/// Section color mapping helper.
pub fn resolve_color(name: &str, palette: &TuiPalette) -> RatatuiColor {
    match name.to_ascii_lowercase().as_str() {
        "green" => palette.green,
        "magenta" | "purple" => palette.magenta,
        "cyan" => palette.cyan,
        "red" => palette.red,
        "blue" => palette.blue,
        "yellow" => palette.yellow,
        "white" => palette.white,
        "gray" | "grey" => palette.gray,
        "black" => palette.black,
        hex => hex
            .parse::<sp_core::Color>()
            .map(|c| RatatuiColor::Rgb(c.r, c.g, c.b))
            .unwrap_or(palette.text),
    }
}

fn render_prompt(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    palette: &TuiPalette,
    state: &AppState,
) {
    let username = if state.username.is_empty() { "excalith" } else { &state.username };
    let hostname = if state.hostname.is_empty() { "firefox" } else { &state.hostname };
    let symbol = if state.prompt_symbol.is_empty() { "❯" } else { &state.prompt_symbol };

    let mut spans = vec![
        Span::styled(username, Style::default().fg(palette.green).add_modifier(Modifier::BOLD)),
        Span::styled("@", Style::default().fg(palette.gray)),
        Span::styled(hostname, Style::default().fg(palette.magenta).add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::styled(symbol, Style::default().fg(palette.red).add_modifier(Modifier::BOLD)),
        Span::raw(" "),
    ];

    if state.typed.is_empty() {
        spans.push(Span::styled("command...", Style::default().fg(palette.gray).add_modifier(Modifier::DIM)));
    } else {
        spans.push(Span::styled(&state.typed, Style::default().fg(palette.text)));
        if let Some(suggestion) = &state.suggestion
            && state.typed.len() <= suggestion.len()
            && suggestion[..state.typed.len()].eq_ignore_ascii_case(&state.typed)
        {
            spans.push(Span::styled(
                &suggestion[state.typed.len()..],
                Style::default().fg(palette.gray).add_modifier(Modifier::DIM),
            ));
        }
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(palette.background)),
        area,
    );
}

fn render_sections(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    palette: &TuiPalette,
    state: &AppState,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let fallback_sections;
    let sections = if !state.sections.is_empty() {
        state.sections.as_slice()
    } else if !state.links.is_empty() {
        fallback_sections = vec![sp_core::Section::new("Links", "green", state.links.clone())];
        fallback_sections.as_slice()
    } else {
        return;
    };

    let num_cols = if area.width >= 75 && sections.len() >= 3 {
        3
    } else if area.width >= 50 && sections.len() >= 2 {
        2
    } else {
        1
    };

    // Running start offset of each section within `state.links`
    let mut section_offsets = Vec::with_capacity(sections.len());
    let mut offset = 0;
    for sec in sections {
        section_offsets.push(offset);
        offset += sec.links.len();
    }

    // Assign sections to columns round-robin (Excalith grid layout)
    let mut col_sections: Vec<Vec<usize>> = vec![Vec::new(); num_cols];
    for sec_idx in 0..sections.len() {
        col_sections[sec_idx % num_cols].push(sec_idx);
    }

    // Calculate height needed for each column
    let mut col_heights = vec![0usize; num_cols];
    for (c, sec_indices) in col_sections.iter().enumerate() {
        let mut h = 0;
        for (i, &sec_idx) in sec_indices.iter().enumerate() {
            if i > 0 {
                h += 1; // blank spacing line between sections in same column
            }
            h += 1; // Section title
            h += sections[sec_idx].links.len(); // Each link
        }
        col_heights[c] = h;
    }
    let max_height = col_heights.into_iter().max().unwrap_or(0);

    let grid_height = (max_height as u16).min(area.height);
    let v_pad = area.height.saturating_sub(grid_height) / 2;

    let target_width = match num_cols {
        3 => 78.min(area.width),
        2 => 54.min(area.width),
        _ => area.width.min(60),
    };
    let h_pad = area.width.saturating_sub(target_width) / 2;

    let centered_v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(v_pad),
            Constraint::Length(grid_height),
            Constraint::Min(0),
        ])
        .split(area)[1];

    let centered_grid = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(h_pad),
            Constraint::Length(target_width),
            Constraint::Min(0),
        ])
        .split(centered_v)[1];

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Ratio(1, num_cols as u32); num_cols])
        .split(centered_grid);

    for (c, sec_indices) in col_sections.iter().enumerate() {
        let col_area = cols[c];
        let col_width = col_area.width as usize;
        let mut lines = Vec::new();

        for (i, &sec_idx) in sec_indices.iter().enumerate() {
            if i > 0 {
                lines.push(Line::raw(""));
            }
            let sec = &sections[sec_idx];
            let sec_color = resolve_color(&sec.color, palette);
            lines.push(Line::from(Span::styled(
                &sec.title,
                Style::default().fg(sec_color).add_modifier(Modifier::BOLD),
            )));

            for (item_idx, link) in sec.links.iter().enumerate() {
                let global_idx = section_offsets[sec_idx] + item_idx;
                let icon = icon_glyph(link.icon.as_deref());
                let max_name_width = col_width.saturating_sub(5);
                let display_name = ellipsize(&link.name, max_name_width);
                let link_text = format!("{icon}  {display_name}");

                let is_selected = global_idx == state.selected;
                let is_filtering = !state.typed.is_empty();
                let is_matched = state.filtered.contains(&global_idx);

                let style = if is_selected {
                    Style::default().fg(palette.black).bg(palette.cyan)
                } else if is_filtering && !is_matched {
                    Style::default().fg(palette.gray).add_modifier(Modifier::DIM)
                } else {
                    Style::default().fg(palette.text)
                };

                lines.push(Line::from(Span::styled(link_text, style)));
            }
        }

        frame.render_widget(Paragraph::new(lines), col_area);
    }
}

pub fn render(frame: &mut Frame, state: &AppState) {
    let palette = TuiPalette::from(&state.theme);
    let area = frame.area();

    // Fill the background of the entire viewport
    frame.render_widget(
        Block::default().style(Style::default().bg(palette.background)),
        area,
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    render_sections(frame, chunks[0], &palette, state);
    render_prompt(frame, chunks[1], &palette, state);
}
