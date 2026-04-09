# Stage 12: Auto-refresh Diagnostics After Cache Reset

## Objective
After "Reset Local Cache", automatically refresh the Storage Diagnostics panel so users immediately see post-reset state without an extra click.

## Scope

1. On successful reset, trigger `run_storage_diagnostics()` automatically.
2. Keep diagnostics panel state and loading indicators coherent during reset + follow-up probe.
3. Update reset status text to indicate diagnostics were refreshed.

## Verification

- `cargo check`
- `just e2e-test`
- manual: click Reset Local Cache while diagnostics panel is open; confirm usage/entry metrics refresh automatically.
