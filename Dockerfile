# syntax=docker/dockerfile:1

# Builds the sp-cli binary ("start-page") for the native (server/tui) target only.
# The wasm32 browser bundle (crates/sp-web-host) is trunk's job, not this image's —
# T2.4 will rust-embed dist/ into the binary; this Dockerfile does not touch wasm.

FROM rust:1.85-bookworm AS builder
WORKDIR /app

# --- Dependency layer -------------------------------------------------------
# Copy manifests only (no .rs files) so this layer's cache key is unaffected by
# source edits. Workspace this small doesn't earn cargo-chef: 9 crates, a
# handful of leaf deps (clap, anyhow, serde, toml, thiserror, directories) —
# the extra chef-plan/chef-cook stages and chef binary compile would cost more
# than they save. Hand-rolled stub build gets the same caching for free.
COPY Cargo.toml ./
COPY crates/sp-core/Cargo.toml crates/sp-core/Cargo.toml
COPY crates/sp-ui/Cargo.toml crates/sp-ui/Cargo.toml
COPY crates/sp-config/Cargo.toml crates/sp-config/Cargo.toml
COPY crates/sp-api/Cargo.toml crates/sp-api/Cargo.toml
COPY crates/sp-engine/Cargo.toml crates/sp-engine/Cargo.toml
COPY crates/sp-server/Cargo.toml crates/sp-server/Cargo.toml
COPY crates/sp-tui-host/Cargo.toml crates/sp-tui-host/Cargo.toml
COPY crates/sp-web-host/Cargo.toml crates/sp-web-host/Cargo.toml
COPY crates/sp-cli/Cargo.toml crates/sp-cli/Cargo.toml

# Stub every crate's source so cargo can resolve the workspace and prebuild
# dependencies without the real code present.
RUN set -eux; \
    for d in crates/*/; do \
        mkdir -p "${d}src"; \
        printf 'fn main() {}\n' > "${d}src/main.rs"; \
        printf '\n' > "${d}src/lib.rs"; \
    done; \
    cargo build --release -p sp-cli

# --- Source layer ------------------------------------------------------------
# Real source overwrites the stubs; cargo only recompiles crates whose content
# actually changed, so dependency compilation above stays cached.
COPY crates crates
# BuildKit's COPY preserves source mtimes from the build context, which can
# predate the stub files' build-time mtimes above — without this, cargo's
# mtime-based fingerprint thinks nothing changed and ships the stub binary.
RUN find crates -name '*.rs' -exec touch {} +
RUN cargo build --release -p sp-cli

# --- Runtime -----------------------------------------------------------------
# distroless/cc: our binary dynamically links glibc + libgcc (no TLS/openssl
# dependency yet), and distroless carries no shell/package manager — smallest
# attack surface that still satisfies the glibc link. :nonroot runs as uid 65532.
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

COPY --from=builder /app/target/release/start-page /usr/local/bin/start-page

# sp-server's bind port isn't defined anywhere in the codebase yet (sp-server
# is an empty stub). 8080 is provisional, chosen here only to document the
# image's intent — no config plumbing exists to honor it.
EXPOSE 8080

USER nonroot
ENTRYPOINT ["/usr/local/bin/start-page"]
