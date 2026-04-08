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
}

impl SearchRow {
    pub fn from_package(pkg: Package) -> Self {
        Self {
            attr_lc: pkg.attr_path.to_lowercase(),
            pname_lc: pkg.pname.to_lowercase(),
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

        let mut out = rows
            .iter()
            .filter_map(|r| {
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

                if score <= 0.0 {
                    None
                } else {
                    Some(ScoredPackage {
                        package: r.pkg.clone(),
                        score,
                    })
                }
            })
            .collect::<Vec<_>>();

        out.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.package.attr_path.cmp(&b.package.attr_path))
        });
        out.truncate(overfetch);
        out
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
