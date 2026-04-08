use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use nix_search_core::types::Package;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const DEFAULT_FETCH_TIMEOUT_SECS: u64 = 30;
const DEFAULT_RETRIES: usize = 3;
const WEB_TARGET_PLATFORMS: &[&str] = &["x86_64-linux", "aarch64-darwin"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedData {
    pub packages: Vec<Package>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepManifest {
    pub version: String,
    pub checksum: String,
    pub package_count: usize,
    pub built_at: u64,
    pub artifact: String,
}

#[derive(Debug, Clone)]
pub struct PrepOutput {
    pub manifest_path: PathBuf,
    pub artifact_path: PathBuf,
    pub manifest: PrepManifest,
}

pub async fn run_local_prep(output_dir: &Path) -> anyhow::Result<PrepOutput> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create output directory {}", output_dir.display()))?;

    let packages = fetch_and_parse_packages().await?;
    let built_at = now_epoch();

    let prepared = PreparedData { packages };
    let artifact_bytes = serde_json::to_vec(&prepared).context("failed to encode artifact JSON")?;
    let checksum = checksum_hex(&artifact_bytes);
    let version = version_from_checksum(&checksum);
    let artifact_name = format!("packages-{}.json", version);
    let artifact_path = output_dir.join(&artifact_name);

    fs::write(&artifact_path, artifact_bytes)
        .with_context(|| format!("failed to write artifact {}", artifact_path.display()))?;

    let manifest = PrepManifest {
        version,
        checksum,
        package_count: prepared.packages.len(),
        built_at,
        artifact: artifact_name,
    };
    let manifest_path = output_dir.join("manifest.json");
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).context("failed to encode manifest JSON")?,
    )
    .with_context(|| format!("failed to write manifest {}", manifest_path.display()))?;

    Ok(PrepOutput {
        manifest_path,
        artifact_path,
        manifest,
    })
}

async fn fetch_and_parse_packages() -> anyhow::Result<Vec<Package>> {
    let primary_url = crate::cache::fetch::CHANNEL_PACKAGES_URL;
    match fetch_channel_packages_with_retry(primary_url, DEFAULT_FETCH_TIMEOUT_SECS, DEFAULT_RETRIES).await {
        Ok(raw) => match nix_search_core::parse::parse_channel_packages(&raw) {
            Ok(pkgs) if !pkgs.is_empty() => return Ok(filter_web_platforms(pkgs)),
            Ok(_) => {
                eprintln!("warning: primary source returned empty set, falling back to pkgforge");
            }
            Err(err) => {
                eprintln!("warning: failed parsing primary source ({err}), falling back to pkgforge");
            }
        },
        Err(err) => {
            eprintln!("warning: failed fetching primary source ({err}), falling back to pkgforge");
        }
    }

    let raw_dump = fetch_snapshot_with_retry(
        crate::cache::fetch::PKGFORGE_URL,
        DEFAULT_FETCH_TIMEOUT_SECS,
        DEFAULT_RETRIES,
    )
    .await?;
    let pkgs = nix_search_core::parse::parse_dump(&raw_dump).context("failed to parse fallback dump")?;
    Ok(filter_web_platforms(pkgs))
}

fn filter_web_platforms(packages: Vec<Package>) -> Vec<Package> {
    let mut out = Vec::with_capacity(packages.len());

    for mut pkg in packages {
        pkg.platforms
            .retain(|p| WEB_TARGET_PLATFORMS.iter().any(|target| target == p));
        if !pkg.platforms.is_empty() {
            out.push(pkg);
        }
    }

    out
}

async fn fetch_channel_packages_with_retry(
    url: &str,
    timeout_secs: u64,
    retries: usize,
) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .context("failed to build HTTP client")?;

    let mut last_err = None;
    for attempt in 1..=retries.max(1) {
        match client.get(url).send().await {
            Ok(resp) => {
                let resp = resp.error_for_status().with_context(|| {
                    format!("channel request failed with HTTP status on attempt {attempt}")
                })?;
                let bytes = resp
                    .bytes()
                    .await
                    .with_context(|| format!("failed to read channel body on attempt {attempt}"))?;
                return decode_brotli_to_string(&bytes)
                    .with_context(|| format!("failed to decode channel body on attempt {attempt}"));
            }
            Err(err) => {
                last_err = Some(err);
                if attempt < retries.max(1) {
                    let backoff_ms = 250u64.saturating_mul(attempt as u64);
                    tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                }
            }
        }
    }

    Err(anyhow::anyhow!(
        "failed to fetch channel packages after {} attempts: {}",
        retries.max(1),
        last_err
            .map(|e| e.to_string())
            .unwrap_or_else(|| "unknown error".to_string())
    ))
}

