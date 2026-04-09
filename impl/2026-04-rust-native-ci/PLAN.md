# CI Migration Plan — Rust-native pipeline in a pre-baked container

Status: **draft for review**
Owner: tonky
Target: replace `.github/workflows/perf-size-budget.yml` and `.github/workflows/wasm-data-publish.yml` with a minimal YAML wrapper around a Rust-native CI binary running inside a pre-built container image.

---

## 1. Goals

- Move all non-trigger CI logic out of YAML and into a statically-typed, testable Rust crate.
- Run byte-identical CI steps locally and in GitHub Actions (`cargo run -p ci -- budget`, or the same command inside the container image).
- Eliminate per-run toolchain installs (Rust, wasm target, trunk, jq, brotli) by baking them into a container image along with a pre-built `ci` binary.
- Make CI changes reviewable as normal Rust PRs with `cargo test` coverage.

## 2. Non-goals

- Replacing GitHub Actions as the trigger/dispatch platform. YAML remains for `on:`, `permissions:`, `concurrency:`, secrets.
- Replacing `actions/checkout`, `actions/configure-pages`, `actions/upload-pages-artifact`, `actions/deploy-pages`. These use GitHub-internal protocols and stay as-is.
- Building a general-purpose pipeline framework. Scope is exactly the two existing workflows.
- Migrating to Nix, Dagger, Earthly, or `act`.

## 3. Current state (snapshot)

Two workflows, both `runs-on: ubuntu-latest`, both reinstalling the world on every run:

- **`perf-size-budget.yml`** (PR gate): checkout → rust toolchain → rust-cache → apt install jq/brotli → install trunk → add wasm target → `cargo run --release -- prep-web` → sync static assets → `scripts/perf/check_budgets.sh` → upload artifacts.
- **`wasm-data-publish.yml`** (nightly + manual): same prep as above → `trunk build --release` → read manifest values → configure-pages → upload-pages-artifact → deploy-pages → step summary.

Shared prep logic is currently duplicated across both YAMLs and partially lives in `scripts/perf/check_budgets.sh`.

## 4. Target architecture

### 4.1 New workspace crate: `crates/ci`

```
crates/ci/
├── Cargo.toml
├── src/
│   ├── main.rs          # clap: `ci budget`, `ci publish`, `ci doctor`
│   ├── lib.rs
│   ├── shell.rs         # Shell trait + RealShell + (cfg(test)) MockShell
│   ├── env.rs           # GHA integration: set_output, summary, group, is_ci
│   ├── manifest.rs      # typed prep-web manifest.json (serde)
│   ├── steps/
│   │   ├── mod.rs
│   │   ├── prep_web.rs
│   │   ├── sync_assets.rs
│   │   ├── trunk_build.rs
│   │   ├── budgets.rs
│   │   └── pages_artifact.rs
│   └── pipelines.rs     # budget() and publish() pipelines
└── tests/
    ├── budget_pipeline.rs
    ├── publish_pipeline.rs
    ├── manifest_parse.rs
    └── budget_thresholds.rs
```

Key crates: `clap` (derive), `xshell`, `anyhow`, `thiserror`, `serde`, `serde_json`, `tracing`, `tracing-subscriber`. Dev: `assert_cmd`, `predicates`, `insta`, `tempfile`.

### 4.2 Binary surface

```
ci budget   [--out DIR] [--perf-mode quick|full]
ci publish  [--out DIR]
ci doctor                 # prints detected tool versions, env, writable paths
```

Everything is a subcommand of one binary so the container only ships one artifact.

### 4.3 Container image: `ghcr.io/tonky/nix-search-ci`

Contents:

- Debian slim base
- Rust stable + `wasm32-unknown-unknown` target (for `trunk build` — the runtime wasm compile still happens at CI time, only the host toolchain is pre-installed)
- `trunk 0.21.14`
- `jq`, `brotli`, `git`, `ca-certificates`
- **Pre-built `/usr/local/bin/ci`** compiled from `crates/ci` at image build time
- Multi-arch: `linux/amd64` (required for GHA) + `linux/arm64` (for local Apple Silicon runs)

Image is tagged with the short SHA of the commit that built it, plus `latest` on main. Workflows pin to a SHA tag to keep runs reproducible; bumps are normal PRs.

### 4.4 New workflow shape

