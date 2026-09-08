# Local Work Items

This document tracks local, pre-GitHub work items and tasks implemented on the repository. Each item follows the work item schema from `docs/plans/phase-0-2-work-items.md` and links to its corresponding commit.

---

## Status Legend

- `[CLOSED]` Implemented, verified, and committed to branch.
- `[OPEN]` Scheduled or in-progress work item.

---

## Phase 0 — Skeleton and De-risking

### T0.1: Workspace skeleton
- **Status:** `[CLOSED]`
- **Commit:** `8167446`
- **Scope:** Create the Cargo workspace skeleton with crate layout, stub modules, and clap CLI dispatch.
- **Verification / Acceptance:**
  - `cargo check --workspace` passing across targets.
  - `cargo check -p sp-core -p sp-ui -p sp-api --target wasm32-unknown-unknown` passing.
  - Dependency boundaries respected (no host dependencies in core/ui/api).

### T0.2: Ratzilla spike & Decision Gate
- **Status:** `[CLOSED]`
- **Commits:** `d27649f` (spike), `efb9677` (decision gate report)
- **Scope:** Prove unified `render()` across terminal and browser; measure bundle size, warm-cache first paint, time-to-keystroke, and Nerd Font glyph rendering.
- **Decision Gate Findings:**
  - Raw WASM: 3.4 MB | gzip: 310 KB | brotli: 94 KB.
  - First contentful paint: 180 ms (budget < 200 ms).
  - Time to first keystroke: 168 ms (budget < 400 ms).
  - Verdict: Proceed to Phase 1.

### T0.3: CI matrix & Boundary Gate
- **Status:** `[CLOSED]`
- **Commit:** `8743fd0`
- **Scope:** GitHub Actions workflow matrix (Linux, macOS, Windows).
- **Verification / Acceptance:**
  - `./scripts/check-ui-boundary.sh` enforcing strict crate isolation.
  - Non-blocking WASM bundle size reporting.
  - Clippy and test gates configured.

### T0.4: Config schema & Theme
- **Status:** `[CLOSED]`
- **Commit:** `770245b`
- **Scope:** `sp-config` schema, default fallback, XDG path resolution (`~/.config/start-page/config.toml`), `sp-core::Theme` with named palette.
- **Verification / Acceptance:**
  - Serde round-trip tests passing.
  - Missing config file defaults safely without error.
  - `start-page config path` CLI command.

### T0.5: Release pipeline
- **Status:** `[CLOSED]`
- **Commit:** `124eda1`
- **Scope:** `cargo-dist` release automation producing binaries for 6 target architectures.
- **Verification / Acceptance:**
  - Release profile configured with `panic = "unwind"`, `lto = "fat"`, `opt-level = 3`.
  - Wasm profile configured with `opt-level = "z"`.

---

## Phase 1 — Core and TUI

### T1.1: State machine
- **Status:** `[CLOSED]`
- **Commit:** `90cf8ea`
- **Scope:** `sp-core` pure synchronous `reduce(&mut AppState, Intent) -> Vec<Effect>`.
- **Verification / Acceptance:**
  - `AppState` compiles without `Send`/`Sync` bounds.
  - Synchronous `reduce()` with zero I/O.
  - Table-driven unit tests for intent sequences.

### T1.2: Shared input mapping
- **Status:** `[CLOSED]`
- **Commit:** `96390df`
- **Scope:** `sp_ui::Key` abstraction and `key_to_intent`. Host key adapters in `sp-tui-host` (crossterm) and `sp-web-host` (ratzilla).
- **Verification / Acceptance:**
  - `sp-ui` isolated from host key types.
  - Identical intents emitted for matching physical keys in both hosts.

### T1.3: Real render pipeline
- **Status:** `[CLOSED]`
- **Commit:** `16d301f`
- **Scope:** `sp-ui::render` implementing link grid, category groupings, prompt line, and theme-driven status line.
- **Verification / Acceptance:**
  - Validated across 80x24 and 200x60 viewports.
  - Text ellipsis on long names; CJK and wide characters handled without grid corruption.

### T1.4: Nucleo fuzzy filtering & Autosuggestion
- **Status:** `[CLOSED]`
- **Commit:** `cad007a`
- **Scope:** Live fuzzy search over links with `nucleo`. Inline ghost autosuggestion accepted via `Tab`.
- **Verification / Acceptance:**
  - Sub-millisecond filter time (benchmarked at ~83 µs for 1,000 links).
  - Clean wasm32 compilation.

### T1.5: Command parser & URL encoding
- **Status:** `[CLOSED]`
- **Commit:** `e910c3c`
- **Scope:** Command dispatcher: `help`, `config path`, `config edit`, `theme <name>`, search prefix shortcuts, direct URL navigation, search fallback.
- **Verification / Acceptance:**
  - Query strings properly percent-encoded.
  - Fallback to search engine for unrecognised commands.

### T1.6: TUI host driver & Crash resilience
- **Status:** `[CLOSED]`
- **Commit:** `7c33764`
- **Scope:** `sp-tui-host` event loop, effect handler (URL opening via `open`), signal handling.
- **Verification / Acceptance:**
  - Terminal raw mode reliably restored on panic hook and `SIGINT`.
  - Resize events handled cleanly without rendering artifacts.

