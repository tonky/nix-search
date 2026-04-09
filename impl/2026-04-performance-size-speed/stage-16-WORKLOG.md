# Stage 16 WORKLOG

## 2026-04-09

- Trigger: user called out missed performance regression detection.
- Observation:
  - current perf budget script treats `search_p95_ms` as warning-only.
- Plan:
  - add fail threshold for full-mode CI,
  - keep warning threshold for observability,
  - expose both in summary output.

## Implementation

- `scripts/perf/check_budgets.sh`:
  - added hard fail threshold support: `SEARCH_P95_FAIL_MS`.
  - full mode default fail threshold set to `120ms`.
  - quick mode default fail threshold disabled (`0`) to stay dev-friendly.
  - retained warn threshold (`SEARCH_P95_WARN_MS`) and now report both thresholds in summary.
- `.github/workflows/perf-size-budget.yml`:
  - CI full-mode run now sets explicit thresholds:
    - `SEARCH_P95_WARN_MS=45`
    - `SEARCH_P95_FAIL_MS=120`
- `README.md`:
  - documented hard search-latency gate in full mode.

## Validation

- `PERF_MODE=quick RUN_LATENCY_PROBE=1 SEARCH_P95_WARN_MS=45 SEARCH_P95_FAIL_MS=120 scripts/perf/check_budgets.sh ...` passed.
- summary includes `search_p95_warn_ms` and `search_p95_fail_ms` fields.
- `cargo check -q` passed.

## Stage status

- complete
