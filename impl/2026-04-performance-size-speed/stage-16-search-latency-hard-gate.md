# Stage 16: Search Latency Hard Regression Gate

## Objective
Prevent severe search-latency regressions from merging by converting latency probe checks from warning-only to fail-capable thresholds in perf budget checks.

## Scope

1. Add hard fail threshold for `search_p95_ms` in `scripts/perf/check_budgets.sh`.
2. Keep warning threshold for softer alerting.
3. Wire explicit threshold values in CI workflow.
4. Keep quick mode developer-friendly while full mode enforces gate.

## Verification

- run budget script with probe enabled and ensure summary includes fail/warn thresholds
- `cargo check`
- existing E2E remains unaffected
