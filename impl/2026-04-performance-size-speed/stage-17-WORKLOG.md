# Stage 17 WORKLOG

## 2026-04-09

- Trigger: user requested CI-time reduction narrowed strictly to Trunk install path.
- Plan:
  - replace source-based Trunk install (`cargo install`) with a prebuilt installer action,
  - keep all other workflow behavior unchanged,
  - verify locally, including optional `act` run before push.

## Implementation

- `.github/workflows/perf-size-budget.yml`:
  - replaced `cargo install trunk --version 0.21.14` with:
    - `uses: taiki-e/install-action@v2`
    - `with: tool: trunk@0.21.14`
- `.github/workflows/wasm-data-publish.yml`:
  - replaced `cargo install trunk --version 0.21.14` with the same pinned action install.

## Validation

- workflow diff verification:
  - both workflows now use `taiki-e/install-action@v2` with `tool: trunk@0.21.14`.
  - no remaining `cargo install trunk` references in workflow files.
- local `act` availability:
  - not on base shell PATH.
  - available in flox dev environment: `flox activate -c 'act --version'` -> `act version 0.2.84`.
- local workflow check:
  - `flox activate -c 'act pull_request -W .github/workflows/perf-size-budget.yml -j budget -n'` succeeded.
  - dry-run logs include `Main Install Trunk` using `taiki-e/install-action` and overall `Job succeeded`.
  - note: `act` reported local upgrade advisory (>= 0.2.86).

## Stage status

- complete