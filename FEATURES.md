# nix-search — Feature Overview

## Goal

A fast, offline-capable Rust CLI for searching Nix packages. The primary use case: you know roughly what you want (e.g. "claude code", "rust lsp", "postgres 16") and need to quickly identify the right package from a list of candidates — without waiting for Nix to evaluate the world.

Designed to integrate with **flox**: find the right package, print its attr path, pipe it straight into `flox install`.

```bash
flox install $(nix-search claude code)
# or interactively:
nix-search claude code   # TUI opens, you navigate, Enter prints attr, shell substitutes it
```

---

## Data Source & Caching

### What the pkgforge dump actually contains

Verified by fetching the live data. Structure:

```json
{
  "legacyPackages.x86_64-linux.claude-code": {
    "pname": "claude-code",
    "version": "25.4.1",
    "description": "Anthropic's official CLI for Claude AI assistant"
  }
}
```

Each key encodes `legacyPackages.{platform}.{attr}`. That's it — **no** homepage, license, maintainers, broken flag, or last_modified.

**What we get for free from this structure:**
- `attr_path` — the short name after the platform prefix (e.g. `claude-code`, `python312Packages.requests`)
- `platform` — encoded in the key; a package appears once per platform it supports, so we can infer platform support by collecting all keys for the same `attr_path`
- `pname`, `version`, `description`

**What's missing:** homepage, license, maintainers, broken, last_modified.

### Two-source strategy

| Source | Fields | When fetched |
|--------|--------|--------------|
| **pkgforge JSON dump** | pname, version, description, attr_path, platforms (inferred) | At cache build time — full bulk fetch |
| **nixos-search Elasticsearch** | homepage, license, maintainers, broken, last_modified | On demand — fetched for the selected package when the detail pane is focused |

The list pane (search + navigation) runs entirely from the local Tantivy index — fast, offline, no requests. The detail pane fires a single ES query when you land on a package, populating the extra fields. This is a negligible single-record lookup, not a search query, so rate limiting is not a concern.

If ES is unreachable, the detail pane shows what the cache has (name, version, description) and marks extended fields as unavailable.

### Per-Platform Splits

Platform support is inferred from the pkgforge dump keys. If `legacyPackages.x86_64-linux.foo` and `legacyPackages.aarch64-darwin.foo` both exist, `foo` supports both platforms. At index build time, the set of platforms per `attr_path` is collapsed and stored with each indexed package document.

The active platform is auto-detected at startup and used as the default filter.

### Cache Layout

```
~/.cache/nix-search/
  nixos-unstable/
    index/                 # Tantivy index directory
    meta.json              # source URL, fetch time, package count, etag headers
    enriched/              # per-attr enriched ES detail JSON files
  nixos-25.05/             # optional additional channels
```

Indexed package fields:
- `attr_path_exact` (exact match field, non-tokenized)
- `attr_path_text` (tokenized/searchable)
- `pname` (tokenized/searchable)
- `version` (stored)
- `description` (tokenized/searchable)
- `platforms` (stored)

### Cache Update Strategy

Two-tier approach to keep updates fast after the initial fetch:

1. **Full fetch** (first run, or `cache update --full`): download the entire dump (~5–20 MB JSON), parse all keys to extract attr + platform, collapse per-attr platforms, write all documents to Tantivy in one commit.
2. **ETag check** (default on subsequent runs): send `If-None-Match` / `If-Modified-Since` to the pkgforge URL. If the server returns `304 Not Modified`, skip the download entirely.
3. **Safe in-place rebuild** (when changed): open the existing index writer, `delete_all_documents()`, add rebuilt documents, commit, then reload readers. Never delete the index directory while the app is running.
4. **Staleness check**: on each invocation, if cache age > TTL (default 24h), a background refresh starts while search runs against existing snapshots. A spinner in the TUI header indicates refresh activity.

No delta endpoint exists — the dump is a full snapshot each time — but ETag avoids the download cost when nothing has changed (the dump updates every 2–3 hours, so a 24h TTL means usually 1 re-download per day).

---

## Search

### Query Modes
1. **Fuzzy full-text** (default) — Tantivy BM25 with field boosts and fuzzy fallback
2. **Exact attr path** — `nix-search nixpkgs#python312` resolves directly without fuzzy scoring
3. **Multi-word** — all terms ANDed; order doesn't matter ("code claude" == "claude code")

### How Fuzzy Search Works

Two-stage pipeline for speed + quality:
1. **Tantivy BM25 query** over `pname + description + attr_path_text` with field boosts (name/path weighted above description)
2. **Tantivy fuzzy fallback** using `FuzzyTermQuery` for typo tolerance when BM25 returns weak/no matches

Everything is in-process and embedded in the binary — no subprocess, no network call.

### Ranking (composite score)
- Tantivy relevance score (BM25)
- **Name / attr bonus** — `pname` and `attr_path` matches weighted above `description`
- **Platform match bonus** — packages supporting the current platform ranked above others (see Platform Filtering)

For v1, `broken` and `last_modified` are detail metadata and are not guaranteed to be globally available at query time, so they are not part of the primary ranking formula.

---

## Platform Filtering

Default behavior: filter results to packages that support the **current system** (auto-detected via `std::env::consts::ARCH` + `OS`, mapped to Nix platform string e.g. `aarch64-darwin`).

Override flags:
```
--platform <PLATFORM>     Show packages for this platform instead [e.g. x86_64-linux]
--all-platforms           Disable platform filtering entirely
```

