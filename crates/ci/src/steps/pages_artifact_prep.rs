use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

pub fn run(dist_dir: &Path) -> Result<()> {
    if !dist_dir.is_dir() {
        bail!("missing web dist directory: {}", dist_dir.display());
    }

    let index_html = dist_dir.join("index.html");
    if !index_html.is_file() {
        bail!("missing web dist index: {}", index_html.display());
    }

    let wasm_exists = fs::read_dir(dist_dir)
        .with_context(|| format!("failed to read dist dir {}", dist_dir.display()))?
        .filter_map(|entry| entry.ok())
        .any(|entry| entry.file_name().to_string_lossy().ends_with(".wasm"));
    if !wasm_exists {
        bail!("missing wasm asset in dist dir: {}", dist_dir.display());
    }

    Ok(())
}