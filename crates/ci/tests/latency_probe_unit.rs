use ci::pipelines::PerfMode;
use ci::steps::budgets::{parse_latency_probe_output, BudgetConfig, BudgetReport, SizeMeasurement, SizeMeasurements};

#[test]
fn parses_latency_probe_output_into_report_fields() {
    let output = r#"artifact=/tmp/artifact
rows=142840
startup_read_ms=12.34
startup_hydrate_ms=56.78
search_avg_ms=9.87
search_p50_ms=8.76
search_p95_ms=7.65
search_p99_ms=6.54
"#;

    let latency = parse_latency_probe_output(output).expect("latency probe output parses");
    let report = BudgetReport {
        config: BudgetConfig::for_perf_mode(PerfMode::Full),
        sizes: SizeMeasurements {
            wasm: SizeMeasurement { raw_bytes: 1, brotli_bytes: 1 },
            js: SizeMeasurement { raw_bytes: 1, brotli_bytes: 1 },
            data: SizeMeasurement { raw_bytes: 1, brotli_bytes: 1 },
        },
        latency,
        warnings: Vec::new(),
        failures: Vec::new(),
        notes: Vec::new(),
    };

    assert_eq!(report.latency.enabled, true);
    assert_eq!(report.latency.startup_read_ms, Some(12.34));
    assert_eq!(report.latency.startup_hydrate_ms, Some(56.78));
    assert_eq!(report.latency.search_avg_ms, Some(9.87));
    assert_eq!(report.latency.search_p50_ms, Some(8.76));
    assert_eq!(report.latency.search_p95_ms, Some(7.65));
    assert_eq!(report.latency.search_p99_ms, Some(6.54));
    assert!(report.to_summary_text().contains("search_p99_ms=6.54"));
    assert!(report.to_markdown().contains("## Latency"));
}
