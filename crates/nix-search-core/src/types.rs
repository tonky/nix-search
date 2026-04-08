use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub attr_path: String,
    pub pname: String,
    pub version: String,
    pub description: String,
    pub platforms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMeta {
    pub channel: String,
    pub fetched_at: u64,
    pub package_count: usize,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub es_url: Option<String>,
    pub es_term_field: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnrichedDetails {
    pub attr_path: String,
    pub homepage: Vec<String>,
    pub license: Vec<String>,
    pub maintainers: Vec<String>,
    pub broken: bool,
    pub long_description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EsConfig {
    pub url: String,
    pub term_field: String,
}
