# Stage 08: Input/IO Decoupling for Responsive Typing

## Objective
Keep text input responsive while search, hydration, and refresh logic are active by decoupling keystroke handling from expensive search/result recomputation.

## Scope

1. Reduce work done on each query update by cloning only final visible results.
2. Add cooperative scheduling for search recomputation so input events can paint first.
3. Keep functional behavior and ranking parity with current search semantics.

## Proposed Changes

1. `search_runtime`:
   - perform ranking on row references/indices and materialize packages only for capped result lists.
2. `lib`:
   - move search recomputation into async scheduled task (next tick), replacing direct synchronous effect compute.
   - preserve cancellation semantics via generation counter so stale query jobs are dropped.
3. Verification:
   - unit tests for search parity-sensitive branches where touched.
   - `cargo check`, web tests, and E2E smoke.

## Verification

- `cargo check`
- `cargo test -p nix-search-web`
- `just e2e-test`
- `just latency-probe-latest` with before/after search p95 notes in WORKLOG
