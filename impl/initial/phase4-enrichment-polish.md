# Phase 4: ES Detail Enrichment + Polish

## Goal

Populate the right pane with rich metadata (homepage, license, maintainers, broken) fetched on-demand from nixos-search Elasticsearch when a package is selected. Add resilient endpoint/query fallback, background cache refresh with a spinner, `o` to open homepage, help overlay, and final polish.

## Prerequisites

Phase 1 + 2 + 3 complete. Full TUI working with cached-only data.

## Deliverable

- Right pane shows homepage, license, maintainers, broken when online
- Graceful degradation when ES is unreachable
- Background refresh spinner in header when cache is stale
- `o` opens homepage in browser
- `?` toggles help overlay
- `--update` flag works

---

## Step-by-Step

### 1. Resolve ES Endpoint + Query Shape

Before writing code, resolve endpoint + field compatibility at runtime.

Candidate backend URL patterns:
```
POST https://search.nixos.org/backend/latest-44-nixos-unstable/_search
POST https://search.nixos.org/backend/latest-nixos-unstable/_search
Content-Type: application/json

{
  "query": {
        "term": { "package_attr_name": "claude-code" }
  },
  "size": 1
}
```

The response schema (based on the nixos-search frontend source) includes fields like:
- `package_attr_name` — attr path
- `package_pname` — package name
- `package_version`
- `package_description`
- `package_longDescription`
- `package_homepage` — array of URLs
- `package_license` — array of `{fullName, spdxId}` objects
- `package_maintainers` — array of `{name, github}` objects
- `package_platforms` — array of platform strings
- `package_broken` — boolean

Probe strategy:
```bash
curl -s -X POST 'https://search.nixos.org/backend/latest-44-nixos-unstable/_search' \
  -H 'Content-Type: application/json' \
  -d '{"query":{"term":{"package_attr_name":"ripgrep"}},"size":1}' | jq .
```

If that fails, retry with:
- `latest-nixos-unstable`
- alternate term key `attr_name`

Persist the first working combination to `meta.json` so future lookups do not re-probe each time. If all probes fail, skip enrichment gracefully and keep Phase 3 behavior.

### 2. `src/cache/enrich.rs` — ES Detail Fetch

```rust
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EnrichedDetails {
    pub attr_path: String,
    pub homepage: Vec<String>,
    pub license: Vec<String>,       // license full names
    pub maintainers: Vec<String>,   // github handles
    pub broken: bool,
    pub long_description: Option<String>,
}

pub async fn fetch_details(attr_path: &str, es_config: &EsConfig) -> anyhow::Result<Option<EnrichedDetails>>
```

Implementation:
```rust
pub async fn fetch_details(attr_path: &str, es_config: &EsConfig) -> anyhow::Result<Option<EnrichedDetails>> {
    let url = es_config.url.clone();
    let term_field = es_config.term_field.as_str();
    let body = serde_json::json!({
        "query": { "term": { term_field: attr_path } },
        "size": 1
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    let resp = client.post(&url)
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        return Ok(None);
    }

    let json: serde_json::Value = resp.json().await?;
    let hit = json["hits"]["hits"].as_array()
        .and_then(|hits| hits.first())
        .and_then(|h| h.get("_source"));

    match hit {
        None => Ok(None),
        Some(src) => Ok(Some(EnrichedDetails {
            attr_path: attr_path.to_string(),
            homepage: src["package_homepage"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            license: src["package_license"].as_array()
                .map(|a| a.iter()
                    .filter_map(|v| v["fullName"].as_str().map(String::from))
                    .collect())
                .unwrap_or_default(),
            maintainers: src["package_maintainers"].as_array()
                .map(|a| a.iter()
                    .filter_map(|v| v["github"].as_str().map(|s| format!("@{}", s)))
                    .collect())
                .unwrap_or_default(),
            broken: src["package_broken"].as_bool().unwrap_or(false),
            long_description: src["package_longDescription"].as_str().map(String::from),
        })),
    }
}
```

### 3. Cache Enriched Data as JSON Files

Enriched data is stored as small JSON files under the channel's `enriched/` directory:

