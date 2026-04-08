#[cfg(target_arch = "wasm32")]
fn main() {
    eprintln!("nix-search CLI is unavailable on wasm32 targets");
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::io::IsTerminal;
    use std::path::PathBuf;

    use anyhow::Context;
    use clap::{Args, CommandFactory, Parser, Subcommand};
    use nix_search::cache;
    use nix_search::cache::index;
    use nix_search::output::{self, OutputMode};
    use nix_search::platform;
    use nix_search::prep;
    use nix_search::search::{self, SearchConfig};
    use nix_search::tui;

    #[derive(Parser, Debug)]
    #[command(name = "nix-search")]
    #[command(about = "Fast offline Nix package search")]
    struct Cli {
        #[command(subcommand)]
        command: Option<Command>,

        #[arg(value_name = "QUERY")]
        query: Vec<String>,

        #[arg(short = 'c', long, default_value = "nixos-unstable")]
        channel: String,

        #[arg(short = 'p', long)]
        platform: Option<String>,

        #[arg(long)]
        all_platforms: bool,

        #[arg(long)]
        json: bool,

        #[arg(long)]
        plain: bool,

        #[arg(long)]
        first: bool,

        #[arg(long)]
        attr: Option<String>,

        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,

        #[arg(long)]
        update: bool,

        #[arg(long)]
        cache_dir: Option<PathBuf>,

        #[arg(long, default_value = "86400")]
        ttl: u64,
    }

    #[derive(Subcommand, Debug)]
    enum Command {
        Cache(CacheArgs),
        PrepWeb(PrepWebArgs),
    }

    #[derive(Args, Debug)]
    struct PrepWebArgs {
        #[arg(long, default_value = "tmp/wasm-data")]
        output: PathBuf,
    }

    #[derive(Args, Debug)]
    struct CacheArgs {
        #[command(subcommand)]
        action: CacheAction,
    }

    #[derive(Subcommand, Debug)]
    enum CacheAction {
        Update,
        Status,
        Clear,
    }

    pub async fn run_main() -> i32 {
        match run().await {
            Ok(code) => code,
            Err(err) => {
                eprintln!("error: {err:#}");
                2
            }
        }
    }

    async fn run() -> anyhow::Result<i32> {
        let cli = Cli::parse();
        let cache_dir = cli
            .cache_dir
            .clone()
            .or_else(|| dirs::cache_dir().map(|p| p.join("nix-search")))
            .context("could not resolve cache directory")?;

        match cli.command {
            Some(Command::Cache(args)) => match args.action {
                CacheAction::Update => {
                    cache::update(&cache_dir, &cli.channel).await?;
                    eprintln!("cache updated");
                    Ok(0)
                }
                CacheAction::Status => {
                    println!("{}", cache::status(&cache_dir, &cli.channel)?);
                    Ok(0)
                }
                CacheAction::Clear => {
                    cache::clear(&cache_dir, &cli.channel)?;
                    eprintln!("cache cleared");
                    Ok(0)
                }
            },
            Some(Command::PrepWeb(args)) => {
                let result = prep::run_local_prep(&args.output).await?;
                println!(
                    "prepared web data: version={} packages={} checksum={}",
                    result.manifest.version,
                    result.manifest.package_count,
                    result.manifest.checksum
                );
                println!("artifact={}", result.artifact_path.display());
                println!("manifest={}", result.manifest_path.display());
                Ok(0)
            }
            None => run_search(cli, cache_dir).await,
        }
    }

    async fn run_search(cli: Cli, cache_dir: PathBuf) -> anyhow::Result<i32> {
        if cli.update {
            eprintln!("updating cache...");
            cache::update(&cache_dir, &cli.channel).await?;
            eprintln!("done");
        }

        if cli.query.is_empty() && cli.attr.is_none() {
            let mut cmd = Cli::command();
            cmd.print_help()?;
            println!();
            return Ok(0);
        }

        let idx_path = cache::index_dir(&cache_dir, &cli.channel);
        if !idx_path.exists() {
            eprintln!("cache not populated - run: nix-search cache update");
            return Ok(1);
        }

        let nix_index = index::open_or_create(&idx_path)?;
        let effective_platform = if cli.all_platforms {
            None
        } else if let Some(p) = &cli.platform {
            Some(p.clone())
        } else {
            Some(platform::detect_current_platform())
        };

        let query = cli.query.join(" ");
        let config = SearchConfig {
            query: query.clone(),
            platform: effective_platform.clone(),
            limit: cli.limit,
            exact_attr: cli.attr.clone(),
        };

        let is_tty = std::io::stdout().is_terminal();
        if is_tty && !cli.json && !cli.plain && !cli.first {
            let selection = tui::run_tui(
                &nix_index,
                &query,
                effective_platform,
                &cli.channel,
                cli.ttl,
            )?;
            if let Some(attr) = selection {
                println!("{}", attr);
                return Ok(0);
            }
            return Ok(1);
        }

        let mode = if cli.json {
            OutputMode::Json
        } else if cli.first {
            OutputMode::First
        } else {
            OutputMode::Plain
        };

        let results = search::search(&nix_index, &config)?;
        output::print_results(&results, mode)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    let code = native::run_main().await;
    std::process::exit(code);
}
