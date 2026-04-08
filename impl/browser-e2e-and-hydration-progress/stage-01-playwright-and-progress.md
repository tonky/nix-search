# Stage 01: Playwright Baseline + Hydration Progress Bar

## Scope

- Introduce Playwright config + smoke tests with `firefox` and `webkit` projects.
- Wire test server startup to existing local prep and Trunk serving.
- Add a non-blocking progress bar for background hydration in UI.
- Add `just` recipe for E2E execution.

## Validation

- `cargo check -p nix-search-web --target wasm32-unknown-unknown`
- `just e2e-install` (dependency/browsers install)
- `just e2e-test` (firefox + webkit)

## Exit Criteria

- Automated browser smoke suite runs without manual interaction.
- Background hydration progress is visible and non-blocking.
