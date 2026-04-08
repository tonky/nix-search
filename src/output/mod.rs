use crate::search::SearchResults;
use crate::types::Package;

#[derive(Debug, Clone, Copy)]
pub enum OutputMode {
    Plain,
    Json,
    First,
}

pub fn print_results(results: &SearchResults, mode: OutputMode) -> anyhow::Result<i32> {
    match mode {
        OutputMode::Plain => print_plain(results),
        OutputMode::Json => print_json(results),
        OutputMode::First => print_first(results),
    }
}

fn flatten(results: &SearchResults) -> Vec<&Package> {
    results
        .matched
        .iter()
        .chain(results.others.iter())
        .map(|sp| &sp.package)
        .collect()
}

fn print_plain(results: &SearchResults) -> anyhow::Result<i32> {
    let mut any = false;
    for sp in &results.matched {
        any = true;
        println!(
            "{:<30} {:>12} {}",
            sp.package.attr_path,
            sp.package.version,
            sp.package.description.replace('\n', " ")
        );
    }

    if !results.others.is_empty() {
        println!("-- other platforms --");
    }

    for sp in &results.others {
        any = true;
        println!(
            "{:<30} {:>12} {}",
            sp.package.attr_path,
            sp.package.version,
            sp.package.description.replace('\n', " ")
        );
    }

    if any {
        Ok(0)
    } else {
        eprintln!("no results");
        Ok(1)
    }
}

fn print_json(results: &SearchResults) -> anyhow::Result<i32> {
    let all = flatten(results);
    println!("{}", serde_json::to_string_pretty(&all)?);
    if all.is_empty() { Ok(1) } else { Ok(0) }
}

fn print_first(results: &SearchResults) -> anyhow::Result<i32> {
    match results.matched.first().or(results.others.first()) {
        Some(sp) => {
            println!("{}", sp.package.attr_path);
            Ok(0)
        }
        None => {
            eprintln!("no results");
            Ok(1)
        }
    }
}
