# Plan

1. Add parser support for channel `packages.json` schema in shared core.
2. Add primary prep source URL for `packages.json.br` and parse it first.
3. Keep existing pkgforge source as fallback for resilience.
4. Validate prep output includes non-`x86_64-linux` platforms (including `aarch64-darwin`).
5. Run build/tests and report resulting platform distribution.