```yaml
# .github/workflows/perf-size-budget.yml
name: Perf and Size Budget
on:
  pull_request:
  workflow_dispatch:
jobs:
  budget:
    runs-on: ubuntu-latest
    container: ghcr.io/tonky/nix-search-ci:<sha>
    steps:
      - uses: actions/checkout@v6
      - run: ci budget --out tmp/bench/perf-size-ci
      - if: always()
        uses: actions/upload-artifact@v7
        with:
          name: perf-size-ci
          path: tmp/bench/perf-size-ci
```

```yaml
# .github/workflows/wasm-data-publish.yml
name: Publish WASM Site
on:
  schedule: [{ cron: "17 3 * * *" }]
  workflow_dispatch:
permissions: { contents: read, pages: write, id-token: write }
concurrency: { group: pages-wasm-site, cancel-in-progress: true }
jobs:
  prepare:
    runs-on: ubuntu-latest
    container: ghcr.io/tonky/nix-search-ci:<sha>
    outputs:
      version:       ${{ steps.run.outputs.version }}
      package_count: ${{ steps.run.outputs.package_count }}
      checksum:      ${{ steps.run.outputs.checksum }}
    steps:
      - uses: actions/checkout@v6
      - id: run
        run: ci publish --out tmp/pages-data
      - uses: actions/configure-pages@v6
      - uses: actions/upload-pages-artifact@v4
        with: { path: crates/nix-search-web/dist }
  deploy:
    needs: prepare
    environment: { name: github-pages, url: ${{ steps.deployment.outputs.page_url }} }
    runs-on: ubuntu-latest
    steps:
      - id: deployment
        uses: actions/deploy-pages@v5
```

Everything else that today lives as YAML steps — manifest parsing, file copying, trunk invocation, budget script, step summary rendering — moves into the `ci` binary.

### 4.5 Image build workflow

New `.github/workflows/ci-image.yml`:

- Triggers on push to `main` affecting `crates/ci/**`, `ci/Dockerfile`, or `Cargo.lock`
- Also on manual `workflow_dispatch`
- Builds `ci` with `cargo build -p ci --release` inside the Dockerfile (multi-stage) so the final image contains only the binary + runtime deps
- Pushes `ghcr.io/tonky/nix-search-ci:<sha>` (consumer workflows pin by SHA; no `:latest` dependency in consumers)
- Updates a `ci/image.sha` pin file on `main` to record the current consumed image SHA. Consumer workflow bumps are manual edits (one file) or auto-pushed by the image workflow — decision made in Stage 4.

