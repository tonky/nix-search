use nix_search_core::search::{
    ScoredPackage, SearchConfig, SearchResults, apply_global_limit, compute_overfetch_limit,
    rerank_with_prefix_bonus,
};
use nix_search_core::types::Package;

#[derive(Clone)]
pub struct SearchRow {
    pub pkg: Package,
    attr_lc: String,
    pname_lc: String,
    attr_compact: String,
    pname_compact: String,
}

impl SearchRow {
    pub fn from_package(pkg: Package) -> Self {
        let attr_lc = pkg.attr_path.to_lowercase();
        let pname_lc = pkg.pname.to_lowercase();
        Self {
            attr_compact: compact_alnum(&attr_lc),
            pname_compact: compact_alnum(&pname_lc),
            attr_lc,
            pname_lc,
            pkg,
        }
    }
}

pub fn run_search(
    rows: &[SearchRow],
    query: &str,
    selected_platform: Option<&str>,
    all_platforms: bool,
    limit: usize,
) -> SearchResults {
    #[derive(Clone, Copy)]
    struct ScoredIndex {
        idx: usize,
        score: f32,
    }

    let config = SearchConfig {
        query: query.to_string(),
        platform: if all_platforms {
            None
        } else {
            selected_platform.map(|s| s.to_string())
        },
        limit,
        exact_attr: None,
    };

    let overfetch = compute_overfetch_limit(&config).min(4000);
    let q = query.trim().to_lowercase();

    // Avoid expensive full scans for one-character input; wait for more intent.
    if !q.is_empty() && q.chars().count() < 2 {
        return SearchResults::default();
    }

    let mut scored = if q.is_empty() {
        rows.iter()
            .take(overfetch)
            .map(|r| ScoredPackage {
                package: r.pkg.clone(),
                score: 0.0,
            })
            .collect::<Vec<_>>()
    } else {
        let tokens = q
            .split_whitespace()
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>();

        let mut ranked = rows
            .iter()
            .enumerate()
            .filter_map(|r| {
                let (idx, r) = r;
                let mut score = 0.0f32;

                if r.attr_lc == q {
                    score += 3000.0;
                }
                if r.pname_lc == q {
                    score += 2400.0;
                }
                if r.attr_lc.starts_with(&q) {
                    score += 700.0;
                }
                if r.pname_lc.starts_with(&q) {
                    score += 550.0;
                }
                if r.attr_lc.contains(&q) {
                    score += 180.0;
                }
                if r.pname_lc.contains(&q) {
                    score += 130.0;
                }
                for token in &tokens {
                    if r.attr_lc.starts_with(token) {
                        score += 220.0;
                    } else if r.pname_lc.starts_with(token) {
                        score += 170.0;
                    } else if r.attr_lc.contains(token) {
                        score += 70.0;
                    } else if r.pname_lc.contains(token) {
                        score += 50.0;
                    }
                }

                // Admit close typo matches so rerank can surface expected packages
                // like "ascinema" -> "asciinema".
                let q_len = q.chars().count();

                if score <= 0.0 && q_len >= 6 {
                    if is_within_one_edit(&q, &r.pname_lc) {
                        score += 85.0;
                    } else if is_within_one_edit(&q, &r.attr_lc) {
                        score += 60.0;
                    }
                }

                // Admit compact subsequence-like fuzzy prefixes (e.g. ascin -> asciinema)
                // so they are eligible for rerank.
                if score <= 0.0 && q_len >= 5 {
                    let q_first = q.as_bytes().first().copied();
                    let pname_len = r.pname_compact.len();
                    let attr_len = r.attr_compact.len();

                    let pname_eligible = q_first
                        .zip(r.pname_compact.as_bytes().first().copied())
                        .map(|(a, b)| a == b)
                        .unwrap_or(false)
                        && pname_len.abs_diff(q.len()) <= 6;

                    let attr_eligible = q_first
                        .zip(r.attr_compact.as_bytes().first().copied())
                        .map(|(a, b)| a == b)
                        .unwrap_or(false)
                        && attr_len.abs_diff(q.len()) <= 8;

                    if pname_eligible {
                        let pname_subseq =
                            subsequence_compactness(&q, &r.pname_compact).unwrap_or(0.0);
                        if pname_subseq >= 0.72 {
                            score += 70.0;
                        }
                    }

                    if score <= 0.0 && attr_eligible {
                        let attr_subseq =
                            subsequence_compactness(&q, &r.attr_compact).unwrap_or(0.0);
                        if attr_subseq >= 0.65 {
                            score += 50.0;
                        }
                    }
                }

                if score <= 0.0 {
                    None
                } else {
                    Some(ScoredIndex { idx, score })
                }
            })
            .collect::<Vec<_>>();

        ranked.sort_by(|a, b| {
            b.score.total_cmp(&a.score).then_with(|| {
                rows[a.idx]
                    .pkg
                    .attr_path
                    .cmp(&rows[b.idx].pkg.attr_path)
            })
        });
        ranked.truncate(overfetch);

        ranked
            .into_iter()
            .map(|entry| ScoredPackage {
                package: rows[entry.idx].pkg.clone(),
                score: entry.score,
            })
            .collect::<Vec<_>>()
    };

    if !q.is_empty() {
        rerank_with_prefix_bonus(&mut scored, &q);
    }

    let (mut matched, mut others) = nix_search_core::split::split_by_platform(
        scored,
        if all_platforms { None } else { selected_platform },
        |sp| &sp.package.platforms,
    );
    apply_global_limit(&mut matched, &mut others, limit);

    SearchResults { matched, others }
}

