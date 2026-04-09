use ci::pipelines::{publish, PublishContext};
use ci::shell::MockShell;
use std::fs;

#[test]
fn trunk_build_failure_mentions_build_log_path() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let repo_root = tempdir.path().to_path_buf();
    let pages_data_dir = repo_root.join("tmp/pages-data");
    let dist_dir = repo_root.join("crates/nix-search-web/dist");
    fs::create_dir_all(&pages_data_dir).expect("create pages data dir");
    fs::create_dir_all(&dist_dir).expect("create dist dir");
    fs::write(
        pages_data_dir.join("manifest.json"),
        r#"{
  "version": "sha256-test",
  "checksum": "checksum",
  "package_count": 1,
  "built_at": 1,
  "artifact": "packages-sha256-test.json",
  "compressed_artifact": null,
  "compressed_format": null,
  "compressed_size_bytes": null
}"#,
    )
    .expect("write manifest");
    fs::write(pages_data_dir.join("packages-sha256-test.json"), "artifact").expect("write artifact");
    fs::write(dist_dir.join("index.html"), "<html></html>").expect("write index");
    fs::write(dist_dir.join("site_bg.wasm"), "wasm").expect("write wasm");

    let context = PublishContext::new(repo_root.clone(), pages_data_dir.clone());

    let mut shell = MockShell::default();
    shell.fail_next_read("boom");

    let error = publish().run(&mut shell, &context).expect_err("pipeline fails");
    let chain = error.chain().map(ToString::to_string).collect::<Vec<_>>().join("\n");
    assert!(chain.contains("01-trunk-build.log"), "{chain}");
}