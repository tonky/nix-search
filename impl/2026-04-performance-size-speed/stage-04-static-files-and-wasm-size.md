# Stage 04: Static Files and Wasm Size Optimization

## Objective
Cut static artifact size, especially wasm and data transfer bytes.

## Baseline (2026-04-09)

- wasm raw: `4,192,901 B` (brotli `661,696 B`)
- js raw: `57,206 B` (brotli `8,089 B`)
- data json raw: `24,913,134 B` (brotli `2,247,291 B`)

## Proposed Changes

1. Cargo profile tuning for wasm target and release builds:
   - `opt-level = "z"`, `lto`, `codegen-units`, `strip`, `panic = "abort"` (validated for compatibility)
2. Audit and reduce code size contributors in web crate:
   - feature-gate debug-only utilities (for example panic hook setup)
   - prune dependency features where possible
3. Improve transfer path for large static data:
   - serve or package compressed data artifacts for browser fetch path
   - ensure fallback path remains available (uncompressed path guaranteed)
4. Add automated size report task and budget thresholds.

## Verification

- Compare wasm/js/data raw and compressed sizes before/after.
- Browser startup + refresh flows pass manual and E2E smoke checks.
- No regression in diagnostics and cache-reset flows.
- Explicit compatibility checks against stage-03 artifact contract pass.

## Estimated Effort
5-6 hours
