use crate::types::{EnrichedDetails, EsConfig};

pub async fn fetch_details(
    attr_path: &str,
    es_config: &EsConfig,
) -> anyhow::Result<Option<EnrichedDetails>> {
    let body = serde_json::json!({
        "query": { "term": { es_config.term_field.clone(): attr_path } },
        "size": 1
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    let resp = client.post(&es_config.url).json(&body).send().await?;
    if !resp.status().is_success() {
        return Ok(None);
    }

    let json: serde_json::Value = resp.json().await?;
    let hit = json["hits"]["hits"]
        .as_array()
        .and_then(|hits| hits.first())
        .and_then(|h| h.get("_source"));

    let Some(src) = hit else {
        return Ok(None);
    };

    Ok(Some(EnrichedDetails {
        attr_path: attr_path.to_string(),
        homepage: src["package_homepage"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
        license: src["package_license"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v["fullName"].as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
        maintainers: src["package_maintainers"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v["github"].as_str().map(|s| format!("@{}", s)))
                    .collect()
            })
            .unwrap_or_default(),
        broken: src["package_broken"].as_bool().unwrap_or(false),
        long_description: src["package_longDescription"].as_str().map(str::to_string),
    }))
}
