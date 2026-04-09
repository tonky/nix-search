use crate::shell::Shell;
use crate::steps::{budgets, manifest_outputs, pages_artifact_prep, prep_web, publish_summary, sync_assets, trunk_build};
use anyhow::{Context, Result};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub struct BudgetContext {
    pub repo_root: PathBuf,
    pub pages_data_dir: PathBuf,
    pub web_static_dir: PathBuf,
    pub out_dir: PathBuf,
    pub perf_mode: PerfMode,
}

pub struct PublishContext {
    pub repo_root: PathBuf,
    pub pages_data_dir: PathBuf,
    pub web_static_dir: PathBuf,
    pub dist_dir: PathBuf,
    pub out_dir: PathBuf,
}

impl PublishContext {
    pub fn new(repo_root: PathBuf, out_dir: PathBuf) -> Self {
        Self {
            dist_dir: repo_root.join("crates/nix-search-web/dist"),
            web_static_dir: repo_root.join("crates/nix-search-web/static"),
            pages_data_dir: out_dir.clone(),
            repo_root,
            out_dir,
        }
    }
}

impl BudgetContext {
    pub fn new(repo_root: PathBuf, out_dir: PathBuf, perf_mode: PerfMode) -> Self {
        Self {
            web_static_dir: repo_root.join("crates/nix-search-web/static"),
            pages_data_dir: repo_root.join("tmp/pages-data"),
            repo_root,
            out_dir,
            perf_mode,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
pub enum PerfMode {
    Quick,
    Full,
}

pub type PipelineStep<C> = fn(&mut dyn Shell, &C) -> Result<()>;

pub struct Pipeline<C> {
    steps: Vec<(&'static str, PipelineStep<C>)>,
}

impl<C> Pipeline<C> {
    pub fn new(steps: Vec<(&'static str, PipelineStep<C>)>) -> Self {
        Self { steps }
    }

    pub fn run<S: Shell>(&self, shell: &mut S, context: &C) -> Result<()> {
        for (name, step) in &self.steps {
            step(shell, context).with_context(|| format!("pipeline step failed: {name}"))?;
        }
        Ok(())
    }
}

pub fn budget() -> Pipeline<BudgetContext> {
    Pipeline::new(vec![
        ("prep-web", budget_prep_web),
        ("sync-assets", budget_sync_assets),
        ("budgets", budget_budgets),
    ])
}

pub fn publish() -> Pipeline<PublishContext> {
    Pipeline::new(vec![
        ("prep-web", publish_prep_web),
        ("sync-assets", publish_sync_assets),
        ("trunk-build", publish_trunk_build),
        ("pages-artifact-prep", publish_pages_artifact_prep),
        ("manifest-outputs", publish_manifest_outputs),
        ("publish-summary", publish_publish_summary),
    ])
}

fn budget_prep_web(shell: &mut dyn Shell, context: &BudgetContext) -> Result<()> {
    prep_web::run(shell, &context.repo_root, &context.pages_data_dir).map(|_| ())
}

fn budget_sync_assets(_shell: &mut dyn Shell, context: &BudgetContext) -> Result<()> {
    let manifest = prep_web::load_manifest(&context.pages_data_dir)?;
    sync_assets::run(&manifest, &context.pages_data_dir, &context.web_static_dir)
}

fn budget_budgets(shell: &mut dyn Shell, context: &BudgetContext) -> Result<()> {
    std::fs::create_dir_all(&context.out_dir)
        .with_context(|| format!("failed to create budget output dir {}", context.out_dir.display()))?;
    budgets::run(shell, context).map(|_| ())
}

fn publish_prep_web(shell: &mut dyn Shell, context: &PublishContext) -> Result<()> {
    prep_web::run(shell, &context.repo_root, &context.pages_data_dir).map(|_| ())
}

fn publish_sync_assets(_shell: &mut dyn Shell, context: &PublishContext) -> Result<()> {
    let manifest = prep_web::load_manifest(&context.pages_data_dir)?;
    sync_assets::run(&manifest, &context.pages_data_dir, &context.web_static_dir)
}

fn publish_trunk_build(shell: &mut dyn Shell, context: &PublishContext) -> Result<()> {
    trunk_build::run(shell, &context.repo_root, &context.out_dir, &context.dist_dir).map(|_| ())
}

fn publish_pages_artifact_prep(_shell: &mut dyn Shell, context: &PublishContext) -> Result<()> {
    pages_artifact_prep::run(&context.dist_dir)
}

fn publish_manifest_outputs(_shell: &mut dyn Shell, context: &PublishContext) -> Result<()> {
    manifest_outputs::run(&context.pages_data_dir).map(|_| ())
}

fn publish_publish_summary(_shell: &mut dyn Shell, context: &PublishContext) -> Result<()> {
    let manifest = prep_web::load_manifest(&context.pages_data_dir)?;
    publish_summary::run(&manifest)
}