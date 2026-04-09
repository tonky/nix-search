# Stage 07: Refresh + Interactive Search Regression Fix

## Objective
Address post-optimization regressions where browser refresh became slower and interactive search became visibly laggy.

## Scope

1. Reduce CPU and allocation pressure in interactive search runtime.
2. Reduce unnecessary cloning in web selection/update effects.
3. Remove refresh-path work that increases wall time without affecting visible correctness.

## Proposed Changes

1. `search_runtime`:
   - score rows by index first, clone package payloads only after top candidates are selected.
2. `lib`:
   - avoid repeated full `search_results` cloning in selection logic.
3. `cache_sync`:
   - disable synchronous stale-prune pass from critical refresh path (keep refresh completion focused on fetch + write).

## Verification

- `cargo check`
- web unit tests (`nix-search-web`)
- Playwright smoke run
- manual validation: perf-strip `search:` latency should improve vs current regressed behavior.
