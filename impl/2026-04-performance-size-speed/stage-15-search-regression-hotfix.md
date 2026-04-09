# Stage 15: Search Regression Hotfix After Fuzzy Expansion

## Objective
Restore interactive search latency after fuzzy admission expansion, while preserving key typo behavior.

## Scope

1. Remove high-cost fuzzy admission work from short queries where it is not needed.
2. Eliminate per-search allocations for compact attr/pname forms.
3. Keep critical typo behavior for longer queries (e.g., `ascinema`, `ascinem`) intact.

## Proposed Changes

1. `search_runtime`:
   - precompute compact normalized fields in `SearchRow` once at hydration time.
   - gate fuzzy admission paths by query length and cheap preconditions (first-char/length spread).
   - avoid Levenshtein-based admission for short queries.
2. Preserve existing rerank pipeline and result cap.

## Verification

- `cargo check`
- `cargo test -p nix-search-web`
- `just latency-probe-latest iterations=200`
- `just e2e-test`
