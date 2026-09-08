//! `AppState`, `Intent`, `reduce`, `Theme`, `Payload`, provider traits.

mod theme;

use nucleo::pattern::{CaseMatching, Normalization, Pattern};
use nucleo::{Matcher, Utf32Str};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;

/// URL-encodes a query string for embedding in a search URL. Percent-encodes everything
/// except ASCII alphanumerics (`NON_ALPHANUMERIC`), so space -> `%20`, `&` -> `%26`,
/// `#` -> `%23`, `+` -> `%2B`, `"` -> `%22`, and non-ASCII bytes are UTF-8 percent-escaped.
/// This is what a browser/URL parser resolves a query value as; the upstream JS project's
/// bug was hand-rolled escaping that missed exactly these characters.
pub fn encode_query(query: &str) -> String {
    utf8_percent_encode(query, NON_ALPHANUMERIC).to_string()
}

/// A search engine: a URL template with a literal `{query}` placeholder, replaced with the
/// percent-encoded query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchEngine {
    pub url_template: String,
}

impl SearchEngine {
    pub fn search_url(&self, query: &str) -> String {
        self.url_template.replace("{query}", &encode_query(query))
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        SearchEngine {
            url_template: "https://www.google.com/search?q={query}".to_string(),
        }
    }
}

/// A configured shortcut: typing `<prefix> <rest>` searches `engine` with `rest` as the
/// query, e.g. `s some bug` -> StackOverflow search for "some bug".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shortcut {
    pub prefix: String,
    pub engine: SearchEngine,
}

pub fn default_shortcuts() -> Vec<Shortcut> {
    vec![Shortcut {
        prefix: "s".to_string(),
        engine: SearchEngine {
            url_template: "https://stackoverflow.com/search?q={query}".to_string(),
        },
    }]
}

/// Heuristic "is this a URL or a search?" decision: a single word (no whitespace)
/// containing a dot that isn't at either edge of its host part, e.g. `github.com`.
/// Deliberately not a full URL parser — direct URLs are single tokens the user typed to
/// navigate, not query strings.
pub fn looks_like_url(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() || text.contains(char::is_whitespace) {
        return false;
    }
    if text.starts_with("http://") || text.starts_with("https://") {
        return true;
    }
    let host = text.split('/').next().unwrap_or(text);
    host.contains('.') && !host.starts_with('.') && !host.ends_with('.')
}

/// Normalizes a value that `looks_like_url` accepted into a navigable URL.
fn normalize_url(text: &str) -> String {
    let text = text.trim();
    if text.starts_with("http://") || text.starts_with("https://") {
        text.to_string()
    } else {
        format!("https://{text}")
    }
}

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
    /// Default search engine for bare text with no filter match and for `Intent::ForceSearch`.
    pub default_engine: SearchEngine,
    /// Configured shortcut prefixes (`s some bug` -> StackOverflow), checked in `reduce`
    /// before URL detection and before the filtered-links/search fallback.
    pub shortcuts: Vec<Shortcut>,
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
            default_engine: SearchEngine::default(),
            shortcuts: default_shortcuts(),
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
    /// `help` command. `reduce` stays I/O-free; the host renders/displays help text.
    ShowHelp,
    /// `config path` command. Resolving and printing the path is filesystem work that
    /// belongs in the host (via `sp-config::config_path`), not in `reduce`.
    PrintConfigPath,
    /// `config edit` command. Spawning `$EDITOR` is host I/O.
    OpenConfigInEditor,
    /// `theme <name>` command. Loading/applying the named theme is host work.
    SetTheme(String),
}

