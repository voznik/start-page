# RFC-002: Unified Ratatui Architecture (Revised Constraint Set)

**Status:** Draft for review — supersedes RFC-001 §1, §2, §4, §5 under the constraints below
**Date:** 2026-09-08
**Relationship to RFC-001:** RFC-001 remains valid *as written for its constraint set*. This document re-derives the decision after removing five requirements. §3 (Docker, bookmarks) carries over largely intact and is summarised rather than repeated.

---

## 0. The correction, stated plainly

RFC-001 recommended Option B (shared core + Ratatui/Leptos). Five of the objections that drove it were:

- (a) wallpaper with blur and terminal-window opacity
- (b) mobile/tablet layout
- (c) text selection
- (d) real anchor semantics
- (e) accessibility

**You are right that I over-weighted these, and in two specific ways beyond simply assuming them.**

First, (a) and (b) were **inherited product assumptions, not requirements from your brief**. I imported them from the upstream Excalith feature set — the wallpaper/blur config keys and the author's note about using the web build on an iPad — and treated them as fixed. They were never in your goals, which say "self-hosted web application" and "TUI," not "responsive touch-friendly web application with graphical chrome." That was a reasoning error, not just a differing assumption.

Second, two of my technical claims were badly calibrated and would have been wrong even under RFC-001's constraints:

- **The frame-rate objection was measured on the wrong workload.** The ~10–30 fps figure comes from the Ratzilla *demo/animations* example, which is a deliberate render stress test that dirties most of the buffer every frame. A start page dirties maybe a few hundred cells every five seconds. Applying an animation benchmark to a near-static dashboard was a category error.
- **I claimed the GPU backends give up text selection.** They do not. Ratzilla's `WebGl2Backend` exports a `SelectionMode` type; selection is a supported feature there. The DOM-vs-GPU trade is narrower than I described.

**Answer to your question: yes. With (a)–(e) out of scope, the unified Ratatui architecture becomes the recommendation, not merely a viable alternative.** The rest of this document works out what that actually costs and what it buys.

### 0.1 What survives from RFC-001's case against Option A

Two objections were never about (a)–(e), and they still stand. Both are manageable; one needs designing around.

**S1. WASM cannot reach the Docker socket or the filesystem.** You still need a daemon, a transport, and DTOs. Option A saves you a *view layer*, not an architecture. This is not a defect — the view layer is precisely what you wanted to save — but it means "one binary, one codebase" does not become "no client/server split." The daemon in §6 is nearly identical either way.

**S2. There is no server-side-rendering path for Ratzilla.** The browser shows nothing until the WASM module downloads, instantiates, and draws its first frame. RFC-001 §0 established that **time-to-first-keystroke on a cold new tab is this product's defining metric** — the user opens a tab and immediately types. Leptos's streaming SSR paints the shell on the first byte; Ratzilla cannot. **This is now the single strongest argument against Option A**, and unlike (a)–(e) you cannot make it go away by declaring it out of scope, because it is a property of the thing you are actually building. §4.3 designs around it.

---

## 1. Revised recommendation

**Adopt Option A: one Ratatui render function, driven by a terminal backend natively and by Ratzilla's `DomBackend` in the browser.**

The pivotal structural move is unchanged from RFC-001 and becomes *more* valuable here, not less: the shared core owns the interaction state machine (`AppState` / `Intent` / `reduce`). But now a second crate joins it on the shared side:

```rust
// sp-ui — the crate that makes this architecture work.
// Backend-agnostic. No I/O. No crossterm. No ratzilla. wasm32-clean.
pub fn render(frame: &mut Frame, state: &AppState) { … }
pub fn key_to_intent(code: KeyCode, mods: KeyModifiers) -> Option<Intent> { … }
```

Everything above that line is shared. Everything below is a thin host:

