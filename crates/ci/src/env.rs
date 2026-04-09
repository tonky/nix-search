use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::PathBuf;

pub fn is_ci() -> bool {
    truthy_env("GITHUB_ACTIONS") || truthy_env("CI")
}

pub fn set_output(key: impl AsRef<str>, value: impl AsRef<str>) -> io::Result<()> {
    let mut stdout = io::stdout();
    set_output_with_writer(&mut stdout, key, value)
}

pub fn set_output_with_writer<W: Write>(mut stdout: W, key: impl AsRef<str>, value: impl AsRef<str>) -> io::Result<()> {
    let key = key.as_ref();
    let value = value.as_ref();
    if let Some(path) = output_path("GITHUB_OUTPUT") {
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        writeln!(file, "{key}={value}")
    } else {
        writeln!(stdout, "{key}={value}")
    }
}

pub fn summary(contents: impl AsRef<str>) -> io::Result<()> {
    let mut stdout = io::stdout();
    summary_with_writer(&mut stdout, contents)
}

pub fn summary_with_writer<W: Write>(mut stdout: W, contents: impl AsRef<str>) -> io::Result<()> {
    let contents = contents.as_ref();
    if let Some(path) = output_path("GITHUB_STEP_SUMMARY") {
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        file.write_all(contents.as_bytes())
    } else {
        stdout.write_all(contents.as_bytes())
    }
}

pub fn group(name: impl AsRef<str>) -> Group {
    let name = name.as_ref().to_owned();
    let active = is_ci();
    if active {
        println!("::group::{name}");
    } else {
        println!("== {name} ==");
    }
    Group { active }
}

pub struct Group {
    active: bool,
}

impl Drop for Group {
    fn drop(&mut self) {
        if self.active {
            println!("::endgroup::");
        }
    }
}

fn truthy_env(key: &str) -> bool {
    matches!(std::env::var(key).ok().as_deref(), Some("1") | Some("true") | Some("TRUE") | Some("yes") | Some("YES"))
}

fn output_path(key: &str) -> Option<PathBuf> {
    std::env::var_os(key).map(PathBuf::from)
}