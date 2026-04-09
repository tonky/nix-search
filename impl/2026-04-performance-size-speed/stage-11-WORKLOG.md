# Stage 11 WORKLOG

## 2026-04-09

- Trigger: user reports typo query (`ascinema`) should match `asciinema`.
- Additional ask: avoid returning more rows than needed for visible list UI.
- Plan:
  - add typo-admission in initial scoring so rerank can act on near-miss terms,
  - cap search result limit to visible-list-oriented value,
  - add tests to lock in behavior.

## Implementation

- `crates/nix-search-web/src/search_runtime.rs`:
  - added one-edit typo admission for longer queries in candidate scoring.
  - added helper `is_within_one_edit`.
  - added tests for one-edit matching, typo query recovery, and global limit enforcement.
- `crates/nix-search-web/src/lib.rs`:
  - replaced hardcoded search limit with `SEARCH_VISIBLE_RESULT_LIMIT = 48`.

## Validation

- `cargo check -q && cargo test -q -p nix-search-web` passed.
- `just e2e-test` passed (8/8).

## Stage status

- complete
