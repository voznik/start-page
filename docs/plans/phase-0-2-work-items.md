# Work items — Phases 0 to 2

Read `CLAUDE.md` first. Rationale for any decision here is in `docs/rfc/RFC-002-*.md`; do not
re-derive it.

Work items are ordered. **T0.2 is a decision gate — stop there and report before continuing.**

Each item is done when every acceptance criterion passes and the verification block in `CLAUDE.md`
is green. If a criterion cannot be met as written, stop and report rather than substituting an
adjacent approach.

---

## Phase 0 — Skeleton and de-risking (target: 1 week)

The goal of this phase is not to build features. It is to prove the browser target works and to
measure the numbers that later budgets depend on.

### T0.1 — Workspace skeleton

Create the Cargo workspace with the crate layout from `CLAUDE.md`. Every crate is a stub with a
`lib.rs` containing only the public types it owns (empty structs are fine). `sp-cli` parses
`serve | tui | sync | config | import` with clap and exits with "not implemented" for each.

**Acceptance**
- `cargo check --workspace` passes on Linux, macOS, and Windows.
- `cargo check -p sp-core -p sp-ui -p sp-api --target wasm32-unknown-unknown` passes.
- No crate violates the dependency table in `CLAUDE.md`.
- `[workspace.dependencies]` holds every shared version; no version strings in member manifests.

### T0.2 — Ratzilla spike **[DECISION GATE — stop and report]**

Build the smallest possible end-to-end proof of the browser target and measure it.

1. In `sp-ui`, write `render(frame, &AppState)` where `AppState` is a placeholder struct with a
   title and a `Vec<String>` of links. Draw a bordered block, a title line, and the list.
2. In `sp-tui-host`, drive it with crossterm.
3. In `sp-web-host`, drive the *same* function with `ratzilla::DomBackend` via `trunk`.
4. Self-host a Nerd Font webfont (do not use the CDN from the Ratzilla template) and confirm
   powerline and icon glyphs render in the browser.

**Report these numbers and stop:**

| Measurement | How |
|---|---|
| WASM bundle, raw / gzip / brotli | `trunk build --release`, then `ls -l` and `gzip -9c \| wc -c` |
| Time to first paint, warm cache | Browser devtools, throttled to Fast 3G and to no throttling |
| Time to first accepted keystroke | Manual, devtools performance trace |
| Nerd Font glyph coverage | Screenshot of a test string with box-drawing, powerline, and icon glyphs |

**Acceptance**
- The same `sp-ui::render` produces visually equivalent output in terminal and browser.
- `cargo tree -p sp-ui` shows neither `crossterm` nor `ratzilla`.
- Numbers reported.

**Do not proceed to T0.3 without a decision on the measurements.** The budget to set is: first
meaningful paint < 200 ms, keystroke accepted < 400 ms, warm cache. If T0.2 misses it badly, the
mitigation is the themed static shell (T2.2) and, failing that, server-side buffer prerender —
raise it rather than starting Phase 1.

### T0.3 — CI

GitHub Actions matrix: `ubuntu-latest`, `macos-latest`, `windows-latest`.

**Acceptance**
- All four commands from `CLAUDE.md` § Verification run on every push.
- A `sp-ui` boundary violation fails the build (add a deliberate violation, confirm red, revert).
- A WASM size gate fails the build if the gzipped bundle exceeds the T0.2 baseline + 25%.

### T0.4 — Config and theme

`sp-config`: schema, defaults, XDG resolution (`~/.config/start-page/config.toml`, honouring
`XDG_CONFIG_HOME`). `sp-core::Theme` with named colours, plus `From<&Theme> for TuiPalette`.

**Acceptance**
- Round-trips through serde with no data loss.
- Missing config file yields working defaults, not an error.
- `start-page config path` prints the resolved path on all three platforms.

### T0.5 — Release pipeline

`cargo-dist` producing tagged binaries for x86_64/aarch64 Linux, macOS, and Windows.

**Acceptance**
- A tagged commit publishes installable artifacts.
- Release profile matches `CLAUDE.md` (`panic = "unwind"`, `lto = "fat"`, `codegen-units = 1`,
  `strip = "symbols"`, `opt-level = 3`; wasm profile at `opt-level = "z"`).

---

## Phase 1 — Core and TUI (target: 2–3 weeks)

Exit criterion for the whole phase: **a start page you would actually use daily**, shipped as v0.1.

### T1.1 — State machine

`sp-core`: `AppState`, `Intent`, `fn reduce(&mut AppState, Intent) -> Vec<Effect>`.

