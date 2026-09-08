//! `AppState`, `Intent`, `reduce`, `Theme`, `Payload`, provider traits.

mod theme;

use nucleo::pattern::{CaseMatching, Normalization, Pattern};
use nucleo::{Matcher, Utf32Str};
use std::cmp::Reverse;

/// Opaque provider identifier. The provider trait and registry are phase 3;
/// this is just enough for `Intent::Refresh`/`Intent::ProviderUpdated` to typecheck.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProviderId(pub String);

/// Data pushed back by a provider. `Raw` is the only variant until phase 3 providers land;
/// keep `serde_json::Value` out of everything else (CLAUDE.md § Do not).
#[derive(Debug, Clone, PartialEq)]
pub enum Payload {
    Raw(String),
}

/// Raw command text. The parser (`s some bug` -> StackOverflow, bare text -> search, etc.)
/// is T1.5's job; this stub only carries the text so `Intent::Command` typechecks.
#[derive(Debug, Clone, PartialEq)]
pub struct Cmd(pub String);

pub struct AppState {
    pub title: String,
    pub links: Vec<String>,
    /// Text typed into the prompt line. Fuzzy filtering (T1.4) and command parsing (T1.5)
    /// build on this; T1.1 only tracks the raw text.
    pub typed: String,
    /// Index into `links`, moved by `Intent::Next`/`Intent::Prev`, opened by `Intent::Activate`.
    pub selected: usize,
    /// Indices into `links` that match `typed`, fuzzy-ranked best-first. Recomputed in
    /// `reduce` whenever `typed` changes. Empty `typed` matches every link in original order.
    /// T1.3 renders the link list from this instead of iterating `links` directly.
    pub filtered: Vec<usize>,
    /// Ghost autosuggest text: the top fuzzy match in `filtered`, or `None` if `typed` is
    /// empty or nothing matches. T1.3 should render `typed` followed by the remainder of
    /// this string (dimmed) when it starts with `typed`; `Intent::AcceptSuggestion` (mapped
    /// to `Tab`) sets `typed` to this value.
    pub suggestion: Option<String>,
    pub should_quit: bool,
    /// Reused across `reduce` calls to avoid reallocating match buffers per keystroke.
    matcher: Matcher,
}

impl Default for AppState {
    fn default() -> Self {
        let links = vec![
            "github.com".to_string(),
            "docs.rs".to_string(),
            "news.ycombinator.com".to_string(),
        ];
        let filtered = (0..links.len()).collect();
        Self {
            title: "start-page".to_string(),
            links,
            typed: String::new(),
            selected: 0,
            filtered,
            suggestion: None,
            should_quit: false,
            matcher: Matcher::default(),
        }
    }
}

/// Fuzzy-filters `state.links` against `state.typed`, ranks matches best-first into
/// `state.filtered`, and sets `state.suggestion` to the top match. Called from `reduce`
/// whenever `typed` changes.
fn refilter(state: &mut AppState) {
    if state.typed.is_empty() {
        state.filtered = (0..state.links.len()).collect();
        state.suggestion = None;
        return;
    }

    let pattern = Pattern::parse(&state.typed, CaseMatching::Ignore, Normalization::Smart);
    let mut buf = Vec::new();
    let mut scored: Vec<(usize, u32)> = state
        .links
        .iter()
        .enumerate()
        .filter_map(|(i, link)| {
            let haystack = Utf32Str::new(link, &mut buf);
            pattern
                .score(haystack, &mut state.matcher)
                .map(|score| (i, score))
        })
        .collect();
    scored.sort_by_key(|&(_, score)| Reverse(score));

    state.suggestion = scored
        .first()
        .map(|&(i, _)| state.links[i].clone());
    state.filtered = scored.into_iter().map(|(i, _)| i).collect();
}

#[derive(Debug, Clone, PartialEq)]
pub enum Intent {
    Char(char),
    Backspace,
    Next,
    Prev,
    Activate,
    ForceSearch,
    /// Accepts the current ghost suggestion (ratatui `Tab`), replacing `typed` with it.
    /// No-op if `suggestion` is `None`.
    AcceptSuggestion,
    Command(Cmd),
    Refresh(ProviderId),
    ProviderUpdated(ProviderId, Payload),
    Quit,
}

/// Side effect a host must perform outside `reduce` (I/O, browser APIs, process spawn).
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    OpenUrl(String),
}

pub fn reduce(state: &mut AppState, intent: Intent) -> Vec<Effect> {
    match intent {
        Intent::Char(c) => {
            state.typed.push(c);
            state.selected = 0;
            refilter(state);
            Vec::new()
        }
        Intent::Backspace => {
            state.typed.pop();
            state.selected = 0;
            refilter(state);
            Vec::new()
        }
        Intent::Next => {
            if !state.links.is_empty() {
                state.selected = (state.selected + 1) % state.links.len();
            }
            Vec::new()
        }
        Intent::Prev => {
            if !state.links.is_empty() {
                state.selected = (state.selected + state.links.len() - 1) % state.links.len();
            }
            Vec::new()
        }
        Intent::Activate => state
            .links
            .get(state.selected)
            .map(|link| vec![Effect::OpenUrl(link.clone())])
            .unwrap_or_default(),
        // Building the search-engine URL (and encoding it) is T1.5's job.
        Intent::ForceSearch => Vec::new(),
        Intent::AcceptSuggestion => {
            if let Some(suggestion) = state.suggestion.clone() {
                state.typed = suggestion;
                state.selected = 0;
                refilter(state);
            }
            Vec::new()
        }
        // Command parsing/dispatch is T1.5's job.
        Intent::Command(_) => Vec::new(),
        // Provider trait and registry are phase 3; nothing to refresh yet.
        Intent::Refresh(_) => Vec::new(),
        Intent::ProviderUpdated(_, _) => Vec::new(),
        Intent::Quit => {
            state.should_quit = true;
            Vec::new()
        }
    }
}

