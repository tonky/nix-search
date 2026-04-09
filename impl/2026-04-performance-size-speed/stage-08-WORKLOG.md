# Stage 08 WORKLOG

## 2026-04-09

- Trigger: user reports search input still lagging while typing.
- Goal: decouple input and other UI/IO updates from search recomputation.
- Implemented in `crates/nix-search-web/src/lib.rs`:
  - introduced async scheduled search recomputation (`TimeoutFuture::new(0)`) so browser can process input/paint before search work.
  - added generation-based stale-job dropping via `search_job_epoch` so outdated query tasks do not update UI.
  - changed search input to uncontrolled (removed `prop:value`) to reduce controlled-input update pressure.
- Next: run compile/tests/E2E and collect latency probe sample.

## Validation

- `cargo check -q` passed.
- `cargo test -q -p nix-search-web` passed.
- `just e2e-test` passed (8/8).
- `just latency-probe-latest iterations=200`:
  - `rows=142840`
  - `startup_read_ms=61.40`
  - `startup_hydrate_ms=8.12`
  - `search_avg_ms=25.74`
  - `search_p50_ms=28.96`
  - `search_p95_ms=34.23`

## Stage status

- complete

## Review

- Subagent conformance review verdict: PASS.
- No required fixes.
- Suggested follow-ups captured for direct input-latency measurement and rapid-fire query stress coverage.