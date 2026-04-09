use crate::env;
use crate::pipelines::{BudgetContext, PerfMode};
use crate::shell::{render_command, CommandSpec, Shell};
use crate::Manifest;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_WASM_MAX_RAW: u64 = 4_500_000;
const DEFAULT_WASM_MAX_BROTLI: u64 = 800_000;
const DEFAULT_JS_MAX_RAW: u64 = 100_000;
const DEFAULT_JS_MAX_BROTLI: u64 = 12_000;
const DEFAULT_DATA_MAX_BROTLI: u64 = 2_600_000;
const DEFAULT_BROTLI_QUALITY: u32 = 9;
const DEFAULT_DATA_BROTLI_QUALITY_QUICK: u32 = 4;
const DEFAULT_DATA_BROTLI_QUALITY_FULL: u32 = 9;
const DEFAULT_SEARCH_P95_WARN_MS: u32 = 40;
const DEFAULT_SEARCH_P95_FAIL_MS_FULL: u32 = 120;
const DEFAULT_SEARCH_P95_FAIL_MS_QUICK: u32 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToggleSetting {
    Auto,
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetConfig {
    pub perf_mode: PerfMode,
    pub run_trunk_build: bool,
    pub run_latency_probe: bool,
    pub wasm_max_raw: u64,
    pub wasm_max_brotli: u64,
    pub js_max_raw: u64,
    pub js_max_brotli: u64,
    pub data_max_brotli: u64,
    pub brotli_quality: u32,
    pub data_brotli_quality: u32,
    pub search_p95_warn_ms: u32,
    pub search_p95_fail_ms: u32,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self::for_perf_mode(PerfMode::Quick)
    }
}

impl BudgetConfig {
    pub fn from_env() -> Result<Self> {
        Self::from_env_with(|key| std::env::var(key).ok())
    }

    pub fn from_env_with<F>(get: F) -> Result<Self>
    where
        F: Fn(&str) -> Option<String>,
    {
        let perf_mode = match get("PERF_MODE").as_deref() {
            Some("quick") | None => PerfMode::Quick,
            Some("full") => PerfMode::Full,
            Some(other) => bail!("invalid PERF_MODE: {other} (expected quick|full)"),
        };
        Self::from_env_with_perf_mode(perf_mode, get)
    }

    pub fn from_env_with_perf_mode<F>(perf_mode: PerfMode, get: F) -> Result<Self>
    where
        F: Fn(&str) -> Option<String>,
    {
        let run_trunk_build = parse_toggle(get("RUN_TRUNK_BUILD").as_deref(), perf_mode, false)?;
        let run_latency_probe = parse_toggle(get("RUN_LATENCY_PROBE").as_deref(), perf_mode, false)?;
        let data_brotli_quality = parse_u32(get("DATA_BROTLI_QUALITY").as_deref()).unwrap_or_else(|| match perf_mode {
            PerfMode::Quick => DEFAULT_DATA_BROTLI_QUALITY_QUICK,
            PerfMode::Full => DEFAULT_DATA_BROTLI_QUALITY_FULL,
        });
        let search_p95_warn_ms = parse_u32(get("SEARCH_P95_WARN_MS").as_deref()).unwrap_or(DEFAULT_SEARCH_P95_WARN_MS);
        let search_p95_fail_ms = parse_u32(get("SEARCH_P95_FAIL_MS").as_deref()).unwrap_or(match perf_mode {
            PerfMode::Quick => DEFAULT_SEARCH_P95_FAIL_MS_QUICK,
            PerfMode::Full => DEFAULT_SEARCH_P95_FAIL_MS_FULL,
        });

        Ok(Self {
            perf_mode,
            run_trunk_build,
            run_latency_probe,
            wasm_max_raw: parse_u64(get("WASM_MAX_RAW").as_deref()).unwrap_or(DEFAULT_WASM_MAX_RAW),
            wasm_max_brotli: parse_u64(get("WASM_MAX_BROTLI").as_deref()).unwrap_or(DEFAULT_WASM_MAX_BROTLI),
            js_max_raw: parse_u64(get("JS_MAX_RAW").as_deref()).unwrap_or(DEFAULT_JS_MAX_RAW),
            js_max_brotli: parse_u64(get("JS_MAX_BROTLI").as_deref()).unwrap_or(DEFAULT_JS_MAX_BROTLI),
            data_max_brotli: parse_u64(get("DATA_MAX_BROTLI").as_deref()).unwrap_or(DEFAULT_DATA_MAX_BROTLI),
            brotli_quality: parse_u32(get("BROTLI_QUALITY").as_deref()).unwrap_or(DEFAULT_BROTLI_QUALITY),
            data_brotli_quality,
            search_p95_warn_ms,
            search_p95_fail_ms,
        })
    }

