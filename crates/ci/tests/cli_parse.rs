use assert_cmd::prelude::*;
use predicates::str::contains;
use std::process::Command;

#[test]
fn cli_help_snapshot() {
    let output = Command::cargo_bin("ci")
        .expect("binary exists")
        .env("COLUMNS", "80")
        .arg("--help")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).expect("help output is utf8");
    insta::assert_snapshot!("help", stdout);
}

#[test]
fn budget_help_exits_zero() {
    Command::cargo_bin("ci")
        .expect("binary exists")
        .arg("budget")
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("Usage:"))
        .stdout(contains("budget"));
}

#[test]
fn doctor_reports_versions_and_paths() {
    Command::cargo_bin("ci")
        .expect("binary exists")
        .arg("doctor")
        .assert()
        .success()
        .stdout(contains("Tool versions"))
        .stdout(contains("Writable paths"))
        .stdout(contains("rustc"));
}

#[test]
fn publish_help_exits_zero() {
    Command::cargo_bin("ci")
        .expect("binary exists")
        .arg("publish")
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("Usage:"))
        .stdout(contains("publish"));
}