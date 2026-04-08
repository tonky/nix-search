use crate::types::Package;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct SearchConfig {
    pub query: String,
    pub platform: Option<String>,
    pub limit: usize,
    pub exact_attr: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ScoredPackage {
    pub package: Package,
    pub score: f32,
}

#[derive(Debug, Clone, Default)]
pub struct SearchResults {
    pub matched: Vec<ScoredPackage>,
    pub others: Vec<ScoredPackage>,
}

pub fn compute_overfetch_limit(config: &SearchConfig) -> usize {
    let mut overfetch = config.limit.saturating_mul(5).max(config.limit);
    let tokens: Vec<&str> = config
        .query
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .collect();

    if tokens.len() >= 2 {
        overfetch = overfetch.max(config.limit.saturating_mul(20));
    }
    if tokens.iter().any(|t| t.chars().count() <= 2) {
        overfetch = overfetch.max(config.limit.saturating_mul(50)).max(500);
    }

    overfetch
}

pub fn apply_global_limit<T>(matched: &mut Vec<T>, others: &mut Vec<T>, limit: usize) {
    if matched.len() >= limit {
        matched.truncate(limit);
        others.clear();
    } else {
        let remaining = limit.saturating_sub(matched.len());
        others.truncate(remaining);
    }
}

pub fn rerank_with_prefix_bonus(results: &mut [ScoredPackage], query: &str) {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return;
    }
    let tokens: Vec<&str> = q.split_whitespace().filter(|t| !t.is_empty()).collect();
    let q_compact = compact_alnum(&q);

    for sp in results.iter_mut() {
        let attr = sp.package.attr_path.to_lowercase();
        let name = sp.package.pname.to_lowercase();
        let attr_parts: Vec<&str> = attr
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|s| !s.is_empty())
            .collect();
        let name_parts: Vec<&str> = name
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|s| !s.is_empty())
            .collect();
        let mut all_parts: Vec<&str> = Vec::new();
        for part in attr_parts.iter().chain(name_parts.iter()) {
            if !all_parts.contains(part) {
                all_parts.push(*part);
            }
        }

        let mut bonus = 0.0f32;
        if attr == q {
            bonus += 10_000.0;
        }
        if name == q {
            bonus += 8_000.0;
        }
        if attr.starts_with(&q) {
            bonus += 2_000.0;
        }
        if name.starts_with(&q) {
            bonus += 1_600.0;
        }
        if attr.contains(&q) {
            bonus += 200.0;
        }
        if name.contains(&q) {
            bonus += 120.0;
        }

        if !q_compact.is_empty() {
            let attr_compact = compact_alnum(&attr);
            let name_compact = compact_alnum(&name);

            if attr_compact.starts_with(&q_compact) {
                let remainder = attr_compact.len().saturating_sub(q_compact.len()) as f32;
                bonus += 1_200.0 - (remainder * 110.0).min(700.0);
            }
            if name_compact.starts_with(&q_compact) {
                let remainder = name_compact.len().saturating_sub(q_compact.len()) as f32;
                bonus += 900.0 - (remainder * 90.0).min(540.0);
            }
        }

        if !tokens.is_empty() {
            let mut matched_tokens = 0usize;
            let mut matched_part_indexes = Vec::new();
            let mut total_token_quality = 0.0f32;
            for token in &tokens {
                let mut token_matched = false;
                let token_len = token.chars().count();
                let best_part = all_parts
                    .iter()
                    .enumerate()
                    .map(|(idx, p)| (idx, token_part_quality(token, p)))
                    .max_by(|a, b| a.1.total_cmp(&b.1));
                let best_part_sim = all_parts
                    .iter()
                    .map(|p| normalized_similarity(token, p))
                    .fold(0.0f32, f32::max);
                let best_subseq_compactness = all_parts
                    .iter()
                    .filter_map(|p| subsequence_compactness(token, p))
                    .fold(0.0f32, f32::max);

                if attr.starts_with(token) {
                    bonus += 1_200.0;
                    token_matched = true;
                } else if name.starts_with(token) {
                    bonus += 900.0;
                    token_matched = true;
                } else if attr_parts.iter().any(|p| p.starts_with(token)) {
                    bonus += 850.0;
                    token_matched = true;
                } else if name_parts.iter().any(|p| p.starts_with(token)) {
                    bonus += 700.0;
                    token_matched = true;
                } else if attr.contains(token) {
                    bonus += 250.0;
                    token_matched = true;
                } else if name.contains(token) {
                    bonus += 150.0;
                    token_matched = true;
                }

                let sim_match_threshold = if token_len >= 5 { 0.82 } else { 0.5 };
                if best_part_sim >= sim_match_threshold {
                    bonus += 400.0 * best_part_sim;
                    token_matched = true;
                } else if best_part_sim >= 0.35 {
                    bonus += 120.0 * best_part_sim;
                }

                if token_len >= 3 {
                    if best_subseq_compactness >= 0.45 {
                        bonus += 260.0 * best_subseq_compactness;
                        token_matched = true;
                    } else if best_subseq_compactness > 0.0 {
                        bonus += 60.0 * best_subseq_compactness;
                    }
                }

                if token_len <= 2 && (attr_parts.contains(token) || name_parts.contains(token)) {
                    bonus += 250.0;
                    token_matched = true;
                }

                if let Some((part_idx, quality)) = best_part
                    && quality >= 0.45
                {
                    token_matched = true;
                    matched_part_indexes.push(part_idx);
                    total_token_quality += quality;
                }

                if token_matched {
                    matched_tokens += 1;
                }
            }

            if matched_tokens == tokens.len() {
                bonus += 3_000.0;
                if !tokens.is_empty() {
                    bonus += (total_token_quality / tokens.len() as f32) * 1_000.0;
                }
                if tokens.len() >= 2 {
                    let distinct_parts = matched_part_indexes
                        .iter()
                        .copied()
                        .collect::<HashSet<_>>()
                        .len();
                    if distinct_parts == tokens.len() {
                        bonus += 1_200.0;
                    } else if distinct_parts <= 1 {
                        bonus -= 1_200.0;
                    }

                    let extra_parts = all_parts.len().saturating_sub(tokens.len());
                    bonus -= (extra_parts as f32 * 120.0).min(1_200.0);

                    let attr_depth = sp.package.attr_path.matches('.').count();
                    bonus -= (attr_depth as f32 * 250.0).min(1_500.0);
                }
            } else if matched_tokens > 0 {
                bonus += matched_tokens as f32 * 100.0;
            }
        }

        sp.score += bonus;
    }

    results.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.package.attr_path.len().cmp(&b.package.attr_path.len()))
            .then_with(|| a.package.attr_path.cmp(&b.package.attr_path))
            .then_with(|| a.package.pname.len().cmp(&b.package.pname.len()))
            .then_with(|| a.package.pname.cmp(&b.package.pname))
    });
}

