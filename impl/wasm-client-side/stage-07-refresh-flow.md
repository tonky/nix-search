# Stage 07: Manual Refresh Cache Flow

Estimated time: 5-6 hours
Depends on: [stage-05-browser-cache-sync.md](stage-05-browser-cache-sync.md), [stage-06-search-and-results.md](stage-06-search-and-results.md)

## Goal

Implement the Refresh Cache button as a full operational flow with safe updates and clear status.

## Scope

In scope:
- Refresh button wired to forced manifest check.
- No-op path when already current.
- Download/apply path when new version is available.
- Progress, success, and failure feedback.

Out of scope:
- Long-term performance tuning.
- PWA/service worker shell caching.

## Checklist

- [x] Enable Refresh Cache button and loading state.
- [x] Implement forced check against remote manifest.
- [x] Implement transactional cache replacement on version change.
- [x] Keep previous usable cache on failed update.
- [x] Show status messages for up-to-date, updated, and failed outcomes.
- [x] Rehydrate in-memory search state after successful refresh.

## Validation

Manual checks:
- With unchanged manifest: refresh reports up-to-date and performs no heavy work.
- With simulated new version: refresh downloads and swaps data successfully.
- With simulated network failure: previous data remains usable.

## Exit Criteria

- Manual refresh is trustworthy and recoverable.
- UX clearly communicates update state to users.
