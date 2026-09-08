//! `AppState`, `Intent`, `reduce`, `Theme`, `Payload`, provider traits.

mod theme;

pub struct AppState {
    pub title: String,
    pub links: Vec<String>,
    /// T0.2 keystroke-path probe: raw characters typed since load, rendered verbatim.
    /// Placeholder until a real input widget exists via Intent (T1.x).
    pub typed: String,
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
        }
    }
}

pub enum Intent {}

pub struct Effect;

pub fn reduce(_state: &mut AppState, _intent: Intent) -> Vec<Effect> {
    Vec::new()
}

pub use theme::{Color, ColorParseError, Theme};

pub enum Payload {}

pub trait Provider {}
