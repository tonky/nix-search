use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use nix_search_core::search::{
    ScoredPackage, SearchConfig, apply_global_limit, compute_overfetch_limit, rerank_with_prefix_bonus,
};
use nix_search_core::types::Package;
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(name = "latency-probe")]
#[command(about = "Measure startup and query latency over prepared web artifact")]
struct Cli {
    #[arg(long)]
    artifact: PathBuf,

    #[arg(long, default_value = "50")]
    iterations: usize,
}

#[derive(Debug, Deserialize)]
struct PreparedData {
    packages: Vec<Package>,
}

#[derive(Clone)]
struct Row {
    pkg: Package,
    attr_lc: String,
    pname_lc: String,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let start_read = Instant::now();
    let bytes = fs::read(&cli.artifact)?;
    let parsed: PreparedData = serde_json::from_slice(&bytes)?;
    let read_ms = start_read.elapsed().as_secs_f64() * 1000.0;

    let start_hydrate = Instant::now();
    let rows = parsed
        .packages
        .into_iter()
        .map(|pkg| Row {
            attr_lc: pkg.attr_path.to_lowercase(),
            pname_lc: pkg.pname.to_lowercase(),
            pkg,
        })
        .collect::<Vec<_>>();
    let hydrate_ms = start_hydrate.elapsed().as_secs_f64() * 1000.0;

    let queries = vec![
        "claude code",
        "claud cod",
        "cld cod",
        "rip",
        "rust analyzer",
        "zzzz-not-a-real-package",
    ];

    let mut timings = Vec::new();
    for _ in 0..cli.iterations {
        for q in &queries {
            let start = Instant::now();
            let _ = run_search(&rows, q, Some("x86_64-linux"), false, 120);
            timings.push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }

    timings.sort_by(|a, b| a.total_cmp(b));
    let avg = timings.iter().sum::<f64>() / timings.len().max(1) as f64;
    let p50 = percentile(&timings, 0.50);
    let p95 = percentile(&timings, 0.95);
    let p99 = percentile(&timings, 0.99);

    println!("artifact={}", cli.artifact.display());
    println!("rows={}", rows.len());
    println!("startup_read_ms={:.2}", read_ms);
    println!("startup_hydrate_ms={:.2}", hydrate_ms);
    println!("search_avg_ms={:.2}", avg);
    println!("search_p50_ms={:.2}", p50);
    println!("search_p95_ms={:.2}", p95);
    println!("search_p99_ms={:.2}", p99);

    Ok(())
}

fn run_search(
    rows: &[Row],
    query: &str,
    selected_platform: Option<&str>,
    all_platforms: bool,
    limit: usize,
) -> nix_search_core::search::SearchResults {
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

    nix_search_core::search::SearchResults { matched, others }
}

fn percentile(samples: &[f64], p: f64) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let idx = ((samples.len() - 1) as f64 * p).round() as usize;
    samples[idx.min(samples.len() - 1)]
}
