# Stage 14 WORKLOG

## 2026-04-09

- Trigger: user requested partial typo-like prefixes (`ascin`, `ascine`, `ascinem`) to fuzzy-match `asciinema`.
- Plan:
  - extend search candidate admission with compact subsequence/similarity gate,
  - keep threshold tight to avoid noisy overmatching,
  - add focused unit tests for these queries.

## Implementation

- `crates/nix-search-web/src/search_runtime.rs`:
  - expanded fuzzy candidate admission with compact subsequence/similarity gating for query length >= 4.
  - retained stricter one-edit path for query length >= 5.
  - added focused tests to ensure `ascin`, `ascine`, and `ascinem` each surface `asciinema`.

## Validation

- `cargo check -q && cargo test -q -p nix-search-web` passed.
- `just e2e-test` passed (10/10).

## Stage status

- complete