fn is_within_one_edit(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }

    let a = a.as_bytes();
    let b = b.as_bytes();
    let la = a.len();
    let lb = b.len();

    if la.max(lb) - la.min(lb) > 1 {
        return false;
    }

    if la == lb {
        let mut mismatches = 0usize;
        for i in 0..la {
            if a[i] != b[i] {
                mismatches += 1;
                if mismatches > 1 {
                    return false;
                }
            }
        }
        return mismatches == 1;
    }

    let (short, long) = if la < lb { (a, b) } else { (b, a) };
    let mut i = 0usize;
    let mut j = 0usize;
    let mut used_skip = false;

    while i < short.len() && j < long.len() {
        if short[i] == long[j] {
            i += 1;
            j += 1;
            continue;
        }

        if used_skip {
            return false;
        }
        used_skip = true;
        j += 1;
    }

    true
}

fn compact_alnum(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

fn subsequence_compactness(needle: &str, haystack: &str) -> Option<f32> {
    if needle.is_empty() || haystack.is_empty() {
        return None;
    }

    let n: Vec<char> = needle.chars().collect();
    let h: Vec<char> = haystack.chars().collect();
    let mut ni = 0usize;
    let mut first = None;
    let mut last = None;

    for (i, hc) in h.iter().enumerate() {
        if ni < n.len() && *hc == n[ni] {
            if first.is_none() {
                first = Some(i);
            }
            last = Some(i);
            ni += 1;
            if ni == n.len() {
                break;
            }
        }
    }

    if ni != n.len() {
        return None;
    }

    match (first, last) {
        (Some(f), Some(l)) if l >= f => {
            let span = (l - f + 1) as f32;
            Some(n.len() as f32 / span)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{SearchRow, is_within_one_edit, run_search};
    use nix_search_core::types::Package;

    fn pkg(attr: &str, pname: &str) -> Package {
        Package {
            attr_path: attr.to_string(),
            pname: pname.to_string(),
            version: "1.0.0".to_string(),
            description: "fixture".to_string(),
            platforms: vec!["x86_64-linux".to_string()],
        }
    }

    #[test]
    fn one_edit_distance_matches_expected_pairs() {
        assert!(is_within_one_edit("ascinema", "asciinema"));
        assert!(is_within_one_edit("asciinema", "ascinema"));
        assert!(is_within_one_edit("asciinemaa", "asciinema"));
        assert!(!is_within_one_edit("ascinema", "asciinemaa-lib"));
    }

    #[test]
    fn typo_query_surfaces_expected_package() {
        let rows = vec![
            SearchRow::from_package(pkg("asciinema", "asciinema")),
            SearchRow::from_package(pkg("ripgrep", "ripgrep")),
        ];

        let results = run_search(&rows, "ascinema", Some("x86_64-linux"), false, 20);

        let found = results
            .matched
            .iter()
            .chain(results.others.iter())
            .any(|sp| sp.package.attr_path == "asciinema");
        assert!(found);
    }

    #[test]
    fn partial_typo_prefix_queries_surface_asciinema() {
        let rows = vec![
            SearchRow::from_package(pkg("asciinema", "asciinema")),
            SearchRow::from_package(pkg("ascii", "ascii")),
            SearchRow::from_package(pkg("ripgrep", "ripgrep")),
        ];

        for q in ["ascin", "ascine", "ascinem"] {
            let results = run_search(&rows, q, Some("x86_64-linux"), false, 20);
            let found = results
                .matched
                .iter()
                .chain(results.others.iter())
                .any(|sp| sp.package.attr_path == "asciinema");
            assert!(found, "expected query '{q}' to match asciinema");
        }
    }

    #[test]
    fn global_limit_is_enforced() {
        let rows = (0..80)
            .map(|i| SearchRow::from_package(pkg(&format!("pkg-{i}"), &format!("pkg-{i}"))))
            .collect::<Vec<_>>();

        let results = run_search(&rows, "pkg", Some("x86_64-linux"), false, 24);
        assert!(results.matched.len() + results.others.len() <= 24);
    }
}
