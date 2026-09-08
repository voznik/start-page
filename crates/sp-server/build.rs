use std::{fs, path::Path};

/// `rust-embed`'s `#[folder]` requires the directory to exist when sp-server compiles, but
/// `crates/sp-web-host/dist` is build output: it is absent on a fresh clone and `trunk build`
/// deletes and recreates it. Tracking a placeholder file there does not work, because trunk
/// removes it on every build.
///
/// Creating the directory here means sp-server always compiles; if `dist` is empty the binary
/// simply embeds nothing, and `cargo xtask dist` is what fills it.
fn main() {
    let dist = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("sp-web-host")
        .join("dist");

    if let Err(err) = fs::create_dir_all(&dist) {
        println!("cargo::warning=could not create {}: {err}", dist.display());
    }

    println!("cargo::rerun-if-changed=../sp-web-host/dist");
}
