# Feature: Browser E2E Coverage + Hydration Progress UI

## Goals

1. Introduce automated browser testing that covers Firefox and Safari-engine behavior via Playwright projects (`firefox`, `webkit`).
2. Reduce manual QA by validating startup/render/refresh/search paths in CI-friendly tests.
3. Add a visible loading/progress bar for background local-index hydration to improve UX with large multi-platform datasets.

## Constraints

- Keep local dev workflow simple (`just` command for E2E).
- Use deterministic local data prepared by existing `prep-web` flow.
- Treat WebKit in Playwright as Safari proxy for automated coverage.
