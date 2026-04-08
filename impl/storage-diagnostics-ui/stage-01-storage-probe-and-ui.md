# Stage 01: Storage Probe And UI

## Outcome
Users can run browser storage diagnostics from the app and see whether IndexedDB and persistent storage are available.

## Tasks
1. Add `StorageDiagnosticsReport` and `StorageDiagnosticsProbe` structs in web cache module.
2. Implement async probe that checks:
- `navigator.storage` availability.
- `navigator.storage.persisted()` result.
- `navigator.storage.persist()` request result (best effort).
- `navigator.storage.estimate()` usage/quota (best effort).
- IndexedDB write capability via tiny write transaction against app DB.
3. Add UI state/signals in app shell for diagnostics visibility and loading state.
4. Add a header button to trigger probe.
5. Render diagnostics panel with readable pass/fail/unknown values and details.
6. Keep probe fully optional and non-blocking for startup/search paths.

## Verification
- `cargo check -p nix-search-web` passes.
- Panel opens and updates after probe run.
- Failure path still renders stable output without crashing.

## Risks
- Browser API support differences across engines.
- Web-sys feature mismatch for StorageManager methods.

## Mitigation
- Treat each probe step as optional and capture errors as strings.
- Use tolerant dynamic field extraction for estimate values.
