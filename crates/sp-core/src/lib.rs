//! `AppState`, `Intent`, `reduce`, `Theme`, `Payload`, provider traits.

mod theme;

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
    pub should_quit: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            title: "start-page".to_string(),
            links: vec![
                "github.com".to_string(),
                "docs.rs".to_string(),
                "news.ycombinator.com".to_string(),
            ],
            typed: String::new(),
            selected: 0,
            should_quit: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Intent {
    Char(char),
    Backspace,
    Next,
    Prev,
    Activate,
    ForceSearch,
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
            Vec::new()
        }
        Intent::Backspace => {
            state.typed.pop();
            state.selected = 0;
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
            should_quit: false,
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
}
