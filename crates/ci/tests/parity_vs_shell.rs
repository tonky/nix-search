use ci::pipelines::PerfMode;
use ci::steps::budgets::{parse_latency_probe_output, BudgetReport};
use ci::Manifest;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const LATENCY_TOLERANCE_MS: f64 = 3.0;

#[test]
#[ignore]
fn parity_vs_shell() {
    let repo_root = repo_root();
    let temp_root = tempfile::tempdir_in(repo_root.join("tmp")).expect("temp dir");
    let fixture_root = temp_root.path().join("fixture");
    seed_fixture(&repo_root, &fixture_root);

    let full_shell = temp_root.path().join("full-shell");
    let full_ci = temp_root.path().join("full-ci");
    let quick_shell = temp_root.path().join("quick-shell");
    let quick_ci = temp_root.path().join("quick-ci");

    run_shell_budget(&repo_root, PerfMode::Full, &full_shell, None);
    run_ci_budget(&repo_root, PerfMode::Full, &full_ci, None);
    assert_reports_match(&full_shell, &full_ci, PerfMode::Full);

    run_shell_budget(&repo_root, PerfMode::Quick, &quick_shell, Some("0"));
    run_ci_budget(&repo_root, PerfMode::Quick, &quick_ci, Some("0"));
    assert_reports_match(&quick_shell, &quick_ci, PerfMode::Quick);
}

fn assert_reports_match(shell_out: &Path, ci_out: &Path, perf_mode: PerfMode) {
    let shell_summary = parse_summary(&fs::read_to_string(shell_out.join("summary.txt")).expect("shell summary"));
    let report: BudgetReport = serde_json::from_str(&fs::read_to_string(ci_out.join("report.json")).expect("ci report json")).expect("ci report parses");
    let shell_budget_lines = fs::read_to_string(shell_out.join("03-budget-check.txt")).unwrap_or_default();

    assert_eq!(summary_value(&shell_summary, "mode"), perf_mode_name(perf_mode));
    assert_eq!(summary_value(&shell_summary, "run_trunk_build"), bool_flag(report.config.run_trunk_build));
    assert_eq!(summary_value(&shell_summary, "run_latency_probe"), bool_flag(report.config.run_latency_probe));
    assert_eq!(summary_value(&shell_summary, "failures"), report.failure_count().to_string());
    assert_eq!(summary_value(&shell_summary, "warnings"), report.warning_count().to_string());
    assert_eq!(summary_value(&shell_summary, "wasm_raw"), report.sizes.wasm.raw_bytes.to_string());
    assert_eq!(summary_value(&shell_summary, "js_raw"), report.sizes.js.raw_bytes.to_string());
    assert_eq!(summary_value(&shell_summary, "data_brotli_quality"), report.config.data_brotli_quality.to_string());

    assert_equal_bytes("wasm brotli", summary_value(&shell_summary, "wasm_brotli"), report.sizes.wasm.brotli_bytes, 0.01);
    assert_equal_bytes("js brotli", summary_value(&shell_summary, "js_brotli"), report.sizes.js.brotli_bytes, 0.01);
    assert_equal_bytes("data brotli", summary_value(&shell_summary, "data_brotli"), report.sizes.data.brotli_bytes, 0.01);

    let shell_budget_lines = shell_budget_lines.lines().filter(|line| !line.trim().is_empty()).map(str::to_owned).collect::<Vec<_>>();
    let report_budget_lines = report
        .warnings
        .iter()
        .chain(report.failures.iter())
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(shell_budget_lines, report_budget_lines);

    if perf_mode == PerfMode::Full {
        let shell_latency = parse_latency_probe_output(&fs::read_to_string(shell_out.join("04-latency-probe.log")).expect("shell latency log"))
            .expect("shell latency log parses");
        assert_eq!(report.latency.enabled, true);
        assert_approx("startup_read_ms", shell_latency.startup_read_ms, report.latency.startup_read_ms, LATENCY_TOLERANCE_MS);
        assert_approx("startup_hydrate_ms", shell_latency.startup_hydrate_ms, report.latency.startup_hydrate_ms, LATENCY_TOLERANCE_MS);
        assert_approx("search_avg_ms", shell_latency.search_avg_ms, report.latency.search_avg_ms, LATENCY_TOLERANCE_MS);
        assert_approx("search_p50_ms", shell_latency.search_p50_ms, report.latency.search_p50_ms, LATENCY_TOLERANCE_MS);
        assert_approx("search_p95_ms", shell_latency.search_p95_ms, report.latency.search_p95_ms, LATENCY_TOLERANCE_MS);
        assert_approx("search_p99_ms", shell_latency.search_p99_ms, report.latency.search_p99_ms, LATENCY_TOLERANCE_MS);
    } else {
        let shell_latency_log = fs::read_to_string(shell_out.join("04-latency-probe.log")).expect("shell latency log");
        assert!(shell_latency_log.contains("skipped latency probe"));
        assert_eq!(report.latency.enabled, false);
    }
}

