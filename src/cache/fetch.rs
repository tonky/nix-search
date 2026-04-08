use anyhow::Context;
use reqwest::header::{ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};

pub const PKGFORGE_URL: &str =
    "https://raw.githubusercontent.com/pkgforge-dev/NixOS-Packages/main/nixpkgs.json";
pub const CHANNEL_PACKAGES_URL: &str =
    "https://channels.nixos.org/nixpkgs-unstable/packages.json.br";

#[derive(Debug)]
pub struct FetchResult {
    pub body: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

pub async fn fetch_dump(
    url: &str,
    etag: Option<&str>,
    last_modified: Option<&str>,
) -> anyhow::Result<FetchResult> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("failed to build HTTP client")?;

    let mut req = client.get(url);
    if let Some(v) = etag {
        req = req.header(IF_NONE_MATCH, v);
    }
    if let Some(v) = last_modified {
        req = req.header(IF_MODIFIED_SINCE, v);
    }

    let resp = req.send().await.context("failed to fetch package dump")?;
    let headers = resp.headers().clone();

    let etag_out = headers
        .get(ETAG)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let modified_out = headers
        .get(LAST_MODIFIED)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    if resp.status() == reqwest::StatusCode::NOT_MODIFIED {
        return Ok(FetchResult {
            body: None,
            etag: etag_out,
            last_modified: modified_out,
        });
    }

    let body = resp.text().await.context("failed to read dump body")?;
    Ok(FetchResult {
        body: Some(body),
        etag: etag_out,
        last_modified: modified_out,
    })
}
