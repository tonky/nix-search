# Stage 02 Worklog: Web Shell and Two-Pane Skeleton

Date: 2026-04-08
Stage: 02 (Web Shell)
Status: Completed

## Planned Scope

- Add Leptos + Trunk web crate to workspace.
- Implement two-pane responsive shell with mock in-memory package data.
- Provide left search pane, right detail pane, and disabled Refresh Cache control.
- Keep real sync/indexeddb/CI work out of this stage.

## Decisions

- Create workspace crate `crates/nix-search-web` to keep web scaffold isolated from native CLI.
- Reuse `nix-search-core::types::Package` in mock data to keep data model continuity.
- Use static stylesheet and responsive CSS breakpoint for mobile single-column fallback.

## Implemented

- Added workspace member `crates/nix-search-web`.
- Added Trunk entrypoint and static assets:
	- `index.html`
	- `static/app.css`
- Implemented Leptos app shell:
	- Header with disabled `Refresh Cache` placeholder.
	- Left pane with search input and mock results list.
	- Right pane with selected package detail placeholder.
- Added in-memory mock dataset with matched/other platform grouping label.
- Added mobile fallback via CSS media query (`max-width: 860px`) to stack panes.

## Validation

- `cargo check -p nix-search-web --target wasm32-unknown-unknown`: pass.
- `trunk build --release`: pass.
- `trunk serve --port 8081 --address 127.0.0.1`: pass (app served locally).

## Notes

- Trunk required explicit binary selection in `index.html` (`data-bin="nix-search-web"`) because both lib and bin targets exist.
- Browser page launch was executed; agentic browser tools were not enabled for DOM/content introspection, so visual verification is limited to successful serve/open smoke check.

## Conformance Review

- Post-stage review executed via `Explore` subagent against Stage 02 checklist.
- Outcome: no critical discrepancies.
