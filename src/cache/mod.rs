use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::{CacheMeta, EnrichedDetails};

pub mod enrich;
pub mod fetch;
pub mod index;
pub mod parse;

pub fn index_dir(cache_dir: &Path, channel: &str) -> PathBuf {
    cache_dir.join(channel).join("index")
}

pub fn meta_path(cache_dir: &Path, channel: &str) -> PathBuf {
    cache_dir.join(channel).join("meta.json")
}

pub fn load_meta(cache_dir: &Path, channel: &str) -> Option<CacheMeta> {
    let path = meta_path(cache_dir, channel);
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn save_meta(cache_dir: &Path, channel: &str, meta: &CacheMeta) -> anyhow::Result<()> {
    let path = meta_path(cache_dir, channel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(meta)?)?;
    Ok(())
}

pub fn enriched_path(cache_dir: &Path, channel: &str, attr_path: &str) -> PathBuf {
    let safe = attr_path.replace('/', "__");
    cache_dir
        .join(channel)
        .join("enriched")
        .join(format!("{}.json", safe))
}

pub fn load_enriched(cache_dir: &Path, channel: &str, attr_path: &str) -> Option<EnrichedDetails> {
    let bytes = fs::read(enriched_path(cache_dir, channel, attr_path)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn store_enriched(
    cache_dir: &Path,
    channel: &str,
    details: &EnrichedDetails,
) -> anyhow::Result<()> {
    let path = enriched_path(cache_dir, channel, &details.attr_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(details)?)?;
    Ok(())
}

pub async fn update(cache_dir: &Path, channel: &str) -> anyhow::Result<()> {
    let t_total = Instant::now();

    let t_fetch = Instant::now();
    let prev = load_meta(cache_dir, channel);
    let fetch = fetch::fetch_dump(
        fetch::PKGFORGE_URL,
        prev.as_ref().and_then(|m| m.etag.as_deref()),
        prev.as_ref().and_then(|m| m.last_modified.as_deref()),
    )
    .await?;
    let fetch_ms = t_fetch.elapsed().as_millis();

    let now = now_epoch();

    match fetch.body {
        None => {
            let mut meta = prev.unwrap_or(CacheMeta {
                channel: channel.to_string(),
                fetched_at: now,
                package_count: 0,
                etag: None,
                last_modified: None,
                es_url: None,
                es_term_field: None,
            });
            meta.fetched_at = now;
            meta.etag = fetch.etag.or(meta.etag);
            meta.last_modified = fetch.last_modified.or(meta.last_modified);
            save_meta(cache_dir, channel, &meta)?;
            eprintln!(
                "[perf][cache-update] channel={} fetch_ms={} parse_ms=0 index_ms=0 meta_ms=0 total_ms={} status=not-modified",
                channel,
                fetch_ms,
                t_total.elapsed().as_millis()
            );
            Ok(())
        }
        Some(body) => {
            let t_parse = Instant::now();
            let packages = parse::parse_dump(&body)?;
            let parse_ms = t_parse.elapsed().as_millis();

            let t_index = Instant::now();
            index::build(&index_dir(cache_dir, channel), &packages)?;
            let index_ms = t_index.elapsed().as_millis();

            let meta = CacheMeta {
                channel: channel.to_string(),
                fetched_at: now,
                package_count: packages.len(),
                etag: fetch.etag,
                last_modified: fetch.last_modified,
                es_url: prev.as_ref().and_then(|m| m.es_url.clone()),
                es_term_field: prev.as_ref().and_then(|m| m.es_term_field.clone()),
            };
            let t_meta = Instant::now();
            save_meta(cache_dir, channel, &meta)?;
            let meta_ms = t_meta.elapsed().as_millis();

            eprintln!(
                "[perf][cache-update] channel={} fetch_ms={} parse_ms={} index_ms={} meta_ms={} total_ms={} package_count={} status=updated",
                channel,
                fetch_ms,
                parse_ms,
                index_ms,
                meta_ms,
                t_total.elapsed().as_millis(),
                packages.len()
            );

            Ok(())
        }
    }
}

pub fn clear(cache_dir: &Path, channel: &str) -> anyhow::Result<()> {
    let path = cache_dir.join(channel);
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

pub fn status(cache_dir: &Path, channel: &str) -> anyhow::Result<String> {
    let meta = load_meta(cache_dir, channel);
    let idx = index_dir(cache_dir, channel);
    let size = dir_size(&idx)?;

    let out = if let Some(meta) = meta {
        format!(
            "channel: {}\npackages: {}\ncached_at: {}\nindex_size_bytes: {}\netag: {}",
            meta.channel,
            meta.package_count,
            meta.fetched_at,
            size,
            meta.etag.unwrap_or_default()
        )
    } else {
        format!(
            "channel: {}\nstatus: empty\nindex_size_bytes: {}",
            channel, size
        )
    };

    Ok(out)
}

fn dir_size(path: &Path) -> anyhow::Result<u64> {
    if !path.exists() {
        return Ok(0);
    }

    let mut total = 0u64;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let p = entry.path();
        let meta = entry.metadata()?;
        if meta.is_dir() {
            total += dir_size(&p)?;
        } else {
            total += meta.len();
        }
    }
    Ok(total)
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
