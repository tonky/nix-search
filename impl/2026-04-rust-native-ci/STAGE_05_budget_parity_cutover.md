# Stage 5 — `perf-size-budget` parity window and cutover

**Estimate:** ~5h active work, plus ≥ 1 week of calendar time for parity runs
**Depends on:** Stage 4
**User-visible change:** new `perf-size-budget-rs.yml` runs on PRs alongside
the legacy workflow; at end of stage the legacy is deleted.

## Goal

Run the new Rust/container pipeline in parallel with the legacy YAML on PRs,
verify parity over ≥ 5 successful runs / ≥ 1 week, then cut over and delete
the legacy workflow.

## Scope

### Phase 5a — add parallel workflow

New `.github/workflows/perf-size-budget-rs.yml`:

```yaml
name: Perf and Size Budget (rs)
on:
  workflow_dispatch:        # day 1: dispatch only — smoke test first
  # pull_request:           # flip on after first successful dispatch run
permissions:
  contents: read
  packages: read            # pull GHCR image
jobs:
  budget:
    runs-on: ubuntu-latest
    container: ghcr.io/tonky/nix-search-ci:__SHA_FROM_ci/image.sha__
    env:
      PERF_MODE: full
      SEARCH_P95_WARN_MS: 45
      SEARCH_P95_FAIL_MS: 120
    steps:
      - uses: actions/checkout@v6
      - run: scripts/ci/verify-image.sh   # drift check
      - run: ci budget --out tmp/bench/perf-size-ci
      - if: always()
        uses: actions/upload-artifact@v7
        with:
          name: perf-size-ci-rs
          path: tmp/bench/perf-size-ci
```

Notes:
- `__SHA_FROM_ci/image.sha__` is a placeholder — real workflow substitutes
  the actual SHA. GHA doesn't support reading a file into `container:` at
  parse time, so the PR that lands this workflow also hard-codes the SHA;
  bumps happen via PRs touching both `ci/image.sha` and any workflow
  files that reference it. A short `scripts/ci/bump-image.sh` helper in
  Stage 4 makes this a one-command edit.
- Env vars mirror the **exact** overrides from the legacy YAML
  (`.github/workflows/perf-size-budget.yml:52-54`). Any drift between
  these env values and `BudgetConfig` defaults is intentional and
  covered by the `defaults_match_shell` test from Stage 3a.
- **Day-1 gating**: workflow starts as `workflow_dispatch`-only to catch
  container/permission issues without hammering every open PR. After the
  first clean dispatch run, a follow-up commit enables `pull_request`.

Legacy `perf-size-budget.yml` stays untouched and continues to run.

### Phase 5b — parity tracking

Add `impl/2026-04-rust-native-ci/PARITY_BUDGET.md` with a table of
(PR #, legacy result, rs result, delta notes) updated as runs land.

Parity criteria:
- Exit code matches.
- `report.json` numeric fields within the tolerance documented in Stage 3.
- Failure cases (intentional budget blowouts in a test branch) produce
  matching fail/pass decisions.

### Phase 5c — cutover

When ≥ 5 successful runs across ≥ 1 week have landed:
- Delete `.github/workflows/perf-size-budget.yml`.
- Rename `perf-size-budget-rs.yml` → `perf-size-budget.yml`. **Keep the
  job id** (`budget`) so branch protection required-check rules don't
  need renaming. If the job id differs, update the rule in the same PR.
- **Update branch protection**: verify the required-check name matches
  the new workflow's job name. If the legacy workflow was a required
  check under the old name, add the new check as required and remove
  the old one in the same admin action.
- Single cleanup PR. Parity table archived in `WORKLOG.md`.

## Tests

- Manual: open a draft PR that intentionally breaks a budget (e.g. adds a
  huge static file) and confirm **both** workflows fail on it.
- Manual: open a clean PR and confirm both pass.

## Acceptance

- ≥ 5 successful parallel runs, ≥ 1 calendar week since first parallel run.
- First `workflow_dispatch` smoke run passes before `pull_request` trigger
  is enabled.
- Canary PR (long-lived draft) accumulates at least 3 comparison runs
  across the parity window.
- Cutover PR merges cleanly.
- Branch protection required-check name updated (or unchanged because job
  id was preserved).
- `perf-size-budget.yml` is ≤ 20 lines of YAML post-cutover (per PLAN §9).
- No unintended regressions on `main` in the first 48h after cutover.

## Out of scope

- Deleting `scripts/perf/check_budgets.sh` (Stage 7 cleanup).
- Publish workflow migration (Stage 6).

## Risks

- **Parity window is too short.** If fewer than 5 PRs land in a week, wait
  for more rather than cutting over early.
- **`container:` job quirks on GHA.** If `HOME` / workdir / permissions
  inside the container cause unexpected failures, investigate and fix
  rather than bypassing with `runs-on: ubuntu-latest`. Document fixes
  in `WORKLOG.md`.
- **Drift-check false positives.** If `verify-image.sh` flags drift on a
  clean PR, the CI image is stale — bump it via a normal `ci-image.yml`
  run before retrying.
