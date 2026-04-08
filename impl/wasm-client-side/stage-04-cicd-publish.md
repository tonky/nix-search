# Stage 04: CI/CD Scheduled Publish

Estimated time: 5-6 hours
Depends on: [stage-03-data-prep-local.md](stage-03-data-prep-local.md)

## Goal

Automate artifact preparation and publication to GitHub Pages on schedule and manual dispatch.

## Scope

In scope:
- Add GitHub Actions workflow.
- Daily scheduled run and manual trigger.
- Run prep command.
- Publish manifest + versioned data artifacts to Pages.

Out of scope:
- Browser sync logic.
- Search UI integration.

## Checklist

- [x] Create workflow file with `schedule` and `workflow_dispatch` triggers.
- [x] Install toolchain and build prep command in CI.
- [x] Generate artifacts into publish directory.
- [x] Publish output to GitHub Pages.
- [x] Preserve immutable versioned artifact naming.
- [x] Add run summary logs (version, package_count, checksum).

## Validation

Manual checks after workflow run:
- GitHub Actions run succeeds.
- Pages site exposes manifest URL.
- Manifest references existing artifact URL.

## Exit Criteria

- Data artifacts are refreshed automatically without local intervention.
- Manual rerun path exists for urgent updates.
