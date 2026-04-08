# Phase 2: Search Pipeline + Non-interactive Output

## Goal

Replace the raw BM25 smoke-test from Phase 1 with a proper search pipeline: exact attr lookup via `TermQuery` → BM25 with field boosts → fuzzy fallback via `FuzzyTermQuery` → platform split → formatted output. All non-interactive output modes (`--plain`, `--json`, `--first`) working. This is the core engine the TUI will also call in Phase 3.

## Prerequisites

Phase 1 complete. Tantivy index populated at `~/.cache/nix-search/{channel}/`.

## Deliverable

```bash
nix-search claude code              # plain table, ranked, platform-filtered by default
nix-search --all-platforms rust     # no platform filter
nix-search --json python            # JSON array output
nix-search --first claude code      # prints "claude-code" only → for shell integration
flox install $(nix-search --first claude code)   # the money shot
```

---

## No new dependencies

Everything needed is already in tantivy. `nucleo-matcher` is not needed — tantivy's `FuzzyTermQuery` handles typo tolerance.

---

## Module Layout

```
src/
  search/
    mod.rs      public search() function + SearchConfig + SearchResults
    query.rs    tantivy query construction (BM25F + fuzzy fallback)
    filter.rs   platform split (matched / others)
  output/
    mod.rs      plain table, JSON, first-only formatters
```

---

## Step-by-Step

### 1. `src/search/mod.rs` — Public Interface

Define the config and result types that Phase 3 (TUI) will also use:

```rust
pub struct SearchConfig {
    pub query: String,
    pub platform: Option<String>,   // None = all platforms
    pub limit: usize,
}

pub struct SearchResults {
    pub matched: Vec<ScoredPackage>,    // support current platform (or all if no filter)
    pub others: Vec<ScoredPackage>,     // don't support current platform
}

pub struct ScoredPackage {
    pub package: Package,
    pub score: f32,                     // tantivy BM25F score
}

pub fn search(nix_index: &NixIndex, config: &SearchConfig) -> anyhow::Result<SearchResults>
```

`search()` flow:
1. Call `query::run(nix_index, &config.query, config.limit * 5)` — over-fetch to give filtering + platform split enough candidates
2. Call `filter::split_by_platform(scored, config.platform.as_deref())`
3. Apply one global limit across `(matched + others)` in that order so output count is stable and predictable
4. Return `SearchResults`

If query is empty, return top packages by index order (arbitrary but fast).

### 2. `src/search/query.rs` — Tantivy Query Construction

```rust
use tantivy::{TantivyDocument, collector::TopDocs, query::{AllQuery, QueryParser, FuzzyTermQuery, TermQuery}, schema::{IndexRecordOption, Term}};

pub fn run(nix_index: &NixIndex, query_str: &str, limit: usize) -> anyhow::Result<Vec<ScoredPackage>>
```

**Three-step strategy:**

Step 0 — exact attr lookup (for `--attr` or `nixpkgs#...` input):
```rust
// Normalize forms:
// --attr ripgrep             -> ripgrep
// nixpkgs#python312Packages.requests -> python312Packages.requests
if let Some(exact_attr) = normalize_exact_attr(query_str) {
    let term = Term::from_field_text(nix_index.fields.f_attr_path_exact, &exact_attr);
    let exact = TermQuery::new(term, IndexRecordOption::Basic);
    let docs = searcher.search(&exact, &TopDocs::with_limit(limit))?;
    if !docs.is_empty() {
        return Ok(to_scored(&searcher, docs, &nix_index.fields));
    }
}

Step 1 — BM25F with field boosts (for normal queries):
```rust
let reader = nix_index.index.reader()?;
let searcher = reader.searcher();

let mut parser = QueryParser::for_index(
    &nix_index.index,
    vec![
        nix_index.fields.f_attr_path_text,
        nix_index.fields.f_pname,
        nix_index.fields.f_description,
    ],
);
parser.set_field_boost(nix_index.fields.f_attr_path_text, 4.0);
parser.set_field_boost(nix_index.fields.f_pname, 5.0);
parser.set_field_boost(nix_index.fields.f_description, 1.0);
parser.set_conjunction_by_default();

