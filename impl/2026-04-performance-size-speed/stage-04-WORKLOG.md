# Stage 04 Worklog

## 2026-04-09
- Stage planned; implementation not started.
- Focus includes wasm/static size and compressed-transfer fallback compatibility.
- Added release profile tuning for size-focused builds:
	- `lto = "fat"`
	- `codegen-units = 1`
	- `panic = "abort"`
	- `strip = "symbols"`
- Added package-specific release optimization for web crate:
	- `[profile.release.package.nix-search-web] opt-level = "z"`
- Gated `console_error_panic_hook::set_once()` to debug builds only to reduce release wasm footprint.
- Validation:
	- `cargo check` passed.
	- `just web-build` succeeded.
	- New dist artifact sizes (`crates/nix-search-web/dist`):
	  - wasm raw: `475,514 B` (prev ~`665,164 B`, about `-28.5%`)
	  - wasm brotli: `161,446 B` (prev ~`194,449 B`, about `-17.0%`)
	  - js raw: `51,730 B` (prev ~`54,688 B`, about `-5.4%`)
	  - js brotli: `7,683 B` (prev ~`7,899 B`, about `-2.7%`)
	- Prepared data artifacts remain available in both forms:
	  - `.json` uncompressed fallback
	  - `.json.br` packaged for transfer/storage experiments
	- Updated web manifest parsing contract to accept optional compressed fields for stage handoff compatibility.
	- Deferred items (explicit):
		- Browser client path to fetch/decompress compressed artifact variants.
		- Dedicated manual/E2E validation pass for startup/refresh/diagnostics flows after compressed-fetch integration.
- Stage status: complete.
