# Perf and Size Budget Parity

Track comparisons between the legacy `perf-size-budget.yml` workflow and the
new Rust/container `perf-size-budget-rs.yml` workflow during the Stage 5
parity window.

## Comparison Table

| PR | Legacy result | Rust result | Delta notes |
| --- | --- | --- | --- |
| TBD | not run | not run | Waiting for the first `workflow_dispatch` smoke run and GHCR bootstrap. |

## Criteria

- Exit code matches.
- `report.json` numeric fields stay within the Stage 3 tolerance.
- Intentional budget failures make both workflows fail together.