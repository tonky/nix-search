# Stage 06: Compressed Artifact Fetch Path (Follow-up)

## Objective
Use `.json.br` artifact when available in manifest to reduce transfer time, with safe fallback to uncompressed JSON.

## Scope

1. Extend web refresh path to prefer compressed artifact when:
   - `compressed_artifact` exists
   - `compressed_format == "brotli"`
2. Fetch compressed payload as bytes and decompress client-side.
3. Parse JSON from decompressed bytes into prepared data.
4. Preserve fallback behavior:
   - if compressed fetch/decode/parse fails, retry uncompressed artifact automatically
5. Keep diagnostics/logging clear for which source path was used.

## Verification

- `cargo check` and `cargo test --lib` pass.
- Browser E2E smoke suite passes.
- Refresh continues working even if compressed artifact is missing/broken.
