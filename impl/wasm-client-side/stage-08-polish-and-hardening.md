# Stage 08: Polish, Hardening, and Docs

Estimated time: 5-6 hours
Depends on: [stage-07-refresh-flow.md](stage-07-refresh-flow.md)

## Goal

Finalize reliability, baseline performance checks, and operator/user documentation.

## Scope

In scope:
- Error boundary polish and resilience.
- Basic offline shell behavior.
- Measured startup/search latency checks.
- Documentation for runbook and architecture.

Out of scope:
- Major architecture rewrites.
- New product features.

## Checklist

- [x] Add user-facing error/empty/loading states for all major flows.
- [x] Add basic offline shell caching strategy (if selected in implementation).
- [x] Measure and record startup and query latency with realistic data.
- [ ] Verify mobile and desktop layout stability.
- [x] Update docs: architecture, CI cadence, refresh model, troubleshooting.
- [x] Final stage review against [PLAN.md](PLAN.md) and [STAGES.md](STAGES.md).

## Validation

Manual checks:
- App remains usable after offline toggle with existing local cache.
- Search responsiveness is acceptable on representative hardware.
- Documentation is sufficient for another contributor to operate and debug the flow.

## Practical Pre-GitHub Gate

- [ ] Confirm desktop and mobile viewport checks are completed and noted in Stage 08 worklog.
- [ ] Run manual refresh scenarios (unchanged/new-version/failure fallback) and record outcomes.
- [ ] Run storage diagnostics in Firefox and Safari/WebKit and record origin + IndexedDB probe results.
- [ ] Verify local loop commands are stable:
	- `just prep-web tmp/pages-data force=0`
	- `just verify-manual`
- [ ] Re-run compile/test sanity before GitHub testing:
	- `cargo check --workspace`
	- `cargo check -p nix-search-web --target wasm32-unknown-unknown`
	- `just e2e-test` (if local browser tooling is available)

## Exit Criteria

- End-to-end browser app is production-ready for OSS static hosting.
- Stage files and plan are aligned and complete.