In the TUI, the active platform is shown in the header. Packages that don't list the current platform (but aren't explicitly broken) are shown below a visual separator with a dim style, rather than hidden entirely — so you can still find them if needed.

---

## TUI — Interactive Mode (default)

Launched when stdout is a TTY. Two-pane layout:

```
┌─────────────────────────────────────────────────────────────────────┐
│ > claude code_                    [unstable | aarch64-darwin] 12/98k│
├──────────────────────────────┬──────────────────────────────────────┤
│ claude-code          25.4.1  │ attr:     nixpkgs.claude-code        │
│ claude-desktop       0.3.2   │ version:  25.4.1                     │
│ anthropic-cli        1.2.0   │ updated:  2026-03-28                 │
│ aider                0.52.0  │                                      │
│ continue             2.1.4   │ desc:     Anthropic's official CLI   │
│ ...                          │          for Claude AI assistant.    │
│                              │                                      │
│ ── other platforms ──        │ homepage: github.com/anthropic/...   │
│ claude-nix (x86_64-linux)    │ license:  MIT                        │
│                              │ maintainers: @user1 @user2           │
│                              │ platforms: linux, darwin             │
│                              │ broken:   no                         │
└──────────────────────────────┴──────────────────────────────────────┘
  ↑/↓ navigate  Enter copy attr  Tab switch pane  ^P platform  ? help
```

### Behavior
- Search updates incrementally as you type (~80ms debounce)
- Left pane: result list with name + version; platform-filtered results first, then a dimmed separator + cross-platform results below
- Right pane: full metadata for highlighted package; updates on navigation
- Header shows: channel, active platform, match count / total indexed
- Background cache refresh shown as a spinner in the header if running

### Keybindings

| Key | Action |
|-----|--------|
| type | Refine search |
| `↑/↓` | Navigate results |
| `Enter` | Print attr path to stdout + exit (for shell integration) |
| `Tab` | Focus right pane (scroll long descriptions) |
| `Ctrl+P` | Cycle / toggle platform filter |
| `Ctrl+U` | Clear query |
| `o` | Open homepage in browser (when right pane focused) |
| `?` | Toggle help overlay |
| `Esc` / `q` | Quit without printing |

On `Enter`: the TUI clears, attr path is written to stdout (e.g. `claude-code`), and the process exits 0. Quitting without selecting exits 1 so scripts can detect cancellation.

**TUI always renders on stderr, result always goes to stdout.** This is the same approach as `fzf`. When stdout is captured by `$(...)`, the TUI still draws correctly to the terminal (via stderr), and only the selected attr path ends up in the substitution. No jank.

---

## Non-interactive / Pipe Mode

When stdout is not a TTY (or `--json` / `--plain` flag is passed), the TUI is skipped and results go directly to stdout:

```bash
nix-search --plain claude code    # plain text table of matches
nix-search --json claude code     # JSON array of matching packages
nix-search --first claude code    # print only the top attr path (for scripting)
nix-search --all-platforms rust   # skip platform filter
```

Shell integration patterns:
```bash
flox install $(nix-search --first claude code)
nix shell nixpkgs#$(nix-search --first rust analyzer)
nix-search --json python | jq '.[].attr_path'
```

---

## CLI Interface

```
nix-search [OPTIONS] [QUERY...]

Options:
  -c, --channel <CHANNEL>    Channel to search [default: nixos-unstable]
  -n, --limit <N>            Max results [default: 50 in TUI, 20 in pipe]
  -p, --platform <PLATFORM>  Filter to platform [default: current system]
  --all-platforms            Disable platform filtering
  --update                   Force cache refresh before searching
  --json                     JSON output (implies non-interactive)
  --plain                    Plain text table output
  --first                    Print only the top result's attr path (for scripting)
  --attr <ATTR>              Lookup exact attribute path
  --cache-dir <DIR>          Override cache directory
  --ttl <SECONDS>            Cache staleness threshold [default: 86400]
  -h, --help                 Show help
```

---

## Cache Management Subcommand

```
nix-search cache status              # show channel, size, age, package count
nix-search cache update              # refresh active channel
nix-search cache update --all        # refresh all cached channels
nix-search cache update --full       # force full re-download (not incremental)
nix-search cache clear               # delete all local cache
```

---

## Tech Stack (Rust)

| Concern | Crate |
|---------|-------|
| TUI | `ratatui` |
| Embedded search index | `tantivy` |
| HTTP client | `reqwest` (async) |
| Async runtime | `tokio` |
| JSON | `serde_json` + `serde` |
| Clipboard | `arboard` |
| CLI parsing | `clap` |
| XDG paths | `dirs` |

---

## Deferred to v2

- **Interactive channel cycling** — deferred to v2. v1 supports explicit `--channel <CHANNEL>` usage; channel switching inside the TUI is intentionally omitted.
- **nix-index integration** — "which package provides binary X" subcommand using `nix-index-database`. Separate use case, separate index.
- **Global broken/freshness ranking** — `broken` and `last_modified` are not present in the pkgforge dump. They can be added later by materializing those fields into the local index during cache refresh. For v1, ranking is purely relevance + name/attr boost + platform split.

## Open Questions

1. **Fuzzy fallback thresholds** — tune when fuzzy fallback triggers (e.g. no hits vs low-confidence hits) to balance typo tolerance and precision.
2. **Detail pane ES query format** — confirm stable endpoint and field mappings for single-attr lookups; keep runtime fallback logic if backend version/path changes.
