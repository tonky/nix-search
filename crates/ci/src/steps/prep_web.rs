use crate::shell::{render_command, CommandSpec, Shell};
use crate::Manifest;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn run(shell: &mut dyn Shell, repo_root: &Path, output_dir: &Path) -> Result<Manifest> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create prep-web output dir {}", output_dir.display()))?;

    let command = CommandSpec::new("cargo")
        .args(["run", "--release", "--", "prep-web", "--output"])
        .arg(output_dir.display().to_string())
        .cwd(repo_root.to_path_buf());
    shell
        .run(command.clone())
        .with_context(|| format!("failed to run prep-web command: {}", render_command(&command)))?;

    load_manifest(output_dir)
}

pub fn load_manifest(output_dir: &Path) -> Result<Manifest> {
    let manifest_path = output_dir.join("manifest.json");
    let manifest_text = fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read manifest {}", manifest_path.display()))?;
    let manifest = serde_json::from_str(&manifest_text)
        .with_context(|| format!("failed to parse manifest {}", manifest_path.display()))?;
    Ok(manifest)
}