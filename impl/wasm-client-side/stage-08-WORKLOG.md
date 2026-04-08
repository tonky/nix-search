# Stage 08 Worklog: Polish, Hardening, and Docs

Date: 2026-04-08
Stage: 08 (Polish + Hardening)
Status: In Progress

## Planned Scope

- Improve major UI flow states (loading/error/empty) for startup/search/refresh.
- Add basic offline shell behavior.
- Record startup/search latency checks.
- Finalize architecture and troubleshooting documentation.

## Decisions

- Add service worker shell caching as a lightweight strategy for static hosting resilience.
- Keep runtime instrumentation in-app (startup/search latency strip) for reproducible manual checks.
- Focus on documentation handoff rather than introducing new architecture.

## Validation Evidence

- Built web artifact successfully:
	- `cargo check -p nix-search-web --target wasm32-unknown-unknown`
	- `trunk build --release` (from `crates/nix-search-web`)
- Added and ran repeatable probe on realistic prepared dataset (`tmp/pages-data/packages-sha256-1c19b7d5b218.json`, 107164 rows):
	- `cargo run -p nix-search-web --bin latency_probe --release -- --artifact tmp/pages-data/packages-sha256-1c19b7d5b218.json --iterations 50`
	- `cargo run -p nix-search-web --bin latency_probe --release -- --artifact tmp/pages-data/packages-sha256-1c19b7d5b218.json --iterations 100`

Latency baseline (release build, local machine):

- startup_read_ms: 30.33-35.40
- startup_hydrate_ms: 7.95-8.04
- search_avg_ms: 38.43-38.74
- search_p50_ms: 41.65-41.72
- search_p95_ms: 49.71-50.04

## Open Item

- Manual viewport verification (desktop + mobile) is still pending explicit browser-side confirmation.
