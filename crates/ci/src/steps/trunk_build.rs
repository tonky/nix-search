use crate::shell::{CommandSpec, Shell};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

pub fn run(shell: &mut dyn Shell, repo_root: &Path, out_dir: &Path, dist_dir: &Path) -> Result<PathBuf> {
    fs::create_dir_all(out_dir).with_context(|| format!("failed to create publish output dir {}", out_dir.display()))?;

    let build_log = out_dir.join("01-trunk-build.log");
    // TODO(repo-rename): keep this public URL in sync with the legacy Pages route.
    let command = CommandSpec::new("trunk")
        .args(["build", "--release", "--public-url", "/nix-search/"])
        .cwd(repo_root.join("crates/nix-search-web"));

    let output = run_with_heartbeat("trunk build --release", || shell.read(command.clone()))
        .with_context(|| format!("failed to run trunk build: {} (output is captured in {})", command.program, build_log.display()))?;

    fs::write(&build_log, output.as_bytes())
        .with_context(|| format!("failed to write build log {}", build_log.display()))?;

    if !dist_dir.is_dir() {
        anyhow::bail!("trunk build did not produce dist dir: {}", dist_dir.display());
    }

    Ok(build_log)
}

fn run_with_heartbeat<T, F>(label: &str, action: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    let started = Instant::now();
    let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let heartbeat_done = done.clone();
    let label = label.to_string();

    let heartbeat = thread::spawn(move || {
        while !heartbeat_done.load(std::sync::atomic::Ordering::SeqCst) {
            thread::sleep(Duration::from_secs(5));
            if !heartbeat_done.load(std::sync::atomic::Ordering::SeqCst) {
                tracing::info!(label = %label, elapsed_ms = started.elapsed().as_millis(), "heartbeat");
            }
        }
    });

    let result = action();
    done.store(true, std::sync::atomic::Ordering::SeqCst);
    let _ = heartbeat.join();
    result
}