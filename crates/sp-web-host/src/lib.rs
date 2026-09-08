//! ratzilla driver, cdylib, wasm32.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use ratzilla::event::{KeyCode, KeyEvent};
use ratzilla::ratatui::backend::Backend as _;
use ratzilla::ratatui::layout::Rect;
use ratzilla::ratatui::{Terminal, TerminalOptions, Viewport};
use ratzilla::{web_sys, DomBackend, WebRenderer};
use sp_core::{reduce, AppState, Effect};
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

/// Performs a side effect returned by `reduce`. Exhaustive on purpose (no `_` arm), mirroring
/// `sp-tui-host`'s `handle_effect`: a new `Effect` variant must fail this match at compile time
/// rather than being silently dropped.
fn handle_effect(effect: Effect) {
    match effect {
        // `window.open`, subject to the browser's popup blocker like any script-initiated
        // `window.open` call. Verified in a real browser: opening several links from one Enter
        // keypress (N filtered links -> N OpenUrl effects, all handled synchronously inside the
        // same keydown callback) opens all of them, because they still share that keypress's
        // user-gesture context. Blockers key off *that*, not the count.
        Effect::OpenUrl(url) => match web_sys::window().map(|w| w.open_with_url(&url)) {
            Some(Ok(_)) => {}
            Some(Err(err)) => {
                web_sys::console::warn_1(&format!("failed to open {url}: {err:?}").into());
            }
            None => web_sys::console::warn_1(&"no window to open url from".into()),
        },
        // No filesystem and no $EDITOR in a browser, so unlike the TUI host these can never do
        // anything useful — say so instead of pretending (and sp-web-host doesn't depend on
        // sp-config to even look the path up).
        Effect::PrintConfigPath | Effect::OpenConfigInEditor => {
            web_sys::console::log_1(
                &"config path/edit is not available in the browser (no filesystem)".into(),
            );
        }
        // Same gap as the TUI host: `render` has no help view and `AppState` carries no theme.
        Effect::ShowHelp => {
            web_sys::console::log_1(&"help is not rendered yet (no help view in sp-ui)".into());
        }
        Effect::SetTheme(name) => {
            web_sys::console::log_1(
                &format!("theme '{name}' not applied: AppState carries no theme yet").into(),
            );
        }
    }
}

/// The paint loop closure, re-armed for the next `requestAnimationFrame` tick on every call.
type FrameCallback = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;

/// Requests the next `requestAnimationFrame` tick for the paint loop closure.
fn request_next_frame(callback: &FrameCallback) {
    let _ = web_sys::window().expect("no window").request_animation_frame(
        callback
            .borrow()
            .as_ref()
            .expect("callback not yet installed")
            .as_ref()
            .unchecked_ref(),
    );
}

