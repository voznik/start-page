# Ratzilla API Brief (for `sp-web-host`)

Verified against published crates.io metadata + the `v0.3.1` git tag of
`github.com/ratatui/ratzilla` (org migrated from `orhun/ratzilla` — old URL
redirects, use `ratatui/ratzilla`) on 2026-09-08. Do not trust `main` branch
examples as "current API" without checking they match a tagged release —
`main` already carries post-0.3.1 backend work (`MultiBackendBuilder`,
`WebGl2BackendOptions`) not in the published crate.

## 1. Versions

| Crate | Latest published | Source |
|---|---|---|
| `ratzilla` | **0.3.1** (2026-06-08) | crates.io `ratzilla` version list |
| `ratatui` | **0.30.1** (2026-06-05) | crates.io `ratatui` version list |
| `tachyonfx` | **0.24.0** (2026-02-14) | crates.io `tachyonfx` version list |

`ratzilla 0.3.1`'s `Cargo.toml` pins:
```toml
ratatui = { version = "0.30.1", default-features = false, features = ["all-widgets", "layout-cache"] }
```
This is a hard pin, not a caret range on a different minor — **`sp-ui`/`sp-core` must build against ratatui 0.30.1 exactly** (or whatever ratzilla's next release pins) or the wasm target will fail to unify the two `ratatui` crate graphs. Ratzilla's own CHANGELOG for 0.3.1: "Ratatui 0.30.1 is here!" (bumped from 0.30.0 in PR #181). Check `ratzilla`'s `Cargo.toml` again if bumping either crate later.

`ratzilla`'s other pinned deps worth knowing (from its `[dependencies]`, not yours to add, but they explain what's already linked in): `web-sys 0.3.81`, `compact_str 0.9.0`, `beamterm-renderer 1` (used by the WebGL2 backend only), `bitvec 1.0.1`, `unicode-width 0.2.2`.

## 2. Minimal `sp-web-host` main — DomBackend

This is the README "Manual Setup" example, reproduced verbatim from the
`v0.3.1` tag (matches the published crate; the repo's `examples/minimal` on
`main` has drifted to a multi-backend demo — ignore that one for a minimal
target):

```rust
use std::{cell::RefCell, io, rc::Rc};

use ratzilla::ratatui::{
    layout::Alignment,
    style::Color,
    widgets::{Block, Paragraph},
    Terminal,
};

use ratzilla::{event::KeyCode, DomBackend, WebRenderer};

fn main() -> io::Result<()> {
    let counter = Rc::new(RefCell::new(0));
    let backend = DomBackend::new()?;
    let mut terminal = Terminal::new(backend)?;

    terminal.on_key_event({
        let counter_cloned = counter.clone();
        move |key_event| {
            if key_event.code == KeyCode::Char(' ') {
                let mut counter = counter_cloned.borrow_mut();
                *counter += 1;
            }
        }
    })?;

    terminal.draw_web(move |f| {
        let counter = counter.borrow();
        f.render_widget(
            Paragraph::new(format!("Count: {counter}"))
                .alignment(Alignment::Center)
                .block(
                    Block::bordered()
                        .title("Ratzilla")
                        .title_alignment(Alignment::Center)
                        .border_style(Color::Yellow),
                ),
            f.area(),
        );
    });

    Ok(())
}
```

Key signatures (from this example — no separate rustdoc fetch was needed,
the shapes are unambiguous from the call sites):

- `DomBackend::new() -> io::Result<DomBackend>` — constructs the DOM-based
  ratatui backend. It renders into `<pre>`/DOM nodes, not a `<canvas>` (that's
  `CanvasBackend`/`WebGl2Backend`, separate types — do not reach for those,
  `sp-server`'s CLAUDE.md constraints call for the simplest path and DOM is
  it).
- `Terminal::new(backend) -> io::Result<Terminal<DomBackend>>` — this is
  `ratzilla::ratatui::Terminal`, i.e. ratatui's own `Terminal` type
  re-exported through `ratzilla::ratatui` (ratzilla re-exports the whole
  `ratatui` crate at that path so version skew between your direct `ratatui`
  dep and ratzilla's internal one can't happen — **import `ratatui` types via
  `ratzilla::ratatui::*` in `sp-web-host`, not via a separate `ratatui`
  crates.io dependency**).
- `terminal.on_key_event(impl FnMut(KeyEvent) + 'static) -> io::Result<()>` —
  registers a DOM `keydown` listener; single registration (calling it again
  presumably replaces/adds — the example only shows one call, treat multiple
  registrations as unverified). `KeyCode` is `ratzilla::event::KeyCode`, not
  `crossterm::event::KeyCode` — this is exactly the boundary `sp-ui::Key`
  exists to abstract per this repo's CLAUDE.md; the web host maps
  `ratzilla::event::KeyEvent`/`KeyCode` into `sp_ui::Key` the same way
  `sp-tui-host` maps `crossterm::event::KeyCode`.
- `terminal.draw_web(impl FnMut(&mut Frame) + 'static)` — **no return value
  shown, and it is not `draw` (ratatui's synchronous single-shot draw).**
  `draw_web` registers the closure as ratzilla's render callback (driven by
  `requestAnimationFrame` internally per the crate description — not
  independently confirmed via rustdoc, but consistent with "runs continuously
  in the browser" framing throughout the README). Call it once at startup
  with your `render(&mut Frame, &AppState)` closure; do not call it in a
  loop.
- Mouse events: `terminal.on_mouse_event(impl FnMut(MouseEvent) + 'static) -> io::Result<()>` exists (seen in the multi-backend `main` example) with `ratzilla::event::{MouseButton, MouseEventKind}` — not needed for a minimal target but the hook is there if `sp-ui` grows mouse support later.

### Cargo features / deps needed in `sp-web-host`

```toml
[dependencies]
ratzilla = "0.3.1"
console_error_panic_hook = "0.1"   # ratzilla already pulls this in; call console_error_panic_hook::set_once() in main for real wasm panic messages
wasm-bindgen = "0.2"               # matches ratzilla's web-sys 0.3.81 generation; do not pin a mismatched wasm-bindgen minor
web-sys = { version = "0.3", features = ["Window"] }  # only if sp-web-host calls window() itself, e.g. window().open_with_url — Hyperlink widget's own web-sys usage is internal to ratzilla and doesn't leak this requirement onto you
```

`window().open_with_url(url) -> Result<Option<Window>, JsValue>` is a method
on `web_sys::Window` gated only by the `"Window"` web-sys feature (no extra
feature needed beyond that — verified against the feature list convention
web-sys uses, not independently fetched from web-sys's own docs this pass).
If `sp-web-host` never calls it directly (e.g. all links render through
ratzilla's own `Hyperlink` widget), this dependency isn't needed at all —
check what `sp-ui`'s render function actually emits before adding it.

`[package] crate-type = ["cdylib"]` is required for the wasm32 target (per
this repo's crate map: `sp-web-host` is cdylib, wasm32). `wasm-bindgen(start)`
on `main`, or a plain `fn main()` — the README example above uses a plain
`fn main() -> io::Result<()>` with no `#[wasm_bindgen(start)]` attribute, so
trunk's default entry-point convention (calling `main()` from the generated
JS glue) is sufficient; no macro needed.

## 3. `index.html` / Trunk requirements

Trunk auto-injects the compiled wasm as an ES module via:
```html
<link data-trunk rel="rust"/>
```
This tag is what trunk rewrites at build time — it must be present in
`index.html` (the README's copy omits it from the collapsed `<details>` block
shown to end users but the live repo's actual `examples/minimal/index.html`
does **not** include it either — trunk's default behavior injects the
`<script type="module">` bundle loader automatically even without the
explicit `<link data-trunk rel="rust"/>` tag when trunk finds a `Cargo.toml`
in the same directory. Include the explicit tag anyway; it's the documented,
unambiguous form and costs nothing.)

Full `index.html` from ratzilla's own minimal example (`v0.3.1` tag,
`examples/minimal/index.html`):
```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta
      name="viewport"
      content="width=device-width, initial-scale=1.0, user-scalable=no"
    />
    <link
      rel="stylesheet"
      href="https://cdnjs.cloudflare.com/ajax/libs/firacode/6.2.0/fira_code.min.css"
    />
    <title>Ratzilla</title>
    <style>
      body {
        margin: 0;
        width: 100%;
        height: 100vh;
        display: flex;
        flex-direction: column;
        justify-content: center;
        align-items: center;
        align-content: center;
        background-color: #121212;
      }
      pre {
        font-family: "Fira Code", monospace;
        font-size: 16px;
        margin: 0px;
      }
    </style>
  </head>
  <body></body>
</html>
```

**The `<link rel="stylesheet" href="https://cdnjs.cloudflare.com/...">` line
is the CDN this repo's CLAUDE.md forbids — see §4 for the local replacement.**

Styling hook: `DomBackend` renders each terminal cell into a `<pre>` element
(confirmed by the `pre { font-family: ...; font-size: ...; }` rule being the
only typography styling in every ratzilla example's `index.html`) inside
whatever container ratzilla mounts to `<body>` — the example never adds an
explicit mount div, so ratzilla appends directly to `document.body`. Font
family and size are controlled purely via that `pre` CSS rule; there is no
Rust-side font API. `monospace` as the fallback in the `font-family` stack is
required — box-drawing/powerline glyph alignment breaks on a proportional
fallback.

No `Trunk.toml` was found at the repo root or in `examples/minimal/` on the
`v0.3.1` tag (`GET` returned 404 both times) — ratzilla's examples build with
trunk's zero-config defaults (serves from the directory containing
`index.html`, output to `dist/`). If `sp-web-host` needs a `Trunk.toml` for
asset copying (see §4), write one from scratch; there's no ratzilla template
to copy.

## 4. Self-hosted Nerd Font (replacing the forbidden CDN)

Nerd Fonts latest release: **v3.5.1** (github.com/ryanoasis/nerd-fonts
`releases/latest` resolves to this tag as of 2026-09-08).

For box-drawing + powerline + icon glyph coverage without shipping a full
monospace typeface (you likely want to keep Fira Code or another font for
the actual letterforms and only patch in the symbol glyphs), the release
publishes a dedicated symbols-only font:

- Asset name: `NerdFontsSymbolsOnly.zip` / `NerdFontsSymbolsOnly.tar.xz`
- Download URL: `https://github.com/ryanoasis/nerd-fonts/releases/download/v3.5.1/NerdFontsSymbolsOnly.zip`
- Inside the archive: `Symbols Nerd Font.ttf` / `SymbolsNerdFont-Regular.ttf`
  (TTF only in this asset — **no WOFF2 is published by nerd-fonts upstream**;
  you must convert the TTF yourself, e.g. `fonttools` /
  `woff2_compress`, to get a WOFF2 for web delivery).

If a full monospace-with-icons font is preferred instead (simpler: one
`@font-face`, no separate symbol-only fallback layer), the same release also
publishes patched full fonts, e.g. `FiraCode.zip` /
`JetBrainsMono.zip` (both matching the family ratzilla's own examples
default to) — same caveat, TTF/OTF only, convert to WOFF2 locally.

### Trunk asset-copy config

Trunk copies static assets via `<link data-trunk rel="copy-file" ...>` (or
`copy-dir` for a directory) in `index.html` — this is trunk's own mechanism,
not a `Trunk.toml` setting (no `Trunk.toml` was found to configure, per §3).
Place the converted `.woff2` under e.g. `assets/fonts/` and add:

```html
<link data-trunk rel="copy-file" href="assets/fonts/SymbolsNerdFontMono-Regular.woff2" />
```

### `@font-face` + `pre` rule (local, no CDN)

```css
@font-face {
  font-family: "Fira Code";
  src: url("/fira-code.woff2") format("woff2");
  font-weight: 400;
  font-display: swap;
}
@font-face {
  font-family: "Symbols Nerd Font";
  src: url("/fonts/SymbolsNerdFontMono-Regular.woff2") format("woff2");
  font-display: swap;
}
pre {
  font-family: "Fira Code", "Symbols Nerd Font", monospace;
  font-size: 16px;
  margin: 0;
}
```
Order matters: the base font first so ordinary glyphs render in it, the
Nerd Font symbols-only face second as fallback so the browser only reaches
into it for glyphs Fira Code doesn't have, `monospace` last as the ultimate
fallback. Trunk copies files to `dist/` flattened relative to the output dir
by default — adjust the `url()` paths to match wherever `copy-file` lands
them (verify against the actual `dist/` output once building; not simulated
here).

## 5. Known gotchas (from ratzilla's own CHANGELOG, `v0.3.1` and `v0.3.0` entries)

- **Cell-dimension / resize handling was unreliable before 0.3.1.** The
  0.3.1 CHANGELOG lists two fixes specifically about this: "_(dom)_ Measure
  terminal cell size" (bug fix, PR #159) and a new "_(backend)_ Add
  `CellSized` trait for querying cell dimensions in physical and CSS pixels"
  (feature, PR #165). If cell sizing looks wrong on resize, that trait
  (`ratzilla`'s `CellSized`, exact path not fetched this pass — check
  `docs.rs/ratzilla/0.3.1` if needed) is the documented fix point, not a
  manual recompute.
- **`DomBackend` had double-buffering removed in 0.3.0** ("_(dom)_ Removes
  double buffering from `DomBackend`", PR #138) — a deliberate simplification
  upstream, not a regression to work around.
- **Initial paint timing**: not documented in the CHANGELOG or README beyond
  what's inferable from the API shape (`draw_web` registers a callback rather
  than drawing synchronously) — treat first-paint timing as
  requestAnimationFrame-driven and unverified beyond that; do not assume
  synchronous first paint on `Terminal::new`.
- **Bundle size**: no numbers published in the README/CHANGELOG for this
  pass. `beamterm-renderer` is pulled in as a dependency but only exercised
  by the WebGL2 backend per the crate's own feature/backend split — using
  `DomBackend` (as this brief recommends) avoids linking that renderer's
  runtime cost, though it's still compiled in since ratzilla doesn't
  feature-gate it. Not independently verified with an actual `wasm-opt`
  build; measure once `sp-web-host` exists rather than trusting this note.

## Sources

- crates.io API: `ratzilla`, `ratatui`, `ratatui/0.30.1`, `tachyonfx` version listings
- `github.com/ratatui/ratzilla` (redirected from `orhun/ratzilla`), tag `v0.3.1`: `README.md`, `Cargo.toml`, `CHANGELOG.md`, `examples/minimal/{src/main.rs,index.html,Cargo.toml}`
- `github.com/ratatui/ratzilla` `main` branch: same files, used only to confirm `main` has drifted past 0.3.1 (not cited as current-API source)
- `github.com/ryanoasis/nerd-fonts` `releases/latest` → v3.5.1, and its release asset list
