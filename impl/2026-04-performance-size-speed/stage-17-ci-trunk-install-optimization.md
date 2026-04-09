# Stage 17: CI Trunk Install Optimization

## Objective
Reduce CI wall time by replacing source-based Trunk installation with a prebuilt installer action while preserving pinned version reproducibility.

## Scope

1. Replace `cargo install trunk --version 0.21.14` with action-based install in current workflows.
2. Keep downstream wasm target setup and existing build/perf steps unchanged.
3. Verify locally before push, including optional `act` run.

## Proposed Changes

1. `.github/workflows/perf-size-budget.yml`:
   - switch Trunk installation step to `taiki-e/install-action@v2` with `tool: trunk@0.21.14`.
2. `.github/workflows/wasm-data-publish.yml`:
   - apply the same installer pattern for consistency.

## Verification

1. Confirm both workflows no longer build Trunk from source in CI.
2. Confirm follow-on steps still invoke `trunk` unchanged.
3. Local pre-push check:
   - `flox activate -c 'act pull_request -W .github/workflows/perf-size-budget.yml -j budget -n'`

## Estimated Effort
1-2 hours