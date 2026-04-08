# Phase 1: Scaffold + Cache

## Goal

Working project that can fetch the pkgforge package dump, parse it, store it in a tantivy index, and expose `cache` subcommands. End state: `nix-search cache update` populates the index; `nix-search cache status` shows age/count; raw search against tantivy works for smoke-testing.

**No fuzzy search, no TUI yet.** Just data plumbing.

## Prerequisites

None. This is the foundation.

## Deliverable

```bash
cargo run -- cache update          # fetches ~100k packages, builds tantivy index
cargo run -- cache status          # prints channel, package count, cache age, dir size
cargo run -- cache clear           # deletes index directory
cargo run -- claude code           # prints raw tantivy BM25 results (unranked), proves data is there
```

---

## Cargo.toml

```toml
[package]
name = "nix-search"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "nix-search"
path = "src/main.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
tantivy = "0.26"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
dirs = "5"
anyhow = "1"
```

Pure Rust — no C compiler dependency. `tantivy` bundles its own storage engine.

---

## Module Layout

```
src/
  main.rs           CLI entry, clap dispatch
  types.rs          Package, CacheMeta structs
  platform.rs       detect_current_platform()
  cache/
    mod.rs          public interface: update(), status(), clear(), open_index()
    fetch.rs        HTTP GET with ETag support
    parse.rs        pkgforge JSON parsing + platform collapsing
    index.rs        tantivy schema, index build, document retrieval
```

---

## Step-by-Step

### 1. `src/types.rs`

