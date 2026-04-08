use std::process::{Command, Output};

fn run_ok(bin: &str, args: &[&str]) -> Output {
    let output = Command::new(bin)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run {:?}: {}", args, e));

    if !output.status.success() {
        panic!(
            "command failed: {:?}\nexit: {:?}\nstdout:\n{}\nstderr:\n{}",
            args,
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    output
}

#[test]
#[ignore = "live integration test; requires internet"]
fn live_cli_against_internet_data() {
    let bin = env!("CARGO_BIN_EXE_nix-search");
    let tmp = tempfile::tempdir().expect("tempdir");
    let cache_dir = tmp.path().join("cache");
    let cache_dir_str = cache_dir.to_string_lossy().to_string();

    // 1) Pull real upstream data and build local index.
    run_ok(bin, &["--cache-dir", &cache_dir_str, "cache", "update"]);

    // 2) User-facing search modes should all work on that live data.
    let first = run_ok(bin, &["--cache-dir", &cache_dir_str, "--first", "ripgrep"]);
    let first_attr = String::from_utf8_lossy(&first.stdout).trim().to_string();
    assert!(!first_attr.is_empty(), "--first returned empty output");

    let exact = run_ok(
        bin,
        &[
            "--cache-dir",
            &cache_dir_str,
            "--first",
            "--attr",
            &first_attr,
        ],
    );
    let exact_attr = String::from_utf8_lossy(&exact.stdout).trim().to_string();
    assert_eq!(exact_attr, first_attr, "--attr exact lookup mismatch");

    let json = run_ok(bin, &["--cache-dir", &cache_dir_str, "--json", "ripgrep"]);
    let parsed: serde_json::Value =
        serde_json::from_slice(&json.stdout).expect("valid JSON from --json");
    let arr = parsed.as_array().expect("JSON output should be an array");
    assert!(!arr.is_empty(), "--json returned empty result array");
    assert!(
        arr[0].get("attr_path").and_then(|v| v.as_str()).is_some(),
        "first JSON result missing attr_path"
    );

    let plain = run_ok(bin, &["--cache-dir", &cache_dir_str, "--plain", "ripgrep"]);
    let plain_text = String::from_utf8_lossy(&plain.stdout);
    assert!(
        plain_text.contains("ripgrep") || plain_text.contains(&first_attr),
        "--plain output did not contain expected result"
    );
}
