use ci::steps::publish_summary;

#[test]
fn publish_summary_snapshot() {
    let manifest = ci::Manifest {
        version: "sha256-test".to_owned(),
        checksum: "checksum-test".to_owned(),
        package_count: 123,
        built_at: Some(1),
        artifact: "packages-test.json".to_owned(),
        compressed_artifact: None,
        compressed_format: None,
        compressed_size_bytes: None,
    };

    let summary = publish_summary::render_summary(&manifest, Some("https://example.invalid/nix-search/"));
    insta::assert_snapshot!(summary);
}