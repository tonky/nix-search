# WASM Implementation Stages

This document is the execution index for staged implementation.

Scope rules:
- Keep [PLAN.md](PLAN.md) as high-level strategy.
- Implement in small, self-contained stages.
- Each stage should be completable and testable in about 5-6 hours.

## Stage Order

- [x] [Stage 01](stage-01-shared-core.md): Shared core extraction and dual-target compile
- [x] [Stage 02](stage-02-web-shell.md): Web shell and two-pane UI skeleton
- [x] [Stage 03](stage-03-data-prep-local.md): Local data preparation pipeline
- [x] [Stage 04](stage-04-cicd-publish.md): Scheduled CI/CD publish to GitHub Pages
- [x] [Stage 05](stage-05-browser-cache-sync.md): Browser cache and startup sync
- [x] [Stage 06](stage-06-search-and-results.md): Fuzzy search and matched/others rendering
- [x] [Stage 07](stage-07-refresh-flow.md): Manual refresh flow and status UX
- [ ] [Stage 08](stage-08-polish-and-hardening.md): Offline polish, performance checks, docs

## Stage Completion Policy

A stage is complete only when:
- The stage checklist is fully checked.
- The stage-specific validation commands succeed.
- The stage leaves the repository in a runnable state.

## Notes

- If a stage reveals scope expansion, split it and create a new stage file.
- Do not pull work from later stages unless it is a strict blocker.