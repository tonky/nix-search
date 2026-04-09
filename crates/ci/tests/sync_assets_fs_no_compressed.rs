use ci::manifest::Manifest;
use ci::steps::sync_assets;
use std::fs;

#[test]
fn sync_assets_allows_missing_compressed_artifact() {
    let pages_data = tempfile::tempdir().expect("pages data tempdir");
    let web_static = tempfile::tempdir().expect("web static tempdir");
    let data_dir = web_static.path().join("data");

    let manifest = Manifest {
        version: "sha256-test".to_owned(),
        checksum: "checksum".to_owned(),
        package_count: 1,
        built_at: Some(1),
        artifact: "packages-sha256-test.json".to_owned(),
        compressed_artifact: None,
        compressed_format: None,
        compressed_size_bytes: None,
    };

    fs::write(pages_data.path().join("manifest.json"), serde_json::to_vec_pretty(&manifest).expect("serialize manifest")).expect("write manifest");
    fs::write(pages_data.path().join("packages-sha256-test.json"), "artifact").expect("write artifact");
    fs::create_dir_all(&data_dir).expect("create data dir");

    sync_assets::run(&manifest, pages_data.path(), web_static.path()).expect("sync assets succeeds");

    assert_eq!(fs::read_to_string(data_dir.join("packages-sha256-test.json")).expect("artifact copied"), "artifact");
    assert!(!data_dir.join("packages-sha256-test.json.br").exists());
}