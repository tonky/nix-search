use ci::manifest::Manifest;
use ci::steps::sync_assets;
use std::fs;

#[test]
fn sync_assets_copies_manifest_and_artifacts_and_removes_stale_files() {
    let pages_data = tempfile::tempdir().expect("pages data tempdir");
    let web_static = tempfile::tempdir().expect("web static tempdir");
    let data_dir = web_static.path().join("data");
    fs::create_dir_all(&data_dir).expect("create data dir");
    fs::write(data_dir.join("packages-old.json"), "stale").expect("seed stale json");
    fs::write(data_dir.join("packages-old.json.br"), "stale").expect("seed stale br");

    let manifest = Manifest {
        version: "sha256-test".to_owned(),
        checksum: "checksum".to_owned(),
        package_count: 1,
        built_at: Some(1),
        artifact: "packages-sha256-test.json".to_owned(),
        compressed_artifact: Some("packages-sha256-test.json.br".to_owned()),
        compressed_format: Some("brotli".to_owned()),
        compressed_size_bytes: Some(10),
    };

    fs::write(pages_data.path().join("manifest.json"), serde_json::to_vec_pretty(&manifest).expect("serialize manifest")).expect("write manifest");
    fs::write(pages_data.path().join("packages-sha256-test.json"), "artifact").expect("write artifact");
    fs::write(pages_data.path().join("packages-sha256-test.json.br"), "compressed").expect("write compressed artifact");

    sync_assets::run(&manifest, pages_data.path(), web_static.path()).expect("sync assets succeeds");

    assert_eq!(fs::read_to_string(data_dir.join("manifest.json")).expect("manifest copied"), serde_json::to_string_pretty(&manifest).expect("serialize manifest"));
    assert_eq!(fs::read_to_string(data_dir.join("packages-sha256-test.json")).expect("artifact copied"), "artifact");
    assert_eq!(fs::read_to_string(data_dir.join("packages-sha256-test.json.br")).expect("compressed copied"), "compressed");
    assert!(!data_dir.join("packages-old.json").exists());
    assert!(!data_dir.join("packages-old.json.br").exists());
}