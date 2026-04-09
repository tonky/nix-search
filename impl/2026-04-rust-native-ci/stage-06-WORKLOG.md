# Stage 6 Worklog

- 2026-04-09: Started the Rust publish pipeline port from the legacy `wasm-data-publish.yml` prepare path.
- 2026-04-09: Added `trunk_build`, `pages_artifact_prep`, `manifest_outputs`, and `publish_summary` steps.
- 2026-04-09: Wired `ci publish --out DIR` to run the publish pipeline with the default `tmp/pages-data` output path.
- 2026-04-09: Added publish ordering, output, summary, and error-path tests, plus a help snapshot update for the new `publish --out` CLI surface.