use std::collections::HashMap;

use tantivy::collector::TopDocs;
use tantivy::query::{AllQuery, FuzzyTermQuery, QueryParser, RegexQuery, TermQuery};
use tantivy::schema::IndexRecordOption;
use tantivy::{DocAddress, Term};

use crate::cache::index::{NixIndex, doc_to_package};

use super::{ScoredPackage, SearchConfig};

pub fn run(
    nix_index: &NixIndex,
    config: &SearchConfig,
    limit: usize,
) -> anyhow::Result<Vec<ScoredPackage>> {
    let reader = nix_index.index.reader()?;
    let searcher = reader.searcher();

    if let Some(exact_attr) = config
        .exact_attr
        .clone()
        .or_else(|| normalize_exact_attr(&config.query))
    {
        let term = Term::from_field_text(nix_index.fields.attr_path_exact, &exact_attr);
        let exact = TermQuery::new(term, IndexRecordOption::Basic);
        let docs = searcher.search(&exact, &TopDocs::with_limit(limit).order_by_score())?;
        if !docs.is_empty() {
            return to_scored(&searcher, docs, nix_index);
        }
    }

    let query_str = config.query.trim();
    if query_str.is_empty() {
        let docs = searcher.search(&AllQuery, &TopDocs::with_limit(limit).order_by_score())?;
        return to_scored(&searcher, docs, nix_index);
    }

    let mut parser = QueryParser::for_index(
        &nix_index.index,
        vec![
            nix_index.fields.attr_path_text,
            nix_index.fields.pname,
            nix_index.fields.description,
        ],
    );
    parser.set_conjunction_by_default();
    parser.set_field_boost(nix_index.fields.attr_path_text, 4.0);
    parser.set_field_boost(nix_index.fields.pname, 5.0);
    parser.set_field_boost(nix_index.fields.description, 1.0);

    let short_query = query_str.chars().take(200).collect::<String>();
    let (query, _warnings) = parser.parse_query_lenient(&short_query);
    let bm25_docs = searcher.search(&query, &TopDocs::with_limit(limit).order_by_score())?;

    let query_tokens: Vec<&str> = short_query
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .collect();
    let token_count = query_tokens.len();
    let has_short_token = query_tokens.iter().any(|t| t.chars().count() <= 2);
    let use_fuzzy = bm25_docs.is_empty()
        || (!has_short_token
            && (token_count >= 2 || (token_count == 1 && short_query.chars().count() <= 12)));

    let docs = if use_fuzzy {
        let mut merged: HashMap<DocAddress, f32> = HashMap::new();
        let fuzzy_tokens: Vec<&str> = if query_tokens.is_empty() {
            vec![short_query.as_str()]
        } else {
            query_tokens.clone()
        };

        for (score, addr) in bm25_docs {
            merged
                .entry(addr)
                .and_modify(|s| *s = s.max(score))
                .or_insert(score);
        }

        for token in &fuzzy_tokens {
            if token.chars().count() <= 2 {
                continue;
            }

            let distance = fuzzy_distance(token);
            let safe_prefix = sanitize_prefix_pattern(token);

            let pname_term = Term::from_field_text(nix_index.fields.pname, token);
            let pname_q = FuzzyTermQuery::new(pname_term, distance, true);
            for (score, addr) in
                searcher.search(&pname_q, &TopDocs::with_limit(limit * 2).order_by_score())?
            {
                merged
                    .entry(addr)
                    .and_modify(|s| *s = s.max(score))
                    .or_insert(score);
            }

            let attr_term = Term::from_field_text(nix_index.fields.attr_path_text, token);
            let attr_q = FuzzyTermQuery::new(attr_term, distance, true);
            for (score, addr) in
                searcher.search(&attr_q, &TopDocs::with_limit(limit * 2).order_by_score())?
            {
                merged
                    .entry(addr)
                    .and_modify(|s| *s = s.max(score))
                    .or_insert(score);
            }

            if let Some(prefix) = &safe_prefix {
                let pattern = format!("{}.*", prefix);
                if let Ok(q) = RegexQuery::from_pattern(&pattern, nix_index.fields.pname) {
                    for (score, addr) in
                        searcher.search(&q, &TopDocs::with_limit(limit * 2).order_by_score())?
                    {
                        merged
                            .entry(addr)
                            .and_modify(|s| *s = s.max(score))
                            .or_insert(score);
                    }
                }
                if let Ok(q) = RegexQuery::from_pattern(&pattern, nix_index.fields.attr_path_text) {
                    for (score, addr) in
                        searcher.search(&q, &TopDocs::with_limit(limit * 2).order_by_score())?
                    {
                        merged
                            .entry(addr)
                            .and_modify(|s| *s = s.max(score))
                            .or_insert(score);
                    }
                }

                if let Some(subseq_pattern) = subsequence_pattern(prefix) {
                    if let Ok(q) = RegexQuery::from_pattern(&subseq_pattern, nix_index.fields.pname)
                    {
                        for (score, addr) in
                            searcher.search(&q, &TopDocs::with_limit(limit * 2).order_by_score())?
                        {
                            merged
                                .entry(addr)
                                .and_modify(|s| *s = s.max(score))
                                .or_insert(score);
                        }
                    }
                    if let Ok(q) =
                        RegexQuery::from_pattern(&subseq_pattern, nix_index.fields.attr_path_text)
                    {
                        for (score, addr) in
                            searcher.search(&q, &TopDocs::with_limit(limit * 2).order_by_score())?
                        {
                            merged
                                .entry(addr)
                                .and_modify(|s| *s = s.max(score))
                                .or_insert(score);
                        }
                    }
                }
            }
        }

        let mut docs: Vec<(f32, DocAddress)> = merged.into_iter().map(|(a, s)| (s, a)).collect();
        docs.sort_by(|a, b| {
            b.0.total_cmp(&a.0)
                .then_with(|| a.1.segment_ord.cmp(&b.1.segment_ord))
                .then_with(|| a.1.doc_id.cmp(&b.1.doc_id))
        });
        docs.truncate(limit);
        docs
    } else {
        bm25_docs
    };

    let mut scored = to_scored(&searcher, docs, nix_index)?;
    nix_search_core::search::rerank_with_prefix_bonus(&mut scored, query_str);
    Ok(scored)
}

