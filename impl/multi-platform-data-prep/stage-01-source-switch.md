# Stage 01: Source Switch to Multi-Platform Metadata

## Scope
- Add `parse_channel_packages()` in shared core parse module.
- Update prep fetch flow to prefer `packages.json.br` channel source.
- Fallback to current pkgforge JSON parser if primary source fails.
- Validate artifact platform distribution via `jq`.

## Validation
- `cargo test -p nix-search-core`
- `cargo run -- prep-web --output tmp/pages-data`
- verify platforms contain `aarch64-darwin` via jq aggregation
