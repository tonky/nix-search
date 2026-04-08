# Stage 05: Browser Cache and Startup Sync

Estimated time: 5-6 hours
Depends on: [stage-02-web-shell.md](stage-02-web-shell.md), [stage-04-cicd-publish.md](stage-04-cicd-publish.md)

## Goal

Implement IndexedDB persistence and startup sync against remote manifest/artifact.

## Scope

In scope:
- IndexedDB schema for packages and metadata.
- Startup sequence: local metadata -> remote manifest check -> update/no-op.
- In-memory state hydration from stored packages.

Out of scope:
- Final fuzzy ranking quality.
- Refresh button behavior details (next stage).

## Checklist

- [x] Define object stores for package records and metadata.
- [x] Implement read/write helpers with typed error mapping.
- [x] Add startup sync orchestrator.
- [x] Handle first-run empty cache path.
- [x] Handle no-network path with existing local cache.
- [x] Surface cache status in UI shell.

## Validation

Manual checks:
- First run downloads and stores dataset.
- Reload uses local cache and does not redownload when unchanged.
- Offline reload works after initial sync.

## Exit Criteria

- Browser app can boot from local data reliably.
- Manifest-version check is functional and safe.
