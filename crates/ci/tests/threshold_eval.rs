use ci::pipelines::PerfMode;
use ci::steps::budgets::{BudgetConfig, LatencyMetrics, SizeMeasurement, SizeMeasurements, evaluate_thresholds};

#[test]
fn quick_mode_warns_on_data_budget_only() {
    let config = BudgetConfig::for_perf_mode(PerfMode::Quick);
    let report = evaluate_thresholds(&config, &sample_sizes(), &LatencyMetrics::disabled());

    assert!(report.failures.is_empty());
    assert_eq!(report.warnings.len(), 1);
    assert!(report.warnings[0].contains("data brotli budget is informational"));
}

#[test]
fn full_mode_fails_on_exact_threshold_breaches() {
    let mut config = BudgetConfig::for_perf_mode(PerfMode::Full);
    config.wasm_max_raw = 10;
    config.wasm_max_brotli = 11;
    config.js_max_raw = 12;
    config.js_max_brotli = 13;
    config.data_max_brotli = 14;

    let sizes = SizeMeasurements {
        wasm: SizeMeasurement { raw_bytes: 11, brotli_bytes: 12 },
        js: SizeMeasurement { raw_bytes: 13, brotli_bytes: 14 },
        data: SizeMeasurement { raw_bytes: 15, brotli_bytes: 15 },
    };

    let latency = LatencyMetrics {
        enabled: true,
        startup_read_ms: Some(0.0),
        startup_hydrate_ms: Some(0.0),
        search_avg_ms: Some(0.0),
        search_p50_ms: Some(0.0),
        search_p95_ms: Some(121.0),
        search_p99_ms: Some(0.0),
    };

    let report = evaluate_thresholds(&config, &sizes, &latency);

    assert_eq!(report.failures.len(), 6);
    assert!(report.failures.iter().any(|line| line.contains("wasm raw")));
    assert!(report.failures.iter().any(|line| line.contains("search_p95_ms")));
}

#[test]
fn fail_threshold_zero_disables_latency_failure() {
    let mut config = BudgetConfig::for_perf_mode(PerfMode::Quick);
    config.run_latency_probe = true;
    config.search_p95_fail_ms = 0;
    let mut latency = LatencyMetrics::stub();
    latency.search_p95_ms = Some(999.0);

    let report = evaluate_thresholds(&config, &sample_sizes(), &latency);

    assert!(report.failures.is_empty());
}

fn sample_sizes() -> SizeMeasurements {
    SizeMeasurements {
        wasm: SizeMeasurement { raw_bytes: 1, brotli_bytes: 1 },
        js: SizeMeasurement { raw_bytes: 1, brotli_bytes: 1 },
        data: SizeMeasurement { raw_bytes: 1, brotli_bytes: 1 },
    }
}