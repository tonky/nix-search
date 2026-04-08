# Stage 06: Fuzzy Search and Results Rendering

Estimated time: 5-6 hours
Depends on: [stage-05-browser-cache-sync.md](stage-05-browser-cache-sync.md)

## Goal

Wire real query behavior to cached data and render matched vs others in two panes.

## Scope

In scope:
- Query input debounce.
- Fuzzy matching/ranking implementation.
- Platform split rendering (`matched` then `others`).
- Selection and detail pane binding.

Out of scope:
- Refresh mutation flows.
- Service worker/offline shell optimization.

## Checklist

- [x] Hook input state to search execution path.
- [x] Implement fuzzy search/ranking strategy for browser runtime.
- [x] Apply platform split semantics consistent with existing behavior.
- [x] Render result separators and empty states.
- [x] Keep interaction responsive under expected dataset size.
- [x] Add regression query fixture list for manual verification.

## Validation

Manual checks using known queries:
- Typo query finds expected package near top.
- Multi-word query ranks relevant packages first.
- Platform toggle changes `matched` vs `others` sections as expected.

## Exit Criteria

- Two-pane UI is functionally useful for package search.
- Search behavior is close enough to native expectations for day-to-day use.
