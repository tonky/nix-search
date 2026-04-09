# Stage 10: Cache Reset and Diagnostics Accuracy

## Objective
Make local cache reset behavior verifiable and remove misleading diagnostics interpretation when browser storage estimates do not immediately shrink.

## Scope

1. Strengthen reset path verification:
   - after reset, verify IndexedDB package/meta entry counts are cleared.
2. Improve diagnostics report fidelity:
   - include explicit IndexedDB entry counts for package and meta stores.
   - add explanatory note that StorageManager usage estimates are origin-wide and may lag reclaim.
3. Keep existing refresh/startup behavior unchanged.

## Verification

- `cargo check`
- `cargo test -p nix-search-web`
- `just e2e-test`
- manual: run Reset Local Cache then Storage Diagnostics; verify entry counts report empty cache.
