# Stage 15 WORKLOG

## 2026-04-09

- Trigger: severe interactive latency regression reported (`rip` -> `ripg` around 1622 ms).
- Hypothesis:
  - fuzzy candidate admission introduced expensive per-row operations for short queries,
  - compact normalization and similarity checks are being recomputed per query across large row sets.
- Plan:
  - precompute compact fields on row creation,
  - aggressively gate fuzzy admission for short queries,
  - keep long-query typo behavior.

## Implementation

- `crates/nix-search-web/src/search_runtime.rs`:
  - added precomputed `attr_compact` and `pname_compact` in `SearchRow` to avoid per-query normalization allocations.
  - gated one-edit fuzzy admission from `>=5` to `>=6` query length.
  - gated subsequence fuzzy admission to `>=5` query length with cheap preconditions:
    - first-byte equality
    - bounded length spread
  - removed expensive similarity/levenshtein admission from the hot path.

## Validation

- `cargo check -q && cargo test -q -p nix-search-web` passed.
- `just latency-probe-latest iterations=200`:
  - `search_avg_ms=25.68`
  - `search_p50_ms=28.63`
  - `search_p95_ms=34.17`
- `just e2e-test` passed (10/10).

## Stage status

- complete
