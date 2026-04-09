# Stage 7 Worklog

- 2026-04-09: Added the parallel Rust publish workflow as `wasm-data-publish-rs.yml` with a parity-window cron offset.
- 2026-04-09: Added `PARITY_PUBLISH.md` to track legacy-vs-Rust deployment comparisons during the Stage 7 window.
- 2026-04-09: Kept the legacy publish workflow and cleanup tasks untouched until nightly parity is proven.
- 2026-04-09: Stage 7 still depends on the GHCR image publish/bootstrap before the new workflow can actually run.
- 2026-04-09: Added a deploy-job summary step to mirror the legacy Pages metadata output.
- 2026-04-09: First `workflow_dispatch` smoke run and OIDC scope verification are still pending maintainer bootstrap because the GHCR image has not been published yet.
- 2026-04-09: The Rust publish step now receives a deterministic `PAGE_URL` for the repo Pages site so its summary can include manifest links even before deploy.