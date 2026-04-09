# Stage 13: Origin Storage Surfaces Reset and Diagnostics

## Objective
Make reset and diagnostics more explicit by covering origin storage surfaces beyond IndexedDB, and clarifying what may still be retained by browser internals.

## Scope

1. Extend diagnostics report with:
   - localStorage entry count
   - sessionStorage entry count
   - CacheStorage cache count
2. Extend reset flow to clear available origin surfaces:
   - IndexedDB stores (existing)
   - localStorage
   - sessionStorage
   - CacheStorage named caches
3. Add diagnostics notes for unsupported/blocked surfaces and estimate lag semantics.

## Verification

- `cargo check`
- `just e2e-test`
- manual: after reset + auto diagnostics refresh, counts for IDB/local/session/cache storage should be near zero if accessible.
