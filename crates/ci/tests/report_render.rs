use ci::pipelines::PerfMode;
use ci::steps::budgets::{BudgetConfig, BudgetReport, LatencyMetrics, SizeMeasurement, SizeMeasurements};

#[test]
fn renders_markdown_report() {
    let report = BudgetReport {
        config: BudgetConfig::for_perf_mode(PerfMode::Quick),
        sizes: SizeMeasurements {
            wasm: SizeMeasurement { raw_bytes: 665_164, brotli_bytes: 194_449 },
            js: SizeMeasurement { raw_bytes: 54_688, brotli_bytes: 7_899 },
            data: SizeMeasurement { raw_bytes: 24_913_134, brotli_bytes: 2_247_291 },
        },
        latency: LatencyMetrics::disabled(),
        warnings: vec!["WARN data brotli budget is informational in quick mode (measured with q=4)".to_owned()],
        failures: Vec::new(),
        notes: vec!["latency probe skipped".to_owned()],
    };

    insta::assert_snapshot!(report.to_markdown());
}