fn is_subsequence(needle: &str, haystack: &str) -> bool {
    let mut chars = haystack.chars();
    for n in needle.chars() {
        if !chars.any(|h| h == n) {
            return false;
        }
    }
    true
}

fn subsequence_compactness(needle: &str, haystack: &str) -> Option<f32> {
    if needle.is_empty() || haystack.is_empty() || !is_subsequence(needle, haystack) {
        return None;
    }

    let n: Vec<char> = needle.chars().collect();
    let h: Vec<char> = haystack.chars().collect();

    let mut first = None;
    let mut last = None;
    let mut ni = 0usize;
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

    match (first, last) {
        (Some(f), Some(l)) if l >= f => {
            let span = (l - f + 1) as f32;
            Some(n.len() as f32 / span)
        }
        _ => None,
    }
}

fn normalized_similarity(a: &str, b: &str) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let dist = levenshtein(a, b);
    let max_len = a.chars().count().max(b.chars().count()) as f32;
    1.0 - (dist as f32 / max_len)
}

fn compact_alnum(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

fn token_part_quality(token: &str, part: &str) -> f32 {
    if token.is_empty() || part.is_empty() {
        return 0.0;
    }

    let mut quality = 0.0f32;
    if part == token {
        quality = quality.max(1.0);
    }
    if part.starts_with(token) {
        quality = quality.max(0.9);
    }
    if part.contains(token) {
        quality = quality.max(0.6);
    }

    let sim = normalized_similarity(token, part);
    let sim_quality_threshold = if token.chars().count() >= 5 {
        0.85
    } else {
        0.5
    };
    if sim >= sim_quality_threshold {
        quality = quality.max(0.45 + 0.4 * sim);
    }

    if token.chars().count() >= 3
        && let Some(compactness) = subsequence_compactness(token, part)
    {
        quality = quality.max(0.4 + 0.5 * compactness);
    }

    quality
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();

    let mut dp = vec![vec![0usize; b_chars.len() + 1]; a_chars.len() + 1];
    for (i, row) in dp.iter_mut().enumerate().take(a_chars.len() + 1) {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate().take(b_chars.len() + 1) {
        *cell = j;
    }

    for i in 1..=a_chars.len() {
        for j in 1..=b_chars.len() {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[a_chars.len()][b_chars.len()]
}

#[cfg(test)]
mod tests {
    use super::{
        ScoredPackage, SearchConfig, apply_global_limit, compute_overfetch_limit,
        rerank_with_prefix_bonus,
    };
    use crate::types::Package;

    fn cfg(query: &str, limit: usize) -> SearchConfig {
        SearchConfig {
            query: query.to_string(),
            platform: None,
            limit,
            exact_attr: None,
        }
    }

    #[test]
    fn overfetch_default_single_token() {
        assert_eq!(compute_overfetch_limit(&cfg("ripgrep", 20)), 100);
    }

    #[test]
    fn overfetch_multi_token_increases_multiplier() {
        assert_eq!(compute_overfetch_limit(&cfg("claude code", 20)), 400);
    }

    #[test]
    fn overfetch_short_token_uses_high_floor() {
        assert_eq!(compute_overfetch_limit(&cfg("cld co", 20)), 1000);
        assert_eq!(compute_overfetch_limit(&cfg("x", 2)), 500);
    }

    #[test]
    fn global_limit_prefers_matched_section() {
        let mut matched = vec![1, 2, 3];
        let mut others = vec![10, 11];
        apply_global_limit(&mut matched, &mut others, 2);
        assert_eq!(matched, vec![1, 2]);
        assert!(others.is_empty());
    }

    #[test]
    fn global_limit_fills_remaining_with_others() {
        let mut matched = vec![1];
        let mut others = vec![10, 11, 12];
        apply_global_limit(&mut matched, &mut others, 3);
        assert_eq!(matched, vec![1]);
        assert_eq!(others, vec![10, 11]);
    }

    #[test]
    fn rerank_prefers_root_package_for_compressed_query() {
        let mut rows = vec![
            ScoredPackage {
                package: Package {
                    attr_path: "code-theme-converter".to_string(),
                    pname: "code-theme-converter".to_string(),
                    version: "1".to_string(),
                    description: "Theme converter".to_string(),
                    platforms: vec!["x86_64-linux".to_string()],
                },
                score: 1.0,
            },
            ScoredPackage {
                package: Package {
                    attr_path: "claude-code".to_string(),
                    pname: "claude-code".to_string(),
                    version: "1".to_string(),
                    description: "Anthropic Claude code assistant".to_string(),
                    platforms: vec!["x86_64-linux".to_string()],
                },
                score: 1.0,
            },
        ];

        rerank_with_prefix_bonus(&mut rows, "cld cod");
        assert_eq!(rows[0].package.attr_path, "claude-code");
    }
}