Bootstrap: the first image build runs on `ubuntu-latest` directly (not in the container it's building).

## 5. Testability strategy

Three layers:

1. **Unit tests** (`cargo test -p ci`): manifest parsing, budget threshold evaluation, env module (`GITHUB_OUTPUT` writer), CLI arg parsing.
2. **Pipeline tests with `MockShell`**: assert command ordering, assert that `sync_assets` runs after `prep_web`, assert failure propagation, assert `--perf-mode quick` skips the latency probe.
3. **Black-box binary tests** (`assert_cmd`): run `ci doctor` and `ci budget --help`, snapshot-test output with `insta`.

Not covered by tests (explicit): the actual `cargo run -- prep-web` execution, `trunk build` output, real network. These are validated by running the pipeline in CI against a real PR during the parity window.

## 6. Migration phases

### Phase 0 — Prep (no user-visible change)
- Land this plan doc.
- Agree on image registry path and tagging convention.

### Phase 1 — Scaffold `crates/ci`
- Add crate to workspace with `clap` + `xshell` + `anyhow` + `serde`.
- Implement `ci doctor` (prints tool versions). Trivial, but exercises shell + env modules.
- Implement `Manifest` struct + test against a fixture copied from a real `prep-web` run.
- Set up `Shell` trait and `MockShell`.
- CI unchanged; `cargo test -p ci` runs as part of existing workspace tests.

### Phase 2 — Port the budget pipeline
- Implement `steps::prep_web`, `steps::sync_assets`, `steps::budgets`.
- Port `scripts/perf/check_budgets.sh` to Rust as `steps::budgets` (no shell wrapper). Preserve current env-var knobs (`PERF_MODE`, `SEARCH_P95_WARN_MS`, `SEARCH_P95_FAIL_MS`, `RUN_TRUNK_BUILD`, `RUN_LATENCY_PROBE`) as typed `BudgetConfig` fields with matching defaults. Unit-test threshold evaluation against fixture inputs.
- Implement `pipelines::budget()` and `ci budget` subcommand.
- Add `MockShell`-based pipeline tests.
- Run the Rust pipeline locally end-to-end against a checkout; compare output to current YAML run.
- Delete `scripts/perf/check_budgets.sh` in Phase 5 once the cutover is complete.

### Phase 3 — Build and publish the container image
- Add `ci/Dockerfile` (multi-stage: `cargo build -p ci --release` → slim runtime).
- Add `.github/workflows/ci-image.yml`.
- First image published manually to GHCR. Verify `podman run ghcr.io/tonky/nix-search-ci:<sha> ci doctor` locally.

### Phase 4 — Parity window for `perf-size-budget`
- Add new workflow `perf-size-budget-rs.yml` running the containerized Rust pipeline.
- Keep existing `perf-size-budget.yml` running in parallel.
- Require both to pass on PRs for at least 5 successful runs / 1 week, whichever comes later.
- Compare outputs, step summaries, and artifact contents.

### Phase 5 — Cut over `perf-size-budget`
- Delete `perf-size-budget.yml`.
- Rename `perf-size-budget-rs.yml` → `perf-size-budget.yml`.

### Phase 6 — Port the publish pipeline
- Implement `steps::trunk_build`, `steps::pages_artifact_prep`.
- Implement `pipelines::publish()` and `ci publish`.
- Emit `version` / `package_count` / `checksum` via `env::set_output`.
- Render step summary via `env::summary`.

### Phase 7 — Parity window for `wasm-data-publish`
- New workflow alongside the old one, scheduled at an offset cron to avoid collision.
- Verify deployed Pages output manually after first successful run (the deploy is real — parity here means "does the same site end up deployed").
- After ≥ 7 successful nightly runs, cut over.

### Phase 8 — Cleanup
- Delete legacy YAML.
- Delete `scripts/perf/check_budgets.sh` (ported to Rust in Phase 2).
- Update `AGENTS.md` / contributor docs to point at `cargo run -p ci -- <subcommand>` as the local-repro story.

## 7. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Pre-built `ci` binary drifts from `crates/ci` source in PRs | CI image is pinned by SHA; a lint step in `perf-size-budget` re-builds `ci` from source inside the job and compares `--version` hash to the baked binary. Mismatch = fail with "bump ci-image". |
| `actions/deploy-pages` OIDC breaks when the prior job runs in a container | Only the `prepare` job runs in the container. `deploy` job stays on plain `ubuntu-latest`. Artifact is handed off via `upload-pages-artifact`. |
| Rust build time for `ci` crate balloons | Keep deps minimal; `ci` should build in under 30s cold. Reviewed in PR. |
| Local runs on Apple Silicon are better validated with Podman | Local smoke uses `podman build`/`podman run`; GitHub Actions still builds the multi-arch image (`linux/amd64,linux/arm64`) via `docker buildx`. |
| Secrets leak into logs from more verbose Rust error messages | `shell.rs` redacts env vars matching `*TOKEN*`, `*SECRET*`, `*KEY*` in its command-echo output. Covered by a unit test. |
| `check_budgets.sh` has subtle bash behavior we miss when porting | Phase 2 ports the script with unit tests covering each env-var knob and a parity run comparing old-script vs new-Rust output on the same fixture before the script is deleted. |
| Image build workflow itself needs maintenance | It's ~40 lines of YAML, touched only when the Dockerfile or `ci` deps change. Net reduction vs today. |
| GHA `container:` jobs have quirks (HOME, permissions, workdir) | `ci doctor` prints the environment on every run for the first month so regressions are obvious in logs. |

## 8. Resolved decisions

1. **Registry**: `ghcr.io/tonky/nix-search-ci` under the user account.
2. **Pinning**: consumer workflows pin by `<sha>` tag; bumps are normal PRs.
3. **`scripts/perf/check_budgets.sh`**: ported to Rust in Phase 2; shell script deleted in Phase 8.
4. **CLI shape**: explicit `ci budget` / `ci publish` / `ci doctor` subcommands.
5. **Publish parity window**: ≥ 7 successful nightly runs before cutover.
6. **`default-members`**: `crates/ci` excluded from the default workspace build.

## 9. Success criteria

- `.github/workflows/perf-size-budget.yml` is ≤ 20 lines of YAML.
- `.github/workflows/wasm-data-publish.yml` is ≤ 40 lines of YAML.
- `cargo test -p ci` covers: manifest parsing, budget thresholds, pipeline ordering, env output writing, CLI parsing.
- A contributor can reproduce any CI failure locally with a single `docker run` command documented in `AGENTS.md`.
- No step installs a toolchain at runtime in either workflow.
- Nightly publish has run successfully for 7 consecutive days on the new pipeline with no manual intervention.
- Branch protection required-check rules point at the new workflow names (or were preserved via job-id reuse at cutover).
