# Stage 04 Worklog: CI/CD Scheduled Publish

Date: 2026-04-08
Stage: 04 (CI/CD Publish)
Status: Completed

## Planned Scope

- Add GitHub Actions workflow with schedule + manual trigger.
- Build and run local prep command in CI.
- Publish manifest + versioned artifacts to GitHub Pages.
- Include run summary with version/package_count/checksum.

## Decisions

- Keep Stage 3 prep command (`prep-web`) as the single source for artifact generation.
- Upload generated `tmp/pages-data` directly as Pages artifact.
- Keep immutable naming guarantee by relying on Stage 3 checksum-derived artifact name.
- Emit summary values by parsing `manifest.json` in workflow.

## Implemented

- Added GitHub Actions workflow: `.github/workflows/wasm-data-publish.yml`
	- Triggers:
		- Daily schedule (`cron: 17 3 * * *`)
		- `workflow_dispatch`
	- CI steps:
		- Checkout
		- Install Rust toolchain
		- Cache Cargo
		- Build prep command (`cargo build --bin nix-search`)
		- Run prep command (`cargo run -- prep-web --output tmp/pages-data`)
		- Parse manifest fields (version, package_count, checksum)
		- Upload and deploy Pages artifact
	- Run summary:
		- version/package_count/checksum
		- page URL and manifest URL

## Validation

- Local prep generation for publish directory:
	- `cargo run -- prep-web --output tmp/pages-data`: pass.
	- `ls tmp/pages-data` + manifest inspect: pass.
- Workflow conformance review against Stage 04 checklist: pass.

## Notes

- A critical quoting bug found during review in jq manifest parsing was fixed.
- GitHub-hosted execution checks (Actions run + live Pages URL verification) require repository-side run after push.

## Conformance Review

- Post-stage review executed via `Explore` subagent.
- Outcome: no critical discrepancies.
