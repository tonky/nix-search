use ci::pipelines::PerfMode;
use ci::steps::budgets::BudgetConfig;

#[test]
fn defaults_match_shell_quick_mode() {
    let config = BudgetConfig::for_perf_mode(PerfMode::Quick);

    assert_eq!(config.wasm_max_raw, 4_500_000);
    assert_eq!(config.wasm_max_brotli, 800_000);
    assert_eq!(config.js_max_raw, 100_000);
    assert_eq!(config.js_max_brotli, 12_000);
    assert_eq!(config.data_max_brotli, 2_600_000);
    assert_eq!(config.brotli_quality, 9);
    assert_eq!(config.data_brotli_quality, 4);
    assert_eq!(config.search_p95_warn_ms, 40);
    assert_eq!(config.search_p95_fail_ms, 0);
    assert!(!config.run_trunk_build);
    assert!(!config.run_latency_probe);
}

#[test]
fn defaults_match_shell_full_mode() {
    let config = BudgetConfig::for_perf_mode(PerfMode::Full);

    assert!(config.run_trunk_build);
    assert!(config.run_latency_probe);
    assert_eq!(config.data_brotli_quality, 9);
    assert_eq!(config.search_p95_fail_ms, 120);
}