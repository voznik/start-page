//! `render(&mut Frame, &AppState)`, `Key`, `key_to_intent`.
//!
//! Must never depend on crossterm, ratzilla, tokio, or any provider crate — each host maps its
//! own key type into `Key`.

use ratatui::Frame;
use sp_core::{AppState, Intent};

pub fn render(_frame: &mut Frame, _state: &AppState) {}

pub enum Key {}

pub fn key_to_intent(_key: Key) -> Option<Intent> {
    None
}
