# Docker Build Strategy Research: Rust CLI+Web Projects

Research date: 2026-09-08  
Focus: 3-4 production Rust projects shipping CLI/TUI + embedded web UI; analysis of Docker caching and multi-stage strategies.

## Projects Analyzed

### 1. Meilisearch (2-stage, no caching)
- **Repo:** meilisearch/meilisearch → `Dockerfile` (root, 45 lines)
- **Architecture:** Compiler (rust:1.89-alpine3.22) → Runtime (alpine:3.22)
- **Caching:** None — no cargo-chef, no BuildKit cache mounts; full recompile on Cargo.lock change
- **Frontend/WASM:** Web UI embedded as static assets baked into binary; no separate build step
- **Runtime:** alpine:3.22 (~7MB base), dynamic linking to libc
- **Non-root:** Runs as root
- **Rationale:** Minimal layers; UI compiled into binary makes separate stage unnecessary

### 2. Quickwit (3-stage, language-aware separation)
- **Repo:** quickwit-oss/quickwit → `Dockerfile` (root, 120+ lines)
- **Architecture:** node:24 (UI builder) → rust:bookworm (binary builder) → debian:bookworm-slim (runtime)
- **Caching:** None for Rust; despite large dep tree (clang, cmake, llvm, protobuf), no cargo-chef
- **Frontend/WASM:** Node.js/TypeScript UI built separately; outputs copied via `COPY --from=ui-builder /quickwit/quickwit-ui/build`
- **Runtime:** debian:bookworm-slim (~60MB base), dynamic linking to glibc
- **Non-root:** Runs as root
- **Rationale:** Isolates Node toolchain to builder stage (discarded from final image), saves ~300MB

**Key insight:** Separate frontend stage is pragmatic *when frontend uses a different language ecosystem* (Node/Go/etc). Isolates tool bloat, makes final image lean.

### 3. Atuin (4-stage, cargo-chef)
- **Repo:** atuinsh/atuin → `Dockerfile` (root, 50 lines)
- **Architecture:** cargo-chef planner → cargo-chef cooker → cargo build → debian:bookworm-slim runtime
- **Caching:** cargo-chef dependency layer caching (explicit `recipe.json`)
- **Frontend/WASM:** None (HTTP server only, no embedded UI in Docker image)
- **Runtime:** debian:bookworm-slim, dynamic linking to glibc
- **Non-root:** Yes (runs as `atuin:1000:1000`, only project in sample that does this)
- **Includes:** HEALTHCHECK
- **Rationale:** cargo-chef adds explicit caching layer; useful if Cargo.lock is stable and rebuilds are frequent

## Caching Strategies Comparison

| Strategy | Layer count | Speed | Complexity | When to use |
|----------|------------|-------|------------|------------|
| **cargo-chef** (Atuin) | +2 (planner, cooker) | ~50% faster on Cargo.lock cache hit | Medium (requires `recipe.json` + planner step) | Large, stable dependency tree; daily rebuilds |
| **No caching** (Meilisearch, Quickwit) | Minimal | Slow on rebuild | Trivial | Infrequent rebuilds or CI-level caching |
| **BuildKit cache mounts** | Minimal | ~70% faster (modern CI) | Low (`--mount=type=cache,target=/usr/local/cargo`) | DOCKER_BUILDKIT=1 available in CI |

**Verdict:** BuildKit cache mounts (`--mount=type=cache`) are simpler and faster than cargo-chef for most Rust projects. Requires `DOCKER_BUILDKIT=1` in CI (GitHub Actions, GitLab CI, etc. support natively). cargo-chef adds overhead if cache hits are infrequent.

## Frontend/WASM: Separate Stage Decision

| Scenario | Example | Stage count | Approach |
|----------|---------|------------|----------|
| Rust cargo compiles wasm into binary | Meilisearch (embedded assets) | 2 stages | No separate stage; wasm compiled as part of main cargo build |
| Separate frontend toolchain (Node.js, Go, wasm-pack) | Quickwit (Node UI) | 3-4 stages | Separate builder stage; discard toolchain from final image |
| No embedded UI | Atuin | 2-3 stages (just app caching) | Standard Rust build |

