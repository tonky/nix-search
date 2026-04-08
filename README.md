# nix-search

![Interactive TUI demo](demo/nix-search-demo.gif)

Fast CLI for searching Nix packages from a local cache.

Disclaimer: this was power-coded with LLM over a couple of hours.

## Demo (VHS)

Interactive TUI demo is shown above and recorded with VHS.

Regenerate it with:

```bash
vhs demo/nix-search.tape
```

## Purpose

`nix-search` helps you quickly find the correct Nix attr path (for `nix shell`, `nix profile install`, `flox install`, etc.) without waiting on full Nix evaluation each time.

## Capabilities

- Offline-first search from a local Tantivy index
- Fuzzy matching for typos and partial queries
- Exact attr lookup via `--attr`
- Interactive TUI when run in a terminal
- Script-friendly output modes: `--first`, `--plain`, `--json`
- Platform-aware ranking/filtering, with `--all-platforms` override
- Cache management commands: `cache update`, `cache status`, `cache clear`

## Toolchain

This repository uses `rustup` with pinned settings from `rust-toolchain.toml`.

- Channel: `stable`
- Components: `rustfmt`, `clippy`
- Target: `wasm32-unknown-unknown`

If you already have `rustup` installed, opening the repo is enough for Cargo commands to pick up the toolchain override.

## WASM Docs

For browser-side architecture, operations, and troubleshooting notes, see:

- `impl/wasm-client-side/RUNBOOK.md`
- `impl/wasm-client-side/PLAN.md`
- `impl/wasm-client-side/STAGES.md`

## Examples

Build/update cache:

```bash
nix-search cache update
```

Interactive search (TUI):

```bash
nix-search claude code
```

Print top match only (good for shell scripts):

```bash
nix-search --first "claude code"
```

Search across all platforms (useful when current platform hides expected packages):

```bash
nix-search --all-platforms --first "cld cod"
```

Exact attr lookup:

```bash
nix-search --attr claude-code --first x
```

JSON output:

```bash
nix-search --json ripgrep
```

Use with `nix shell`:

```bash
nix shell nixpkgs#$(nix-search --first "rust analyzer")
```

## How It Works

### Data Sources

1. Package snapshot (primary source):
	- URL: `https://raw.githubusercontent.com/pkgforge-dev/NixOS-Packages/main/nixpkgs.json`
	- Used for searchable package records: attr path, pname, version, description, and platform inference from keys like `legacyPackages.<platform>.<attr>`.
2. NixOS Search Elasticsearch (enrichment source, optional):
	- Endpoint currently resolved from built-in candidates under `https://search.nixos.org/backend/.../_search`.
	- Used only for detail metadata (homepage, license, maintainers, broken, longDescription).

### Exactly When Each Source Is Called

1. Package snapshot is called when:
	- `nix-search cache update` is run.
	- `nix-search --update ...` is used before search.
	- TUI starts and cache is older than `--ttl`; refresh runs in background.
2. Package snapshot request behavior:
	- HTTP GET with conditional headers (`If-None-Match`, `If-Modified-Since`) from stored metadata.
	- If server returns `304 Not Modified`, no body is downloaded and cache timestamps/headers are refreshed.
	- If changed, JSON is parsed, grouped by attr path, and rebuilt into the local Tantivy index.
3. Elasticsearch is called only during TUI detail loading:
	- On selection change, local enriched JSON cache is checked first.
	- If missing and ES config is available, one POST request is sent with `size: 1` and term query for the selected attr.
	- Response is cached to disk per attr path for instant reuse.

### Local Storage and Reuse

1. Tantivy index and metadata are stored per channel under `~/.cache/nix-search/<channel>/`.
2. Enriched details are cached as per-package JSON files under `~/.cache/nix-search/<channel>/enriched/`.
3. Normal CLI searches (`--first`, `--plain`, `--json`) read only local index data unless `--update` is explicitly requested.

### Query Execution Path

1. Try exact attr lookup first (from `--attr` or `nixpkgs#attr` style input).
2. Otherwise run BM25 search on attr path, pname, and description.
3. Apply fuzzy fallback and reranking heuristics.
4. Apply platform split/filter (current platform by default, or `--all-platforms`).
5. Render as interactive TUI or non-interactive output mode.

This design keeps search fast and mostly offline after cache build, while still allowing on-demand rich metadata in the TUI.
