use crate::Manifest;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn run(manifest: &Manifest, pages_data_dir: &Path, web_static_dir: &Path) -> Result<()> {
    let data_dir = web_static_dir.join("data");
    fs::create_dir_all(&data_dir)
        .with_context(|| format!("failed to create static data dir {}", data_dir.display()))?;

    remove_stale_packages(&data_dir)?;

    copy_file(
        pages_data_dir.join("manifest.json"),
        data_dir.join("manifest.json"),
    )?;

    copy_file(
        pages_data_dir.join(&manifest.artifact),
        data_dir.join(&manifest.artifact),
    )?;

    if let Some(compressed_artifact) = manifest.compressed_artifact.as_ref() {
        copy_file(pages_data_dir.join(compressed_artifact), data_dir.join(compressed_artifact))?;
    }

    Ok(())
}

fn remove_stale_packages(data_dir: &Path) -> Result<()> {
    for entry in fs::read_dir(data_dir)
        .with_context(|| format!("failed to read static data dir {}", data_dir.display()))?
    {
        let entry = entry.with_context(|| format!("failed to read entry in {}", data_dir.display()))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("packages-") && (name.ends_with(".json") || name.ends_with(".json.br")) {
            fs::remove_file(entry.path())
                .with_context(|| format!("failed to remove stale asset {}", entry.path().display()))?;
        }
    }
    Ok(())
}

fn copy_file(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<()> {
    let from = from.as_ref();
    let to = to.as_ref();
    fs::copy(from, to).with_context(|| format!("failed to copy {} to {}", from.display(), to.display()))?;
    Ok(())
}