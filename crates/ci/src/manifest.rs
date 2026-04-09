use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub checksum: String,
    pub package_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub built_at: Option<u64>,
    pub artifact: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compressed_artifact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compressed_format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compressed_size_bytes: Option<u64>,
}