# Storage Diagnostics UI

## Problem
Firefox users can end up in session-only cache mode when IndexedDB writes fail due to browser/profile storage constraints. Current UI reports fallback status but does not provide direct diagnostics to identify root cause quickly.

## Goal
Add an in-app storage diagnostics panel in the web shell that surfaces browser storage capability and IndexedDB health so failures can be triaged without external tooling.

## Scope
- Add a diagnostics action in the header.
- Run a lightweight storage probe in browser runtime.
- Display concise diagnostics fields in UI.
- Keep behavior non-blocking and safe in constrained browsers.

## Out of Scope
- Automatic browser setting mutation.
- Large migration of cache schema.
- Server-side diagnostics endpoint.