### T1.7: Golden snapshot tests
- **Status:** `[CLOSED]`
- **Commit:** `fce0410`
- **Scope:** 12 `insta` golden snapshot tests covering empty state, filtered links, commands, and error boundaries.
- **Verification / Acceptance:**
  - All snapshots reviewed and committed.

### T1.8: TUI integration & v0.1 release
- **Status:** `[CLOSED]`
- **Commit:** `e1ce71c`
- **Scope:** Wire `start-page tui` CLI subcommand, create initial `README.md` and `CHANGELOG.md`.

---

## Phase 2 — Browser Target & Web Distribution

### T2.1: Browser web host & Resize handling
- **Status:** `[CLOSED]`
- **Commit:** `d2790f9`
- **Scope:** `sp-web-host` driven by `ratzilla::DomBackend`. Browser effect handler opening links via `web_sys::window().open_with_url`.
- **Verification / Acceptance:**
  - Byte-equivalent rendering between TUI and browser DOM.
  - Viewport pinned to avoid Ratzilla 0.3.1 redraw resize panic.

### T2.2 / Fix: Link URL normalization
- **Status:** `[CLOSED]`
- **Commit:** `aa575cc`
- **Scope:** Ensure links have valid URI schemes (`https://`) before passing to browser/TUI effect handlers.

### T2.3: Static deployment mode
- **Status:** `[CLOSED]`
- **Commit:** `5d187fe`
- **Scope:** Self-contained static site output via `trunk build --release --no-default-features`.
- **Verification / Acceptance:**
  - Documented `file://` limitation (WASM fetch constraints under browser CORS policies).
  - Deployable to static HTTP hosting (GitHub Pages, Cloudflare Pages, S3).

### T2.4: Binary asset embedding & Brotli serving
- **Status:** `[CLOSED]`
- **Commit:** `b3671fa`
- **Scope:** `sp-server` embeds compiled web distribution via `rust-embed`. Assets pre-compressed with Brotli and served via `tower-http`.
- **Verification / Acceptance:**
  - `start-page serve` runs as a fully self-contained single binary with zero runtime file dependencies.
  - `xtask dist` builds web bundle and embeds into native binary in one step.

### T2.5: Excalith UI Overhaul & YAML Config Migration
- **Status:** `[CLOSED]`
- **Commit:** `cab56f2`
- **Scope:** Multi-column section grid with accent colors, Nerd Font icon mapping, terminal prompt line, YAML configuration schema (`config.yaml`), Excalith JSON importer (`start-page import <path>`), zero-allocation render loop, and Ponytail simplification.
- **Verification / Acceptance:**
  - Passed WASM portability, UI boundary script, clippy strict, 47 unit/integration tests, and 12 golden snapshots.
  - Net -118 lines pruned in Ponytail pass.

---

## Phase 3 — Docker Provider & Provider Engine

### T3.1: Provider Traits, Engine Supervision & Fan-out
- **Status:** `[OPEN]`
- **Scope:** Define `DataProvider` trait and actor supervision in `sp-engine`. Runs actors in isolated Tokio tasks (`panic = "unwind"`), restarting with exponential backoff on failure. Broadcast state updates over `tokio::sync::watch` (last-value-wins).

### T3.2: Docker Native Data Provider
- **Status:** `[OPEN]`
- **Scope:** `sp-docker` actor using `bollard 0.21.1`. Auto-discovers Docker/Podman sockets, negotiates version, streams container events with 30s ping watchdog, polls resource metrics (stream=false), and computes CPU/memory deltas.

### T3.3: Docker UI Dashboard Widget
- **Status:** `[OPEN]`
- **Scope:** `sp-docker/ui` wasm32-clean widget rendering container status, health badges, ports, and resource bars in Ratatui without I/O or host dependencies.

### T3.4: Server SSE Pipeline & Web Client
- **Status:** `[OPEN]`
- **Scope:** Wire `sp-server` `/events` endpoint (SSE) streaming serialized `Payload` updates. Connect `EventSource` in `sp-api` / `sp-web-host` and dispatch `Intent::ProviderUpdated` into state.

### T3.5: Multi-Pane Dashboard Layout & Navigation
- **Status:** `[OPEN]`
- **Scope:** Adapt `sp-ui::render` into a responsive multi-pane layout (Links grid + Container monitor). Add keyboard navigation (`Tab`, arrow keys) for switching focus and scrolling containers.

---

## Tooling, Infrastructure & Containerization

### Toolchain Pinning
- **Status:** `[CLOSED]`
- **Commit:** `5158cf9`
- **Scope:** Pinned Rust toolchain (`rust-toolchain.toml`) across local development, CI workflows, and container builds.

### Binary Containerization
- **Status:** `[CLOSED]`
- **Commit:** `c84107d`
- **Scope:** Minimal Dockerfile for building and executing `start-page`.

### Multi-stage Web Container Build
- **Status:** `[CLOSED]`
- **Commit:** `ebc09f9`
- **Scope:** Container build compiling both browser WASM bundle and server binary; serving pre-compressed assets.
