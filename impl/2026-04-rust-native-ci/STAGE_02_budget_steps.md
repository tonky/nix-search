# Stage 2 — Budget pipeline: prep, sync, pipeline wiring

**Estimate:** ~7h
**Depends on:** Stage 1
**User-visible change:** none (still no workflow touches; `ci budget` runnable locally)

## Goal

Implement the non-budget-script parts of the budget pipeline — `prep_web`,
`sync_assets` — and wire them into `pipelines::budget()` + the `ci budget`
subcommand. `ci budget` runs end-to-end locally up to (but not including) the
budget-check itself, which is stubbed and lands in Stage 3.

## Scope

- `steps/prep_web.rs`: invokes `cargo run --release -- prep-web --output <out>`,
  then loads and returns a typed `Manifest`.
- `steps/sync_assets.rs`: the jq + cp + rm dance from the current YAML, as
  typed Rust. Inputs: `Manifest`, `pages_data_dir`, `web_static_dir`. Operations:
  - create `crates/nix-search-web/static/data/`
  - remove stale `packages-*.json` and `packages-*.json.br`
  - copy `manifest.json`
  - copy the artifact referenced by `manifest.artifact`
  - copy `manifest.compressed_artifact` if present
- `pipelines.rs`: `budget()` returns a `Pipeline` with ordered steps
  `prep-web → sync-assets → budgets (stub)`. `Pipeline::run(&dyn Shell, &Ctx)`.
- `ci budget [--out DIR] [--perf-mode quick|full]` subcommand wires
  everything and exits non-zero on first step failure, with `anyhow::Context`
  around each command (cwd + full argv in the error chain).
- `steps/budgets.rs`: stub that logs "budget check stubbed — see Stage 3" and
  returns Ok. Lands for real in Stage 3.

## Tests

- `pipeline_ordering.rs`: `MockShell` records calls; assert `prep_web` runs
  before `sync_assets`; assert failure in `prep_web` short-circuits the
  pipeline.
- `sync_assets_fs.rs`: use `tempfile::tempdir` for both `pages_data` and
  `web_static`; seed fake `manifest.json` + referenced artifact; assert the
  final layout matches expectations, including stale-file cleanup.
- `sync_assets_fs_no_compressed.rs`: same but with no `compressed_artifact`
  field — must not fail.
- `error_context.rs`: when `prep_web` fails (mocked), the error message
  includes the argv and the working directory.

## Acceptance

- `cargo test -p ci` passes.
- On the maintainer's machine, `cargo run -p ci -- budget --out tmp/bench/ci-stage2`
  runs the real `prep-web`, syncs assets, and exits 0 at the stub.
- The resulting `crates/nix-search-web/static/data/` matches what the current
  YAML produces (spot-check manifest + packages file presence).

## Out of scope

- Porting `scripts/perf/check_budgets.sh` (Stage 3).
- Container image (Stage 4).
- Any workflow changes.