| Host | Responsibility | Est. size |
|---|---|---|
| `sp-tui-host` | crossterm raw mode, event loop, `ratatui::init()`/`restore()` | ~300 LOC |
| `sp-web-host` | `DomBackend::new()`, `terminal.draw_web(…)`, `on_key_event`, SSE subscription | ~250 LOC |

Two hosts of ~300 lines each, against RFC-001's Option B where `sp-web` was an entire Leptos application with its own component tree, its own CSS, its own build pipeline, and its own re-implementation of every widget. That is the trade, and under the revised constraints it is not close.

### 1.1 A technical constraint that falls out of this, and matters

Ratzilla's API is single-threaded and `'static`-closure based:

```rust
let counter = Rc::new(RefCell::new(0));
terminal.on_key_event(move |key_event| { … })?;
terminal.draw_web(move |f| { … });
```

In the browser, `AppState` lives in an `Rc<RefCell<AppState>>` inside a `requestAnimationFrame`-driven loop. There are no threads.

Therefore: **`AppState` must not require `Send + Sync`, and `reduce()` must be synchronous.** Async work (fetching a provider snapshot, subscribing to SSE) happens *outside* `reduce` and lands as an `Intent::ProviderUpdated(id, payload)`. This is a good discipline anyway — it is what makes `reduce` trivially testable — but it must be a stated invariant from day one, because it is easy to violate accidentally on the native side where `Arc<Mutex<_>>` is the reflexive choice, and you will not discover the violation until the WASM build fails weeks later.

Enforce it in CI alongside the wasm32 check:

```bash
cargo check -p sp-core -p sp-ui -p sp-api --target wasm32-unknown-unknown
```

### 1.2 Revised trade-off matrix

Same scale as RFC-001 (1–5), re-scored with (a)–(e) out of scope and the two miscalibrations corrected. Rows struck from scope are shown for traceability but excluded from the verdict.

| Criterion | **A: Unified Ratatui** | B: Ratatui + Leptos |
|---|:---:|:---:|
| Terminal fidelity / aesthetic match | **5** | 4 |
| Web UX (desktop, keyboard-driven) | **4** | 5 |
| ~~Web UX (mobile/tablet)~~ | ~~1~~ | ~~5~~ |
| ~~Wallpaper, blur, graphical chrome~~ | ~~2~~ | ~~5~~ |
| ~~Accessibility~~ | ~~2~~ | ~~5~~ |
| Text selection | 4 (DOM & WebGL2) | 5 |
| Steady-state render perf (this workload) | **4** | 4 |
| Time-to-first-keystroke (cold tab) | 2 → **4** with §4.3 prerender | **5** |
| Client bundle size | 4 | 3 |
| Widget reuse across both targets | **5** | 2 |
| Build pipeline complexity | **5** | 2 |
| Single-binary packaging | **5** | 4 |
| Initial implementation cost | **4** | 2 |
| Long-run drift risk | **5** (structurally impossible) | 3 |
| Ecosystem risk | 3 | 2 |
| Escape hatch if a key dep dies | **4** (fork `Backend` impl) | 2 (cannot fork Leptos) |
| **Verdict** | **Adopt** | Fallback if §9 triggers |

Note the two rows that flip hardest. **Widget reuse** and **drift risk** are where Option A's advantage is structural rather than incremental — see §3.

---

## 2. What you are still giving up

These are the honest costs. None is a blocker under the revised constraints, but you should decide about them deliberately rather than discover them in Phase 3.

**2.1 First paint.** Covered in §0.1 S2 and designed around in §4.3. The residual cost after mitigation is a theme-coloured static frame for ~100–300 ms instead of live content.

**2.2 Images.** Excalith's `fetch` block supports a custom image. In a terminal, `ratatui-image` can do this via kitty/iTerm2/sixel protocols — but only in terminals that support them, so it is already not portable. In Ratzilla there is no image path at all. Practical options: ASCII/ANSI art (which is arguably more on-theme), or — since `DomBackend` renders `<span>` cells inside a `<pre>` — absolutely position an `<img>` behind the `<pre>` from `index.html` and leave a reserved region of transparent cells over it. That second trick works but couples your layout to a hand-maintained CSS offset; use it only if the image genuinely matters.

**2.3 Browser-native affordances.** Middle-click-to-open-in-new-tab, right-click context menu, and Ctrl+F find-in-page work on real anchors and real text. Ratzilla ships a `Hyperlink` widget, so `DomBackend` does emit real anchors — you keep more of this than you might expect. On `CanvasBackend`/`WebGl2Backend` you lose it and would hand-roll hit-testing plus `window.open`.

**2.4 Fonts and glyph coverage.** On `DomBackend` this is just CSS: load a Nerd Font webfont and every glyph works (the Ratzilla template does exactly this with Fira Code from a CDN — self-host it instead). On `WebGl2Backend` glyphs come from a font atlas: `FontAtlasConfig` is either `Static(FontAtlasData)` (pre-generated with the `beamterm-atlas` CLI, fast but limited to the characters baked in) or `Dynamic` (rasterised on demand). Nerd Fonts have thousands of glyphs, so a static atlas means committing to a build-time subset of the icons you actually use. This is a real build-pipeline cost and a good reason to default to `DomBackend`.

**2.5 Ecosystem risk changes shape.** RFC-001 worried about two pre-1.0 frameworks (Leptos 0.8 with 0.9 in beta, Dioxus 0.7 with 0.8 in alpha). Option A replaces that with a single dependency on `ratzilla 0.3.1` — smaller, younger, maintained by the Ratatui org (orhun, junkdog) with ~1.4k stars, 35 open issues and 14 open PRs, and a visible lag when tracking Ratatui breaking changes (the 0.30 upgrade took several weeks and needed coordinated `tachyonfx` updates).

That sounds worse but the risk profile is better in the way that counts: **Ratzilla implements `ratatui::backend::Backend`, which is a small trait.** If it stalls, forking it is a tractable few-thousand-line ownership. You cannot say that about Leptos. RFC-001 rated Option B's dependency risk as lower; on the metric of *recoverability* rather than *likelihood*, that was backwards.

**2.6 A single point of failure across both renderers.** Under Option B, a Ratatui breaking change hits the TUI only and the web keeps shipping. Under Option A, Ratatui 0.31 hits both simultaneously. Pin exactly (`ratatui = "=0.30.2"`, `ratzilla = "=0.3.1"`) and treat Ratatui upgrades as coordinated events. Two-sided: you also only pay the *cost* of an upgrade once.

**2.7 Wallpaper may partly come back for free.** Since `DomBackend` emits spans in a `<pre>`, a CSS `background-image` on the container plus `Color::Reset` cell backgrounds gives you a wallpaper behind the grid, and `backdrop-filter: blur()` on the `<pre>` is a single CSS line. What you cannot get is per-cell alpha compositing. Filed here as a bonus, not a requirement — do not let it back into the requirements set.

---

## 3. What RFC-001 undersold about Option A

Three things I did not credit properly, and they are the reasons the verdict flips rather than merely narrows.

### 3.1 The Ratatui widget ecosystem serves both targets for free

This is the largest under-weighted factor. Every one of these works, unchanged, in the terminal *and* in the browser:

| Need | Crate | Under Option B you would… |
|---|---|---|
| Config editor (Excalith's `config edit`) | `tui-textarea` | build a JSON editor component or embed CodeMirror |
| Bookmark folder tree | `tui-tree-widget` | build a tree component |
| CPU/memory over time | `Sparkline`, `Chart` (built in) | add a charting library |
| Container resource bars | `Gauge`, `LineGauge` (built in) | build or import |
| Transitions, fades, reveals | `tachyonfx` | write CSS animations |

`tachyonfx` deserves specific mention because it directly answers "but you lose CSS animation": TachyonFX FTL, a DSL editor and previewer for the effects library, is itself listed as a production Ratzilla site. Shader-style transitions and effects run in the browser through the same code path as the terminal.

Under Option B every row above is built twice. Under Option A it is `cargo add`.

### 3.2 Drift risk is eliminated, not mitigated

RFC-001's R2 was "the two renderers drift," mitigated by golden tests asserting that the same `Intent` sequence yields matching visible tokens in a `TestBackend` buffer and in SSR'd HTML. That mitigation is real but it is a discipline you must sustain forever.

With one `render()` function there is nothing to drift. The `TestBackend` golden-buffer suite is not a cross-renderer consistency check — it is just the test suite, and it covers both targets because there is only one target. **This removes a permanent tax, not a one-time cost.**

### 3.3 The build pipeline collapses

| | Option B | **Option A** |
|---|---|---|
| Build tools | `cargo-leptos`, Tailwind CLI, `wasm-bindgen` | `trunk` |
| Stylesheet | Tailwind config + CSS + theme→CSS-vars codegen | none (theme → `ratatui::Style` only) |
| Rendering modes | SSR + hydration + CSR (three states, and hydration-mismatch bugs) | one |
| Theme representations | `ratatui::Style` **and** CSS custom properties **and** Tailwind color config | `ratatui::Style` |
| `xtask dist` steps | tailwind → cargo-leptos → embed → cargo build | trunk → embed → cargo build |

The whole `to_css_vars()` machinery from RFC-001 §2.4 disappears, and with it an entire class of bug where the terminal and web themes are subtly out of sync.

### 3.4 Static deployment mode, free

`trunk build --release` with the daemon-backed providers disabled yields a self-contained WASM bundle that serves config-driven links, the prompt, fuzzy search, and search shortcuts with **no server at all**. Host it on GitHub Pages. This reproduces Excalith's "online version" exactly, and it falls out of the architecture rather than being built. Under Option B you would get it too, but carrying SSR machinery you are not using.

---

## 4. The framework question, restated

### 4.1 Dioxus vs Leptos is now moot

Option A removes Leptos, Dioxus, `server_fn`, Tailwind, `cargo-leptos`, and the hydration model from the dependency graph entirely. RFC-001 §2 no longer applies. You need an HTTP server — **Axum 0.8** — for the provider API, SSE, and static asset serving. That is the whole web-framework decision.

This is worth pausing on: the most contested, most churn-prone, highest-research-cost part of RFC-001 is deleted by the constraint change, not merely re-decided.

### 4.2 Which Ratzilla backend

| | `DomBackend` | `CanvasBackend` | `WebGl2Backend` |
|---|---|---|---|
| Rendering | `<span>` per cell in `<pre>` | 2D canvas | WebGL2 via `beamterm-renderer` |
| Text selection | native | limited | yes (`SelectionMode`) |
| Real anchors (`Hyperlink` widget) | **yes** | no | no |
| Fonts | any webfont, via CSS | canvas font | font atlas (`beamterm-atlas`, static or dynamic) |
| Ctrl+F, right-click, DevTools inspection | **yes** | no | no |
| Throughput under heavy change | weakest (browser layout cost) | good | **sub-millisecond target** |
| Prerender/SSR trick possible (§4.3) | **yes** | no | no |

**Recommendation: `DomBackend` as the default; expose `backend = "webgl2"` as a config option.** The DOM backend's weakness is throughput under heavy change, which is the one thing a start page does not do. Its strengths — anchors, webfonts, selection, inspectability, and being the only backend the prerender trick works with — all map directly onto this product. Reserve WebGL2 for users with 60-container hosts or effects-heavy themes, where its font-atlas build cost is worth paying.

### 4.3 Solving the first-paint problem

Objection S2 is the one thing worth engineering around. Two options, in increasing cost:

**(i) Themed static shell (Phase 1).** `index.html` ships a `<pre>` containing a hand-written static frame — the border, the title bar, the prompt line — styled with the theme's colours inlined by the server. The user sees a correctly-themed, correctly-positioned terminal frame immediately; WASM boots and replaces it. Cost: ~15 lines. Removes the *white flash*, not the wait.

**(ii) Server-side buffer prerender (only if the budget is missed).** This is where `DomBackend`'s span-based DOM pays off. The daemon already has `sp-ui::render` and `AppState` — both are host-agnostic. So the server can:

1. Build an `AppState` from config and cached provider snapshots.
2. Render it into a `ratatui::buffer::Buffer` via `TestBackend` at the client's reported viewport size (from a cookie, or a fixed default).
3. Serialise that `Buffer` to exactly the `<span>` markup `DomBackend` produces.
4. Inline it into `index.html`.

The browser paints a **fully populated dashboard on the first byte**, and Ratzilla's first `draw_web` replaces it with the live version. This is genuine SSR for a TUI, and it is only possible because `sp-ui` was built as a pure `fn render(&mut Frame, &AppState)` with no host coupling.

Two caveats: it needs the client's cell dimensions (cookie set on first visit, default until then), and the markup must track `DomBackend`'s internal structure — so pin `ratzilla` exactly and add a test that diffs your serialiser's output against a real `DomBackend` render.

**Do (i) in Phase 2, measure, and only build (ii) if you miss budget.** Set the budget explicitly now: *first meaningful paint < 200 ms, keystroke accepted < 400 ms, on a warm cache.*

---

## 5. Subsystems: deltas from RFC-001 §3

**Docker (RFC-001 §3.1): unchanged and carries over verbatim.** `bollard 0.21.1`, `negotiate_version()`, the reconcile-and-subscribe actor, `watch` rather than `broadcast`, no stats streaming, CPU % as a delta computation, health absent when no `HEALTHCHECK`, the socket path matrix, and the root-equivalence warning. One delta: in the browser there is no Tokio. The web host consumes SSE via `EventSource` and dispatches into `Rc<RefCell<AppState>>` through `wasm_bindgen_futures::spawn_local`.

**Bookmarks (RFC-001 §3.2): unchanged, including the macOS "Golden Gate" risk** — macOS 27 extended `com.apple.macl` protection to an XProtect-updatable allowlist of non-sandboxed apps' `~/Library/Application Support/<name>` folders, and Apple DTS confirms there is no API to detect Full Disk Access. The fallback chain (direct read → HTML export file → companion WebExtension) still ships in Phase 4. One improvement: with a single renderer, `tui-tree-widget` gives you the bookmark folder tree on both targets at no extra cost.

**Provider trait (RFC-001 §3.3): keep the `Payload` enum, with revised rationale, plus one new option.**

RFC-001 justified presentation-shape payloads by the need for *two* renderers to consume new widgets for free. With one renderer that pressure drops — but keep the enum anyway, because it is still the wire format between daemon and WASM client and it still means a new provider is one new crate.

However, one renderer unlocks a design Option B could not support: **let a provider ship its own widget.**

```rust
// sp-docker      — data only, no ratatui, runs server-side
impl DataProvider for DockerProvider { … }

