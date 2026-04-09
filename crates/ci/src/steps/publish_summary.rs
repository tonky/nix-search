use crate::env;
use crate::Manifest;
use anyhow::Result;

pub fn run(manifest: &Manifest) -> Result<()> {
    // Set by the calling workflow when available; falls back to omitting URL lines.
    let summary = render_summary(manifest, std::env::var("PAGE_URL").ok().as_deref());
    env::summary(summary)?;
    Ok(())
}

pub fn render_summary(manifest: &Manifest, page_url: Option<&str>) -> String {
    let mut markdown = String::new();
    markdown.push_str("# WASM Site Publish\n\n");
    markdown.push_str(&format!("- Version: {}\n", manifest.version));
    markdown.push_str(&format!("- Package count: {}\n", manifest.package_count));
    markdown.push_str(&format!("- Checksum: {}\n", manifest.checksum));
    if let Some(page_url) = page_url {
        markdown.push_str(&format!("- Page URL: {}\n", page_url));
        markdown.push_str(&format!("- Manifest URL: {}data/manifest.json\n", page_url));
    }
    markdown
}