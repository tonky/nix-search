# Stage 5 Worklog

- 2026-04-09: Added the Rust/container parity workflow as `perf-size-budget-rs.yml` with `workflow_dispatch` only for the initial smoke period.
- 2026-04-09: Added `PARITY_BUDGET.md` to track legacy-vs-Rust comparisons during the Stage 5 window.
- 2026-04-09: Kept the legacy `perf-size-budget.yml` untouched for now; cutover will wait for the required parity history.
- 2026-04-09: Stage 5 still depends on the GHCR image being published before the new workflow can actually run.