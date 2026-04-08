# Plan

1. Add a lightweight startup status API in web cache layer that avoids loading full package data.
2. Add explicit deferred local-cache hydration API for later invocation.
3. Rewire app startup to use lightweight status first and mark app ready quickly.
4. Trigger deferred hydration asynchronously when needed (first meaningful query) with existing progress UI.
5. Keep refresh flow behavior intact.
6. Validate with wasm check + trunk build.