// sp-docker-ui   — rendering only, depends on ratatui + sp-core, wasm32-clean
impl DashboardWidget for DockerWidget {
    fn render(&self, state: &WidgetState, area: Rect, buf: &mut Buffer);
}
```

Keep the crates split. The data crate must stay `ratatui`-free so it can run in the daemon without dragging a UI library in, and the UI crate must stay I/O-free so it compiles to wasm32. With that split, a fully bespoke widget costs *two small crates and one registration line* — versus RFC-001's Option B where it cost a core type, a Ratatui renderer, and a Leptos component. Retain `Payload`'s standard shapes for the common case (a weather provider should still render for free) and reach for `DashboardWidget` only when a provider genuinely needs custom layout.

---

## 6. Revised workspace

```
start-page/
├── Cargo.toml                    # [workspace], resolver = "3"
├── crates/
│   ├── sp-core/                  # AppState, Intent, reduce, Theme, Payload, traits   [wasm-clean, !Send ok]
│   ├── sp-ui/                    # ★ fn render(&mut Frame, &AppState); key_to_intent   [wasm-clean, ratatui only]
│   ├── sp-config/                # figment/serde, XDG paths, excalith JSON importer
│   ├── sp-api/                   # DTOs + client (reqwest native / EventSource wasm)  [wasm-clean]
│   ├── sp-engine/                # Registry, scheduler, supervision                    [tokio, native]
│   ├── providers/
│   │   ├── sp-docker/  sp-bookmarks/  sp-sysinfo/  sp-httpcheck/  sp-feed/   (data)
│   │   └── */ui/                 # optional DashboardWidget impls                      [wasm-clean]
│   ├── sp-server/                # axum: /api + /events(SSE) + embedded wasm assets
│   ├── sp-tui-host/              # crossterm driver               (~300 LOC)
│   ├── sp-web-host/              # ratzilla DomBackend driver     (~250 LOC)  [cdylib, wasm32]
│   └── sp-cli/                   # single binary: serve | tui | sync | config | import
└── xtask/                        # dist: trunk build → embed → cargo build
```

Compare to RFC-001: `sp-web` (a full Leptos application) is replaced by `sp-web-host` (a driver), and the new `sp-ui` crate absorbs all rendering for both targets.

**The two CI gates that hold it together:**

```bash
cargo check -p sp-core -p sp-ui -p sp-api --target wasm32-unknown-unknown   # shared code stays portable
cargo tree -p sp-ui | grep -qE 'crossterm|ratzilla|tokio' && exit 1         # sp-ui stays host-agnostic
```

The second is the one people forget. The moment `sp-ui` imports `crossterm::event::KeyCode` instead of defining its own, or reaches for a `tokio::time::Instant`, the architecture quietly reverts to two renderers. Ratzilla defines its own `ratzilla::event::KeyCode` precisely because crossterm's is not available in the browser — so `sp-ui` must define `sp_ui::Key` and each host maps into it.

**Release profile** as RFC-001 §4, with a wasm profile at `opt-level = "z"`. Keep `panic = "unwind"` for per-provider panic isolation. Expect a smaller total binary than Option B: no Leptos SSR machinery, no embedded CSS, and a WASM bundle that is `ratatui` + `ratzilla` + your app.

> **Measure this in Phase 0, do not estimate it.** I have no verified figure for a Ratzilla dashboard bundle and will not invent one. `trunk build --release` a hello-world plus a realistic `sp-ui` skeleton in week one, record gzip and Brotli sizes, and set the CI budget from the measurement.

---

## 7. Revised roadmap

The saving over RFC-001 is roughly **4 weeks**, concentrated in what was Phase 3.

| Phase | Scope | RFC-001 | **RFC-002** |
|---|---|:---:|:---:|
| 0. Skeleton | Workspace, config, theme, CI matrix + both wasm gates, **bundle-size baseline**, `cargo-dist` | 1 wk | 1 wk |
| 1. Core + UI | `AppState`/`Intent`/`reduce`, `sp-ui::render`, prompt, autosuggest, `nucleo` fuzzy filter, static links, `sp-tui-host` | 2–3 wk | 2–3 wk |
| 2. **Web host** | `sp-web-host` (~250 LOC), `trunk` build, themed static shell (§4.3 i), asset embedding, **static-only deploy mode** | — | **3–4 days** |
| 3. Docker | `sp-engine`, `DockerActor`, `watch` fan-out, SSE endpoint, container pane (renders on both targets at once) | 2 wk | 2 wk |
| 4. Web dashboard | *(was: entire Leptos app, Tailwind, SSR/hydration, theme→CSS)* | 3–4 wk | **absorbed into Phase 2** |
| 5. Bookmarks | Chromium reader, profile enumeration, `notify` watcher, HTML-export fallback, Firefox snapshot-copy, `tui-tree-widget` | 2 wk | 2 wk |
| 6. Extensibility | Extract `DataProvider` + `DashboardWidget`; port sysinfo/httpcheck/feed as proof; docs | 2 wk | 2 wk |
| 7. Optional | Buffer prerender (§4.3 ii) if budget missed; WebGL2 backend option; companion WebExtension; remote daemons via bollard `ssh` | — | — |

**Phase 2 moves ahead of Docker.** This is deliberate and it is the point of the whole architecture: once `sp-ui::render` exists, the browser target is a few days of work, so you should prove it immediately — before you have built anything on top of it. If Ratzilla turns out not to work for you, you find out in week four with one renderer written, having spent four days, rather than in week ten.

---

## 8. Revised risks

| # | Risk | Likelihood | Impact | Mitigation |
|---|---|:---:|:---:|---|
| **R1** | **First paint misses budget** (no SSR) | Medium | High | Themed static shell in Phase 2; explicit budget (FMP <200 ms, keystroke <400 ms); buffer prerender (§4.3 ii) held in reserve; Brotli pre-compression |
| **R2** | **`sp-ui` acquires a host dependency**, silently reverting to two renderers | Medium | High | `cargo tree` CI gate; `sp_ui::Key` defined locally, mapped by each host |
| **R3** | **`AppState` acquires `Send + Sync` bounds**, breaking the wasm build | Medium | Medium | Stated invariant; wasm32 CI gate from Phase 0; `reduce` stays synchronous |
| **R4** | **Ratzilla stalls or lags a Ratatui release** | Medium | Medium | Pin `=0.3.1` / `=0.30.2`; treat Ratatui upgrades as coordinated events; `Backend` is a small trait — forking is a tractable escape hatch |
| **R5** | Ratatui breaking change hits both renderers at once | Medium | Medium | Exact pins; single coordinated upgrade (also a saving — you pay once) |
| **R6** | macOS Golden Gate blocks browser profile reads | High & rising | High | Unchanged from RFC-001 R1: fallback chain in Phase 5, `Unavailable` as a designed state, extension route as the durable path |
| **R7** | Docker socket root-equivalence | Certain | High | Unchanged from RFC-001 R4: socket-proxy default for containers, `:ro` is not a mitigation, refuse root without a flag |
| **R8** | Docker event stream dies silently | High | Medium | Unchanged from RFC-001 R5: 30 s ping watchdog, backoff, full reconcile |
| **R9** | WASM bundle grows past budget | Medium | Medium | Baseline measured in Phase 0; CI size gate; `twiggy` in the runbook |
| **R10** | Windows named pipes + long-lived streams undertested | Medium | Medium | `windows-latest` in CI from Phase 0 |
| **R11** | Accidental write to Chromium `Bookmarks` corrupts the checksum | Low | High | Unchanged from RFC-001 R9: read-only APIs only, enforced by test |
| **R12** | Provider panic kills the daemon | Medium | High | Unchanged: `panic = "unwind"`, supervised `JoinHandle`s, restart with backoff |
| ~~R13~~ | ~~Renderers drift~~ | — | — | **Eliminated by construction** (§3.2) |
| ~~R14~~ | ~~Pre-1.0 web framework churn~~ | — | — | **Eliminated** — no web UI framework in the graph (§4.1) |

Two risks from RFC-001 are gone outright. That is the clearest single summary of what the constraint change bought.

---

## 9. What would flip this back to Option B

Write these down now, because the failure mode is discovering one of them in Phase 5 and rationalising around it.

1. **Mobile or tablet use returns to scope.** An 80×24 cell grid is not a responsive layout, and no amount of engineering makes it one. This is the hardest reversal — it is a rewrite of the web target.
2. **Accessibility becomes a requirement.** Shared/institutional use, or any legal obligation. `DomBackend` emits spans, not semantic structure; a screen reader gets a character soup.
3. **Rich media becomes central** — favicons per link, screenshots, charts as images. Cell grids do glyphs.
4. **The first-paint budget cannot be met even with the buffer prerender.** Measure before concluding this; §4.3 (ii) is genuinely strong.
5. **Ratzilla goes unmaintained *and* Ratatui changes the `Backend` trait.** Either alone is survivable; together they mean owning a WASM terminal renderer.
6. **You want to share the dashboard with people who do not like terminals.** This is a product decision, not a technical one, and it is the most likely of the six to actually happen.

If (1) or (2) arrives, the migration path is not catastrophic: `sp-core`, `sp-config`, `sp-api`, `sp-engine`, and all the providers survive untouched, because none of them ever knew what a renderer was. You would rebuild `sp-ui` + `sp-web-host` as a Leptos application and land approximately where RFC-001 recommended — about six weeks of work, not a restart. **Keeping that path cheap is the actual reason for the strict crate boundaries in §6**, more than any purity argument.

---

## Appendix A: Version pins (verified 2026-09-08)

| Crate | Pin | Note |
|---|---|---|
| `ratatui` | `=0.30.2` | Modular workspace split; `ratatui::run()`; `Block::title` takes `Into<Line>`; `widgets::block::Title` removed |
| `ratzilla` | `=0.3.1` | `DomBackend` / `CanvasBackend` / `WebGl2Backend`; tracks `ratatui ^0.30.1`; own `event::KeyCode`; `widgets::Hyperlink` |
| `tachyonfx` | current | Effects/transitions; must move in lockstep with Ratatui upgrades |
| `tui-textarea` | current | Config editor |
| `tui-tree-widget` | current | Bookmark tree |
| `bollard` | `0.21.1` | Docker + Podman, rootless auto-discovery, Windows named pipes, moby API 1.52 |
| `sysinfo` | `0.39.x` | MSRV 1.88; disable default features |
| `axum` | `0.8.x` | API + SSE + static assets — the *only* web framework in the graph |
| `notify` | `8.x` | Watch the directory, not the file |
| `rusqlite` | `0.3x`, `bundled` | Firefox `places.sqlite`, copied snapshot only |
| `nucleo` | `0.5.x` | Fuzzy matcher; wasm-clean |
| `trunk` | latest | Replaces `cargo-leptos` + Tailwind CLI entirely |
| ~~`leptos`~~ / ~~`dioxus`~~ / ~~`tailwindcss`~~ | — | Removed from the dependency graph |

## Appendix B: Claims corrected from RFC-001

| RFC-001 claim | Correction |
|---|---|
| "DOM backend throughput is marginal (~10–30 fps)" | Figure is from the animations/demo stress test, not a representative start-page workload |
| "Canvas/WebGL2 trade away text selection" | `WebGl2Backend` exports `SelectionMode`; selection is supported |
| "Option A: no real anchor semantics" | Ratzilla ships a `Hyperlink` widget; `DomBackend` emits real anchors |
| "Option B has lower ecosystem risk" | Lower *likelihood*, far worse *recoverability* — Ratzilla's `Backend` impl is forkable, Leptos is not |
| Wallpaper/blur and mobile treated as requirements | Inherited from upstream Excalith, never in the brief |

## Appendix C: Sources

- Ratzilla README, API, backends, production sites — https://github.com/ratatui/ratzilla
- Ratzilla `lib.rs` (`SelectionMode`, `Hyperlink`, backend exports) — https://docs.rs/crate/ratzilla/latest/source/src/lib.rs
- `FontAtlasConfig` Static/Dynamic and `beamterm-atlas` — Ratzilla API docs
- DOM-backend frame-rate discussion (Ratatui 0.30 upgrade) — https://github.com/ratatui/ratzilla/pull/141
- Ratatui 0.30 highlights and breaking changes — https://ratatui.rs/highlights/v030
- bollard — https://docs.rs/bollard
- macOS 27 "Golden Gate" Application Support protection — https://mjtsai.com/blog/2026/07/24/golden-gate-application-support-protection/
- Apple DTS: no Full Disk Access detection API — https://developer.apple.com/forums/thread/841091
