# syntax=docker/dockerfile:1

# Builds the "start-page" binary, including the wasm32 browser bundle that
# sp-server embeds. Both modes (tui and serve) work from the resulting image.
#
# Requires BuildKit for the cache mounts below (default in modern Docker; set
# DOCKER_BUILDKIT=1 on older daemons).

# Must match rust-toolchain.toml, which is copied in and would otherwise make
# rustup download a second compiler mid-build.
FROM rust:1.96-bookworm AS builder
WORKDIR /app

# trunk builds the wasm bundle. Installed as a release binary rather than
# `cargo install` (which would compile it from source on every cold build).
ARG TARGETARCH
ARG TRUNK_VERSION=0.21.14
RUN set -eux; \
    case "${TARGETARCH:-amd64}" in \
      amd64) arch=x86_64-unknown-linux-gnu ;; \
      arm64) arch=aarch64-unknown-linux-gnu ;; \
      *) echo "unsupported TARGETARCH: ${TARGETARCH}" >&2; exit 1 ;; \
    esac; \
    wget -qO- "https://github.com/trunk-rs/trunk/releases/download/v${TRUNK_VERSION}/trunk-${arch}.tar.gz" \
      | tar -xzf - -C /usr/local/bin trunk

COPY . .

# `xtask dist` is the same one command a developer runs locally: trunk build,
# brotli-compress each asset, then build sp-cli. Order matters — sp-server
# embeds crates/sp-web-host/dist at compile time via rust-embed, and building
# sp-cli first still succeeds (build.rs creates an empty dist/) while producing
# a daemon that serves nothing. The brotli step matters too: ServeDir's
# .precompressed_br() looks for .br siblings, so running plain `trunk build`
# here would silently drop compression.
#
# Cache mounts replace the hand-rolled manifest-stub layer this file used to
# carry: cargo's own incremental state persists across builds instead of being
# approximated by dummy source files. Note /app/target is a cache mount and so
# is NOT part of the image layer — the binary must be copied out inside the
# same RUN, or it vanishes with the mount.
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
    cargo xtask dist && \
    cp target/release/start-page /usr/local/bin/start-page

# --- Runtime -----------------------------------------------------------------
# distroless/cc: the binary dynamically links glibc + libgcc (no TLS/openssl
# dependency yet), and distroless carries no shell or package manager — smallest
# attack surface that still satisfies the glibc link. :nonroot is uid 65532.
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

COPY --from=builder /usr/local/bin/start-page /usr/local/bin/start-page

# start-page serve's default port. It binds 127.0.0.1 by default, which inside a
# container is unreachable from a published port — run `serve --expose` for the
# mapping to work.
EXPOSE 7878

USER nonroot
ENTRYPOINT ["/usr/local/bin/start-page"]
