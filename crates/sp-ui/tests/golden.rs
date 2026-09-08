//! T1.7: `TestBackend` snapshot tests. Each snapshot renders `AppState` after a known `Intent`
//! sequence, driven through the real `sp_core::reduce`, then dumps the `Buffer` as text.
//!
//! Every row is prefixed `> ` when it contains the selected-item highlight (cyan bg), `  `
//! otherwise, so a reviewer can see selection state without decoding colour codes.

use ratatui::backend::TestBackend;
use ratatui::Terminal;
use sp_core::{reduce, AppState, Intent};
use sp_ui::TuiPalette;

fn state_with_links(links: Vec<&str>) -> AppState {
    let mut state = AppState::default();
    state.links = links.into_iter().map(str::to_string).collect();
    state.filtered = (0..state.links.len()).collect();
    state
}

fn apply(state: &mut AppState, intents: Vec<Intent>) {
    for intent in intents {
        reduce(state, intent);
    }
}

fn type_str(state: &mut AppState, text: &str) {
    apply(state, text.chars().map(Intent::Char).collect());
}

/// Renders `state` into a `width`x`height` buffer and dumps it as text: one line per row, each
/// prefixed `> ` if it holds the selected-item highlight, `  ` otherwise. Deterministic — no
/// timestamps, no HashMap-order dependence (`AppState::links`/`filtered` are `Vec`s).
fn render_state(state: &AppState, width: u16, height: u16) -> String {
    let selected_bg = TuiPalette::from(&sp_core::Theme::default()).cyan;
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| sp_ui::render(frame, state)).unwrap();
    let buffer = terminal.backend().buffer();

    let mut out = String::new();
    for y in 0..height {
        let row_start = (y as usize) * (width as usize);
        let row = &buffer.content()[row_start..row_start + width as usize];
        let marker = if row.iter().any(|cell| cell.bg == selected_bg) {
            "> "
        } else {
            "  "
        };
        out.push_str(marker);
        for cell in row {
            out.push_str(cell.symbol());
        }
        out.push('\n');
    }
    out
}

#[test]
fn empty_state_80x24() {
    let state = state_with_links(vec!["github.com", "docs.rs", "news.ycombinator.com"]);
    insta::assert_snapshot!(render_state(&state, 80, 24));
}

#[test]
fn empty_state_200x60() {
    let state = state_with_links(vec!["github.com", "docs.rs", "news.ycombinator.com"]);
    insta::assert_snapshot!(render_state(&state, 200, 60));
}

#[test]
fn filtered_state_narrows_the_list_and_shows_ghost_suggestion() {
    let mut state = state_with_links(vec!["github.com", "docs.rs", "news.ycombinator.com"]);
    type_str(&mut state, "git");
    insta::assert_snapshot!(render_state(&state, 80, 24));
}

#[test]
fn accept_suggestion_replaces_typed_with_the_ghost() {
    let mut state = state_with_links(vec!["github.com", "docs.rs", "news.ycombinator.com"]);
    type_str(&mut state, "git");
    apply(&mut state, vec![Intent::AcceptSuggestion]);
    insta::assert_snapshot!(render_state(&state, 80, 24));
}

#[test]
fn selection_moved_forward_with_next() {
    let mut state = state_with_links(vec!["github.com", "docs.rs", "news.ycombinator.com"]);
    apply(&mut state, vec![Intent::Next]);
    insta::assert_snapshot!(render_state(&state, 80, 24));
}

#[test]
fn selection_wraps_forward_past_the_last_link() {
    let mut state = state_with_links(vec!["github.com", "docs.rs", "news.ycombinator.com"]);
    apply(&mut state, vec![Intent::Next, Intent::Next, Intent::Next]);
    insta::assert_snapshot!(render_state(&state, 80, 24));
}

#[test]
fn selection_wraps_backward_past_the_first_link() {
    let mut state = state_with_links(vec!["github.com", "docs.rs", "news.ycombinator.com"]);
    apply(&mut state, vec![Intent::Prev]);
    insta::assert_snapshot!(render_state(&state, 80, 24));
}

#[test]
fn command_mode_config_path() {
    let mut state = state_with_links(vec!["github.com", "docs.rs"]);
    type_str(&mut state, "config path");
    insta::assert_snapshot!(render_state(&state, 80, 24));
}

#[test]
fn command_mode_help() {
    let mut state = state_with_links(vec!["github.com", "docs.rs"]);
    type_str(&mut state, "help");
    insta::assert_snapshot!(render_state(&state, 80, 24));
}

#[test]
fn command_mode_theme() {
    let mut state = state_with_links(vec!["github.com", "docs.rs"]);
    type_str(&mut state, "theme foo");
    insta::assert_snapshot!(render_state(&state, 80, 24));
}

// "Error state" per T1.7's acceptance wording has no counterpart in AppState today — there is
// no error variant/field to render. The nearest real state is a query that matches nothing:
// the link list goes empty and the status line reads "0/N links".
#[test]
fn query_matching_nothing_leaves_an_empty_list() {
    let mut state = state_with_links(vec!["github.com", "docs.rs", "news.ycombinator.com"]);
    type_str(&mut state, "zzz_no_match_zzz");
    insta::assert_snapshot!(render_state(&state, 80, 24));
}

#[test]
fn long_typed_prompt_at_a_narrow_width_still_lays_out() {
    let mut state = state_with_links(vec!["github.com", "docs.rs"]);
    type_str(&mut state, "s some bug");
    insta::assert_snapshot!(render_state(&state, 80, 24));
}