/// Builds a fresh `DomBackend` + fixed-viewport `Terminal`, sized from `window_size()`.
///
/// ratzilla 0.3.1's `DomBackend::size()` (used by ratatui's `Viewport::Fullscreen` autoresize)
/// computes terminal size from raw `window.innerWidth`/`innerHeight` with a hardcoded 10x20px
/// cell-size guess, completely independent of the grid_parent-bounding-rect + measured-glyph
/// calculation `DomBackend` actually populates its DOM cells with. The two diverge (any font
/// other than that exact 10x20 assumption), so autoresize's buffer size stops matching
/// `DomBackend`'s own `cells` vec and indexing panics on the first post-init redraw. Pin a fixed
/// viewport sized from `window_size()` (the value `DomBackend` actually built its cells with) so
/// ratatui never calls the broken `size()` path: `Viewport::Fixed` skips autoresize entirely.
fn build_terminal() -> Result<Terminal<DomBackend>, JsValue> {
    let mut backend = DomBackend::new().map_err(|e| JsValue::from_str(&e.to_string()))?;
    let size = backend
        .window_size()
        .map_err(|e| JsValue::from_str(&e.to_string()))?
        .columns_rows;
    Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Fixed(Rect::new(0, 0, size.width, size.height)),
        },
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Wires the keydown -> `Intent` -> `reduce` -> `Effect` pipeline onto a terminal's backend.
fn attach_key_listener(
    terminal: &mut Terminal<DomBackend>,
    state: &Rc<RefCell<AppState>>,
) -> Result<(), JsValue> {
    let state = state.clone();
    terminal
        .on_key_event(move |key_event| {
            if let Some((key, modifiers)) = map_key(&key_event)
                && let Some(intent) = key_to_intent(key, modifiers)
            {
                for effect in reduce(&mut state.borrow_mut(), intent) {
                    handle_effect(effect);
                }
            }
        })
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

// Ratzilla is single-threaded (browser event loop): Rc<RefCell<_>>, never Arc<Mutex<_>>
// (CLAUDE.md invariant 3).
#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let state = Rc::new(RefCell::new(AppState::default()));

    let mut terminal = build_terminal()?;
    attach_key_listener(&mut terminal, &state)?;
    // Shared with the paint-loop closure below: a resize rebuilds the `Terminal` in place (see
    // the `resize_pending` handling there), which needs mutable access from outside `run`.
    let terminal = Rc::new(RefCell::new(terminal));

    // `DomBackend`'s own "resize" listener (attached inside `DomBackend::new`) flips a private
    // `initialized` flag so the *next* `Backend::draw()` remeasures cell size and repopulates
    // `cells[]` to the new grid dimensions -- but it never tells ratatui's `Terminal`, whose
    // buffers stay sized to the old viewport (`Viewport::Fixed` skips `autoresize`, deliberately,
    // per `build_terminal`'s doc comment). Diffing that stale, larger buffer against the
    // freshly-shrunk `cells[]` is exactly the documented dom.rs:321 index-out-of-bounds panic,
    // and it reproduces even skipping a paint on the tick where the mismatch would land, because
    // ratatui's double-buffering doesn't guarantee an empty diff on that tick. The reliable fix
    // is to not depend on straddling that internal remeasurement race at all: this listener just
    // flags that a resize happened, and the paint loop below responds by throwing the whole
    // `Terminal` (and its `DomBackend`, DOM grid, and key listener) away and building a new one
    // from scratch with `build_terminal`, the same synchronous, already-correct measurement path
    // used for the very first paint.
    let resize_pending = Rc::new(Cell::new(false));
    {
        let resize_pending = resize_pending.clone();
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
        let on_resize = Closure::<dyn FnMut(web_sys::Event)>::new(move |_: web_sys::Event| {
            resize_pending.set(true);
        });
        window
            .add_event_listener_with_callback("resize", on_resize.as_ref().unchecked_ref())
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
        on_resize.forget();
    }

    // DomBackend attaches its keydown listener to the #grid element (tabindex="0"), not
    // document, so nothing is typeable until it has focus. Grab it the first frame after
    // DomBackend::draw() has created and appended #grid (it doesn't exist before that), and
    // again after every resize rebuild, since that replaces #grid with a new element.
    let mut focused = false;

    let frame_callback: FrameCallback = Rc::new(RefCell::new(None));
    *frame_callback.borrow_mut() = Some(Closure::wrap(Box::new({
        let frame_callback = frame_callback.clone();
        let terminal = terminal.clone();
        let state = state.clone();
        move || {
            if resize_pending.replace(false) {
                // `DomBackend::draw`'s first-frame branch treats a pre-existing `#grid` (found
                // via `getElementById`) as "the same backend redrawing after a resize", and reuses
                // *that* element's tabindex/key-listener setup instead of the fresh backend's own
                // `self.grid` (already configured by `attach_key_listener` below, but never
                // appended). Left in place, the freshly built terminal would render into a grid
                // with no tabindex and no key listener, and the page would go permanently deaf to
                // the keyboard. Remove the old element first so the fresh backend's first `draw`
                // takes the plain "nothing here yet" path instead, exactly like the very first
                // paint on page load.
                if let Some(old_grid) = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.get_element_by_id("grid"))
                {
                    old_grid.remove();
                }
                match build_terminal().and_then(|mut fresh| {
                    attach_key_listener(&mut fresh, &state)?;
                    Ok(fresh)
                }) {
                    Ok(fresh) => {
                        *terminal.borrow_mut() = fresh;
                        focused = false;
                    }
                    Err(err) => web_sys::console::warn_1(
                        &format!("resize: failed to rebuild terminal: {err:?}").into(),
                    ),
                }
            }

            let mut term = terminal.borrow_mut();
            let _ = term.draw(|frame| {
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

            drop(term);
            request_next_frame(&frame_callback);
        }
    }) as Box<dyn FnMut()>));
    request_next_frame(&frame_callback);

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