`Intent` covers at minimum: `Char(char)`, `Backspace`, `Next`, `Prev`, `Activate`, `ForceSearch`,
`Command(Cmd)`, `Refresh(ProviderId)`, `ProviderUpdated(ProviderId, Payload)`, `Quit`.

**Acceptance**
- `reduce` is synchronous and performs no I/O.
- `AppState` compiles without `Send`/`Sync` bounds anywhere.
- Table-driven tests: a sequence of `Intent`s produces an expected `AppState`.

### T1.2 — Input mapping

`sp_ui::Key` and `key_to_intent(Key, Modifiers) -> Option<Intent>`. Each host maps its own key type
into `sp_ui::Key` — crossterm in `sp-tui-host`, `ratzilla::event::KeyCode` in `sp-web-host`.

**Acceptance**
- `sp-ui` has no host key types in its dependency tree.
- Both hosts produce identical `Intent`s for the same physical keypress.

### T1.3 — Render

`sp-ui::render`: layout, link grid grouped by category, prompt line, status line. Theme-driven
styling throughout — no hardcoded colours.

**Acceptance**
- Renders correctly at 80×24 and at 200×60.
- Long link names ellipsize rather than wrapping or panicking.
- Zero-width and CJK characters do not corrupt the grid.

### T1.4 — Filter and autosuggest

`nucleo` fuzzy matching over links. Typing filters live; the top match is shown inline as a ghost
suggestion (zsh/fish style). `Tab` accepts the suggestion.

**Acceptance**
- Filtering 1,000 links stays under 1 ms per keystroke (bench it).
- `nucleo` appears in the wasm32 check without error.

### T1.5 — Command parser

- Bare text with no match → default search engine.
- `Ctrl+Enter` → force search without opening filtered links.
- `Enter` with matches → open all filtered links.
- Configured shortcuts: `s some bug` → StackOverflow search.
- Direct URLs: `github.com` → navigate.
- `help`, `config path`, `config edit`, `theme <name>`.

**Acceptance**
- Query strings are URL-encoded (the upstream project shipped a bug here).
- Unknown commands fall through to search rather than erroring.

### T1.6 — TUI host

`sp-tui-host`: `ratatui::init()`/`restore()`, event loop, `Effect` handling (opening URLs via
`open` crate).

**Acceptance**
- Terminal state is restored on panic and on `SIGINT`.
- Resize is handled without artifacts.
- Works in Windows Terminal, iTerm2, and a bare Linux tty.

### T1.7 — Golden tests

`TestBackend` snapshot tests: `Intent` sequence → `Buffer` → committed snapshot. Use `insta`.

**Acceptance**
- At least 10 snapshots covering empty state, filtered state, command mode, and error state.
- Snapshots are reviewed diffs, not blind accepts.

### T1.8 — Ship v0.1

Tag, release notes, README with a screenshot and install instructions.

---

## Phase 2 — Browser target (target: 3–4 days)

This is deliberately ahead of Docker. It is the architectural proof: if the browser target is more
than a few days of work, the shared-render assumption is wrong and we need to know now.

### T2.1 — Web host

`sp-web-host`: `DomBackend`, `terminal.draw_web`, `on_key_event`, `Rc<RefCell<AppState>>`, and the
same `sp-ui::render` from T1.3. Links open via `web_sys::window().open_with_url`.

**Acceptance**
- Byte-equivalent layout to the TUI at the same cell dimensions.
- Browser resize re-lays-out without artifacts.
- No code duplicated from `sp-tui-host` beyond the key mapping.

### T2.2 — Themed static shell

`index.html` ships a static `<pre>` with the border, title bar, and prompt line, styled with the
theme's colours, so the user sees a correctly themed frame before WASM boots.

**Acceptance**
- No white flash on a cold load.
- Shell dimensions match the booted app's first frame (no visible reflow).

### T2.3 — Static deployment mode

A build with daemon-backed providers disabled, serving config-driven links, prompt, and search with
no server. Deployable to any static host.

**Acceptance**
- `trunk build --release --no-default-features` produces a self-contained `dist/`.
- Opening `dist/index.html` from `file://` works, or the limitation is documented.

### T2.4 — Asset embedding

`sp-server` embeds `dist/` via `rust-embed`, driven by `xtask dist`. Brotli pre-compression served
through `tower-http`.

**Acceptance**
- `start-page serve` is a single binary with no external asset files.
- `xtask dist` is one command from clean checkout to shippable binary.

---

## Stop here

Do not plan Docker, bookmarks, or the provider trait yet. Phase 3 onward depends on what T0.2 and
Phase 2 actually measure, and on whether the `Payload` shape in RFC-002 §5 survives contact with a
real integration. Report at the end of T2.4 and the next phase will be scoped then.
