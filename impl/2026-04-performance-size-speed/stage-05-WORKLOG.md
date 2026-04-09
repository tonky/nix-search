# Stage 05 Worklog

## 2026-04-09
- Stage planned; implementation not started.
- Startup/hydration latency metrics + E2E integration captured in plan.
- Added web perf instrumentation logs (`[perf][web]`) for:
	- startup status phase latency
	- deferred hydration load/apply/total latency
	- refresh total and apply-packages latency
- Updated `apply_packages_async` to return measured elapsed time for callers.
- Reduced hydration-loop update churn to lower startup overhead:
	- `CHUNK_SIZE: 200 -> 400`
	- `PROGRESS_UPDATE_EVERY: 1000 -> 2000`
- Added browser E2E integration coverage for startup/search perf indicators in `tests/e2e/specs/smoke.spec.ts`.
- Validation:
	- `cargo check` passed.
	- `cargo test --lib` passed (16/16).
	- `just latency-probe-latest iterations=40` sample metrics:
	  - `startup_read_ms=64.29`
	  - `startup_hydrate_ms=7.27`
	  - `search_avg_ms=25.24`
	  - `search_p50_ms=28.02`
	  - `search_p95_ms=33.66`
- Deferred to follow-up:
	- dedicated refresh-scenario probe extensions
	- broader browser fallback E2E coverage for storage/offline variants
- Stage status: complete.