Core data types shared across modules:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Package {
    pub attr_path: String,      // e.g. "claude-code"
    pub pname: String,          // e.g. "claude-code"
    pub version: String,
    pub description: String,
    pub platforms: Vec<String>, // e.g. ["x86_64-linux", "aarch64-darwin"]
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CacheMeta {
    pub channel: String,
    pub fetched_at: u64,        // unix timestamp
    pub package_count: usize,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}
```

### 2. `src/platform.rs`

Map Rust's `std::env::consts` to Nix platform strings:

```rust
pub fn detect_current_platform() -> String {
    let arch = std::env::consts::ARCH; // "x86_64", "aarch64"
    let os = std::env::consts::OS;     // "linux", "macos", "windows"
    let nix_os = match os {
        "macos" => "darwin",
        other => other,
    };
    format!("{}-{}", arch, nix_os)
}
```

### 3. `src/cache/fetch.rs`

Fetch the pkgforge dump with ETag-based conditional GET:

```rust
pub struct FetchResult {
    pub body: Option<String>,   // None = 304 Not Modified
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

pub async fn fetch_dump(url: &str, etag: Option<&str>, last_modified: Option<&str>) -> anyhow::Result<FetchResult>
```

- Build `reqwest::Client`
- Add `If-None-Match: {etag}` header if present
- Add `If-Modified-Since: {last_modified}` header if present
- If response is `304`, return `FetchResult { body: None, ... }`
- Otherwise return the body text + response headers

URL constant: `https://raw.githubusercontent.com/pkgforge-dev/NixOS-Packages/main/nixpkgs.json`

### 4. `src/cache/parse.rs`

Parse the pkgforge JSON format and collapse per-platform entries:

Input format (JSON object, not array):
```json
{
  "legacyPackages.x86_64-linux.claude-code": { "pname": "claude-code", "version": "...", "description": "..." },
  "legacyPackages.aarch64-darwin.claude-code": { "pname": "claude-code", "version": "...", "description": "..." }
}
```

```rust
pub fn parse_dump(json: &str) -> anyhow::Result<Vec<Package>>
```

Steps:
1. Deserialize as `HashMap<String, RawEntry>` where `RawEntry` has `pname`, `version`, `description`
2. Group entries by their attr (key after stripping the platform prefix)
3. For each group, collect all platforms, keep one version/description
4. Emit one `Package` per attr_path with the platforms list populated

Key parsing logic:
```rust
// "legacyPackages.x86_64-linux.claude-code" → ("x86_64-linux", "claude-code")
// "legacyPackages.x86_64-linux.python312Packages.requests" → ("x86_64-linux", "python312Packages.requests")
fn parse_key(key: &str) -> Option<(&str, &str)> {
    let rest = key.strip_prefix("legacyPackages.")?;
    let dot = rest.find('.')?;
    Some((&rest[..dot], &rest[dot + 1..]))
}
```

Group by `attr` (not `pname`), since `pname` alone isn't unique for sub-packages like `python312Packages.requests` vs `python311Packages.requests`.

### 5. `src/cache/index.rs`

Tantivy schema definition and index operations:

```rust
use tantivy::{doc, schema::*, Index, IndexWriter, TantivyDocument};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;

pub struct NixIndex {
    pub index: Index,
    pub schema: Schema,
    pub fields: NixFields,
}

pub struct NixFields {
    pub f_attr_path_exact: Field,
    pub f_attr_path_text: Field,
    pub f_pname: Field,
    pub f_version: Field,
    pub f_description: Field,
    pub f_platforms: Field,
}
```

**Schema definition:**
```rust
pub fn build_schema() -> (Schema, NixFields) {
    let mut builder = Schema::builder();
    // STRING = non-tokenized exact-match index; TEXT = tokenized full-text index
    let f_attr_path_exact = builder.add_text_field("attr_path_exact", STRING | STORED);
    let f_attr_path_text  = builder.add_text_field("attr_path_text",  TEXT | STORED);
    let f_pname       = builder.add_text_field("pname",       TEXT | STORED);
    let f_version     = builder.add_text_field("version",     STORED);        // not searchable
    let f_description = builder.add_text_field("description", TEXT | STORED);
    let f_platforms   = builder.add_text_field("platforms",   STORED);        // not searchable
    (builder.build(), NixFields { f_attr_path_exact, f_attr_path_text, f_pname, f_version, f_description, f_platforms })
}
```

`platforms` is stored as a space-separated string (`"x86_64-linux aarch64-darwin"`). Not tokenized for search — just stored for retrieval and post-filtering.

**Open or create index:**
```rust
pub fn open_or_create(index_dir: &Path) -> anyhow::Result<NixIndex> {
    let (schema, fields) = build_schema();
    let index = if index_dir.exists() {
        match Index::open_in_dir(index_dir) {
            Ok(idx) => idx,
            Err(_) => {
                // Corrupt or incompatible schema: recreate from source snapshot.
                std::fs::remove_dir_all(index_dir)?;
                std::fs::create_dir_all(index_dir)?;
                Index::create_in_dir(index_dir, schema.clone())?
            }
        }
    } else {
        std::fs::create_dir_all(index_dir)?;
        Index::create_in_dir(index_dir, schema.clone())?
    };
    Ok(NixIndex { index, schema, fields })
}
```

**Build index (full rebuild, safe for background refresh):**
```rust
pub fn build(index_dir: &Path, packages: &[Package]) -> anyhow::Result<()> {
    std::fs::create_dir_all(index_dir)?;
    let nix_index = open_or_create(index_dir)?;
    let index = nix_index.index;
    let fields = nix_index.fields;
    let mut writer: IndexWriter = index.writer(64_000_000)?; // 64MB buffer

    // In-place snapshot refresh: no directory deletion while readers are active.
    writer.delete_all_documents()?;

    for pkg in packages {
        writer.add_document(doc!(
            fields.f_attr_path_exact => pkg.attr_path.as_str(),
            fields.f_attr_path_text  => pkg.attr_path.as_str(),
            fields.f_pname       => pkg.pname.as_str(),
            fields.f_version     => pkg.version.as_str(),
            fields.f_description => pkg.description.as_str(),
            fields.f_platforms   => pkg.platforms.join(" ").as_str(),
        ))?;
    }
    writer.commit()?;
    Ok(())
}
```

Rebuilding the document set on each update avoids upsert/merge complexity while remaining safe for concurrent readers. 100k docs at 64MB writer buffer takes ~1-2 seconds.

**Raw BM25 search (smoke-test only — proper search comes in Phase 2):**
```rust
pub fn search_raw(nix_index: &NixIndex, query_str: &str, limit: usize) -> anyhow::Result<Vec<Package>> {
    let reader = nix_index.index.reader()?;
    let searcher = reader.searcher();
    let parser = QueryParser::for_index(
        &nix_index.index,
        vec![
            nix_index.fields.f_attr_path_text,
            nix_index.fields.f_pname,
            nix_index.fields.f_description,
        ],
    );
    let query = parser.parse_query(query_str)
        .unwrap_or_else(|_| parser.parse_query_lenient(query_str).0);
    let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

    top_docs.into_iter().map(|(_, addr)| {
        let doc: TantivyDocument = searcher.doc(addr)?;
        Ok(doc_to_package(&doc, &nix_index.fields))
    }).collect()
}

fn doc_to_package(doc: &TantivyDocument, fields: &NixFields) -> Package {
    let get = |f: Field| doc.get_first(f)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Package {
        attr_path:   get(fields.f_attr_path_exact),
        pname:       get(fields.f_pname),
        version:     get(fields.f_version),
        description: get(fields.f_description),
        platforms:   get(fields.f_platforms).split_whitespace().map(String::from).collect(),
    }
}
```

### 6. `src/cache/mod.rs`

Public interface; handles `meta.json` alongside the index directory:

```rust
pub async fn update(cache_dir: &Path, channel: &str) -> anyhow::Result<()>
pub fn status(cache_dir: &Path, channel: &str) -> anyhow::Result<()>
pub fn clear(cache_dir: &Path, channel: &str) -> anyhow::Result<()>
pub fn index_dir(cache_dir: &Path, channel: &str) -> PathBuf  // ~/.cache/nix-search/{channel}/index/
pub fn meta_path(cache_dir: &Path, channel: &str) -> PathBuf  // ~/.cache/nix-search/{channel}/meta.json
pub fn load_meta(cache_dir: &Path, channel: &str) -> Option<CacheMeta>
pub fn save_meta(cache_dir: &Path, channel: &str, meta: &CacheMeta) -> anyhow::Result<()>
```

Cache directory layout:
```
~/.cache/nix-search/
  nixos-unstable/
    index/            ← tantivy index directory (segment files, meta.json)
    meta.json         ← CacheMeta (our own, not tantivy's internal meta.json)
    enriched/         ← per-package enriched JSON (Phase 4)
```

`update()` flow:
1. Load existing `CacheMeta` (if any) to get etag/last_modified
2. Call `fetch::fetch_dump()` with those headers
3. If `304`: bump `fetched_at` in meta, save, return early
4. Parse JSON with `parse::parse_dump()`
5. Build tantivy index with `index::build()` (`delete_all_documents` + add + commit)
6. Save updated meta with new etag, timestamp, package count

`status()` prints:
```
channel:   nixos-unstable
packages:  98432
cached at: 2026-04-07 14:23 (6h ago)
index size: 22.1 MB
etag:      "abc123..."
```

For index size: sum file sizes under the index directory with `fs::read_dir`.

`clear()`: `std::fs::remove_dir_all(cache_dir.join(channel))` — removes index + meta + enriched.

### 7. `src/main.rs`

```rust
#[derive(clap::Parser)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Search query (when no subcommand given)
    query: Vec<String>,

    #[arg(long, default_value = "nixos-unstable")]
    channel: String,

    #[arg(long)]
    cache_dir: Option<PathBuf>,
}

#[derive(clap::Subcommand)]
enum Command {
    Cache(CacheArgs),
}

#[derive(clap::Args)]
struct CacheArgs {
    #[command(subcommand)]
    action: CacheAction,
}

#[derive(clap::Subcommand)]
enum CacheAction {
    Update,
    Status,
    Clear,
}
```

Default cache dir: `dirs::cache_dir().unwrap().join("nix-search")`

Query path (no subcommand): open tantivy index, call `index::search_raw()`, print results as plain lines. Temporary smoke-test — proper search/output comes in Phase 2.

If the index directory doesn't exist when searching, print:
```
cache not populated — run: nix-search cache update
```
and exit 1.

---

## Definition of Done

- [ ] `cargo build` with no warnings
- [ ] `cargo run -- cache update` completes, prints package count, index lives at `~/.cache/nix-search/nixos-unstable/`
- [ ] Running update a second time immediately: prints "cache up to date (ETag matched)" without re-downloading
- [ ] `cargo run -- cache status` shows correct count, age, directory size
- [ ] `cargo run -- cache clear` deletes the channel directory entirely
- [ ] `cargo run -- claude` returns some BM25 results (unranked/unsorted is fine)
- [ ] `cargo run -- python312Packages` returns matching packages
- [ ] Index directory is at `~/.cache/nix-search/nixos-unstable/index/`
