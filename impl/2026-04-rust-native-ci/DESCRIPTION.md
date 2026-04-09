# Rust-native CI pipeline in a pre-baked container

## Problem

Maintaining and debugging GitHub Actions YAML is painful. The two current workflows
(`.github/workflows/perf-size-budget.yml` and `.github/workflows/wasm-data-publish.yml`)
duplicate prep logic, reinstall the full toolchain (Rust, wasm target, trunk, jq, brotli)
on every run, and rely on stringly-typed step outputs that are hard to test or reproduce
locally. Changes to CI require push-and-pray iteration.

## Idea

Move all non-trigger CI logic out of YAML and into a statically-typed, testable Rust
crate inside this workspace. A pre-built container image carries the toolchain **and**
the compiled `ci` binary, so GHA jobs reduce to a ~10-line wrapper that runs
`ci budget` or `ci publish` inside the container.

The same command runs locally via `docker run` (or `cargo run -p ci`), giving
byte-identical behavior between a developer laptop and CI.

## Why Rust

- The project already builds Rust — no new toolchain for contributors.
- Static types catch the exact class of bugs YAML hides: step input/output contracts,
  env var names, file paths, manifest schema.
- `cargo test` gives real unit + integration tests for pipeline logic, with a
  `MockShell` for asserting command ordering and error propagation.
- One binary, two runtimes (local and CI) — no "works on my machine" delta.

## Scope

**In scope**
- New `crates/ci` workspace crate with `clap` subcommands `ci budget`, `ci publish`,
  `ci doctor`.
- New container image `ghcr.io/tonky/nix-search-ci` containing the toolchain and a
  pre-compiled `ci` binary at `/usr/local/bin/ci`.
- Image build workflow that rebuilds and publishes the image when `crates/ci/**` or
  the Dockerfile changes, and bumps the SHA pin in consumer workflows.
- Rewritten `perf-size-budget.yml` and `wasm-data-publish.yml` as thin wrappers.
- Parity windows running old and new workflows in parallel before cutover.

**Out of scope**
- Replacing GHA as the trigger/dispatch platform. `on:`, `permissions:`,
  `concurrency:`, and secrets stay in YAML.
- Replacing `actions/checkout`, `actions/configure-pages`, `actions/upload-pages-artifact`,
  `actions/deploy-pages`. They use GitHub-internal protocols and remain as actions.
- A general-purpose pipeline framework. Scope is exactly the two existing workflows.
- Alternatives considered and rejected: Nix devshell for CI, Dagger, Earthly, `act`,
  `cargo-make` / `just`.

## Initial discussion notes

- **Trigger: user pain with GHA YAML.** "Maintaining and debugging yaml in GHA isn't
  fun." First option explored was a minimalistic bash wrapper in a container, which
  solved reproducibility but not typing/testability.
- **Upgrade to Rust wrapper.** Since the project is already Rust, adding a `ci` crate
  costs nothing in toolchain, gives static types and `cargo test`, and makes CI
  changes reviewable as normal Rust PRs.
- **Pre-baked binary.** To avoid paying Rust compile time for the `ci` crate on every
  CI run, the binary is baked into the container image during image build, not
  rebuilt per job. A drift-check step compares the baked `--version` hash against a
  fresh build in PRs to catch source/image mismatches.
- **GitHub Pages caveat.** `actions/deploy-pages` + `upload-pages-artifact` use OIDC
  and GitHub-internal artifact protocols — they stay as actions. Only the
  `prepare` job runs inside the container; the `deploy` job stays on plain
  `ubuntu-latest`.
- **Parity windows.** Both workflows get parallel runs of old + new before cutover.
  Budget workflow: ≥ 5 successful runs / 1 week. Publish workflow: ≥ 7 successful
  nightly runs (deploy is real, so parity means "same deployed site").
- **`check_budgets.sh`.** First ported as a shell wrapper called from `steps::budgets`.
  A Rust port is deferred to a follow-up.

## Success criteria

- `perf-size-budget.yml` ≤ 20 lines of YAML.
- `wasm-data-publish.yml` ≤ 40 lines of YAML.
- `cargo test -p ci` covers manifest parsing, budget thresholds, pipeline ordering,
  env output writing, and CLI parsing.
- A contributor can reproduce any CI failure locally with a single documented
  `docker run` command.
- No step installs a toolchain at runtime in either workflow.
- Nightly publish has run successfully for 7 consecutive days on the new pipeline
  with no manual intervention.

## Resolved decisions

1. **Registry**: `ghcr.io/tonky/nix-search-ci` under the user account.
2. **Pinning**: consumer workflows pin the image by `<sha>` tag. Bumps are normal PRs.
3. **`scripts/perf/check_budgets.sh`**: ported to Rust as part of this effort
   (not kept as a shell wrapper). The shell script is deleted once the Rust port
   is green.
4. **CLI shape**: explicit `ci budget` / `ci publish` / `ci doctor` subcommands.
5. **Publish parity window**: ≥ 7 successful nightly runs before cutover.
6. **Workspace `default-members`**: `crates/ci` is excluded so root-level
   `cargo build` does not pull in `clap` et al.
