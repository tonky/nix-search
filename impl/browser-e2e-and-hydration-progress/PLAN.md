# Plan

1. Add Playwright test workspace under `tests/e2e` with dependencies/config.
2. Configure Playwright `webServer` to run local app (`just prep-web` + `trunk serve`) for tests.
3. Add smoke tests for:
   - app shell startup render,
   - refresh flow completion (including session-only fallback),
   - platform selector containing expected multi-platform options (`aarch64-darwin`).
4. Add scripts/just recipes for local execution.
5. Add UI background hydration progress bar in search pane.
6. Validate Rust build and E2E config integrity.
7. Add diagnostics/storage-constrained fallback browser coverage:
   - diagnostics panel render + key field assertions,
   - constrained-storage simulation with session-only fallback assertions.