/// Parses and dispatches typed command text: exact commands (`help`, `config path`,
/// `config edit`, `theme <name>`), then configured shortcuts (`<prefix> <rest>`), then a
/// direct-URL heuristic, then falling back to opening every filtered link, then falling
/// back further to a default-engine search. Unknown commands never error — the last two
/// fallbacks are exactly that guarantee.
fn dispatch_command(state: &AppState, text: &str) -> Vec<Effect> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    match trimmed {
        "help" => return vec![Effect::ShowHelp],
        "config path" => return vec![Effect::PrintConfigPath],
        "config edit" => return vec![Effect::OpenConfigInEditor],
        _ => {}
    }
    if let Some(name) = trimmed.strip_prefix("theme ") {
        let name = name.trim();
        if !name.is_empty() {
            return vec![Effect::SetTheme(name.to_string())];
        }
    }

    if let Some((prefix, query)) = trimmed.split_once(' ')
        && let Some(shortcut) = state.shortcuts.iter().find(|s| s.prefix == prefix)
    {
        return vec![Effect::OpenUrl(shortcut.engine.search_url(query.trim()))];
    }

    if looks_like_url(trimmed) {
        return vec![Effect::OpenUrl(normalize_url(trimmed))];
    }

    if !state.filtered.is_empty() {
        return state
            .filtered
            .iter()
            .filter_map(|&i| state.links.get(i))
            .map(|link| Effect::OpenUrl(normalize_url(link)))
            .collect();
    }

    vec![Effect::OpenUrl(state.default_engine.search_url(trimmed))]
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
        // Empty prompt: plain browse mode, Enter opens the highlighted link (Next/Prev
        // moves `selected`). Non-empty prompt: command/filter mode, dispatch the typed
        // text (shortcuts, URLs, commands, or open-all-filtered/search fallback).
        Intent::Activate => {
            if state.typed.trim().is_empty() {
                state
                    .links
                    .get(state.selected)
                    .map(|link| vec![Effect::OpenUrl(normalize_url(link))])
                    .unwrap_or_default()
            } else {
                dispatch_command(state, &state.typed.clone())
            }
        }
        // Ctrl+Enter: always search the typed text with the default engine, bypassing
        // shortcuts/URL detection/filtered-links-open. Empty prompt has nothing to search.
        Intent::ForceSearch => {
            let trimmed = state.typed.trim();
            if trimmed.is_empty() {
                Vec::new()
            } else {
                vec![Effect::OpenUrl(state.default_engine.search_url(trimmed))]
            }
        }
        Intent::AcceptSuggestion => {
            if let Some(suggestion) = state.suggestion.clone() {
                state.typed = suggestion;
                state.selected = 0;
                refilter(state);
            }
            Vec::new()
        }
        Intent::Command(Cmd(text)) => dispatch_command(state, &text),
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
            default_engine: SearchEngine::default(),
            shortcuts: default_shortcuts(),
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
        assert_eq!(effects, vec![Effect::OpenUrl("https://b".to_string())]);
    }

    /// A schemeless link must not be handed to a host as a bare hostname: the browser resolves
    /// it against the current origin (http://localhost:PORT/github.com) instead of navigating to
    /// the site.
    #[test]
    fn opening_a_link_gives_it_a_scheme() {
        let mut state = base_state();
        state.links = vec!["github.com".into(), "https://docs.rs".into()];
        state.selected = 0;
        assert_eq!(
            reduce(&mut state, Intent::Activate),
            vec![Effect::OpenUrl("https://github.com".to_string())]
        );

        state.selected = 1;
        assert_eq!(
            reduce(&mut state, Intent::Activate),
            vec![Effect::OpenUrl("https://docs.rs".to_string())],
            "an explicit scheme must be left alone, not doubled"
        );
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

    // --- T1.5: command parser ---

    /// Adversarial query-encoding cases, not happy-path: spaces, `&`, `#`, `+`, quotes,
    /// and non-ASCII must all survive a round trip into the search URL. Verified two ways:
    /// the literal percent-escapes a browser resolves, and a decode round trip.
    #[test]
    fn encode_query_survives_adversarial_characters() {
        let query = "rust 東京 & \"quotes\" #1+2";
        let encoded = encode_query(query);

        assert_eq!(
            encoded,
            "rust%20%E6%9D%B1%E4%BA%AC%20%26%20%22quotes%22%20%231%2B2"
        );

        let decoded = percent_encoding::percent_decode_str(&encoded)
            .decode_utf8()
            .unwrap();
        assert_eq!(decoded, query);
    }

    #[test]
    fn search_engine_builds_url_from_encoded_query() {
        let engine = SearchEngine {
            url_template: "https://example.com/search?q={query}".to_string(),
        };
        assert_eq!(
            engine.search_url("a b&c"),
            "https://example.com/search?q=a%20b%26c"
        );
    }

    #[test]
    fn looks_like_url_accepts_bare_hosts_and_rejects_search_terms() {
        assert!(looks_like_url("github.com"));
        assert!(looks_like_url("https://github.com/foo"));
        assert!(!looks_like_url("some bug"));
        assert!(!looks_like_url("help"));
        assert!(!looks_like_url(".com"));
        assert!(!looks_like_url(""));
    }

    #[test]
    fn bare_text_with_no_match_falls_back_to_default_search() {
        let mut state = base_state();
        for c in "zzz nonsense".chars() {
            reduce(&mut state, Intent::Char(c));
        }
        assert!(state.filtered.is_empty(), "fixture should not fuzzy-match");
        let effects = reduce(&mut state, Intent::Activate);
        assert_eq!(
            effects,
            vec![Effect::OpenUrl(
                "https://www.google.com/search?q=zzz%20nonsense".to_string()
            )]
        );
    }

    #[test]
    fn unknown_command_falls_through_to_search_rather_than_erroring() {
        let mut state = base_state();
        state.links.clear();
        state.filtered.clear();
        let effects = reduce(&mut state, Intent::Command(Cmd("totally unknown thing".to_string())));
        assert_eq!(
            effects,
            vec![Effect::OpenUrl(
                "https://www.google.com/search?q=totally%20unknown%20thing".to_string()
            )]
        );
    }

    #[test]
    fn enter_with_n_filtered_matches_emits_n_open_url_effects() {
        let mut state = base_state();
        state.links = vec![
            "apple.com".to_string(),
            "apricot.com".to_string(),
            "banana.com".to_string(),
        ];
        for c in "ap".chars() {
            reduce(&mut state, Intent::Char(c));
        }
        assert_eq!(state.filtered.len(), 2, "fixture should match two links");
        let effects = reduce(&mut state, Intent::Activate);
        assert_eq!(effects.len(), 2);
        for effect in &effects {
            assert!(matches!(effect, Effect::OpenUrl(_)));
        }
    }

    #[test]
    fn configured_shortcut_searches_the_shortcut_engine() {
        let mut state = base_state();
        let effects = reduce(
            &mut state,
            Intent::Command(Cmd("s some bug".to_string())),
        );
        assert_eq!(
            effects,
            vec![Effect::OpenUrl(
                "https://stackoverflow.com/search?q=some%20bug".to_string()
            )]
        );
    }

    #[test]
    fn direct_url_navigates_instead_of_searching() {
        let mut state = base_state();
        let effects = reduce(&mut state, Intent::Command(Cmd("github.com".to_string())));
        assert_eq!(
            effects,
            vec![Effect::OpenUrl("https://github.com".to_string())]
        );
    }

    #[test]
    fn help_config_and_theme_commands_resolve_to_effects() {
        let mut state = base_state();
        assert_eq!(
            reduce(&mut state, Intent::Command(Cmd("help".to_string()))),
            vec![Effect::ShowHelp]
        );
        assert_eq!(
            reduce(&mut state, Intent::Command(Cmd("config path".to_string()))),
            vec![Effect::PrintConfigPath]
        );
        assert_eq!(
            reduce(&mut state, Intent::Command(Cmd("config edit".to_string()))),
            vec![Effect::OpenConfigInEditor]
        );
        assert_eq!(
            reduce(&mut state, Intent::Command(Cmd("theme dracula".to_string()))),
            vec![Effect::SetTheme("dracula".to_string())]
        );
    }

    #[test]
    fn force_search_bypasses_shortcuts_and_url_detection() {
        let mut state = base_state();
        for c in "github.com".chars() {
            reduce(&mut state, Intent::Char(c));
        }
        let effects = reduce(&mut state, Intent::ForceSearch);
        // `.` is not alphanumeric, so NON_ALPHANUMERIC percent-encodes it too (%2E) —
        // correct and reversible, just more aggressive than a typical query-string encoder.
        assert_eq!(
            effects,
            vec![Effect::OpenUrl(
                "https://www.google.com/search?q=github%2Ecom".to_string()
            )]
        );
    }

    #[test]
    fn force_search_on_empty_prompt_is_a_noop() {
        let mut state = base_state();
        assert!(reduce(&mut state, Intent::ForceSearch).is_empty());
    }
}
