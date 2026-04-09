# Stage 07 WORKLOG

## 2026-04-09

- Trigger: user reported regressions in both refresh latency and interactive search responsiveness.
- Investigation:
  - Found clone-heavy search scoring path in `search_runtime` (full package clones before top-N cut).
  - Found repeated `search_results.get()` cloning in selection/effect paths.
  - Found stale-key prune pass in refresh critical path adding extra IndexedDB scan/delete work.
- Planned fixes:
  - rank-by-index and clone-late in search runtime.
  - use `search_results.with(...)` in selection/effect logic.
  - skip stale prune in foreground refresh path.

## Implementation

- `crates/nix-search-web/src/search_runtime.rs`:
  - switched query scoring pipeline to rank rows by index first and clone package payloads only after top candidates are selected.
- `crates/nix-search-web/src/lib.rs`:
  - removed repeated full `search_results.get()` cloning in selected-row and selected-existence effects.
  - switched results render mapping to `search_results.with(...)` to avoid cloning the whole `SearchResults` container per render.
- `crates/nix-search-web/src/cache_sync.rs`:
  - gated stale-key prune out of foreground refresh path (`ENABLE_STALE_PRUNE_ON_REFRESH = false`) to reduce refresh wall time.

## Validation

- `cargo check -q` passed.
- `cargo test -q -p nix-search-web` passed.
- `just e2e-test` passed (8/8).
- `just latency-probe-latest iterations=200`:
  - `rows=142840`
  - `startup_read_ms=61.44`
  - `startup_hydrate_ms=8.31`
  - `search_avg_ms=25.55`
  - `search_p50_ms=28.54`
  - `search_p95_ms=33.93`

## Stage status

- complete

## Review

- Subagent conformance review verdict: PASS.
- Noted tradeoff: stale-key prune is deferred from foreground refresh; tracked in `FOLLOW_UP.toml` for background/periodic maintenance design.
