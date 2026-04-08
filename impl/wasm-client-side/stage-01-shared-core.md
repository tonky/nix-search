# Stage 01: Shared Core Extraction

Estimated time: 5-6 hours
Depends on: none

## Goal

Create a shared Rust core layer that compiles for native and wasm targets and contains reusable logic only.

## Scope

In scope:
- Extract pure/domain modules used by CLI and web.
- Keep terminal, filesystem, tokio runtime, and native HTTP out of shared core.
- Preserve behavior for parse/group/platform split and ranking entry points.

Out of scope:
- Web UI.
- CI workflows.
- Browser storage.

## Checklist

- [ ] Create shared crate/module boundary (workspace member or internal crate).
- [ ] Move reusable types and parsing logic into shared core.
- [ ] Move reusable platform split/search orchestration interfaces into shared core.
- [ ] Add target-safe feature gating where needed.
- [ ] Wire existing native code to consume shared core without behavior regression.
- [ ] Add/port unit tests for parse and split behavior.

## Validation

Run:

```bash
cargo test
cargo check --target wasm32-unknown-unknown
```

Expected:
- All existing tests pass.
- Shared core compiles for wasm target.

## Exit Criteria

- Native app still builds and runs.
- Shared core exports the minimal APIs needed by later web stages.