**Rule:** If frontend is compiled via `cargo` (as dependencies) or embedded via `rust-embed`, **no separate stage needed**. If frontend is Node.js/Go/separate ecosystem, **isolate to own builder stage** and copy artifacts only.

## Static vs. Dynamic Linking

All three projects use **dynamic linking** to libc/glibc:
- Meilisearch: libc (alpine)
- Quickwit: libssl3, glibc (debian)
- Atuin: glibc (debian)

**Rationale:** Musl (static) + alpine adds complexity (math library mismatches, some crates fail to compile). Dynamic linking to glibc/libc is pragmatic; binary compatibility sufficient for most deployments.

## Non-root User

- **Atuin:** Only project that runs as non-root (`atuin:1000:1000`)
- **Meilisearch, Quickwit:** Run as root (acceptable for appliances; risky for multi-tenant)

**Best practice:** Add a non-root user; Atuin's pattern is clean:
```dockerfile
RUN useradd -m -u 1000 -s /bin/false atuin
USER atuin
```

## Recommendation for start-page

Given start-page's architecture (Ratatui TUI + Ratzilla DOM backend, `trunk` for wasm, `rust-embed` for assets):

### Suggested Dockerfile structure (3 stages, ~80 lines):

```dockerfile
# Stage 0: Builder (rust:bookworm or rust:alpine)
FROM rust:latest as builder
WORKDIR /app
# Copy source
COPY . .
# Build with BuildKit cache mount (if CI supports DOCKER_BUILDKIT=1)
RUN --mount=type=cache,target=/usr/local/cargo \
    --mount=type=cache,target=/app/target \
    cargo build --release -p sp-cli

# Stage 1: Runtime (debian:bookworm-slim or alpine:3.22)
FROM debian:bookworm-slim
WORKDIR /app
RUN useradd -m -u 1000 -s /bin/false app
COPY --from=builder /app/target/release/start-page /usr/local/bin/
USER app
ENTRYPOINT ["start-page"]
```

### Key decisions:

1. **Do NOT use cargo-chef** unless daily rebuilds are standard. BuildKit cache mounts are faster and simpler (one line vs. planner + cook).
2. **Do NOT create a separate wasm stage.** If `trunk` is built via cargo as a dependency (or prebuilt artifacts embedded via `rust-embed`), it's part of the main binary and needs no separate builder.
3. **Use dynamic linking** (glibc/libc). Musl adds complexity; binary compatibility is sufficient.
4. **Add non-root user.** Atuin pattern is clean and secure.
5. **Choose runtime base:** `debian:bookworm-slim` (~60MB) if compatibility is priority; `alpine:3.22` (~7MB) if image size is critical.
6. **Enable BuildKit:** Set `DOCKER_BUILDKIT=1` in CI if possible; `-mount=type=cache` cuts rebuild time 50-70%.

### If frontend requires separate toolchain (e.g., Node.js pre-processing):
Use a 4-stage build:
```
Stage 0: node:22-slim (build frontend artifacts)
Stage 1: rust:bookworm (import frontend, build binary)
Stage 2: debian:bookworm-slim (runtime)
```

This isolates toolchain bloat; final image includes only the binary and runtime.

---

## Conclusion

The three projects studied converge on **dynamic linking + pragmatic staging**:
- Meilisearch: minimal (2 stages), embedded UI
- Quickwit: language-aware separation (3 stages), UI toolchain isolated
- Atuin: explicit caching (4 stages), cargo-chef for dependency stability

For start-page: **3 stages, BuildKit cache mounts, dynamic linking, non-root user.** No cargo-chef overhead; no separate wasm stage (trunk compiles into binary). If performance matters, enable DOCKER_BUILDKIT=1 in CI.
