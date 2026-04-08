use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use crate::types::Package;

#[derive(Debug, Deserialize)]
struct RawEntry {
    pname: Option<String>,
    version: Option<String>,
    description: Option<String>,
}

#[derive(Debug)]
struct GroupedEntry {
    pname: String,
    version: String,
    description: String,
    platforms: HashSet<String>,
}

#[derive(Debug, Deserialize)]
struct ChannelRoot {
    packages: HashMap<String, ChannelEntry>,
}

#[derive(Debug, Deserialize)]
struct ChannelEntry {
    pname: Option<String>,
    version: Option<String>,
    system: Option<String>,
    #[serde(default)]
    meta: ChannelMeta,
}

#[derive(Debug, Deserialize, Default)]
struct ChannelMeta {
    description: Option<serde_json::Value>,
    platforms: Option<serde_json::Value>,
}

pub fn parse_key(key: &str) -> Option<(&str, &str)> {
    let rest = key.strip_prefix("legacyPackages.")?;
    let dot = rest.find('.')?;
    Some((&rest[..dot], &rest[dot + 1..]))
}

pub fn parse_dump(json: &str) -> anyhow::Result<Vec<Package>> {
    let raw: HashMap<String, RawEntry> = serde_json::from_str(json)?;
    let mut grouped: HashMap<String, GroupedEntry> = HashMap::new();

    for (key, entry) in raw {
        let Some((platform, attr)) = parse_key(&key) else {
            continue;
        };
        let attr = attr.to_string();
        let g = grouped.entry(attr).or_insert_with(|| GroupedEntry {
            pname: entry.pname.clone().unwrap_or_default(),
            version: entry.version.clone().unwrap_or_default(),
            description: entry.description.clone().unwrap_or_default(),
            platforms: HashSet::new(),
        });
        g.platforms.insert(platform.to_string());

        if g.pname.is_empty() {
            g.pname = entry.pname.unwrap_or_default();
        }
        if g.version.is_empty() {
            g.version = entry.version.unwrap_or_default();
        }
        if g.description.is_empty() {
            g.description = entry.description.unwrap_or_default();
        }
    }

    let mut out = Vec::with_capacity(grouped.len());
    for (attr_path, g) in grouped {
        let mut platforms: Vec<String> = g.platforms.into_iter().collect();
        platforms.sort();
        out.push(Package {
            attr_path: attr_path.clone(),
            pname: if g.pname.is_empty() {
                attr_path
            } else {
                g.pname
            },
            version: g.version,
            description: g.description,
            platforms,
        });
    }
    out.sort_by(|a, b| a.attr_path.cmp(&b.attr_path));
    Ok(out)
}

pub fn parse_channel_packages(json: &str) -> anyhow::Result<Vec<Package>> {
    let root: ChannelRoot = serde_json::from_str(json)?;
    let mut out = Vec::with_capacity(root.packages.len());

    for (attr_path, entry) in root.packages {
        let mut platforms = parse_platforms(entry.meta.platforms, entry.system);
        platforms.sort();
        platforms.dedup();

        out.push(Package {
            pname: entry.pname.unwrap_or_else(|| attr_path.clone()),
            version: entry.version.unwrap_or_default(),
            description: parse_description(entry.meta.description),
            platforms,
            attr_path,
        });
    }

    out.sort_by(|a, b| a.attr_path.cmp(&b.attr_path));
    Ok(out)
}

fn parse_description(value: Option<serde_json::Value>) -> String {
    match value {
        Some(serde_json::Value::String(s)) => s,
        _ => String::new(),
    }
}

fn parse_platforms(value: Option<serde_json::Value>, system: Option<String>) -> Vec<String> {
    let mut out = Vec::new();

    match value {
        Some(serde_json::Value::Array(arr)) => {
            for v in arr {
                if let serde_json::Value::String(s) = v {
                    out.push(s);
                }
            }
        }
        Some(serde_json::Value::String(s)) => out.push(s),
        Some(serde_json::Value::Object(map)) => {
            for (k, v) in map {
                match v {
                    serde_json::Value::Bool(true)
                    | serde_json::Value::Null
                    | serde_json::Value::String(_)
                    | serde_json::Value::Number(_)
                    | serde_json::Value::Array(_)
                    | serde_json::Value::Object(_) => out.push(k),
                    serde_json::Value::Bool(false) => {}
                }
            }
        }
        _ => {}
    }

    if out.is_empty() {
        if let Some(s) = system {
            out.push(s);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::{parse_channel_packages, parse_dump, parse_key};

    #[test]
    fn parse_key_works() {
        let (plat, attr) =
            parse_key("legacyPackages.x86_64-linux.python312Packages.requests").unwrap();
        assert_eq!(plat, "x86_64-linux");
        assert_eq!(attr, "python312Packages.requests");
    }

    #[test]
    fn parse_dump_groups_by_attr() {
        let json = r#"{
            "legacyPackages.x86_64-linux.foo": {"pname":"foo","version":"1","description":"d"},
            "legacyPackages.aarch64-darwin.foo": {"pname":"foo","version":"1","description":"d"}
        }"#;

        let pkgs = parse_dump(json).unwrap();
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].attr_path, "foo");
        assert_eq!(pkgs[0].platforms.len(), 2);
    }

    #[test]
    fn parse_channel_packages_uses_meta_platforms() {
        let json = r#"{
            "packages": {
                "hello": {
                    "pname": "hello",
                    "version": "1.0",
                    "meta": {
                        "description": "hello package",
                        "platforms": ["x86_64-linux", "aarch64-darwin"]
                    }
                }
            }
        }"#;

        let pkgs = parse_channel_packages(json).unwrap();
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].attr_path, "hello");
        assert_eq!(pkgs[0].platforms, vec!["aarch64-darwin", "x86_64-linux"]);
    }
}
