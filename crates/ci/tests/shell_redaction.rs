use ci::shell::{CommandSpec, MockShell, Shell};

#[test]
fn redacts_secret_env_vars() {
    let mut shell = MockShell::default();
    shell.run(CommandSpec::new("echo").arg("hello").env("GITHUB_TOKEN", "abc")).expect("mock shell run");

    let echoed = &shell.echoes()[0];
    assert!(!echoed.contains("abc"), "secret value leaked in echo: {echoed}");
    assert!(echoed.contains("GITHUB_TOKEN=[redacted]"));
}