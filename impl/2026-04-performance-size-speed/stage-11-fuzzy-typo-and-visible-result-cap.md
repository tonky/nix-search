# Stage 11: Typo-Tolerant Fuzzy Match and Visible Result Cap

## Objective
Improve search UX so close typos (e.g. `ascinema` -> `asciinema`) still return expected packages, while avoiding unnecessary large result sets beyond what is visible in the list UI.

## Scope

1. Add typo-tolerant candidate admission in web search runtime:
   - include one-edit-distance matches for meaningful query lengths.
2. Cap returned search rows to a visible-list target instead of large static limits.
3. Preserve ranking behavior for exact/prefix/contains matches.

## Proposed Changes

1. `crates/nix-search-web/src/search_runtime.rs`:
   - if a row has zero direct score and query length is sufficiently long, allow one-edit match against package name/attr path with modest score.
   - add unit tests for typo recovery and result cap behavior.
2. `crates/nix-search-web/src/lib.rs`:
   - replace hardcoded search limit with a visible-list cap constant.

## Verification

- `cargo check`
- `cargo test -p nix-search-web`
- `just e2e-test`
- manual: typing `ascinema` should surface `asciinema` in results
