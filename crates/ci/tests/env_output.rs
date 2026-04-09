use ci::env;
use std::fs;

#[test]
fn set_output_appends_to_github_output_file() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let output_path = tempdir.path().join("output.txt");
    unsafe {
        std::env::set_var("GITHUB_OUTPUT", &output_path);
    }

    env::set_output("k", "v").expect("set_output");

    let contents = fs::read_to_string(&output_path).expect("read output file");
    assert_eq!(contents, "k=v\n");
    unsafe {
        std::env::remove_var("GITHUB_OUTPUT");
    }
}

#[test]
fn set_output_writes_to_stdout_without_github_output() {
    unsafe {
        std::env::remove_var("GITHUB_OUTPUT");
    }
    let mut stdout = Vec::new();

    env::set_output_with_writer(&mut stdout, "k", "v").expect("set_output_with_writer");

    assert_eq!(String::from_utf8(stdout).expect("utf8"), "k=v\n");
}