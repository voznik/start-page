# start-page

A single-binary personal dashboard and start page. Runs two ways from the same binary:
`start-page tui` (terminal) and `start-page serve` (HTTP daemon + browser UI).

Both modes render through **one Ratatui render function**. The browser target uses Ratzilla's
`DomBackend`. There is no web UI framework in this project.

---

## Architecture (settled — do not re-litigate)

| Decision | Value |
|---|---|
| UI | Ratatui, one `render()` for both targets |
| Browser renderer | `ratzilla::DomBackend` |
| HTTP server | Axum (API + SSE + static assets only) |
| Web bundle | `trunk` |
| Docker client | `bollard` |
| Async runtime | Tokio — **native only**, never in `sp-ui`/`sp-core`/`sp-api` |
| State model | `AppState` + `Intent` + synchronous `reduce()` in `sp-core` |

Rationale lives in `docs/rfc/RFC-002-unified-ratatui-architecture.md`. Read it only when you need
to know *why*. It is not a spec.

---

## Hard invariants

These break silently and are expensive to unwind. Both are CI-enforced; do not disable the gates.

1. **`sp-core`, `sp-ui`, and `sp-api` must compile to `wasm32-unknown-unknown`.**
   No `std::fs`, no `std::time::Instant` in any serialized type (use epoch millis), no Tokio
   runtime, no `reqwest`.

2. **`sp-ui` must not depend on `crossterm`, `ratzilla`, `tokio`, or any provider crate.**
   It defines `sp_ui::Key`; each host maps its own key type into it. The moment `sp-ui` imports
   `crossterm::event::KeyCode`, this becomes a two-renderer project.

3. **`AppState` must not require `Send + Sync`. `reduce()` must be synchronous.**
   Ratzilla is single-threaded: in the browser `AppState` lives in `Rc<RefCell<_>>`. Async work
   happens outside `reduce` and arrives as `Intent::ProviderUpdated(id, payload)`.
   Do not reach for `Arc<Mutex<_>>` on the native side.

4. **`sp-bookmarks` is read-only.** Use `File::open` / `fs::read_to_string` only. The Chromium
   `Bookmarks` file has an MD5 `checksum` field; writing it triggers Chromium's corruption
   recovery path, which can drop nodes. Enforced by a test.

5. **`panic = "unwind"` in the release profile.** Deliberate: a panicking provider must be
   isolated and restarted, not take down the daemon. Do not set `panic = "abort"` for binary size.

6. **Pin exact versions** for `ratatui`, `ratzilla`, and `tachyonfx`. They must move together.
   Currently `ratatui = "=0.30.1"` because `ratzilla 0.3.1` hard-pins that version; a mismatch
   produces two incompatible `ratatui` graphs and unreadable type errors. Bump them as a set.

7. **`ratatui` is `default-features = false, features = ["std"]` workspace-wide.** Its default
   features pull `crossterm` into `sp-ui` and silently break invariant 2. `sp-tui-host` enables
   the `crossterm` feature on its own dependency line, never workspace-wide.

---

## Known upstream defects

Worked around in our code, not forked. Re-check on any `ratzilla` bump; if upstream fixes one,
delete the workaround rather than leaving both.

- **`ratzilla 0.3.1` panics on the first redraw after any state change** — `index out of bounds`
  at `dom.rs:321`, which kills the render loop. `DomBackend` keeps two unsynchronized sizes:
  `cells[]` from real measured glyph size, and `Backend::size()` — the one ratatui's `autoresize()`
  reads every frame — from a hardcoded `window/10x20` guess. First paint survives by luck.
  `sp-web-host` pins `Viewport::Fixed` from `window_size()`, which keeps `autoresize()` off the
  broken path. Unfixed on upstream `main` as of 2026-09-08.

- **`DomBackend` attaches `keydown` to `#grid` (`tabindex="0"`), not `document`.** Nothing is
  typed until that element has focus, which is fatal for a type-and-go start page. `sp-web-host`
  focuses it on the first frame; the element does not exist before that.

- **trunk's bundled `wasm-opt` rejects rustc's default wasm output** (`wasm-validator error ...
  unexpected false`). `crates/sp-web-host/.cargo/config.toml` disables bulk-memory and
  nontrapping-fptoint at codegen. That file is force-tracked: a common global gitignore excludes
  `.cargo`, and without it a fresh clone cannot build the browser target.

---

## Crate map

Dependency direction is strictly left-to-right. Cycles are a build failure.

| Crate | Owns | May NOT depend on |
|---|---|---|
| `sp-core` | `AppState`, `Intent`, `reduce`, `Theme`, `Payload`, provider traits | tokio-rt, std::fs, ratatui, any host |
| `sp-ui` | `render(&mut Frame, &AppState)`, `Key`, `key_to_intent` | crossterm, ratzilla, tokio, providers |
| `sp-config` | Config schema, XDG paths, Excalith JSON importer | any host |
| `sp-api` | Wire DTOs, client (reqwest native / EventSource wasm) | tokio-rt, any host |
| `sp-engine` | Registry, scheduler, task supervision | ratatui, any host |
| `providers/sp-*` | One integration each; data only, no ratatui | each other, any host |
| `providers/sp-*/ui` | Optional `DashboardWidget` impls; ratatui only, no I/O | tokio, std::fs |
| `sp-server` | Axum router, SSE, embedded assets, bind policy | ratatui |
| `sp-tui-host` | crossterm driver (~300 LOC) | ratzilla, axum |
| `sp-web-host` | ratzilla driver (~250 LOC), cdylib, wasm32 | tokio, std::fs, engine, providers |
| `sp-cli` | Arg parsing, mode dispatch | — |

---

## Verification

Run all of these before declaring any task done.

```bash
cargo check -p sp-core -p sp-ui -p sp-api --target wasm32-unknown-unknown
./scripts/check-ui-boundary.sh
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The boundary check is a script, not an inline `cargo tree | grep`: grep exits 1 when the tree is
*clean*, so the obvious one-liner reports backwards in a shell step. CI runs the same script.

A green `cargo check` or `trunk build` proves the browser target compiles, never that it runs.
Anything touching `sp-web-host` needs a real browser load — the ratzilla redraw panic above passed
every compile-time gate we had.

---

## Do not

One line each. Rationale is in the RFC; do not ask for it inline.

- Do not add Leptos, Dioxus, Yew, Sycamore, or Tailwind. Rejected.
- Do not add `cargo-leptos`. The bundle tool is `trunk`.
- Do not build a second render path for the browser.
- Do not write a bespoke Docker HTTP-over-socket client. Use `bollard`.
- Do not use `stats(stream=true)`. It holds one connection per container. Poll `stream=false`.
- Do not use `broadcast` for provider fan-out. Use `watch` — last-value-wins, no backpressure.
- Do not skip `Docker::negotiate_version()`.
- Do not use `inventory` or distributed slices for provider registration. Explicit list.
- Do not put `serde_json::Value` in `Payload` except behind the `Raw` variant.
- Do not act on `docs/rfc/RFC-001-*.md`. It is superseded and recommends a different architecture.

---

## Conventions

- Rust 2024 edition, `resolver = "3"`.
- Errors: `thiserror` in libraries, `anyhow` in `sp-cli` only.
- A provider that cannot run is `Status::Unavailable { reason }`, not `Status::Error`.
  "Docker not installed", "no Chrome profile", "macOS denied access" are normal states and render
  muted, never red.
- Commit per work item. Reference the item ID (`T0.2`) in the message.
- When a task's acceptance criteria can't be met as written, stop and report. Do not substitute an
  adjacent approach.
