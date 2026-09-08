//! Axum router, embedded assets, bind policy.
//!
//! `dist/` (the `sp-web-host` browser bundle, built by `xtask dist`) is embedded into
//! this binary via `rust-embed`. At startup the embedded files are extracted to a
//! tempdir so `tower-http`'s `ServeDir` — a real filesystem service — can serve them
//! with `.precompressed_br()` content negotiation (serves `foo.wasm.br` in place of
//! `foo.wasm` when the client's `Accept-Encoding` allows it). The tempdir is deleted
//! on process exit; nothing is written next to the binary.

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;

use axum::Router;
use rust_embed::RustEmbed;
use tower_http::services::ServeDir;

#[derive(RustEmbed)]
#[folder = "../sp-web-host/dist"]
struct Assets;

/// Bind policy: localhost only unless the caller explicitly widens it. A start-page
/// daemon binding `0.0.0.0` by default would expose a user's dashboard on their LAN.
#[derive(Debug, Clone, Copy)]
pub struct BindPolicy {
    pub host: IpAddr,
    pub port: u16,
}

impl Default for BindPolicy {
    fn default() -> Self {
        Self {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 7878,
        }
    }
}

impl BindPolicy {
    /// Explicit opt-in to widen the bind address beyond localhost.
    pub fn expose(mut self, host: IpAddr) -> Self {
        self.host = host;
        self
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    fn addr(&self) -> SocketAddr {
        SocketAddr::new(self.host, self.port)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("failed to build tokio runtime: {0}")]
    Runtime(#[source] io::Error),
    #[error("failed to extract embedded assets: {0}")]
    Extract(#[source] io::Error),
    #[error("failed to bind {0}: {1}")]
    Bind(SocketAddr, #[source] io::Error),
    #[error("server error: {0}")]
    Serve(#[source] io::Error),
}

pub struct Server;

impl Server {
    /// Runs `start-page serve` to completion (until the process is killed). Builds
    /// its own Tokio runtime so callers (`sp-cli`) stay synchronous — Tokio is
    /// allowed in `sp-server` but must not leak into `sp-core`/`sp-ui`/`sp-api`.
    pub fn run(policy: BindPolicy) -> Result<(), ServerError> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_io()
            .build()
            .map_err(ServerError::Runtime)?;
        rt.block_on(serve(policy))
    }
}

async fn serve(policy: BindPolicy) -> Result<(), ServerError> {
    let assets_dir = tempfile::tempdir().map_err(ServerError::Extract)?;
    extract_assets(assets_dir.path()).map_err(ServerError::Extract)?;

    let router: Router = Router::new().fallback_service(
        ServeDir::new(assets_dir.path())
            .precompressed_br()
            .append_index_html_on_directories(true),
    );

    let addr = policy.addr();
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| ServerError::Bind(addr, e))?;

    let result = axum::serve(listener, router).await.map_err(ServerError::Serve);

    // Keep the tempdir alive for the whole server lifetime; drop it explicitly
    // once serving stops rather than relying on scope-end ordering.
    drop(assets_dir);
    result
}

fn extract_assets(dest: &Path) -> io::Result<()> {
    for file in Assets::iter() {
        let asset = Assets::get(&file).expect("iter() only yields paths that exist");
        let out_path = dest.join(file.as_ref());
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&out_path, asset.data)?;
    }
    Ok(())
}