// parse_query_lenient never returns an error — gracefully handles special chars
let (query, _warnings) = parser.parse_query_lenient(query_str);
let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;
```

`parse_query_lenient` is preferred over `parse_query` — it never errors on input like `*`, `(`, `++`, etc. from users typing partial queries in the TUI.

Step 2 — Fuzzy fallback (when step 1 returns 0 results or query is a single short word):
```rust
if top_docs.is_empty() || (query_str.split_whitespace().count() == 1 && query_str.len() <= 12) {
    let mut fuzzy_docs = vec![];

    // Fuzzy on package name
    let pname_term = Term::from_field_text(nix_index.fields.f_pname, query_str);
    let pname_q = FuzzyTermQuery::new(pname_term, 1, true);
    fuzzy_docs.extend(searcher.search(&pname_q, &TopDocs::with_limit(limit))?);

    // Fuzzy on attr path
    let attr_term = Term::from_field_text(nix_index.fields.f_attr_path_text, query_str);
    let attr_q = FuzzyTermQuery::new(attr_term, 1, true);
    fuzzy_docs.extend(searcher.search(&attr_q, &TopDocs::with_limit(limit))?);

    // Merge with BM25 docs, dedup by DocAddress, keep best score per doc
}
```

**Document extraction:**
```rust
fn to_scored(searcher: &Searcher, top_docs: Vec<(f32, DocAddress)>, fields: &NixFields) -> Vec<ScoredPackage> {
    top_docs.into_iter().filter_map(|(score, addr)| {
        let doc: TantivyDocument = searcher.doc(addr).ok()?;
        Some(ScoredPackage {
            score,
            package: doc_to_package(&doc, fields),
        })
    }).collect()
}
```

`doc_to_package` is the same helper defined in Phase 1's `index.rs` — move it to a shared location or re-export.

**Edge cases:**
- Empty query: return `searcher.search(&AllQuery, &TopDocs::with_limit(limit))` — gets the first N docs in index order
- Very long query: `parse_query_lenient` handles it; truncate at e.g. 200 chars before passing
- Multi-word query ("claude code"): parser is configured with conjunction-by-default, so all terms are required while still searching across `attr_path_text|pname|description` with boosts

### 3. `src/search/filter.rs` — Platform Split

```rust
pub fn split_by_platform(
    packages: Vec<ScoredPackage>,
    platform: Option<&str>,
) -> (Vec<ScoredPackage>, Vec<ScoredPackage>)  // (matched, others)
```

```rust
pub fn split_by_platform(packages: Vec<ScoredPackage>, platform: Option<&str>) -> (Vec<ScoredPackage>, Vec<ScoredPackage>) {
    match platform {
        None => (packages, vec![]),  // --all-platforms: everything goes in matched
        Some(plat) => packages.into_iter().partition(|sp| {
            sp.package.platforms.iter().any(|p| p == plat)
        }),
    }
}
```

`platforms` is a `Vec<String>` on the `Package` struct (already deserialized from the stored space-separated string in `doc_to_package`).

### 4. `src/output/mod.rs` — Output Formatters

```rust
pub enum OutputMode { Plain, Json, First }

pub fn print_results(results: &SearchResults, mode: OutputMode)
```

**Plain table** (default non-TTY or `--plain`):
```
claude-code          25.4.1   Anthropic's official CLI for Claude AI assistant
claude-desktop        0.3.2   Desktop application for Claude AI
── other platforms ──
claude-nix            1.0.0   Nix wrapper for Claude (linux only)
```

Column widths: `attr_path` padded to 24 chars, `version` to 10, then description truncated to `terminal_width - 36` (use `terminal_size` crate or default to 80).

**JSON** (`--json`): serialize final limited results as a single JSON array, matched first:
```rust
let all: Vec<&Package> = results.matched.iter()
    .chain(results.others.iter())
    .map(|sp| &sp.package)
    .collect();
println!("{}", serde_json::to_string_pretty(&all)?);
```

**First** (`--first`): print only the top match's attr_path, or exit 1 if no results:
```rust
match results.matched.first().or(results.others.first()) {
    Some(sp) => { println!("{}", sp.package.attr_path); std::process::exit(0); }
    None     => { eprintln!("no results"); std::process::exit(1); }
}
```

### 5. Update `src/main.rs`

Add flags and wire up the search path:

```rust
#[derive(clap::Parser)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    query: Vec<String>,

    #[arg(long, default_value = "nixos-unstable")]
    channel: String,

    #[arg(short = 'p', long)]
    platform: Option<String>,

    #[arg(long)]
    all_platforms: bool,

    #[arg(long)]
    json: bool,

    #[arg(long)]
    plain: bool,

    #[arg(long)]
    first: bool,

    #[arg(long)]
    attr: Option<String>,

    #[arg(short = 'n', long, default_value = "20")]
    limit: usize,

    #[arg(long)]
    update: bool,

    #[arg(long)]
    cache_dir: Option<PathBuf>,
}
```

Main dispatch (no subcommand):
1. If `--update`: run `cache::update()` first
2. Resolve effective platform:
   - `--platform <X>`: use that
   - `--all-platforms`: `None`
   - default: `Some(platform::detect_current_platform())`
3. Open tantivy index with `cache::index::open_or_create()`
4. Build query string:
    - `--attr <ATTR>` takes precedence
    - otherwise use joined positional query
5. Call `search::search()`
6. Determine output mode from flags; call `output::print_results()`

---

## Edge Cases

- **Special FTS chars** in query (`*`, `"`, `(`, `+`): `parse_query_lenient` handles gracefully
- **No results from BM25**: fuzzy fallback kicks in automatically
- **Single character query**: both BM25 prefix and fuzzy return results
- **Exact attr input**: `--attr ripgrep` and `nixpkgs#ripgrep` both go through `TermQuery` on `attr_path_exact`
- **No index**: print "cache not populated, run: nix-search cache update" to stderr, exit 1
- **Empty query with no args**: show clap help

---

## Definition of Done

- [ ] `nix-search claude code` returns ranked results, platform-filtered to current system
- [ ] `nix-search --all-platforms claude` shows packages for all platforms, no separator line
- [ ] `nix-search --json rust` outputs a valid JSON array to stdout
- [ ] `nix-search --first claude code` prints exactly `claude-code\n` and nothing else
- [ ] `echo "$(nix-search --first claude code)"` works cleanly in shell
- [ ] Typo tolerance: `nix-search claud` finds `claude-code` (fuzzy fallback)
- [ ] Multi-word: `nix-search code claude` same results as `nix-search claude code`
- [ ] `nix-search xyznotapackage` exits 1, "no results" to stderr
- [ ] `nix-search` with no args shows clap help
