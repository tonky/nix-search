# Stage 01 Worklog: Shared Core Extraction

Date: 2026-04-08
Stage: 01 (Shared Core Extraction)
Status: Completed

## Scope Implemented

- Created shared crate boundary: `crates/nix-search-core`.
- Extracted reusable pure/domain modules into shared core:
  - `types` (Package, CacheMeta, EnrichedDetails, EsConfig)
  - `parse` (parse_key, parse_dump)
  - `platform` (detect_current_platform)
  - `split` (generic platform partition helper)
  - `search` (SearchConfig, ScoredPackage, SearchResults, overfetch policy, global result limit, reranking)
- Rewired native app to consume shared core via re-exports/adapters.
- Kept native-only concerns out of shared core (tantivy, ratatui/crossterm, reqwest/tokio, filesystem cache code).

## Native/WASM Boundary Decisions

- Added target-specific dependency gating in root `Cargo.toml`:
  - Native-only dependencies moved under `cfg(not(target_arch = "wasm32"))`.
- Added module gating in `src/lib.rs`:
  - Native-only modules (`cache`, `output`, `search`, `tui`) compiled only for non-wasm targets.
  - Shared modules remain available cross-target (`types`, `platform` + core split re-export).
- Added target-specific entrypoints in `src/main.rs`:
  - Native CLI unchanged for non-wasm.
  - wasm32 placeholder main emits explicit unsupported message.

## Tradeoffs

- Kept Tantivy query execution in native crate for Stage 01 to avoid over-coupling shared core to backend-specific search infra.
- Moved orchestration/rerank policy into core now to maximize future web reuse without introducing browser-runtime dependencies.
- Used re-exports for behavior-preserving migration with minimal callsite churn.

## Validation Executed

- `cargo test` (root): pass.
- `cargo test -p nix-search-core`: pass.
- `cargo check --target wasm32-unknown-unknown` (root): pass.
- `cargo check -p nix-search-core --target wasm32-unknown-unknown`: pass.

## Conformance Review

- Ran post-stage review via `Explore` subagent against Stage 01 plan/checklist.
- Review outcome: no critical discrepancies.
- Follow-up fix applied from review: re-exported core split utilities from top-level crate (`src/lib.rs`).

## Exit Criteria Check

- Native app still builds/tests: yes.
- Shared core exports minimal reusable APIs for next web stages: yes.
- Shared core compiles for wasm target: yes.
