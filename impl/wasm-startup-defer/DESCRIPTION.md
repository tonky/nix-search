# Feature: Defer Heavy Startup Hydration

## Problem

Firefox reports long-running script timeouts during app startup. The current startup path eagerly loads and prepares the full package corpus before the UI can become reliably responsive.

## Goal

Make startup shell-first and responsive:
- perform only lightweight cache/manifest status checks at mount,
- defer full package hydration until explicitly needed,
- keep visible loading/progress UX while deferred hydration runs.

## Non-Goals

- Rewriting search algorithm or data format.
- Introducing worker threads in this iteration.
