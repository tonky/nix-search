use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};

use crate::cache;
use crate::cache::index::NixIndex;
use crate::search::{self, SearchConfig};
use crate::types::EsConfig;

use super::model::Model;
use super::msg::Msg;

#[derive(Debug)]
pub enum Cmd {
    RunSearch {
        query: String,
        platform: Option<String>,
        limit: usize,
        exact_attr: Option<String>,
    },
    LoadEnrichment {
        attr_path: String,
    },
    ResolveEsConfig,
    OpenHomepage,
}

#[derive(Debug)]
pub enum WorkerTask {
    FetchEnrichment {
        attr_path: String,
        es_config: EsConfig,
        cache_dir: PathBuf,
        channel: String,
    },
    RefreshCache {
        cache_dir: PathBuf,
        channel: String,
    },
}

pub fn spawn_worker(ui_tx: Sender<Msg>) -> Sender<WorkerTask> {
    let (tx, rx) = mpsc::channel::<WorkerTask>();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().ok();
        while let Ok(task) = rx.recv() {
            match task {
                WorkerTask::FetchEnrichment {
                    attr_path,
                    es_config,
                    cache_dir,
                    channel,
                } => {
                    let result = runtime
                        .as_ref()
                        .and_then(|rt| {
                            rt.block_on(cache::enrich::fetch_details(&attr_path, &es_config))
                                .ok()
                        })
                        .flatten();

                    if let Some(details) = &result {
                        let _ = cache::store_enriched(&cache_dir, &channel, details);
                    }
                    let _ = ui_tx.send(Msg::EnrichmentLoaded(result));
                }
                WorkerTask::RefreshCache { cache_dir, channel } => {
                    let ok = runtime
                        .as_ref()
                        .and_then(|rt| rt.block_on(cache::update(&cache_dir, &channel)).ok())
                        .is_some();
                    let _ = ui_tx.send(Msg::CacheRefreshFinished(ok));
                }
            }
        }
    });
    tx
}

pub fn execute_all(model: &mut Model, nix_index: &NixIndex, cmds: Vec<Cmd>) -> anyhow::Result<()> {
    for cmd in cmds {
        execute(model, nix_index, cmd)?;
    }
    Ok(())
}

fn execute(model: &mut Model, nix_index: &NixIndex, cmd: Cmd) -> anyhow::Result<()> {
    match cmd {
        Cmd::RunSearch {
            query,
            platform,
            limit,
            exact_attr,
        } => {
            let cfg = SearchConfig {
                query,
                platform,
                limit,
                exact_attr,
            };
            let msg = match search::search(nix_index, &cfg) {
                Ok(results) => Msg::SearchCompleted(results),
                Err(e) => Msg::SearchFailed(e.to_string()),
            };
            model.internal_tx.send(msg).ok();
        }
        Cmd::ResolveEsConfig => {
            let cfg = resolve_es_config(model);
            model.internal_tx.send(Msg::EsConfigResolved(cfg)).ok();
        }
        Cmd::LoadEnrichment { attr_path } => {
            if let Some(cached) = cache::load_enriched(&model.cache_dir, &model.channel, &attr_path)
            {
                model
                    .internal_tx
                    .send(Msg::EnrichmentLoaded(Some(cached)))
                    .ok();
                return Ok(());
            }
            let Some(es_config) = model.es_config.clone() else {
                model
                    .internal_tx
                    .send(Msg::EnrichmentFailed("enrichment unavailable".to_string()))
                    .ok();
                return Ok(());
            };

            model
                .worker_tx
                .send(WorkerTask::FetchEnrichment {
                    attr_path,
                    es_config,
                    cache_dir: model.cache_dir.clone(),
                    channel: model.channel.clone(),
                })
                .ok();
        }
        Cmd::OpenHomepage => {
            if let Some(details) = &model.enriched
                && let Some(url) = details.homepage.first()
            {
                open_url(url);
            }
        }
    }
    Ok(())
}

fn resolve_es_config(model: &Model) -> Option<EsConfig> {
    let candidates = vec![
        (
            "https://search.nixos.org/backend/latest-44-nixos-unstable/_search".to_string(),
            "package_attr_name".to_string(),
        ),
        (
            "https://search.nixos.org/backend/latest-nixos-unstable/_search".to_string(),
            "package_attr_name".to_string(),
        ),
        (
            "https://search.nixos.org/backend/latest-nixos-unstable/_search".to_string(),
            "attr_name".to_string(),
        ),
    ];

    // Keep this lightweight: resolve first candidate by default, runtime failures degrade gracefully.
    if let Some((url, field)) = candidates.into_iter().next() {
        let cfg = EsConfig {
            url,
            term_field: field,
        };

        if let Some(mut meta) = cache::load_meta(&model.cache_dir, &model.channel) {
            meta.es_url = Some(cfg.url.clone());
            meta.es_term_field = Some(cfg.term_field.clone());
            let _ = cache::save_meta(&model.cache_dir, &model.channel, &meta);
        }

        return Some(cfg);
    }
    None
}

fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}
