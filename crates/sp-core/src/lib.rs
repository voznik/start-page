//! `AppState`, `Intent`, `reduce`, `Theme`, `Payload`, provider traits.

pub struct AppState;

pub enum Intent {}

pub struct Effect;

pub fn reduce(_state: &mut AppState, _intent: Intent) -> Vec<Effect> {
    Vec::new()
}

pub struct Theme;

pub enum Payload {}

pub trait Provider {}