pub use theme::{Color, ColorParseError, Theme};

pub trait Provider {}

#[cfg(test)]
mod tests {
    use super::*;

    struct Case {
        name: &'static str,
        intents: Vec<Intent>,
        expected: AppState,
    }

    fn base_state() -> AppState {
        AppState {
            title: "start-page".to_string(),
            links: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            typed: String::new(),
            selected: 0,
            filtered: vec![0, 1, 2],
            suggestion: None,
            should_quit: false,
            matcher: Matcher::default(),
        }
    }

    #[test]
    fn reduce_table() {
        let cases = vec![
            Case {
                name: "typing then backspace",
                intents: vec![
                    Intent::Char('h'),
                    Intent::Char('i'),
                    Intent::Backspace,
                    Intent::Char('!'),
                ],
                expected: AppState {
                    typed: "h!".to_string(),
                    ..base_state()
                },
            },
            Case {
                name: "next wraps past the last link",
                intents: vec![Intent::Next, Intent::Next, Intent::Next],
                expected: AppState {
                    selected: 0,
                    ..base_state()
                },
            },
            Case {
                name: "prev wraps past the first link",
                intents: vec![Intent::Prev],
                expected: AppState {
                    selected: 2,
                    ..base_state()
                },
            },
            Case {
                name: "quit sets should_quit",
                intents: vec![Intent::Quit],
                expected: AppState {
                    should_quit: true,
                    ..base_state()
                },
            },
        ];

        for case in cases {
            let mut state = base_state();
            for intent in case.intents {
                reduce(&mut state, intent);
            }
            assert_eq!(
                (
                    &state.title,
                    &state.links,
                    &state.typed,
                    state.selected,
                    state.should_quit
                ),
                (
                    &case.expected.title,
                    &case.expected.links,
                    &case.expected.typed,
                    case.expected.selected,
                    case.expected.should_quit
                ),
                "case {} failed",
                case.name
            );
        }
    }

    #[test]
    fn activate_opens_the_selected_link() {
        let mut state = base_state();
        state.selected = 1;
        let effects = reduce(&mut state, Intent::Activate);
        assert_eq!(effects, vec![Effect::OpenUrl("b".to_string())]);
    }

    #[test]
    fn activate_on_empty_links_produces_no_effect() {
        let mut state = base_state();
        state.links.clear();
        let effects = reduce(&mut state, Intent::Activate);
        assert!(effects.is_empty());
    }

    #[test]
    fn typing_filters_and_suggests_the_top_match() {
        let mut state = base_state();
        state.links = vec!["github.com".to_string(), "docs.rs".to_string()];
        for c in "git".chars() {
            reduce(&mut state, Intent::Char(c));
        }
        assert_eq!(state.filtered, vec![0]);
        assert_eq!(state.suggestion.as_deref(), Some("github.com"));
    }

    #[test]
    fn empty_typed_filters_nothing() {
        let state = base_state();
        assert_eq!(state.filtered, vec![0, 1, 2]);
        assert_eq!(state.suggestion, None);
    }

    #[test]
    fn accept_suggestion_replaces_typed_with_the_top_match() {
        let mut state = base_state();
        state.links = vec!["github.com".to_string(), "docs.rs".to_string()];
        reduce(&mut state, Intent::Char('g'));
        reduce(&mut state, Intent::AcceptSuggestion);
        assert_eq!(state.typed, "github.com");
    }

    #[test]
    fn accept_suggestion_is_a_noop_without_a_match() {
        let mut state = base_state();
        reduce(&mut state, Intent::Char('z'));
        assert_eq!(state.suggestion, None);
        reduce(&mut state, Intent::AcceptSuggestion);
        assert_eq!(state.typed, "z");
    }

    /// Acceptance: filtering 1,000 links must stay under 1ms per keystroke. Measured
    /// 89.8us/keystroke with `cargo test --release`. Debug builds are 10-50x slower
    /// (fuzzy matching is not optimized without LTO/opt-level=3), so `cargo test`
    /// (the workspace default) only checks a loose sanity bound here to avoid flaking;
    /// the real 1ms budget is enforced under `--release`.
    #[test]
    fn filtering_1000_links_stays_under_a_millisecond_per_keystroke() {
        let mut state = base_state();
        state.links = (0..1000)
            .map(|i| format!("https://example{i}.com/path/to/resource"))
            .collect();
        refilter(&mut state);

        let query = "example42resource";
        let start = std::time::Instant::now();
        for c in query.chars() {
            reduce(&mut state, Intent::Char(c));
        }
        let elapsed = start.elapsed();
        let per_keystroke = elapsed / query.len() as u32;

        eprintln!("per-keystroke: {per_keystroke:?} over {} links", state.links.len());
        let budget = if cfg!(debug_assertions) {
            50_000 // debug sanity bound only; the acceptance budget needs --release
        } else {
            1_000
        };
        assert!(
            per_keystroke.as_micros() < budget,
            "per-keystroke filtering took {per_keystroke:?}, budget is {budget}us"
        );
    }
}