fn run_shell_budget(repo_root: &Path, perf_mode: PerfMode, out_dir: &Path, run_trunk_build: Option<&str>) {
    let mut command = Command::new("/bin/bash");
    command.arg(repo_root.join("scripts/perf/check_budgets.sh"));
    command.arg(out_dir);
    command.current_dir(repo_root);
    command.env("PERF_MODE", perf_mode_name(perf_mode));
    if let Some(value) = run_trunk_build {
        command.env("RUN_TRUNK_BUILD", value);
    }
    let output = command.output().expect("run shell budget");
    assert!(output.status.success(), "shell budget failed\nstdout:\n{}\nstderr:\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
}

fn run_ci_budget(repo_root: &Path, perf_mode: PerfMode, out_dir: &Path, run_trunk_build: Option<&str>) {
    let mut command = Command::new("cargo");
    command.args(["run", "-p", "ci", "--", "budget", "--perf-mode", perf_mode_name(perf_mode), "--out"]);
    command.arg(out_dir);
    command.current_dir(repo_root);
    if let Some(value) = run_trunk_build {
        command.env("RUN_TRUNK_BUILD", value);
    }
    let output = command.output().expect("run ci budget");
    assert!(output.status.success(), "ci budget failed\nstdout:\n{}\nstderr:\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
}

fn seed_fixture(repo_root: &Path, fixture_root: &Path) {
    fs::create_dir_all(fixture_root).expect("create fixture root");
    let source = repo_root.join("tmp/pages-data");
    let manifest: Manifest = serde_json::from_str(&fs::read_to_string(source.join("manifest.json")).expect("read manifest fixture")).expect("manifest parses");

    let fixture_data = fixture_root.join("crates/nix-search-web/static/data");
    fs::create_dir_all(&fixture_data).expect("create fixture data dir");
    copy_file(&source.join("manifest.json"), &fixture_data.join("manifest.json"));
    copy_file(&source.join(&manifest.artifact), &fixture_data.join(&manifest.artifact));
    if let Some(compressed) = manifest.compressed_artifact.as_ref() {
        copy_file(&source.join(compressed), &fixture_data.join(compressed));
    }

    seed_repo_static_data(repo_root, &fixture_data);
}

fn seed_repo_static_data(repo_root: &Path, fixture_data: &Path) {
    let repo_data = repo_root.join("crates/nix-search-web/static/data");
    fs::create_dir_all(&repo_data).expect("create repo static data dir");
    for entry in fs::read_dir(&repo_data).expect("read repo static data dir") {
        let entry = entry.expect("static data entry");
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("packages-") {
            let _ = fs::remove_file(entry.path());
        }
    }

    copy_file(&fixture_data.join("manifest.json"), &repo_data.join("manifest.json"));
    let manifest: Manifest = serde_json::from_str(&fs::read_to_string(fixture_data.join("manifest.json")).expect("read fixture manifest")).expect("fixture manifest parses");
    copy_file(&fixture_data.join(&manifest.artifact), &repo_data.join(&manifest.artifact));
    if let Some(compressed) = manifest.compressed_artifact.as_ref() {
        copy_file(&fixture_data.join(compressed), &repo_data.join(compressed));
    }
}

fn assert_equal_bytes(label: &str, summary_value: String, report_value: u64, tolerance_ratio: f64) {
    let shell_value = summary_value.parse::<u64>().expect(label);
    if shell_value == report_value {
        return;
    }

    let delta = shell_value.abs_diff(report_value);
    let max_delta = ((shell_value.max(report_value) as f64) * tolerance_ratio).ceil() as u64;
    assert!(
        delta <= max_delta,
        "{label} drifted beyond tolerance: shell={shell_value} ci={report_value} delta={delta} max_delta={max_delta}"
    );
}

fn assert_approx(label: &str, shell_value: Option<f64>, report_value: Option<f64>, tolerance_ms: f64) {
    let shell_value = shell_value.unwrap_or_else(|| panic!("missing shell value for {label}"));
    let report_value = report_value.unwrap_or_else(|| panic!("missing ci value for {label}"));
    let delta = (shell_value - report_value).abs();
    assert!(delta <= tolerance_ms, "{label} drifted beyond tolerance: shell={shell_value:.2} ci={report_value:.2} delta={delta:.2} tolerance={tolerance_ms:.2}");
}

fn parse_summary(contents: &str) -> HashMap<String, String> {
    let mut values = HashMap::new();
    for token in contents.split_whitespace() {
        if let Some((key, value)) = token.split_once('=') {
            values.insert(key.trim_matches(|ch: char| ch == ',' || ch == ':').to_owned(), value.trim_matches(',').to_owned());
        }
    }
    values
}

fn summary_value(values: &HashMap<String, String>, key: &str) -> String {
    values.get(key).cloned().unwrap_or_else(|| panic!("missing summary value for {key}"))
}

fn perf_mode_name(perf_mode: PerfMode) -> &'static str {
    match perf_mode {
        PerfMode::Quick => "quick",
        PerfMode::Full => "full",
    }
}

fn bool_flag(value: bool) -> &'static str {
    if value { "1" } else { "0" }
}

fn copy_file(source: &Path, destination: &Path) {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    fs::copy(source, destination).expect("copy fixture file");
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}
