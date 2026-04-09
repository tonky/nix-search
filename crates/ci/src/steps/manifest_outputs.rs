use crate::env;
use crate::steps::prep_web;
use anyhow::Result;
use std::path::Path;

pub fn run(pages_data_dir: &Path) -> Result<crate::Manifest> {
    let manifest = prep_web::load_manifest(pages_data_dir)?;
    env::set_output("version", &manifest.version)?;
    env::set_output("package_count", manifest.package_count.to_string())?;
    env::set_output("checksum", &manifest.checksum)?;
    Ok(manifest)
}