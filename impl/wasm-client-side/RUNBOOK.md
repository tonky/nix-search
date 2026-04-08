# WASM Web Runbook

## Architecture Snapshot

Web crate: `crates/nix-search-web`

Main runtime layers:

1. `cache_sync.rs`
- IndexedDB stores:
  - `packages` (keyed by `attr_path`)
  - `meta` (single key: `current`)
- Startup sync: local cache load -> manifest check -> optional artifact download -> transactional replace.
- Manual refresh: forced manifest check with safe fallback to previous local data.

2. `search_runtime.rs`
- In-memory query execution over cached packages.
- Uses shared-core search helpers for overfetch/rerank/platform-split/global-limit consistency.

3. `lib.rs`
- Leptos UI state and orchestration.
- Debounced input search.
- Matched/others rendering.
- Refresh UX and status messaging.

4. Shell resilience
- `static/sw.js` caches basic shell resources for offline app shell behavior.

## CI Cadence

Workflow: `.github/workflows/wasm-data-publish.yml`

- Scheduled daily run and manual dispatch.
- Generates `manifest.json` + versioned package artifact.
- Publishes to GitHub Pages.

## Refresh Model

Button: "Refresh Cache"

Flow:

1. Force remote manifest fetch.
2. If version unchanged: no heavy work, report up-to-date.
3. If version changed: fetch artifact and transactional replace in IndexedDB.
4. On any failure: keep previous in-memory/local cache usable, report failure.

## Troubleshooting

1. Startup says no cache/offline
- Cause: first run without network or missing manifest/artifact.
- Action: connect network and refresh/reload.

2. Refresh fails but data remains
- Expected behavior: previous usable cache is retained.
- Check browser console and network tab for manifest/artifact fetch errors.

3. Stale shell assets after deploy
- Hard refresh browser tab.
- Check service worker registration and cache entries.

4. Empty search results unexpectedly
- Verify data loaded (`rows` count in perf strip).
- Confirm platform toggle/state is expected.

## Local Verification Commands

From repo root:

```bash
cargo check -p nix-search-web --target wasm32-unknown-unknown
```

From web crate:

```bash
trunk build --release
trunk serve --port 8081 --address 127.0.0.1
```

## Pre-GitHub Test Gate

Before running repository-side GitHub validation, complete this gate:

1. Manual viewport verification
- Check desktop and mobile layouts in browser for Stage 08 shell stability.

2. Refresh runtime scenarios
- Validate unchanged-manifest refresh path.
- Validate changed-manifest refresh path.
- Validate failure path where previous cache remains usable.

3. Storage diagnostics sanity
- Run diagnostics once in Firefox and once in Safari/WebKit.
- Confirm current origin is expected and IndexedDB write probe result is recorded.

4. Local commands sanity
- `just prep-web tmp/pages-data force=0` reuses cached artifact.
- `just verify-manual` starts cleanly with no rapid rebuild loop.

5. Build/test sanity
- `cargo check --workspace`
- `cargo check -p nix-search-web --target wasm32-unknown-unknown`
- `just e2e-test` (when browser environment/tooling is available locally)
