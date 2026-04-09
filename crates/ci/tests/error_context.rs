use ci::pipelines::{BudgetContext, PerfMode};
use ci::shell::MockShell;

#[test]
fn prep_web_failure_includes_command_and_cwd() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let repo_root = tempdir.path().to_path_buf();
    let context = BudgetContext {
        repo_root: repo_root.clone(),
        pages_data_dir: repo_root.join("tmp/pages-data"),
        web_static_dir: repo_root.join("crates/nix-search-web/static"),
        out_dir: repo_root.join("tmp/bench/perf-size-ci"),
        perf_mode: PerfMode::Quick,
    };

    let mut shell = MockShell::default();
    shell.fail_next_run("boom");

    let error = ci::pipelines::budget().run(&mut shell, &context).expect_err("pipeline fails");
    let chain = error.chain().map(ToString::to_string).collect::<Vec<_>>().join("\n");
    assert!(chain.contains("prep-web"), "{chain}");
    assert!(chain.contains(&repo_root.display().to_string()), "{chain}");
}