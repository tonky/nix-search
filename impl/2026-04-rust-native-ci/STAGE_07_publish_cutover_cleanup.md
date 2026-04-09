# Stage 7 — Publish parity, cutover, and final cleanup

**Estimate:** ~4h active work, plus ≥ 7 nightly runs of calendar time
**Depends on:** Stage 6
**User-visible change:** `wasm-data-publish.yml` migrated to the Rust/container
pipeline; legacy YAML and `scripts/perf/check_budgets.sh` deleted.

## Goal

Deploy the new publish pipeline behind a parity window, verify ≥ 7
consecutive successful nightly runs produce equivalent GitHub Pages output,
then cut over and delete all legacy artifacts.

## Scope

### Phase 7a — add parallel publish workflow

New `.github/workflows/wasm-data-publish-rs.yml`:

```yaml
name: Publish WASM Site (rs)
on:
  schedule:
    - cron: "47 3 * * *"   # offset from legacy 17 3 * * * to avoid collision
  workflow_dispatch:
permissions: { contents: read, pages: write, id-token: write, packages: read }
concurrency: { group: pages-wasm-site-rs, cancel-in-progress: true }
jobs:
  prepare:
    runs-on: ubuntu-latest
    container: ghcr.io/tonky/nix-search-ci:<pinned-sha>
    outputs:
      version:       ${{ steps.run.outputs.version }}
      package_count: ${{ steps.run.outputs.package_count }}
      checksum:      ${{ steps.run.outputs.checksum }}
    steps:
      - uses: actions/checkout@v6
      - run: scripts/ci/verify-image.sh
      - id: run
        run: ci publish --out tmp/pages-data
      - uses: actions/configure-pages@v6
      - uses: actions/upload-pages-artifact@v4
        with: { path: crates/nix-search-web/dist }
  deploy:
    needs: prepare
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    runs-on: ubuntu-latest
    steps:
      - id: deployment
        uses: actions/deploy-pages@v5
```

**Important**: the `rs` workflow uses a **separate Pages concurrency group**
(`pages-wasm-site-rs`) and **deploys to the same Pages site**. During parity,
the last workflow to finish wins the published site. That's acceptable
because parity checks the content is equivalent — whichever wins should
produce an indistinguishable site.

Alternative if parity collisions cause real site flapping: deploy the `rs`
pipeline to a preview environment instead. Decision in Stage 7 kick-off.

### Phase 7b — parity tracking

`impl/2026-04-rust-native-ci/PARITY_PUBLISH.md` with a table of
(date, legacy result, rs result, manifest checksums match, deployed-size delta).

Parity criteria:
- Both workflows succeed on the same night.
- `version`, `package_count`, `checksum` outputs match.
- `dist/` size delta < 1%.

### Phase 7c — cutover

When ≥ 7 consecutive nights show passing parity:

- Delete `.github/workflows/wasm-data-publish.yml`.
- Rename `wasm-data-publish-rs.yml` → `wasm-data-publish.yml`. Keep job
  ids (`prepare`, `deploy`) so any required-check rules survive.
- Restore the original cron (`17 3 * * *`) and concurrency group
  (`pages-wasm-site`).
- Update branch protection if the old workflow's checks were required —
  re-point at the new workflow name.
- Merge the cutover PR at a time when no nightly run is in flight
  (mid-day UTC) to avoid concurrency-group collisions.

### Phase 7d — final cleanup

- Delete `scripts/perf/check_budgets.sh` (ported in Stages 3a/3b; parity
  gate is the `parity_vs_shell` test green in Stage 3b).
- Delete `.github/workflows/ci-parity-once.yml` (its job is done).
- Update `AGENTS.md` contributor section with the new local-repro story:
  `docker run --rm -v $PWD:/w -w /w ghcr.io/tonky/nix-search-ci:<sha> budget`.
- Extend `crates/ci/README.md` (stub created in Stage 1) with the
  stabilized usage + image pin story.
- Archive parity tables into `WORKLOG.md`.
- Review `FOLLOW_UP.toml` per AGENTS.md and file deferred items. Candidates:
  image-bump auto-PR bot, porting log formatting parity, removing rust
  toolchain duplication from the container image.

## Tests

- Manual verification of deployed site after first `rs` nightly: fetch
  `page_url`, check `data/manifest.json` equals the workflow output.
- `gh api` spot-check of Pages deployment history to confirm both
  workflows wrote equivalent deployments during parity.

## Acceptance

- ≥ 7 consecutive successful `rs` nightly runs with parity green.
- Cutover PR merges cleanly.
- `wasm-data-publish.yml` is ≤ 40 lines of YAML (per PLAN §9).
- `scripts/perf/check_budgets.sh` deleted.
- `cargo test -p ci` still passes.
- `main` is stable for 48h post-cutover.
- All PLAN.md §9 success criteria met.

## Out of scope

- Any further Rust CI extensions — those go in `FOLLOW_UP.toml` for later
  features.

## Risks

- **Same-site double deploy during parity** — mitigated by acceptance that
  parity content is equivalent; fallback is preview environment.
- **OIDC token scope issues** when `prepare` runs in a container and
  `deploy` does not — verified in Stage 5 pattern, but the Pages token
  flow is different from artifact upload. Test in `workflow_dispatch` first
  before relying on the nightly schedule.
- **Concurrency group rename** at cutover could theoretically conflict with
  an in-flight legacy run. Schedule the cutover PR merge for a time when no
  nightly is running (e.g., mid-day UTC).