fn to_scored(
    searcher: &tantivy::Searcher,
    docs: Vec<(f32, DocAddress)>,
    nix_index: &NixIndex,
) -> anyhow::Result<Vec<ScoredPackage>> {
    let mut out = Vec::with_capacity(docs.len());
    for (score, addr) in docs {
        let doc: tantivy::TantivyDocument = searcher.doc(addr)?;
        out.push(ScoredPackage {
            score,
            package: doc_to_package(&doc, &nix_index.fields),
        });
    }
    Ok(out)
}

fn normalize_exact_attr(query: &str) -> Option<String> {
    let q = query.trim();
    if q.is_empty() {
        return None;
    }
    if let Some((_, attr)) = q.split_once('#')
        && !attr.is_empty()
    {
        return Some(attr.to_string());
    }
    None
}

fn fuzzy_distance(token: &str) -> u8 {
    if token.chars().count() >= 5 { 2 } else { 1 }
}

fn sanitize_prefix_pattern(token: &str) -> Option<String> {
    let s: String = token
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>()
        .to_lowercase();
    if s.is_empty() { None } else { Some(s) }
}

fn subsequence_pattern(safe_token: &str) -> Option<String> {
    if safe_token.chars().count() < 3 {
        return None;
    }

    let mut out = String::new();
    for (i, c) in safe_token.chars().enumerate() {
        if i > 0 {
            out.push_str(".*");
        }
        out.push(c);
    }
    out.push_str(".*");
    Some(out)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::cache::index;
    use crate::search::{self, SearchConfig};
    use crate::types::Package;

    #[test]
    fn multi_token_fuzzy_matches_typo_phrase() {
        let dir = tempdir().expect("tempdir");
        let index_dir = dir.path().join("index");
        let pkgs = vec![
            Package {
                attr_path: "claude-code".to_string(),
                pname: "claude-code".to_string(),
                version: "1.0".to_string(),
                description: "Anthropic Claude code assistant".to_string(),
                platforms: vec!["x86_64-linux".to_string()],
            },
            Package {
                attr_path: "ripgrep".to_string(),
                pname: "ripgrep".to_string(),
                version: "14.0".to_string(),
                description: "line-oriented search tool".to_string(),
                platforms: vec!["x86_64-linux".to_string()],
            },
        ];

        index::build(&index_dir, &pkgs).expect("build index");
        let nix_index = index::open_or_create(&index_dir).expect("open index");

        let cfg = SearchConfig {
            query: "claud cod".to_string(),
            platform: Some("x86_64-linux".to_string()),
            limit: 10,
            exact_attr: None,
        };

        let results = search::search(&nix_index, &cfg).expect("search");
        let top = results
            .matched
            .first()
            .expect("expected at least one matched result");
        assert_eq!(top.package.attr_path, "claude-code");
    }

    #[test]
    fn single_token_partial_finds_ripgrep() {
        let dir = tempdir().expect("tempdir");
        let index_dir = dir.path().join("index");
        let pkgs = vec![
            Package {
                attr_path: "ripgrep".to_string(),
                pname: "ripgrep".to_string(),
                version: "14.0".to_string(),
                description: "fast text search".to_string(),
                platforms: vec!["x86_64-linux".to_string()],
            },
            Package {
                attr_path: "vimplugins.ripgrep-helper".to_string(),
                pname: "ripgrep-helper".to_string(),
                version: "1.0".to_string(),
                description: "vim plugin integration".to_string(),
                platforms: vec!["x86_64-linux".to_string()],
            },
        ];

        index::build(&index_dir, &pkgs).expect("build index");
        let nix_index = index::open_or_create(&index_dir).expect("open index");

        let cfg = SearchConfig {
            query: "rip".to_string(),
            platform: Some("x86_64-linux".to_string()),
            limit: 10,
            exact_attr: None,
        };

        let results = search::search(&nix_index, &cfg).expect("search");
        let top = results.matched.first().expect("expected one result");
        assert_eq!(top.package.attr_path, "ripgrep");
    }

    #[test]
    fn single_token_typo_distance_two_finds_ripgrep() {
        let dir = tempdir().expect("tempdir");
        let index_dir = dir.path().join("index");
        let pkgs = vec![Package {
            attr_path: "ripgrep".to_string(),
            pname: "ripgrep".to_string(),
            version: "14.0".to_string(),
            description: "fast text search".to_string(),
            platforms: vec!["x86_64-linux".to_string()],
        }];

        index::build(&index_dir, &pkgs).expect("build index");
        let nix_index = index::open_or_create(&index_dir).expect("open index");

        let cfg = SearchConfig {
            query: "ripgr".to_string(),
            platform: Some("x86_64-linux".to_string()),
            limit: 10,
            exact_attr: None,
        };

        let results = search::search(&nix_index, &cfg).expect("search");
        let top = results.matched.first().expect("expected one result");
        assert_eq!(top.package.attr_path, "ripgrep");
    }

    #[test]
    fn multi_token_typo_prefers_claude_over_unrelated_code_hits() {
        let dir = tempdir().expect("tempdir");
        let index_dir = dir.path().join("index");
        let pkgs = vec![
            Package {
                attr_path: "haskellPackages.codeworld-api".to_string(),
                pname: "codeworld-api".to_string(),
                version: "1.0".to_string(),
                description: "Haskell package for code samples".to_string(),
                platforms: vec!["x86_64-linux".to_string()],
            },
            Package {
                attr_path: "claude-code".to_string(),
                pname: "claude-code".to_string(),
                version: "1.0".to_string(),
                description: "Anthropic Claude code assistant".to_string(),
                platforms: vec!["x86_64-linux".to_string()],
            },
        ];

        index::build(&index_dir, &pkgs).expect("build index");
        let nix_index = index::open_or_create(&index_dir).expect("open index");

        let cfg = SearchConfig {
            query: "claud cod".to_string(),
            platform: Some("x86_64-linux".to_string()),
            limit: 10,
            exact_attr: None,
        };

        let results = search::search(&nix_index, &cfg).expect("search");
        let top = results.matched.first().expect("expected one result");
        assert_eq!(top.package.attr_path, "claude-code");
    }

    #[test]
    fn compressed_typo_prefers_claude_over_codesnap() {
        let dir = tempdir().expect("tempdir");
        let index_dir = dir.path().join("index");
        let pkgs = vec![
            Package {
                attr_path: "codesnap".to_string(),
                pname: "codesnap".to_string(),
                version: "1.0".to_string(),
                description: "Code screenshot utility".to_string(),
                platforms: vec!["x86_64-linux".to_string()],
            },
            Package {
                attr_path: "claude-code".to_string(),
                pname: "claude-code".to_string(),
                version: "1.0".to_string(),
                description: "Anthropic Claude code assistant".to_string(),
                platforms: vec!["x86_64-linux".to_string()],
            },
        ];

        index::build(&index_dir, &pkgs).expect("build index");
        let nix_index = index::open_or_create(&index_dir).expect("open index");

        let cfg = SearchConfig {
            query: "cld cod".to_string(),
            platform: Some("x86_64-linux".to_string()),
            limit: 10,
            exact_attr: None,
        };

        let results = search::search(&nix_index, &cfg).expect("search");
        let top = results.matched.first().expect("expected one result");
        assert_eq!(top.package.attr_path, "claude-code");
    }

    #[test]
    fn user_example_cld_cod_prefers_claude_over_cl_collider() {
        let dir = tempdir().expect("tempdir");
        let index_dir = dir.path().join("index");
        let pkgs = vec![
            Package {
                attr_path: "sbclPackages.cl-collider".to_string(),
                pname: "cl-collider".to_string(),
                version: "1.0".to_string(),
                description: "Common Lisp audio package".to_string(),
                platforms: vec!["x86_64-linux".to_string()],
            },
            Package {
                attr_path: "claude-code".to_string(),
                pname: "claude-code".to_string(),
                version: "1.0".to_string(),
                description: "Anthropic Claude code assistant".to_string(),
                platforms: vec!["x86_64-linux".to_string()],
            },
        ];

        index::build(&index_dir, &pkgs).expect("build index");
        let nix_index = index::open_or_create(&index_dir).expect("open index");

        let cfg = SearchConfig {
            query: "cld cod".to_string(),
            platform: Some("x86_64-linux".to_string()),
            limit: 10,
            exact_attr: None,
        };

        let results = search::search(&nix_index, &cfg).expect("search");
        let top = results.matched.first().expect("expected one result");
        assert_eq!(top.package.attr_path, "claude-code");
    }

    #[test]
    fn user_example_cld_co_prefers_claude_over_coloredlogs() {
        let dir = tempdir().expect("tempdir");
        let index_dir = dir.path().join("index");
        let pkgs = vec![
            Package {
                attr_path: "python314Packages.coloredlogs".to_string(),
                pname: "coloredlogs".to_string(),
                version: "1.0".to_string(),
                description: "Python colored logging".to_string(),
                platforms: vec!["x86_64-linux".to_string()],
            },
            Package {
                attr_path: "claude-code".to_string(),
                pname: "claude-code".to_string(),
                version: "1.0".to_string(),
                description: "Anthropic Claude code assistant".to_string(),
                platforms: vec!["x86_64-linux".to_string()],
            },
        ];

        index::build(&index_dir, &pkgs).expect("build index");
        let nix_index = index::open_or_create(&index_dir).expect("open index");

        let cfg = SearchConfig {
            query: "cld co".to_string(),
            platform: Some("x86_64-linux".to_string()),
            limit: 10,
            exact_attr: None,
        };

        let results = search::search(&nix_index, &cfg).expect("search");
        let top = results.matched.first().expect("expected one result");
        assert_eq!(top.package.attr_path, "claude-code");
    }

    #[test]
    fn user_example_cld_cod_prefers_claude_over_code_family() {
        let dir = tempdir().expect("tempdir");
        let index_dir = dir.path().join("index");
        let pkgs = vec![
            Package {
                attr_path: "code-theme-converter".to_string(),
                pname: "code-theme-converter".to_string(),
                version: "1.0".to_string(),
                description: "Theme converter for code editors".to_string(),
                platforms: vec!["x86_64-linux".to_string()],
            },
            Package {
                attr_path: "code-cursor-fhs".to_string(),
                pname: "code-cursor-fhs".to_string(),
                version: "1.0".to_string(),
                description: "Cursor wrapper in FHS environment".to_string(),
                platforms: vec!["x86_64-linux".to_string()],
            },
            Package {
                attr_path: "cld2".to_string(),
                pname: "cld2".to_string(),
                version: "1.0".to_string(),
                description: "Compact language detector".to_string(),
                platforms: vec!["x86_64-linux".to_string()],
            },
            Package {
                attr_path: "claude-code".to_string(),
                pname: "claude-code".to_string(),
                version: "1.0".to_string(),
                description: "Anthropic Claude code assistant".to_string(),
                platforms: vec!["x86_64-linux".to_string()],
            },
        ];

        index::build(&index_dir, &pkgs).expect("build index");
        let nix_index = index::open_or_create(&index_dir).expect("open index");

        let cfg = SearchConfig {
            query: "cld cod".to_string(),
            platform: Some("x86_64-linux".to_string()),
            limit: 20,
            exact_attr: None,
        };

        let results = search::search(&nix_index, &cfg).expect("search");
        let top = results.matched.first().expect("expected one result");
        assert_eq!(top.package.attr_path, "claude-code");
    }

    #[test]
    fn user_example_cld_cod_prefers_root_package_over_namespaced_variants() {
        let dir = tempdir().expect("tempdir");
        let index_dir = dir.path().join("index");
        let pkgs = vec![
            Package {
                attr_path: "gnomeExtensions.claude-code-usage-indicator".to_string(),
                pname: "claude-code-usage-indicator".to_string(),
                version: "1.0".to_string(),
                description: "Panel indicator for Claude Code".to_string(),
                platforms: vec!["x86_64-linux".to_string()],
            },
            Package {
                attr_path: "vscode-extensions.anthropic.claude-code".to_string(),
                pname: "claude-code".to_string(),
                version: "1.0".to_string(),
                description: "VSCode extension".to_string(),
                platforms: vec!["x86_64-linux".to_string()],
            },
            Package {
                attr_path: "claude-code-bin".to_string(),
                pname: "claude-code-bin".to_string(),
                version: "1.0".to_string(),
                description: "Binary package for Claude Code".to_string(),
                platforms: vec!["x86_64-linux".to_string()],
            },
            Package {
                attr_path: "claude-code".to_string(),
                pname: "claude-code".to_string(),
                version: "1.0".to_string(),
                description: "Anthropic Claude code assistant".to_string(),
                platforms: vec!["x86_64-linux".to_string()],
            },
        ];

        index::build(&index_dir, &pkgs).expect("build index");
        let nix_index = index::open_or_create(&index_dir).expect("open index");

        let cfg = SearchConfig {
            query: "cld cod".to_string(),
            platform: Some("x86_64-linux".to_string()),
            limit: 20,
            exact_attr: None,
        };

        let results = search::search(&nix_index, &cfg).expect("search");
        let top = results.matched.first().expect("expected one result");
        assert_eq!(top.package.attr_path, "claude-code");
    }

    #[test]
    fn user_example_claude_co_prefers_root_package_over_bin_variant() {
        let dir = tempdir().expect("tempdir");
        let index_dir = dir.path().join("index");
        let pkgs = vec![
            Package {
                attr_path: "claude-code-bin".to_string(),
                pname: "claude-code-bin".to_string(),
                version: "1.0".to_string(),
                description: "Binary package variant".to_string(),
                platforms: vec!["aarch64-darwin".to_string()],
            },
            Package {
                attr_path: "gnomeExtensions.claude-code-usage".to_string(),
                pname: "claude-code-usage".to_string(),
                version: "1.0".to_string(),
                description: "GNOME extension".to_string(),
                platforms: vec!["aarch64-darwin".to_string()],
            },
            Package {
                attr_path: "claude-code".to_string(),
                pname: "claude-code".to_string(),
                version: "1.0".to_string(),
                description: "Anthropic Claude code assistant".to_string(),
                platforms: vec!["aarch64-darwin".to_string()],
            },
        ];

        index::build(&index_dir, &pkgs).expect("build index");
        let nix_index = index::open_or_create(&index_dir).expect("open index");

        let cfg = SearchConfig {
            query: "claude co".to_string(),
            platform: Some("aarch64-darwin".to_string()),
            limit: 20,
            exact_attr: None,
        };

        let results = search::search(&nix_index, &cfg).expect("search");
        let top = results
            .matched
            .first()
            .or(results.others.first())
            .expect("expected one result");
        assert_eq!(top.package.attr_path, "claude-code");
    }
}
