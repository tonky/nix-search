# Stage 03 Worklog: Local Data Preparation Pipeline

Date: 2026-04-08
Stage: 03 (Local Data Prep)
Status: Completed

## Planned Scope

- Add a local prep command to fetch upstream snapshot and emit local web artifacts.
- Reuse shared parse/group logic from nix-search-core.
- Emit versioned artifact and manifest with version/checksum/package_count/built_at.
- Validate via local run into tmp/wasm-data.

## Decisions

- Implement as a native CLI subcommand (`prep-web`) to keep workflow simple for local and future CI invocation.
- Keep artifact format plain JSON (`PreparedData { packages }`) for inspectability in this stage.
- Use SHA-256 checksum-derived version string (`sha256-<12 hex>`) to provide deterministic versioning from content.
- Keep retries basic (fixed linear backoff) and timeout explicit.

## Implemented

- Added native module `src/prep.rs` with:
	- Upstream fetch with timeout and retry.
	- Shared parse/group reuse via `nix_search_core::parse::parse_dump`.
	- Artifact emission to `packages-<version>.json`.
	- Manifest emission to `manifest.json` with required fields.
- Added CLI subcommand `prep-web` in `src/main.rs`:
	- `cargo run -- prep-web --output tmp/wasm-data`
- Added deterministic fixture-based tests for transform/checksum behavior and manifest fields.

## Validation

- `cargo test`: pass.
- `cargo run -- prep-web --output tmp/wasm-data`: pass.
- `ls tmp/wasm-data`: pass.

Observed output sample:

- `manifest.json`
- `packages-sha256-1c19b7d5b218.json`

Manifest fields populated:

- `version`: `sha256-1c19b7d5b218`
- `checksum`: populated SHA-256 hex string
- `package_count`: populated (107164 in this run)
- `built_at`: populated epoch timestamp

## Conformance Review

- Post-stage review executed via `Explore` subagent against Stage 03 checklist.
- Outcome: no critical discrepancies.