    pub fn for_perf_mode(perf_mode: PerfMode) -> Self {
        Self {
            perf_mode,
            run_trunk_build: matches!(perf_mode, PerfMode::Full),
            run_latency_probe: matches!(perf_mode, PerfMode::Full),
            wasm_max_raw: DEFAULT_WASM_MAX_RAW,
            wasm_max_brotli: DEFAULT_WASM_MAX_BROTLI,
            js_max_raw: DEFAULT_JS_MAX_RAW,
            js_max_brotli: DEFAULT_JS_MAX_BROTLI,
            data_max_brotli: DEFAULT_DATA_MAX_BROTLI,
            brotli_quality: DEFAULT_BROTLI_QUALITY,
            data_brotli_quality: match perf_mode {
                PerfMode::Quick => DEFAULT_DATA_BROTLI_QUALITY_QUICK,
                PerfMode::Full => DEFAULT_DATA_BROTLI_QUALITY_FULL,
            },
            search_p95_warn_ms: DEFAULT_SEARCH_P95_WARN_MS,
            search_p95_fail_ms: match perf_mode {
                PerfMode::Quick => DEFAULT_SEARCH_P95_FAIL_MS_QUICK,
                PerfMode::Full => DEFAULT_SEARCH_P95_FAIL_MS_FULL,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedInputs {
    pub manifest: Manifest,
    pub artifact_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SizeMeasurement {
    pub raw_bytes: u64,
    pub brotli_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SizeMeasurements {
    pub wasm: SizeMeasurement,
    pub js: SizeMeasurement,
    pub data: SizeMeasurement,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LatencyMetrics {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup_read_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup_hydrate_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_avg_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_p50_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_p95_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_p99_ms: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LatencyProbeRun {
    pub metrics: LatencyMetrics,
    pub raw_output: String,
}

impl LatencyMetrics {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            startup_read_ms: None,
            startup_hydrate_ms: None,
            search_avg_ms: None,
            search_p50_ms: None,
            search_p95_ms: None,
            search_p99_ms: None,
        }
    }

    pub fn stub() -> Self {
        Self {
            enabled: true,
            startup_read_ms: Some(0.0),
            startup_hydrate_ms: Some(0.0),
            search_avg_ms: Some(0.0),
            search_p50_ms: Some(0.0),
            search_p95_ms: Some(0.0),
            search_p99_ms: Some(0.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BudgetReport {
    pub config: BudgetConfig,
    pub sizes: SizeMeasurements,
    pub latency: LatencyMetrics,
    pub warnings: Vec<String>,
    pub failures: Vec<String>,
    pub notes: Vec<String>,
}

impl BudgetReport {
    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    pub fn failure_count(&self) -> usize {
        self.failures.len()
    }

    pub fn to_summary_text(&self) -> String {
        let mut lines = vec![
            format!("size budgets: failures={} warnings={}", self.failure_count(), self.warning_count()),
            format!(
                "mode={} run_trunk_build={} run_latency_probe={}",
                self.config.perf_mode.as_str(),
                bool_flag(self.config.run_trunk_build),
                bool_flag(self.config.run_latency_probe),
            ),
            format!(
                "wasm_raw={} wasm_brotli={}",
                self.sizes.wasm.raw_bytes, self.sizes.wasm.brotli_bytes
            ),
            format!(
                "js_raw={} js_brotli={}",
                self.sizes.js.raw_bytes, self.sizes.js.brotli_bytes
            ),
            format!(
                "data_brotli={} data_brotli_quality={}",
                self.sizes.data.brotli_bytes, self.config.data_brotli_quality
            ),
        ];

        if self.latency.enabled {
            if let Some(value) = self.latency.startup_read_ms {
                lines.push(format!("startup_read_ms={value:.2}"));
            }
            if let Some(value) = self.latency.startup_hydrate_ms {
                lines.push(format!("startup_hydrate_ms={value:.2}"));
            }
            if let Some(value) = self.latency.search_avg_ms {
                lines.push(format!("search_avg_ms={value:.2}"));
            }
            if let Some(value) = self.latency.search_p50_ms {
                lines.push(format!("search_p50_ms={value:.2}"));
            }
            if let Some(value) = self.latency.search_p95_ms {
                lines.push(format!("search_p95_ms={value:.2}"));
            }
            if let Some(value) = self.latency.search_p99_ms {
                lines.push(format!("search_p99_ms={value:.2}"));
            }
        }

        lines.push(format!(
            "search_p95_warn_ms={} search_p95_fail_ms={}",
            self.config.search_p95_warn_ms, self.config.search_p95_fail_ms
        ));
        lines.join("\n") + "\n"
    }

    pub fn to_markdown(&self) -> String {
        let mut markdown = String::new();
        markdown.push_str("# Perf Size Budget\n\n");
        markdown.push_str(&format!("- Mode: {}\n", self.config.perf_mode.as_str()));
        markdown.push_str(&format!("- Trunk build: {}\n", if self.config.run_trunk_build { "enabled" } else { "skipped" }));
        markdown.push_str(&format!("- Latency probe: {}\n", if self.latency.enabled { "enabled" } else { "skipped" }));
        markdown.push_str(&format!("- Failures: {}\n", self.failure_count()));
        markdown.push_str(&format!("- Warnings: {}\n", self.warning_count()));

        markdown.push_str("\n## Measurements\n\n");
        markdown.push_str("| Artifact | Raw bytes | Brotli bytes |\n| --- | ---: | ---: |\n");
        markdown.push_str(&format!("| wasm | {} | {} |\n", self.sizes.wasm.raw_bytes, self.sizes.wasm.brotli_bytes));
        markdown.push_str(&format!("| js | {} | {} |\n", self.sizes.js.raw_bytes, self.sizes.js.brotli_bytes));
        markdown.push_str(&format!("| data | {} | {} |\n", self.data_raw_bytes(), self.sizes.data.brotli_bytes));

        markdown.push_str("\n## Thresholds\n\n");
        markdown.push_str("| Metric | Value | Limit | Status |\n| --- | ---: | ---: | --- |\n");
        markdown.push_str(&format!("| wasm raw | {} | {} | {} |\n", self.sizes.wasm.raw_bytes, self.config.wasm_max_raw, status_for(self.sizes.wasm.raw_bytes > self.config.wasm_max_raw)));
        markdown.push_str(&format!("| wasm brotli | {} | {} | {} |\n", self.sizes.wasm.brotli_bytes, self.config.wasm_max_brotli, status_for(self.sizes.wasm.brotli_bytes > self.config.wasm_max_brotli)));
        markdown.push_str(&format!("| js raw | {} | {} | {} |\n", self.sizes.js.raw_bytes, self.config.js_max_raw, status_for(self.sizes.js.raw_bytes > self.config.js_max_raw)));
        markdown.push_str(&format!("| js brotli | {} | {} | {} |\n", self.sizes.js.brotli_bytes, self.config.js_max_brotli, status_for(self.sizes.js.brotli_bytes > self.config.js_max_brotli)));
        markdown.push_str(&format!("| data brotli | {} | {} | {} |\n", self.sizes.data.brotli_bytes, self.config.data_max_brotli, if self.config.perf_mode == PerfMode::Quick { "info" } else { status_for(self.sizes.data.brotli_bytes > self.config.data_max_brotli) }));

        if self.latency.enabled {
            markdown.push_str("\n## Latency\n\n");
            markdown.push_str("| Metric | Value |\n| --- | ---: |\n");
            if let Some(value) = self.latency.startup_read_ms {
                markdown.push_str(&format!("| startup read | {:.2} |\n", value));
            }
            if let Some(value) = self.latency.startup_hydrate_ms {
                markdown.push_str(&format!("| startup hydrate | {:.2} |\n", value));
            }
            if let Some(value) = self.latency.search_avg_ms {
                markdown.push_str(&format!("| search avg | {:.2} |\n", value));
            }
            if let Some(value) = self.latency.search_p50_ms {
                markdown.push_str(&format!("| search p50 | {:.2} |\n", value));
            }
            if let Some(value) = self.latency.search_p95_ms {
                markdown.push_str(&format!("| search p95 | {:.2} |\n", value));
            }
            if let Some(value) = self.latency.search_p99_ms {
                markdown.push_str(&format!("| search p99 | {:.2} |\n", value));
            }
        }

        if !self.notes.is_empty() {
            markdown.push_str("\n## Notes\n\n");
            for note in &self.notes {
                markdown.push_str(&format!("- {note}\n"));
            }
        }

        if !self.warnings.is_empty() {
            markdown.push_str("\n## Warnings\n\n");
            for warning in &self.warnings {
                markdown.push_str(&format!("- {warning}\n"));
            }
        }

        if !self.failures.is_empty() {
            markdown.push_str("\n## Failures\n\n");
            for failure in &self.failures {
                markdown.push_str(&format!("- {failure}\n"));
            }
        }

        markdown
    }

    fn data_raw_bytes(&self) -> u64 {
        self.sizes.data.raw_bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetEvaluation {
    pub warnings: Vec<String>,
    pub failures: Vec<String>,
    pub notes: Vec<String>,
}

pub fn evaluate_thresholds(config: &BudgetConfig, sizes: &SizeMeasurements, latency: &LatencyMetrics) -> BudgetEvaluation {
    let mut warnings = Vec::new();
    let mut failures = Vec::new();
    let mut notes = Vec::new();

    if sizes.wasm.raw_bytes > config.wasm_max_raw {
        failures.push(format!("FAIL wasm raw size {} > {}", sizes.wasm.raw_bytes, config.wasm_max_raw));
    }
    if sizes.wasm.brotli_bytes > config.wasm_max_brotli {
        failures.push(format!("FAIL wasm brotli size {} > {}", sizes.wasm.brotli_bytes, config.wasm_max_brotli));
    }
    if sizes.js.raw_bytes > config.js_max_raw {
        failures.push(format!("FAIL js raw size {} > {}", sizes.js.raw_bytes, config.js_max_raw));
    }
    if sizes.js.brotli_bytes > config.js_max_brotli {
        failures.push(format!("FAIL js brotli size {} > {}", sizes.js.brotli_bytes, config.js_max_brotli));
    }

    if config.perf_mode == PerfMode::Full {
        if sizes.data.brotli_bytes > config.data_max_brotli {
            failures.push(format!("FAIL data brotli size {} > {}", sizes.data.brotli_bytes, config.data_max_brotli));
        }
    } else {
        warnings.push(format!(
            "WARN data brotli budget is informational in quick mode (measured with q={})",
            config.data_brotli_quality
        ));
    }

    if latency.enabled {
        if let Some(search_p95_ms) = latency.search_p95_ms {
            if config.search_p95_fail_ms > 0 && search_p95_ms > config.search_p95_fail_ms as f64 {
                failures.push(format!("FAIL search_p95_ms {:.2} > {}", search_p95_ms, config.search_p95_fail_ms));
            }
            if search_p95_ms > config.search_p95_warn_ms as f64 {
                warnings.push(format!("WARN search_p95_ms {:.2} > {}", search_p95_ms, config.search_p95_warn_ms));
            }
        }
    } else {
        notes.push("latency probe skipped".to_string());
    }

    BudgetEvaluation { warnings, failures, notes }
}

pub fn run(shell: &mut dyn Shell, context: &BudgetContext) -> Result<BudgetReport> {
    let config = BudgetConfig::from_env_with_perf_mode(context.perf_mode, |key| std::env::var(key).ok())?;
    let prepared = load_prepared_inputs(&context.pages_data_dir)?;

    let dist_dir = if config.run_trunk_build {
        run_trunk_build(shell, context)?;
        context.repo_root.join("tmp/trunk-dist-budget")
    } else {
        locate_existing_dist(&context.repo_root)?
    };

    let sizes = measure_sizes(&prepared, &dist_dir, &config)?;
    let latency_run = if config.run_latency_probe {
        Some(run_latency_probe(shell, &context.repo_root, &prepared.artifact_path)?)
    } else {
        None
    };
    let latency = latency_run
        .as_ref()
        .map(|run| run.metrics.clone())
        .unwrap_or_else(LatencyMetrics::disabled);

    let evaluation = evaluate_thresholds(&config, &sizes, &latency);
    let report = BudgetReport {
        config,
        sizes,
        latency,
        warnings: evaluation.warnings,
        failures: evaluation.failures,
        notes: evaluation.notes,
    };

    write_outputs(&report, &context.out_dir, latency_run.as_ref().map(|run| run.raw_output.as_str()))?;

    if report.failure_count() > 0 {
        bail!("budget checks failed ({} failures, {} warnings)", report.failure_count(), report.warning_count());
    }

    Ok(report)
}

pub fn load_prepared_inputs(pages_data_dir: &Path) -> Result<PreparedInputs> {
    let manifest_path = pages_data_dir.join("manifest.json");
    let manifest_text = fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read manifest {}", manifest_path.display()))?;
    let manifest: Manifest = serde_json::from_str(&manifest_text)
        .with_context(|| format!("failed to parse manifest {}", manifest_path.display()))?;

    let artifact_path = pages_data_dir.join(&manifest.artifact);
    if !artifact_path.is_file() {
        bail!("missing prepared artifact: {}", artifact_path.display());
    }

    Ok(PreparedInputs { manifest, artifact_path })
}

fn run_trunk_build(shell: &mut dyn Shell, context: &BudgetContext) -> Result<()> {
    let command = CommandSpec::new("trunk")
        .args(["build", "--release", "--dist", "../../tmp/trunk-dist-budget"])
        .cwd(context.repo_root.join("crates/nix-search-web"));

    run_with_heartbeat("trunk build --release", || shell.run(command.clone()))
        .with_context(|| format!("failed to run trunk build: {}", render_command(&command)))
}

fn run_latency_probe(shell: &mut dyn Shell, repo_root: &Path, artifact_path: &Path) -> Result<LatencyProbeRun> {
    let command = CommandSpec::new("cargo")
        .args([
            "run",
            "-p",
            "nix-search-web",
            "--bin",
            "latency_probe",
            "--release",
            "--",
            "--artifact",
        ])
        .arg(artifact_path.display().to_string())
        .arg("--iterations")
        .arg("40")
        .cwd(repo_root.to_path_buf());

    let output = run_with_heartbeat("latency probe", || shell.read(command.clone()))
        .with_context(|| format!("failed to run latency probe: {}", render_command(&command)))?;

    let metrics = parse_latency_probe_output(&output)
        .with_context(|| format!("failed to parse latency probe output from {}", render_command(&command)))?;

    Ok(LatencyProbeRun { metrics, raw_output: output })
}

fn run_with_heartbeat<T, F>(label: &str, action: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    let done = Arc::new(AtomicBool::new(false));
    let heartbeat_done = done.clone();
    let started = Instant::now();
    let label = label.to_string();

    let heartbeat = thread::spawn(move || {
        while !heartbeat_done.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_secs(5));
            if !heartbeat_done.load(Ordering::SeqCst) {
                tracing::info!(label = %label, elapsed_ms = started.elapsed().as_millis(), "heartbeat");
            }
        }
    });

    let result = action();
    done.store(true, Ordering::SeqCst);
    let _ = heartbeat.join();
    result
}

pub fn parse_latency_probe_output(output: &str) -> Result<LatencyMetrics> {
    let mut metrics = LatencyMetrics::disabled();
    metrics.enabled = true;

    for line in output.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        match key.trim() {
            "startup_read_ms" => metrics.startup_read_ms = Some(parse_f64(value.trim()).with_context(|| format!("invalid latency metric {key}"))?),
            "startup_hydrate_ms" => metrics.startup_hydrate_ms = Some(parse_f64(value.trim()).with_context(|| format!("invalid latency metric {key}"))?),
            "search_avg_ms" => metrics.search_avg_ms = Some(parse_f64(value.trim()).with_context(|| format!("invalid latency metric {key}"))?),
            "search_p50_ms" => metrics.search_p50_ms = Some(parse_f64(value.trim()).with_context(|| format!("invalid latency metric {key}"))?),
            "search_p95_ms" => metrics.search_p95_ms = Some(parse_f64(value.trim()).with_context(|| format!("invalid latency metric {key}"))?),
            "search_p99_ms" => metrics.search_p99_ms = Some(parse_f64(value.trim()).with_context(|| format!("invalid latency metric {key}"))?),
            _ => {}
        }
    }

    if metrics.search_p95_ms.is_none() {
        bail!("latency probe output did not contain search_p95_ms");
    }

    Ok(metrics)
}

fn locate_existing_dist(repo_root: &Path) -> Result<PathBuf> {
    for candidate in [repo_root.join("tmp/trunk-dist-budget"), repo_root.join("tmp/trunk-dist-web")] {
        if is_dist_dir_ready(&candidate)? {
            return Ok(candidate);
        }
    }

    bail!("missing web dist artifacts. Run just web-build once, or set RUN_TRUNK_BUILD=1.")
}

fn is_dist_dir_ready(dist_dir: &Path) -> Result<bool> {
    if !dist_dir.is_dir() {
        return Ok(false);
    }

    Ok(find_matching_file(dist_dir, |name| name.ends_with("_bg.wasm")).is_some()
        && find_matching_file(dist_dir, |name| name.ends_with(".js")).is_some())
}

fn measure_sizes(prepared: &PreparedInputs, dist_dir: &Path, config: &BudgetConfig) -> Result<SizeMeasurements> {
    let wasm_file = find_matching_file(dist_dir, |name| name.ends_with("_bg.wasm"))
        .ok_or_else(|| anyhow::anyhow!("missing wasm dist artifact in {}", dist_dir.display()))?;
    let js_file = find_matching_file(dist_dir, |name| name.ends_with(".js"))
        .ok_or_else(|| anyhow::anyhow!("missing js dist artifact in {}", dist_dir.display()))?;

    let wasm = measure_file(&wasm_file, config.brotli_quality)?;
    let js = measure_file(&js_file, config.brotli_quality)?;
    let data = measure_file(&prepared.artifact_path, config.data_brotli_quality)?;

    Ok(SizeMeasurements { wasm, js, data })
}

fn measure_file(path: &Path, quality: u32) -> Result<SizeMeasurement> {
    let raw_bytes = fs::metadata(path)
        .with_context(|| format!("failed to stat {}", path.display()))?
        .len();
    let brotli_bytes = compress_brotli(path, quality)?;
    Ok(SizeMeasurement { raw_bytes, brotli_bytes })
}

fn compress_brotli(path: &Path, quality: u32) -> Result<u64> {
    match Command::new("brotli")
        .args(["-q", &quality.to_string(), "-c"])
        .arg(path)
        .output()
    {
        Ok(output) if output.status.success() => Ok(output.stdout.len() as u64),
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("brotli exited with {}: {}", output.status, stderr.trim()))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
            Ok(compress_brotli_rust(&bytes, quality)?.len() as u64)
        }
        Err(error) => Err(error.into()),
    }
}

fn compress_brotli_rust(bytes: &[u8], quality: u32) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    {
        let mut params = brotli::enc::backward_references::BrotliEncoderParams::default();
        params.quality = quality as i32;
        params.mode = brotli::enc::backward_references::BrotliEncoderMode::BROTLI_MODE_TEXT;
        params.lgwin = 22;

        let mut writer = brotli::CompressorWriter::with_params(&mut out, 64 * 1024, &params);
        writer.write_all(bytes).context("failed to write brotli payload")?;
        writer.flush().context("failed to flush brotli payload")?;
    }
    Ok(out)
}

fn write_outputs(report: &BudgetReport, out_dir: &Path, latency_log: Option<&str>) -> Result<()> {
    fs::create_dir_all(out_dir).with_context(|| format!("failed to create output dir {}", out_dir.display()))?;

    let report_json = out_dir.join("report.json");
    fs::write(&report_json, serde_json::to_vec_pretty(report)?).with_context(|| format!("failed to write {}", report_json.display()))?;

    let report_md = out_dir.join("report.md");
    let markdown = report.to_markdown();
    fs::write(&report_md, &markdown).with_context(|| format!("failed to write {}", report_md.display()))?;
    let _ = env::summary(&markdown);

    let summary = out_dir.join("summary.txt");
    fs::write(&summary, report.to_summary_text()).with_context(|| format!("failed to write {}", summary.display()))?;

    let budget_check = out_dir.join("03-budget-check.txt");
    let mut budget_check_file = File::create(&budget_check).with_context(|| format!("failed to create {}", budget_check.display()))?;
    for warning in &report.warnings {
        writeln!(budget_check_file, "{warning}")?;
    }
    for failure in &report.failures {
        writeln!(budget_check_file, "{failure}")?;
    }

    let size_report = out_dir.join("02-size-report.tsv");
    let mut size_report_file = File::create(&size_report).with_context(|| format!("failed to create {}", size_report.display()))?;
    writeln!(size_report_file, "file\traw_bytes\tbrotli_bytes")?;
    writeln!(size_report_file, "{}\t{}\t{}", report_file_label("wasm"), report.sizes.wasm.raw_bytes, report.sizes.wasm.brotli_bytes)?;
    writeln!(size_report_file, "{}\t{}\t{}", report_file_label("js"), report.sizes.js.raw_bytes, report.sizes.js.brotli_bytes)?;
    writeln!(size_report_file, "{}\t{}\t{}", report_file_label("data"), report.sizes.data.raw_bytes, report.sizes.data.brotli_bytes)?;

    let latency_log_path = out_dir.join("04-latency-probe.log");
    let mut latency_file = File::create(&latency_log_path).with_context(|| format!("failed to create {}", latency_log_path.display()))?;
    if let Some(latency_log) = latency_log {
        write!(latency_file, "{latency_log}")?;
    } else {
        writeln!(latency_file, "skipped latency probe")?;
    }

    Ok(())
}

fn find_matching_file<F>(dir: &Path, predicate: F) -> Option<PathBuf>
where
    F: Fn(&str) -> bool,
{
    let mut matches = fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy().into_owned();
            if predicate(&name) {
                Some(entry.path())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    matches.sort();
    matches.into_iter().next()
}

fn report_file_label(kind: &str) -> String {
    match kind {
        "wasm" => "tmp/trunk-dist-budget/nix-search-web-*_bg.wasm".to_owned(),
        "js" => "tmp/trunk-dist-budget/nix-search-web-*.js".to_owned(),
        "data" => "crates/nix-search-web/static/data/packages-*.json".to_owned(),
        _ => kind.to_owned(),
    }
}

fn parse_toggle(value: Option<&str>, perf_mode: PerfMode, default_auto: bool) -> Result<bool> {
    match value {
        None | Some("auto") => Ok(default_auto || matches!(perf_mode, PerfMode::Full)),
        Some("1") | Some("true") | Some("yes") => Ok(true),
        Some("0") | Some("false") | Some("no") => Ok(false),
        Some(other) => bail!("invalid toggle value: {other} (expected auto|0|1|false|true|no|yes)"),
    }
}

fn parse_u32(value: Option<&str>) -> Option<u32> {
    value.and_then(|value| value.parse().ok())
}

fn parse_u64(value: Option<&str>) -> Option<u64> {
    value.and_then(|value| value.parse().ok())
}

fn parse_f64(value: &str) -> Result<f64> {
    value.parse().context("expected floating-point value")
}

fn bool_flag(value: bool) -> &'static str {
    if value { "1" } else { "0" }
}

fn status_for(failed: bool) -> &'static str {
    if failed { "FAIL" } else { "PASS" }
}

trait PerfModeExt {
    fn as_str(self) -> &'static str;
}

impl PerfModeExt for PerfMode {
    fn as_str(self) -> &'static str {
        match self {
            PerfMode::Quick => "quick",
            PerfMode::Full => "full",
        }
    }
}