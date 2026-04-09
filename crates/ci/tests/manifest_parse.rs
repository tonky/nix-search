use ci::Manifest;

#[test]
fn parses_real_manifest_fixture() {
    let manifest_text = include_str!("fixtures/manifest.json");
    let manifest: Manifest = serde_json::from_str(manifest_text).expect("manifest parses");

    assert_eq!(manifest.version, "sha256-5baf0491bca5");
    assert_eq!(manifest.package_count, 142840);
    assert_eq!(manifest.checksum, "5baf0491bca56988c9517c4595d517475b7868a4a7ed3a4e0eb6754e4a859213");
    assert_eq!(manifest.artifact, "packages-sha256-5baf0491bca5.json");
    assert_eq!(manifest.compressed_artifact.as_deref(), Some("packages-sha256-5baf0491bca5.json.br"));

    let roundtrip = serde_json::to_value(&manifest).expect("manifest serializes");
    let original = serde_json::from_str::<serde_json::Value>(manifest_text).expect("fixture parses as value");
    assert_eq!(roundtrip, original);
}