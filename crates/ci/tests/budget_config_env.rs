use ci::pipelines::PerfMode;
use ci::steps::budgets::BudgetConfig;

#[test]
fn parses_quick_defaults_from_env() {
    let config = BudgetConfig::from_env_with(|_| None).expect("config parses");

    assert_eq!(config.perf_mode, PerfMode::Quick);
    assert!(!config.run_trunk_build);
    assert!(!config.run_latency_probe);
    assert_eq!(config.data_brotli_quality, 4);
    assert_eq!(config.search_p95_fail_ms, 0);
}

#[test]
fn parses_full_defaults_from_env() {
    let config = BudgetConfig::from_env_with(|key| match key {
        "PERF_MODE" => Some("full".to_owned()),
        _ => None,
    })
    .expect("config parses");

    assert_eq!(config.perf_mode, PerfMode::Full);
    assert!(config.run_trunk_build);
    assert!(config.run_latency_probe);
    assert_eq!(config.data_brotli_quality, 9);
    assert_eq!(config.search_p95_fail_ms, 120);
}

#[test]
fn honors_explicit_overrides() {
    let config = BudgetConfig::from_env_with(|key| match key {
        "PERF_MODE" => Some("quick".to_owned()),
        "RUN_TRUNK_BUILD" => Some("1".to_owned()),
        "RUN_LATENCY_PROBE" => Some("0".to_owned()),
        "DATA_BROTLI_QUALITY" => Some("7".to_owned()),
        "SEARCH_P95_WARN_MS" => Some("45".to_owned()),
        "SEARCH_P95_FAIL_MS" => Some("120".to_owned()),
        _ => None,
    })
    .expect("config parses");

    assert!(config.run_trunk_build);
    assert!(!config.run_latency_probe);
    assert_eq!(config.data_brotli_quality, 7);
    assert_eq!(config.search_p95_warn_ms, 45);
    assert_eq!(config.search_p95_fail_ms, 120);
}

#[test]
fn rejects_invalid_perf_mode() {
    let error = BudgetConfig::from_env_with(|key| match key {
        "PERF_MODE" => Some("nope".to_owned()),
        _ => None,
    })
    .expect_err("invalid perf mode should fail");

    assert!(error.to_string().contains("invalid PERF_MODE"));
}