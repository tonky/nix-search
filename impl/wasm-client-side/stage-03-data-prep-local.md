# Stage 03: Local Data Preparation Pipeline

Estimated time: 5-6 hours
Depends on: [stage-01-shared-core.md](stage-01-shared-core.md)

## Goal

Implement a local tool that fetches upstream data, normalizes it, and emits versioned web artifacts plus manifest.

## Scope

In scope:
- Create a data prep command/tool.
- Fetch upstream snapshot.
- Apply shared parse/group logic.
- Emit versioned artifact and latest manifest.
- Write outputs to local folder for inspection.

Out of scope:
- CI automation.
- Browser consumption.

## Checklist

- [ ] Add prep command (bin or xtask) with deterministic output path.
- [ ] Implement fetch with timeout and basic retry.
- [ ] Reuse shared parse/group pipeline.
- [ ] Emit artifact (grouped package payload).
- [ ] Emit manifest containing version, checksum, package_count, built_at.
- [ ] Add small fixture-based tests for deterministic transform behavior.

## Validation

Run:

```bash
cargo run -- <prep-command> --output tmp/wasm-data
ls tmp/wasm-data
```

Expected:
- Manifest file exists.
- At least one versioned data artifact exists.
- package_count and checksum are populated.

## Exit Criteria

- Data prep can be run locally end-to-end.
- Output format is stable enough for CI and browser sync stages.
