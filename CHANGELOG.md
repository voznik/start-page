# Changelog

## v0.1.0 — unreleased

First release of the Rust rewrite. The previous Next.js implementation is archived on the
`archive/nextjs-start-page` branch; nothing carried over but the theme and settings fixtures under
`data/`.

### The terminal UI works

- Type-to-filter over links with `nucleo` fuzzy matching, 83us per keystroke over 1,000 links.
- Inline ghost suggestion, zsh style, accepted with `Tab`.
- Command parsing: `help`, `config path`, `config edit`, `theme <name>`, configured prefix
  shortcuts, direct URLs, and search fallback for anything unrecognised. Query strings are
  percent-encoded — the upstream project shipped a bug here.
- Terminal state is restored on panic and on an external `SIGINT`, verified by triggering both.
- Config at `~/.config/start-page/config.toml` (`XDG_CONFIG_HOME` honoured, native paths
  elsewhere); a missing file yields defaults.

### The browser target renders

One `sp_ui::render` drives both the terminal and the browser, the latter through Ratzilla's DOM
backend and WebAssembly. Measured warm-cache: 180ms to first contentful paint, 168ms to the first
accepted keystroke, 94KB brotli. It does not open links yet.

### Two upstream defects worked around

- Ratzilla 0.3.1 panics on the first redraw after any state change, because `DomBackend` keeps two
  unsynchronized size computations. Pinned to a fixed viewport to keep ratatui's autoresize off the
  broken path. Still unfixed upstream.
- Ratzilla attaches its key listener to the grid element rather than the document, and never calls
  `preventDefault` — so the page ignored typing until clicked, then went permanently deaf after one
  `Tab`. Both fixed in `sp-web-host`.

### Not in this release

`serve`, `sync` and `import` exit "not implemented". There is no error state in `AppState`, no
help view, and themes are defined but not yet selectable at runtime. Links are not grouped by
category — the layout supports it, the data doesn't carry it yet.
