use anyhow::Result;
use clap::{Parser, Subcommand};
use ci::env;
use ci::pipelines::{BudgetContext, PerfMode, PublishContext};
use ci::shell::RealShell;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Parser)]
#[command(name = "ci", version, about = "Rust-native CI runner", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Placeholder for the budget pipeline.
    Budget {
        #[arg(long, default_value = "tmp/bench/perf-size-ci")]
        out: PathBuf,
        #[arg(long, value_enum, default_value_t = PerfMode::Quick)]
        perf_mode: PerfMode,
    },
    /// Placeholder for the publish pipeline.
    Publish {
        #[arg(long, default_value = "tmp/pages-data")]
        out: PathBuf,
    },
    /// Print tool versions and environment diagnostics.
    Doctor,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt().with_target(false).without_time().init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Budget { out, perf_mode } => {
            let mut shell = RealShell::new()?;
            let context = BudgetContext::new(std::env::current_dir()?, out, perf_mode);
            ci::pipelines::budget().run(&mut shell, &context)
        }
        Commands::Publish { out } => {
            let mut shell = RealShell::new()?;
            let context = PublishContext::new(std::env::current_dir()?, out);
            ci::pipelines::publish().run(&mut shell, &context)
        }
        Commands::Doctor => run_doctor(),
    }
}

fn run_doctor() -> Result<()> {
    let _tool_group = env::group("Tool versions");
    for tool in ["rustc", "cargo", "trunk", "jq", "brotli", "git"] {
        println!("{tool}: {}", tool_version(tool));
    }

    let _env_group = env::group("Environment");
    println!("is_ci: {}", env::is_ci());
    for key in ["CI", "GITHUB_ACTIONS", "GITHUB_OUTPUT", "GITHUB_STEP_SUMMARY", "GITHUB_WORKSPACE"] {
        println!("{key}: {}", env_value(key));
    }

    let _paths_group = env::group("Writable paths");
    for path in writable_paths() {
        println!("{}: {}", path.display(), if is_writable(&path) { "writable" } else { "not writable" });
    }

    Ok(())
}

fn tool_version(tool: &str) -> String {
    match Command::new(tool).arg("--version").output() {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        Ok(output) => format!("failed (exit status {})", output.status),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => "not found".to_owned(),
        Err(error) => format!("error: {error}"),
    }
}

fn env_value(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| "unset".to_owned())
}

fn writable_paths() -> Vec<PathBuf> {
    let mut paths = vec![std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")), std::env::temp_dir()];
    if let Ok(workspace) = std::env::var("GITHUB_WORKSPACE") {
        paths.push(PathBuf::from(workspace));
    }
    paths
}

fn is_writable(path: &Path) -> bool {
    let probe = path.join(".ci-doctor-write-probe");
    match OpenOptions::new().create(true).write(true).truncate(true).open(&probe) {
        Ok(mut file) => {
            let _ = writeln!(file, "probe");
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}