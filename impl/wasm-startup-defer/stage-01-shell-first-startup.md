# Stage 01: Shell-First Startup and Deferred Hydration

## Scope

- Introduce lightweight startup status function in `cache_sync.rs`.
- Add deferred local package loading function in `cache_sync.rs`.
- Update `lib.rs` to:
  - finish startup quickly,
  - defer heavy hydration,
  - run hydration on first meaningful query,
  - preserve progress messaging.
- Add/adjust status text for update-available scenarios.

## Validation

- `cargo check -p nix-search-web --target wasm32-unknown-unknown`
- `cd crates/nix-search-web && trunk build`

## Exit Criteria

- App shell renders without requiring Firefox script-timeout stop.
- No blocking full corpus load on initial mount.