fn decode_brotli_to_string(bytes: &[u8]) -> anyhow::Result<String> {
    let mut reader = brotli::Decompressor::new(bytes, 4096);
    let mut out = String::new();
    reader
        .read_to_string(&mut out)
        .context("brotli decode to UTF-8 string failed")?;
    Ok(out)
}

async fn fetch_snapshot_with_retry(
    url: &str,
    timeout_secs: u64,
    retries: usize,
) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .context("failed to build HTTP client")?;

    let mut last_err = None;
    for attempt in 1..=retries.max(1) {
        match client.get(url).send().await {
            Ok(resp) => {
                let resp = resp
                    .error_for_status()
                    .with_context(|| format!("request failed with HTTP status on attempt {attempt}"))?;
                return resp
                    .text()
                    .await
                    .with_context(|| format!("failed to read body on attempt {attempt}"));
            }
            Err(err) => {
                last_err = Some(err);
                if attempt < retries.max(1) {
                    let backoff_ms = 250u64.saturating_mul(attempt as u64);
                    tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                }
            }
        }
    }

    Err(anyhow::anyhow!(
        "failed to fetch upstream snapshot after {} attempts: {}",
        retries.max(1),
        last_err
            .map(|e| e.to_string())
            .unwrap_or_else(|| "unknown error".to_string())
    ))
}

fn checksum_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for b in digest {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

fn version_from_checksum(checksum: &str) -> String {
    let short = checksum.chars().take(12).collect::<String>();
    format!("sha256-{}", short)
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{
        PreparedData, PrepManifest, checksum_hex, filter_web_platforms, version_from_checksum,
    };
    use nix_search_core::types::Package;

    #[test]
    fn deterministic_transform_for_same_input() {
        let fixture = r#"{
            "legacyPackages.x86_64-linux.foo": {"pname":"foo","version":"1","description":"d"},
            "legacyPackages.aarch64-darwin.foo": {"pname":"foo","version":"1","description":"d"},
            "legacyPackages.x86_64-linux.bar": {"pname":"bar","version":"2","description":"desc"}
        }"#;

        let pkgs1 = nix_search_core::parse::parse_dump(fixture).unwrap();
        let pkgs2 = nix_search_core::parse::parse_dump(fixture).unwrap();

        let b1 = serde_json::to_vec(&PreparedData { packages: pkgs1 }).unwrap();
        let b2 = serde_json::to_vec(&PreparedData { packages: pkgs2 }).unwrap();

        assert_eq!(b1, b2);

        let c1 = checksum_hex(&b1);
        let c2 = checksum_hex(&b2);
        assert_eq!(c1, c2);
        assert!(c1.len() >= 64);

        let v = version_from_checksum(&c1);
        assert!(v.starts_with("sha256-"));
        assert_eq!(v.len(), "sha256-".len() + 12);
    }

    #[test]
    fn manifest_fields_populate_expected_values() {
        let manifest = PrepManifest {
            version: "sha256-123456789abc".to_string(),
            checksum: "123456789abcdef0".to_string(),
            package_count: 42,
            built_at: 1_700_000_000,
            artifact: "packages-sha256-123456789abc.json".to_string(),
        };

        let value = serde_json::to_value(manifest).unwrap();
        assert_eq!(value["package_count"], 42);
        assert_eq!(value["checksum"], "123456789abcdef0");
        assert_eq!(value["artifact"], "packages-sha256-123456789abc.json");
    }

    #[test]
    fn filter_web_platforms_keeps_only_targets() {
        let pkgs = vec![
            Package {
                attr_path: "foo".to_string(),
                pname: "foo".to_string(),
                version: "1".to_string(),
                description: "d".to_string(),
                platforms: vec!["x86_64-linux".to_string(), "aarch64-linux".to_string()],
            },
            Package {
                attr_path: "bar".to_string(),
                pname: "bar".to_string(),
                version: "1".to_string(),
                description: "d".to_string(),
                platforms: vec!["aarch64-darwin".to_string()],
            },
            Package {
                attr_path: "baz".to_string(),
                pname: "baz".to_string(),
                version: "1".to_string(),
                description: "d".to_string(),
                platforms: vec!["x86_64-darwin".to_string()],
            },
        ];

        let filtered = filter_web_platforms(pkgs);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].attr_path, "foo");
        assert_eq!(filtered[0].platforms, vec!["x86_64-linux"]);
        assert_eq!(filtered[1].attr_path, "bar");
        assert_eq!(filtered[1].platforms, vec!["aarch64-darwin"]);
    }
}