```
~/.cache/nix-search/nixos-unstable/
  index/           ← tantivy index
  meta.json        ← CacheMeta
  enriched/
    claude-code.json
    ripgrep.json
    ...
```

Add to `cache/mod.rs`:
```rust
pub fn enriched_path(cache_dir: &Path, channel: &str, attr_path: &str) -> PathBuf {
    // attr_path may contain slashes in theory; sanitize to be safe
    let safe_name = attr_path.replace('/', "__");
    cache_dir.join(channel).join("enriched").join(format!("{}.json", safe_name))
}

pub fn load_enriched(cache_dir: &Path, channel: &str, attr_path: &str) -> Option<EnrichedDetails> {
    let path = enriched_path(cache_dir, channel, attr_path);
    let bytes = std::fs::read(&path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn store_enriched(cache_dir: &Path, channel: &str, details: &EnrichedDetails) -> anyhow::Result<()> {
    let path = enriched_path(cache_dir, channel, &details.attr_path);
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, serde_json::to_vec_pretty(details)?)?;
    Ok(())
}
```

No schema migration, no SQL. `cache clear` already deletes the entire channel directory including `enriched/`. The directory is created lazily on first write.

### 4. Wire Enrichment into Elm Commands

Integrate enrichment as Elm commands/events, not ad-hoc mutation.

Model additions:

In `Model` (alongside Phase 3 fields):
```rust
pub cache_dir: PathBuf,             // needed for enriched JSON file paths
pub enriched: Option<EnrichedDetails>,
pub enriched_loading: bool,
pub enriched_for: Option<String>,   // attr_path the current enriched data is for
pub detail_error: Option<String>,
```

Message additions:
```rust
pub enum Msg {
    // ... Phase 3 msgs ...
    SelectionChanged(String),
    EnrichmentLoaded(Option<EnrichedDetails>),
    EnrichmentFailed(String),
}
```

Command additions:
```rust
pub enum Cmd {
    // ...
    LoadEnrichment { attr_path: String },
}
```

`update()` behavior:
- On `SelectionChanged(attr)`: set `enriched_loading = true`, return `Cmd::LoadEnrichment { attr }`.
- On `EnrichmentLoaded(details)`: set `enriched`, clear loading, cache to disk.
- On `EnrichmentFailed(err)`: clear loading, set `detail_error`.

`cmd::execute()` behavior for `LoadEnrichment`:
- Check local file cache first.
- If missing, enqueue network request on the shared background runtime worker.
- Send completion back via `internal_tx` as `Msg::EnrichmentLoaded` / `Msg::EnrichmentFailed`.

### 5. Update Right Pane Rendering

In `ui.rs`, expand the detail pane to show enriched fields when available:

```rust
// After the basic fields (attr, version, desc, platforms):
if app.enriched_loading {
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled("loading...", Style::default().fg(Color::DarkGray)),
    ]));
} else if let Some(details) = &app.enriched {
    if !details.homepage.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled("homepage: ", Style::default().bold()),
            Span::raw(details.homepage.first().cloned().unwrap_or_default()),
        ]));
    }
    if !details.license.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("license:  ", Style::default().bold()),
            Span::raw(details.license.join(", ")),
        ]));
    }
    if !details.maintainers.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("by:       ", Style::default().bold()),
            Span::raw(details.maintainers.join(" ")),
        ]));
    }
    if details.broken {
        lines.push(Line::from(vec![
            Span::styled("broken:   ", Style::default().bold()),
            Span::styled("YES", Style::default().fg(Color::Red).bold()),
        ]));
    }
}
```

### 6. `o` Key — Open Homepage in Browser

In `events.rs`:
```rust
(_, KeyCode::Char('o')) => {
    if let Some(details) = &app.enriched {
        if let Some(url) = details.homepage.first() {
            open_url(url);
        }
    }
    AppAction::None
}
```

```rust
fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}
```

### 7. Background Cache Refresh

In `tui/mod.rs`, after terminal setup, check if cache is stale and kick off a refresh:

