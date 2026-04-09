# Stage 09 WORKLOG

## 2026-04-09

- Trigger: user requested stronger input/IO decoupling with Elm-like architecture.
- Plan:
  - introduce reducer-style search message queue and async dispatcher,
  - route input/debounce/search-recompute through this queue,
  - keep refresh/startup logic unchanged for low-risk incremental adoption.

## Implementation

- `crates/nix-search-web/src/lib.rs`:
  - added `SearchMsg` reducer events (`InputChanged`, `DebouncedQueryReady`, `RunSearch`, `SearchComputed`).
  - added queue-backed async dispatcher (`Rc<RefCell<VecDeque<SearchMsg>>>`) that drains on next tick.
  - moved input mutation and debounced query commit through dispatcher instead of direct signal writes.
  - moved search recompute trigger and completion application through reducer events.
  - preserved stale-job suppression via epoch check in `SearchComputed` handling.

## Validation

- `cargo check -q` passed.
- `cargo test -q -p nix-search-web` passed.
- `just e2e-test` passed (8/8).
- `just latency-probe-latest iterations=200`:
  - `rows=142840`
  - `startup_read_ms=61.29`
  - `startup_hydrate_ms=7.52`
  - `search_avg_ms=25.91`
  - `search_p50_ms=29.04`
  - `search_p95_ms=34.27`

## Stage status

- complete

## Review

- Subagent conformance review verdict: PASS.
- No required fixes.
- Optional follow-ups remain: browser input-to-render latency probe and rapid-fire query stress E2E.
