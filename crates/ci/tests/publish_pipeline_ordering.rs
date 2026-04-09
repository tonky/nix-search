use ci::pipelines::{publish, PublishContext};
use ci::shell::MockShell;
use std::fs;
use std::path::Path;

#[test]
fn publish_pipeline_runs_steps_in_order() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let repo_root = tempdir.path().to_path_buf();
    let pages_data_dir = repo_root.join("tmp/pages-data");
    let dist_dir = repo_root.join("crates/nix-search-web/dist");
    fs::create_dir_all(&pages_data_dir).expect("create pages data dir");
    fs::create_dir_all(&dist_dir).expect("create dist dir");
    seed_manifest_and_artifacts(&pages_data_dir);
    seed_dist(&dist_dir);

    let context = PublishContext::new(repo_root.clone(), pages_data_dir.clone());

    let mut shell = MockShell::default();
    publish().run(&mut shell, &context).expect("pipeline succeeds");

    assert_eq!(shell.commands().len(), 2);
    assert_eq!(shell.commands()[0].program, "cargo");
    assert!(shell.commands()[0].args.join(" ").contains("prep-web"));
    assert_eq!(shell.commands()[1].program, "trunk");
    assert!(shell.commands()[1].args.join(" ").contains("--public-url /nix-search/"));
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

fn seed_dist(dist_dir: &Path) {
    fs::write(dist_dir.join("index.html"), "<html></html>").expect("write index");
    fs::write(dist_dir.join("site_bg.wasm"), "wasm").expect("write wasm");
}