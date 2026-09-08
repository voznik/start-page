//! ratzilla driver, cdylib, wasm32.

use std::{cell::RefCell, rc::Rc};

use ratzilla::event::{KeyCode, KeyEvent};
use ratzilla::ratatui::backend::Backend as _;
use ratzilla::ratatui::layout::Rect;
use ratzilla::ratatui::{Terminal, TerminalOptions, Viewport};
use ratzilla::{web_sys, DomBackend, WebRenderer};
use sp_core::{reduce, AppState};
use sp_ui::{key_to_intent, Key, Modifiers};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Maps a ratzilla key event into `sp-ui`'s own `Key`/`Modifiers`. `None` for keys the app
/// doesn't handle (function keys, page up/down, etc).
pub fn map_key(event: &KeyEvent) -> Option<(Key, Modifiers)> {
    let modifiers = Modifiers {
        ctrl: event.ctrl,
        alt: event.alt,
        shift: event.shift,
    };
    let key = match event.code {
        KeyCode::Char(c) => Key::Char(c),
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Enter => Key::Enter,
        KeyCode::Tab => Key::Tab,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Esc => Key::Esc,
        _ => return None,
    };
    Some((key, modifiers))
}

// Ratzilla is single-threaded (browser event loop): Rc<RefCell<_>>, never Arc<Mutex<_>>
// (CLAUDE.md invariant 3).
#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let state = Rc::new(RefCell::new(AppState::default()));

    let mut backend = DomBackend::new().map_err(|e| JsValue::from_str(&e.to_string()))?;

    // ratzilla 0.3.1's DomBackend::size() (used by ratatui's Viewport::Fullscreen autoresize)
    // computes terminal size from raw window.innerWidth/innerHeight with a hardcoded 10x20px
    // cell-size guess, completely independent of the grid_parent-bounding-rect + measured-glyph
    // calculation DomBackend actually populates its DOM cells with. The two diverge (any font
    // other than that exact 10x20 assumption), so autoresize's buffer size stops matching
    // DomBackend's own `cells` vec and indexing panics on the first post-init redraw. Pin a fixed
    // viewport sized from `window_size()` (the value DomBackend actually built its cells with) so
    // ratatui never calls the broken `size()` path: Viewport::Fixed skips autoresize entirely.
    let size = backend
        .window_size()
        .map_err(|e| JsValue::from_str(&e.to_string()))?
        .columns_rows;
    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Fixed(Rect::new(0, 0, size.width, size.height)),
        },
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;

    terminal.on_key_event({
        let state = state.clone();
        move |key_event| {
            if let Some((key, modifiers)) = map_key(&key_event)
                && let Some(intent) = key_to_intent(key, modifiers)
            {
                reduce(&mut state.borrow_mut(), intent);
            }
        }
    }).map_err(|e| JsValue::from_str(&e.to_string()))?;

    // DomBackend attaches its keydown listener to the #grid element (tabindex="0"), not
    // document, so nothing is typeable until it has focus. Grab it the first frame after
    // DomBackend::draw() has created and appended #grid (it doesn't exist before that).
    let mut focused = false;
    terminal.draw_web(move |frame| {
        sp_ui::render(frame, &state.borrow());
        if !focused
            && let Some(grid) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("grid"))
                .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let _ = grid.focus();
            suppress_browser_key_defaults(&grid);
            focused = true;
        }
    });

    Ok(())
}

/// Stops the browser acting on keys the app handles itself.
///
/// Ratzilla's own keydown listener never calls `preventDefault`, so the browser still runs its
/// default behaviour afterwards. Tab is the damaging one: it moves focus off `#grid`, and since
/// that element is where ratzilla listens, the app goes permanently deaf to the keyboard after a
/// single Tab. Arrows and space scroll the page for the same reason.
///
/// Capture phase, so this runs before ratzilla's listener and applies even if the event would
/// otherwise be consumed.
fn suppress_browser_key_defaults(grid: &web_sys::HtmlElement) {
    let handler = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(
        move |event: web_sys::KeyboardEvent| {
            let blocked = matches!(
                event.key().as_str(),
                "Tab" | "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight" | " "
            );
            if blocked {
                event.prevent_default();
            }
        },
    );

    let _ = grid.add_event_listener_with_callback_and_bool(
        "keydown",
        handler.as_ref().unchecked_ref(),
        true,
    );
    // The listener must outlive this function; the grid lives for the page's lifetime.
    handler.forget();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(code: KeyCode, ctrl: bool, alt: bool, shift: bool) -> KeyEvent {
        KeyEvent {
            code,
            ctrl,
            alt,
            shift,
        }
    }

    #[test]
    fn maps_ctrl_c_quit() {
        let (key, modifiers) = map_key(&event(KeyCode::Char('c'), true, false, false)).unwrap();
        assert_eq!(key_to_intent(key, modifiers), Some(sp_core::Intent::Quit));
    }

    #[test]
    fn maps_shift_tab_to_prev() {
        let (key, modifiers) = map_key(&event(KeyCode::Tab, false, false, true)).unwrap();
        assert_eq!(key_to_intent(key, modifiers), Some(sp_core::Intent::Prev));
    }

    #[test]
    fn unhandled_key_is_none() {
        assert_eq!(map_key(&event(KeyCode::PageUp, false, false, false)), None);
    }
}
