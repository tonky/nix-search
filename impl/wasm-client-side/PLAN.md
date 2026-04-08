# WASM Client-Side Plan

Date: 2026-04-08
Scope: Browser UI only (not CLI/TUI)
Target: Fully client-side static app on GitHub Pages

## Goal

Build a two-pane browser UI for package search that runs entirely client-side with WASM, supports fuzzy search, and includes a manual Refresh Cache button.

## Product Requirements

1. Two-pane layout:
- Left pane: query input + fuzzy results list.
- Right pane: selected package details.

2. Data behavior:
- App works from local browser cache after initial sync.
- Manual Refresh Cache button checks for new prepared dataset and updates local cache.

3. Hosting:
- Static deployment on GitHub Pages as an OSS project.

4. Runtime model:
- No server search API.
- Browser-only execution.

## Architecture Decision

Use CI/CD to pre-build and publish prepared data artifacts, then let the browser app sync those artifacts into IndexedDB.

Why:
- Avoid large/expensive parse and grouping work at page load.
- Keep browser app fast and predictable.
- Make updates operationally simple (scheduled + manual).

## Data Pipeline (CI/CD)

### Inputs
- Upstream source: pkgforge nixpkgs JSON snapshot.

### CI Jobs
1. Scheduled daily job.
2. Manual dispatch job for urgent updates.

### CI Steps
1. Fetch upstream dump (with conditional request headers when possible).
2. Parse and group by attr path (same semantics as current parse logic).
3. Emit prepared browser artifact(s), versioned by timestamp/hash.
4. Emit latest manifest with metadata:
- version
- checksum
- package_count
- built_at
- source_etag/last_modified (if available)

### Publish
- Deploy manifest + versioned artifacts to GitHub Pages.
- Cache policy:
- Manifest: no-cache.
- Versioned data files: long immutable cache.

## Browser Storage Strategy

1. Primary store: IndexedDB.
2. Store content:
- Package records.
- Metadata record (version, package_count, fetched_at, checksum).

3. Startup sync flow:
1. Read local metadata.
2. Fetch remote manifest.
3. If same version: load local data.
4. If new version: download prepared artifact, transactional replace, then load.

4. Manual Refresh Cache flow:
1. Force manifest check.
2. If unchanged: show up-to-date status.
3. If changed: download + replace + reindex in memory.
4. Show progress and error states.

## Search Strategy

Phase 1 (MVP):
- In-memory fuzzy ranking over loaded package records.
- Preserve existing matched vs others platform split behavior.

Phase 2 (if needed):
- Move heavy search/index work to Web Worker.
- Add optional compact index artifact if latency targets are missed.

## UI Plan

### Core UX
- Left pane:
- Query input.
- Result rows with attr path + version.
- Group separator for other platforms.

- Right pane:
- Description.
- Platforms.
- Enriched metadata if available.

- Header/footer controls:
- Refresh Cache button.
- Cache status (ready/updating/error, version, package count).

### Framework
- Rust-first web UI with Leptos + WASM.

## Code Reuse Plan

Reuse first:
- types models.
- parse/grouping logic.
- platform split logic.
- ranking heuristics where browser-safe.

Replace for web target:
- ratatui/crossterm rendering.
- filesystem cache operations.
- tokio runtime thread worker pattern.
- native reqwest/tls assumptions.

## Repository Strategy

Recommended sequence:
1. Keep work in this repository first to maximize code reuse and regression checks.
2. Split into a dedicated web repository only after core/shared boundaries are stable.

## Milestones

M1: Foundation
1. Define shared core crate boundaries.
2. Compile shared domain logic for native and wasm targets.
3. Scaffold Leptos app shell with two-pane layout.

M2: Data Pipeline
1. Implement CI data-prep script/job.
2. Publish manifest + versioned artifacts to GitHub Pages.
3. Add retention policy for old artifacts.

M3: Browser Cache + Search
1. IndexedDB persistence.
2. Startup sync logic.
3. Fuzzy search + two-pane result rendering.

M4: Refresh + Polish
1. Manual Refresh Cache button and progress UI.
2. Error handling and recovery flows.
3. Mobile layout and performance tuning.

## Verification Checklist

1. Data prep reproducibility:
- Same input -> stable checksum/artifact.

2. Startup behavior:
- Cold start downloads and stores data.
- Warm start uses local cache.

3. Refresh behavior:
- No-op when no new version.
- Safe replacement when new version exists.

4. Search quality:
- Typo/partial queries produce expected top results.
- Platform split behavior matches current logic.

5. Hosting:
- GitHub Pages serves app and data correctly.
- Cache headers behave as intended.

## Sizing Expectations (Current)

- Network transfer for full prepared dataset: roughly 4-12 MB compressed.
- Browser local storage after grouping: roughly 25-60 MB in IndexedDB.

These numbers should be re-measured after first concrete artifact format is implemented.

## Risks and Mitigations

1. Risk: Large initial sync on slow networks.
- Mitigation: show progress, support cancel/retry, optimize artifact format.

2. Risk: Search latency on low-end devices.
- Mitigation: debounce input, cap result count, move heavy compute to worker.

3. Risk: Browser storage eviction.
- Mitigation: detect missing cache and recover automatically.

4. Risk: Drift from current ranking behavior.
- Mitigation: add regression query fixtures and compare top-N outputs.

## Next Implementation Tasks

1. Create a shared domain crate for types + parse + platform split.
2. Add CI workflow that prepares and publishes manifest/artifacts.
3. Scaffold Leptos two-pane UI with local fake data.
4. Implement IndexedDB sync layer and wire Refresh Cache button.
