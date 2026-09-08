//! ratzilla driver, cdylib, wasm32.

use std::{cell::RefCell, rc::Rc};

use ratzilla::event::KeyCode;
use ratzilla::ratatui::backend::Backend as _;
use ratzilla::ratatui::layout::Rect;
use ratzilla::ratatui::{Terminal, TerminalOptions, Viewport};
use ratzilla::{web_sys, DomBackend, WebRenderer};
use sp_core::AppState;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

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

    // T0.2 keystroke-path probe only: sp_ui::Key is uninhabited and sp_core::Intent has zero
    // variants, so there's nothing to route through sp_ui::key_to_intent yet. Mutate AppState
    // directly here; replace with a KeyCode -> sp_ui::Key -> key_to_intent -> reduce() pipeline
    // once those types grow real variants (T1.x), mirroring sp-tui-host's crossterm mapping.
    terminal.on_key_event({
        let state = state.clone();
        move |key_event| match key_event.code {
            KeyCode::Char(c) => state.borrow_mut().typed.push(c),
            KeyCode::Backspace => {
                state.borrow_mut().typed.pop();
            }
            _ => {}
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
            focused = true;
        }
    });

    Ok(())
}
