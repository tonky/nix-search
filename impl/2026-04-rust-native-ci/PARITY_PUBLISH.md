# Publish Parity

Track comparisons between the legacy `wasm-data-publish.yml` workflow and the
new Rust/container `wasm-data-publish-rs.yml` workflow during the Stage 7
parity window.

## Comparison Table

| Date | Legacy result | Rust result | Manifest outputs match | Deployed-size delta | Notes |
| --- | --- | --- | --- | --- | --- |
| TBD | not run | not run | not run | not run | Waiting for GHCR publish/bootstrap and the first `workflow_dispatch` smoke run. |

## Run Book

- Record each parity night manually after both workflows finish.
- Fill `Legacy result` and `Rust result` with the GitHub Actions conclusion (`success` or failure summary).
- Compare `version`, `package_count`, and `checksum` from the workflow outputs.
- Measure `dist/` size from the uploaded artifacts and note the percentage delta.
- Add a brief note if the deploy summary or Pages URL differs from the legacy job.

## Criteria

- Both workflows succeed on the same night.
- `version`, `package_count`, and `checksum` outputs match.
- Deployed `dist/` size delta stays under 1%.