# Stage 02: Web Shell and Two-Pane Skeleton

Estimated time: 5-6 hours
Depends on: [stage-01-shared-core.md](stage-01-shared-core.md)

## Goal

Scaffold a Rust-first wasm web app with a working two-pane layout and mock data.

## Scope

In scope:
- Create web app package (Leptos + Trunk).
- Render two panes and top-level controls.
- Use static/mock records in-memory.
- Add responsive layout rules for desktop and mobile.

Out of scope:
- Real data sync.
- IndexedDB.
- CI publish.

## Checklist

- [ ] Add web app crate/package to workspace.
- [ ] Add trunk entry point and static assets.
- [ ] Implement left pane (search input + list placeholder).
- [ ] Implement right pane (detail placeholder for selected item).
- [ ] Add header controls placeholder including Refresh Cache button (disabled).
- [ ] Add mobile layout behavior (single-column fallback).

## Validation

Run:

```bash
trunk build --release
```

Manual checks:
- App loads in browser.
- Two-pane layout appears on desktop.
- Mobile viewport stacks correctly.

## Exit Criteria

- A static wasm app shell exists and builds reproducibly.
- UI layout is ready to receive real data from later stages.
