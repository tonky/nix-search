# Stage 02: Diagnostics And Storage-Constrained Fallback E2E

## Scope

- Add Playwright E2E coverage for diagnostics panel visibility/fields.
- Add Playwright E2E scenario that simulates IndexedDB-unavailable runtime and validates session-only fallback behavior.
- Keep tests stable in Firefox + WebKit project matrix.

## Checklist

- [x] Add diagnostics-panel smoke test asserting key fields exist.
- [x] Add constrained-storage simulation via init script override of `indexedDB.open`.
- [x] Assert startup or refresh status indicates storage fallback behavior.
- [x] Run `just e2e-test` and capture pass status.

## Validation

- `just e2e-test`

## Exit Criteria

- Browser E2E suite covers diagnostics panel and constrained-storage fallback path.
- Deferred FOLLOW_UP item for diagnostics E2E can be marked complete.
