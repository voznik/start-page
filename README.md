# start-page

A personal dashboard and start page that runs in your terminal and in your browser — from one
binary, through one render function.

```
> docs.rs
┌start-page────────────────────────────────────────────────────────────────┐
│docs.rs                                                                   │
│                                                                          │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
1/3 links
```

Type to filter. The top match completes inline as you go, zsh style; `Tab` accepts it, `Enter`
opens it. Anything that isn't a link becomes a search.

## Why

Start pages are usually a web app you maintain like a web app. This one is a Rust binary. The
terminal UI and the browser UI are the *same* Ratatui render function — the browser target runs it
through WebAssembly against a DOM backend. There is no second UI to keep in sync, and no
JavaScript framework underneath it.

## Status

**v0.1 — the terminal UI.** `start-page tui` is usable daily. The browser target builds and
renders (that was proven before anything else was written), but it does not open links yet, so it
is currently look-don't-touch. `serve`, `sync` and `import` are not implemented.

## Install

From source, which is the only channel until the first tag is pushed:

```bash
cargo install --path crates/sp-cli
start-page tui
```

Tagged releases build binaries for x86_64 and aarch64 across Linux, macOS and Windows.

In Docker (the binary only — `serve` isn't implemented, so this is mostly useful for CI):

```bash
docker build -t start-page .
docker run --rm start-page config path
```

## Keys

| Key | Does |
|---|---|
| any character | filter links |
| `Tab` | accept the inline suggestion |
| `Shift+Tab` | previous link |
| `Up` / `Down` | previous / next link |
| `Enter` | open the selection, or all filtered links |
| `Ctrl+Enter` | search, without opening links |
| `Backspace` | delete a character |
| `Esc` / `Ctrl+C` | quit |

`q` is not a quit key — you need to be able to type it.

## Commands

Typed at the prompt: `help`, `config path`, `config edit`, `theme <name>`. Configured shortcuts
expand a prefix, so `s some bug` searches StackOverflow. Bare text that looks like a host
(`github.com`) navigates; anything else searches. Unknown commands search rather than erroring,
and queries are percent-encoded properly.

## Config

`~/.config/start-page/config.toml`, honouring `XDG_CONFIG_HOME`; native locations on macOS and
Windows. `start-page config path` prints the resolved path. A missing file is not an error — you
get working defaults.

## Building the browser target

```bash
cd crates/sp-web-host
trunk build --release
```

Output lands in `dist/`. The Nerd Font is self-hosted from `assets/`, not a CDN.

## Development

```bash
cargo check -p sp-core -p sp-ui -p sp-api --target wasm32-unknown-unknown
./scripts/check-ui-boundary.sh
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

All four run in CI on Linux, macOS and Windows. `CLAUDE.md` documents the invariants those checks
protect and the upstream defects worked around; read it before changing crate boundaries.

## License

MIT.
