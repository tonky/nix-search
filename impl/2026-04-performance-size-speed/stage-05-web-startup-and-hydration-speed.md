# Stage 05: Web Startup and Hydration Speed

## Objective
Reduce startup latency and hydration overhead after initial page load.

## Proposed Changes

1. Add startup metrics around:
   - manifest check
   - local package load
   - row hydration/transforms
2. Optimize heavy transforms:
   - avoid repeated string allocations where possible
   - batch/chunk conversion to avoid main-thread stalls
3. Revisit cache write/read chunk sizing with measured browser behavior.
4. Add latency probe extensions for startup and refresh scenarios.
5. Integrate startup checks with existing browser E2E suite for regression visibility.

## Verification

- Startup p50/p95 from probe and browser manual runs.
- Search responsiveness remains stable or improved.
- E2E smoke tests continue to pass.
- Offline/cache fallback behavior remains correct with compressed and uncompressed artifacts.

## Estimated Effort
5-6 hours
