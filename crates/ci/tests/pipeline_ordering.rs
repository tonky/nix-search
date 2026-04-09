use ci::pipelines::{budget, BudgetContext, PerfMode};
use ci::shell::MockShell;
use std::fs;
use std::path::Path;

#[test]
fn prep_web_runs_before_sync_assets_and_budgets() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let repo_root = tempdir.path().to_path_buf();
    let pages_data_dir = repo_root.join("tmp/pages-data");
    fs::create_dir_all(&pages_data_dir).expect("create pages data dir");
    seed_manifest_and_artifacts(&pages_data_dir);

    let dist_dir = repo_root.join("tmp/trunk-dist-web");
    fs::create_dir_all(&dist_dir).expect("create dist dir");
    fs::write(dist_dir.join("nix-search-web-test_bg.wasm"), "wasm").expect("write wasm");
    fs::write(dist_dir.join("nix-search-web-test.js"), "js").expect("write js");

    let web_static_dir = repo_root.join("crates/nix-search-web/static");
    fs::create_dir_all(web_static_dir.join("data")).expect("create static data dir");

    let context = BudgetContext {
        repo_root: repo_root.clone(),
        pages_data_dir: pages_data_dir.clone(),
        web_static_dir,
        out_dir: repo_root.join("tmp/bench/perf-size-ci"),
        perf_mode: PerfMode::Quick,
    };

    let mut shell = MockShell::default();
    budget().run(&mut shell, &context).expect("pipeline succeeds");

    assert_eq!(shell.commands().len(), 1);
    assert_eq!(shell.commands()[0].program, "cargo");
    assert!(shell.commands()[0].args.join(" ").contains("prep-web"));

    let copied_manifest = repo_root.join("crates/nix-search-web/static/data/manifest.json");
    assert!(copied_manifest.exists());
    assert!(repo_root.join("crates/nix-search-web/static/data/packages-sha256-test.json").exists());
    assert!(repo_root.join("crates/nix-search-web/static/data/packages-sha256-test.json.br").exists());
    assert!(repo_root.join("tmp/bench/perf-size-ci/report.json").exists());
}

#[test]
fn prep_web_failure_short_circuits_pipeline() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let repo_root = tempdir.path().to_path_buf();
    let context = BudgetContext {
        repo_root: repo_root.clone(),
        pages_data_dir: repo_root.join("tmp/pages-data"),
        web_static_dir: repo_root.join("crates/nix-search-web/static"),
        out_dir: repo_root.join("tmp/bench/perf-size-ci"),
        perf_mode: PerfMode::Quick,
    };

    let mut shell = MockShell::default();
    shell.fail_next_run("prep-web failed");

    let error = budget().run(&mut shell, &context).expect_err("pipeline fails");
    let chain = error.chain().map(ToString::to_string).collect::<Vec<_>>().join("\n");
    assert!(chain.contains("prep-web"), "{chain}");
    assert!(chain.contains(&repo_root.display().to_string()), "{chain}");
}

fn seed_manifest_and_artifacts(dir: &Path) {
    fs::write(
        dir.join("manifest.json"),
        r#"{
  "version": "sha256-test",
  "checksum": "checksum",
  "package_count": 1,
  "built_at": 1,
  "artifact": "packages-sha256-test.json",
  "compressed_artifact": "packages-sha256-test.json.br",
  "compressed_format": "brotli",
  "compressed_size_bytes": 1
}"#,
    )
    .expect("write manifest");
    fs::write(dir.join("packages-sha256-test.json"), "artifact").expect("write artifact");
    fs::write(dir.join("packages-sha256-test.json.br"), "compressed").expect("write compressed artifact");
}