```rust
// In AppState:
pub cache_refreshing: bool,

// At TUI startup:
let meta = cache::load_meta(cache_dir, channel);
let stale = meta.map(|m| {
    let age = SystemTime::now()
        .duration_since(UNIX_EPOCH).unwrap().as_secs() - m.fetched_at;
    age > ttl
}).unwrap_or(true);

if stale {
    app.cache_refreshing = true;
    let cache_dir = cache_dir.to_path_buf();
    let channel = channel.to_string();
    cmds.push(Cmd::RefreshCacheIfStale { cache_dir, channel, ttl });
}
```

Use a single long-lived background worker runtime for all async work (enrichment + refresh):
- Worker owns one Tokio runtime
- UI thread sends `WorkerTask` messages
- Worker sends `Msg` replies back to UI thread

This avoids creating a new runtime per request and keeps the Elm loop clean.

Because Phase 1 refresh is in-place Tantivy writer update (`delete_all_documents + add + commit`) instead of deleting index directories, concurrent readers stay valid.

Show in header:
```rust
// In header line:
if app.cache_refreshing {
    let spinner = ["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏"];
    let tick = (app.tick % spinner.len() as u64) as usize;
    // app.tick increments each render frame
    header_right.push_str(&format!(" {}", spinner[tick]));
}
```

### 8. `?` Help Overlay

Add a bool `show_help: bool` to `AppState`. Toggle on `?`.

In `ui.rs`, render a centered popup when `show_help` is true:

```rust
if app.show_help {
    let area = centered_rect(60, 50, frame.area()); // 60% wide, 50% tall
    let help_text = vec![
        Line::from("  nix-search keyboard shortcuts"),
        Line::raw(""),
        Line::from("  ↑/↓             navigate results"),
        Line::from("  Enter           select + print attr path"),
        Line::from("  Tab             switch pane focus"),
        Line::from("  Ctrl+U          clear query"),
        Line::from("  Ctrl+P          toggle platform filter"),
        Line::from("  o               open homepage in browser"),
        Line::from("  Esc / q         quit without selecting"),
        Line::raw(""),
        Line::from("  ?               close this help"),
    ];
    let popup = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title(" help "))
        .style(Style::default().bg(Color::DarkGray));
    frame.render_widget(Clear, area); // clear background before popup
    frame.render_widget(popup, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    use ratatui::layout::{Constraint, Direction, Layout};
    let vert = Layout::default().direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ]).split(r);
    Layout::default().direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ]).split(vert[1])[1]
}
```

### 9. `--update` Flag

Already planned in Phase 2 but wire it up properly:
```rust
if cli.update {
    eprintln!("updating cache...");
    cache::update(&cache_dir, &cli.channel).await?;
    eprintln!("done");
}
```

Run before opening the DB for search.

### 10. Polish Pass

- **Error messages**: all errors go to stderr, never stdout (so shell substitution doesn't capture them)
- **Exit codes**: 0 = selected, 1 = cancelled or no results, 2 = error (cache missing, network failure, etc.)
- **`cache status`**: pretty-print age as "2h ago" / "just now" instead of raw timestamps
- **Version column alignment**: right-align version strings in the left pane list
- **Broken packages**: dim + strikethrough style in the list for packages where `broken = true` (once enriched data is cached)
- **Broken packages**: dim + strikethrough style only when broken is known locally; do not globally reorder by broken status in v1
- **Long attr paths**: truncate with `…` if wider than the left pane column
- **Resize handling**: `crossterm::event::Event::Resize(w, h)` → update layout, redraw
- **Single runtime worker**: one Tokio runtime per process, not per enrichment/refresh request

---

## Definition of Done

- [ ] Right pane shows "loading..." immediately on navigation, then fills in homepage/license/maintainers
- [ ] ES-fetched data is cached as a JSON file; subsequent views of the same package are instant
- [ ] If ES is unreachable (no network), right pane shows available cached fields without crashing
- [ ] `o` opens the homepage URL in the default browser
- [ ] `?` toggles a readable help overlay
- [ ] Header shows a spinner while background cache refresh is in progress
- [ ] `nix-search --update claude` refreshes cache then opens TUI
- [ ] Exit codes: 0 on selection, 1 on cancel, 2 on hard error
- [ ] Broken packages are visually distinct in the list (once broken flag is available via enrichment)
- [ ] Terminal resize is handled without crashing
