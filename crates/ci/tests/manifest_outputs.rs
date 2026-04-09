use ci::steps::manifest_outputs;
use std::fs;

#[test]
fn writes_manifest_outputs() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let pages_data_dir = tempdir.path();
    fs::write(
        pages_data_dir.join("manifest.json"),
        r#"{
  "version": "sha256-test",
  "checksum": "checksum-test",
  "package_count": 123,
  "built_at": 1,
  "artifact": "packages-test.json",
  "compressed_artifact": null,
  "compressed_format": null,
  "compressed_size_bytes": null
}"#,
    )
    .expect("write manifest");

    let output_path = tempdir.path().join("output.txt");
    unsafe {
        std::env::set_var("GITHUB_OUTPUT", &output_path);
    }

    manifest_outputs::run(pages_data_dir).expect("manifest outputs");

    let contents = fs::read_to_string(&output_path).expect("read output file");
    assert!(contents.contains("version=sha256-test"));
    assert!(contents.contains("package_count=123"));
    assert!(contents.contains("checksum=checksum-test"));

    unsafe {
        std::env::remove_var("GITHUB_OUTPUT");
    }
}