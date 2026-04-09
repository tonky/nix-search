# Stage 1 — Scaffold `crates/ci`

**Estimate:** ~6h
**Depends on:** none
**User-visible change:** none (new crate, no workflow touches)

## Goal

Add a new Rust workspace crate that compiles, runs `ci doctor`, and has the core
abstractions (`Shell` trait, `MockShell`, `Env`, `Manifest`) in place with unit
tests. No real pipeline logic yet.

## Scope

- Add `crates/ci/` to the workspace.
- `Cargo.toml` deps: `clap` (derive), `xshell`, `anyhow`, `thiserror`, `serde`,
  `serde_json`, `tracing`, `tracing-subscriber`. Dev: `assert_cmd`, `predicates`,
  `insta`, `tempfile`.
- Exclude `crates/ci` from root `default-members` so `cargo build` at the repo
  root does not pull in `clap` et al.
- Module layout per PLAN §4.1.
- Implement:
  - `shell.rs`: `Shell` trait, `RealShell` (backed by `xshell`), `MockShell`
    behind `#[cfg(test)]` (or a `testing` feature). Command-echo redacts env
    vars matching `*TOKEN*`, `*SECRET*`, `*KEY*`.
  - `env.rs`: `set_output(key, val)`, `summary(md)`, `group(name)`, `is_ci()`.
    Writes to `$GITHUB_OUTPUT` / `$GITHUB_STEP_SUMMARY` when present, falls
    back to stdout locally.
  - `manifest.rs`: `Manifest` struct mirroring `tmp/pages-data/manifest.json`
    (fields: `version`, `package_count`, `checksum`, `artifact`,
    `compressed_artifact`). Serde derive.
  - `main.rs`: `clap` derive with subcommands `budget`, `publish`, `doctor`.
    `budget` and `publish` print "not implemented" for now.
  - `ci doctor`: prints tool versions (`rustc`, `cargo`, `trunk`, `jq`, `brotli`,
    `git`), env summary, and writable path checks.

## Tests

- `manifest_parse.rs`: load a real fixture from `tests/fixtures/manifest.json`
  (copy from a local `prep-web` run). Round-trip parse.
- `shell_redaction.rs`: `MockShell` + an env with `GITHUB_TOKEN=abc` — the
  recorded command echo must not contain `abc`.
- `env_output.rs`: `set_output` appends `k=v\n` to a tempfile when
  `GITHUB_OUTPUT` points at it; prints to stdout otherwise.
- `cli_parse.rs` (via `assert_cmd`): `ci --help`, `ci budget --help`,
  `ci doctor` all exit 0 and contain expected strings. `insta` snapshot
  for `ci --help`.

## Acceptance

- `cargo build -p ci --release` succeeds.
- `cargo test -p ci` passes.
- `cargo run -p ci -- doctor` prints a sensible report on the maintainer's
  machine.
- `cargo build` at repo root does **not** compile `crates/ci` (verified by
  running `cargo build -v` at the root and confirming no `Compiling ci`
  line, or by inspecting `cargo metadata --format-version 1` output).
- `crates/ci/README.md` stub created (one paragraph pointing at
  `impl/2026-04-rust-native-ci/` for history and design rationale).

## Out of scope

- Any real pipeline steps (`prep_web`, `sync_assets`, `budgets`).
- The container image.
- Any workflow changes.
