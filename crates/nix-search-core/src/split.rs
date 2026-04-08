pub fn split_by_platform<T, F>(
    items: Vec<T>,
    platform: Option<&str>,
    mut platforms_for: F,
) -> (Vec<T>, Vec<T>)
where
    F: FnMut(&T) -> &[String],
{
    match platform {
        None => (items, vec![]),
        Some(plat) => items
            .into_iter()
            .partition(|item| platforms_for(item).iter().any(|p| p == plat)),
    }
}

#[cfg(test)]
mod tests {
    use super::split_by_platform;

    #[derive(Debug)]
    struct Item {
        id: &'static str,
        platforms: Vec<String>,
    }

    #[test]
    fn split_by_platform_partitions_items() {
        let items = vec![
            Item {
                id: "a",
                platforms: vec!["x86_64-linux".to_string()],
            },
            Item {
                id: "b",
                platforms: vec!["aarch64-darwin".to_string()],
            },
            Item {
                id: "c",
                platforms: vec!["x86_64-linux".to_string(), "aarch64-darwin".to_string()],
            },
        ];

        let (matched, others) = split_by_platform(items, Some("x86_64-linux"), |i| &i.platforms);
        assert_eq!(matched.len(), 2);
        assert_eq!(others.len(), 1);
        assert_eq!(matched[0].id, "a");
        assert_eq!(matched[1].id, "c");
        assert_eq!(others[0].id, "b");
    }
}
