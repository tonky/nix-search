# Stage 6 — Publish pipeline in Rust

**Estimate:** ~7h
**Depends on:** Stage 5 (so the pattern is proven on the lower-risk workflow
first)
**User-visible change:** none (new `ci publish` subcommand, no workflow touches)

## Goal

Implement `pipelines::publish()` and the `ci publish` subcommand, covering
everything the current `wasm-data-publish.yml` `prepare` job does **except**
the three GitHub-owned Pages actions (`configure-pages`, `upload-pages-artifact`,
`deploy-pages`), which stay as YAML.

## Scope

### New steps

- `steps/trunk_build.rs`: runs
  `cd crates/nix-search-web && trunk build --release --public-url /nix-search/`
  with heartbeat logging.
- `steps/pages_artifact_prep.rs`: ensures `crates/nix-search-web/dist` exists
  and is ready for `actions/upload-pages-artifact` to pick up. (This is just
  a sanity check — `trunk build` produces it.)
- `steps/manifest_outputs.rs`: reads the manifest and emits
  `version` / `package_count` / `checksum` via `env::set_output`.
- `steps/publish_summary.rs`: renders the markdown step summary currently
  emitted by the `deploy` job's shell block and writes it via `env::summary`.

### Pipeline wiring

`pipelines::publish()` in order:

1. `prep_web` (reused from Stage 2)
2. `sync_assets` (reused from Stage 2)
3. `trunk_build`
4. `pages_artifact_prep`
5. `manifest_outputs`
6. `publish_summary`

### CLI

`ci publish [--out DIR]` wires the pipeline. Default `--out tmp/pages-data`
matching the legacy YAML.

## Tests

- `publish_pipeline_ordering.rs`: `MockShell` asserts order.
- `manifest_outputs.rs`: given a manifest fixture and a tempfile pointed at
  by `GITHUB_OUTPUT`, assert the file contains the expected `k=v` lines.
- `publish_summary_snapshot.rs`: `insta` snapshot of the markdown summary.
- `publish_pipeline_error.rs`: failure in `trunk_build` short-circuits and
  the error contains the build log path.

## Acceptance

- `cargo test -p ci` passes.
- On maintainer's machine,
  `cargo run -p ci -- publish --out tmp/pages-data-stage6` produces a
  `crates/nix-search-web/dist/` matching the legacy YAML output (spot-check
  file list and size).
- Manifest outputs written to a local `$GITHUB_OUTPUT` file mirror what the
  legacy workflow emits.

## Out of scope

- New publish workflow (Stage 7).
- Deleting the legacy workflow (Stage 7).
- Rebuilding the container image if only source changed — Stage 4's
  `ci-image.yml` handles that automatically.

## Risks

- **`trunk build` heartbeat output** not parity-checked against the legacy
  step logs. Only the resulting `dist/` matters.
- **Public-URL drift**: `/nix-search/` is hardcoded in both the legacy YAML
  and the new step. If the repo is ever renamed, both must change. Document
  in `steps/trunk_build.rs` with a `TODO(repo-rename)` comment.
