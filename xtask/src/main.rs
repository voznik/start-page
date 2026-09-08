//! `cargo xtask dist`: clean-checkout to shippable `start-page` binary.
//!
//! 1. `trunk build --release` the browser bundle into `crates/sp-web-host/dist/`.
//! 2. Brotli-compress every file in `dist/` to a `.br` sibling (tower-http's
//!    `ServeDir::precompressed_br()` expects both variants present).
//! 3. `cargo build --release -p sp-cli`, which compiles `sp-server`'s `rust-embed`
//!    over the now-populated `dist/` — the embed only sees files that exist yet.

use anyhow::{Context, Result, bail};
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() -> Result<()> {
    let cmd = std::env::args().nth(1);
    match cmd.as_deref() {
        Some("dist") => dist(),
        _ => bail!("usage: cargo xtask dist"),
    }
}

fn repo_root() -> PathBuf {
    // xtask/Cargo.toml's parent is the workspace root.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask always lives one level below the workspace root")
        .to_path_buf()
}

fn dist() -> Result<()> {
    let root = repo_root();
    let web_host = root.join("crates/sp-web-host");
    let dist_dir = web_host.join("dist");

    run(Command::new("trunk").arg("build").arg("--release").current_dir(&web_host))
        .context("trunk build --release failed (is `trunk` installed and on PATH?)")?;

    brotli_compress_dir(&dist_dir)?;

    run(Command::new("cargo")
        .args(["build", "--release", "-p", "sp-cli"])
        .current_dir(&root))
    .context("cargo build --release -p sp-cli failed")?;

    let binary = root.join("target/release/start-page");
    println!("built {}", binary.display());
    Ok(())
}

fn run(cmd: &mut Command) -> Result<()> {
    let status = cmd.status().with_context(|| format!("failed to spawn {cmd:?}"))?;
    if !status.success() {
        bail!("{cmd:?} exited with {status}");
    }
    Ok(())
}

/// Brotli-compresses every regular file under `dir` (recursively) to a `.br`
/// sibling at quality 11 (max — this runs once at build time, not per request).
fn brotli_compress_dir(dir: &Path) -> Result<()> {
    if !dir.is_dir() {
        bail!("{} does not exist — trunk build did not produce a dist/", dir.display());
    }
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            brotli_compress_dir(&path)?;
            continue;
        }
        if path.extension().is_some_and(|ext| ext == "br") {
            continue;
        }
        let input = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        let mut br_name = path.file_name().expect("read_dir entries always have a name").to_owned();
        br_name.push(".br");
        let br_path = path.with_file_name(br_name);
        let out =
            fs::File::create(&br_path).with_context(|| format!("creating {}", br_path.display()))?;
        // quality 11 (max), 22-bit window (default) — build-time only, not per request.
        let mut writer = brotli::CompressorWriter::new(out, 4096, 11, 22);
        writer
            .write_all(&input)
            .with_context(|| format!("brotli-compressing {}", path.display()))?;
        writer.flush()?;
    }
    Ok(())
}
