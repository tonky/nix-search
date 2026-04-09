# Stage 02 Worklog

## 2026-04-09
- Stage planned; implementation not started.
- Determinism stress tests called out as mandatory before completion.
- Implemented phase-level timing logs in `cache::update` (`fetch`, `parse`, `index`, `meta`, `total`).
- Implemented index build timings in `cache::index::build` (`open`, `prepare`, `write`, `commit`).
- Added safe parallel document preparation with `rayon` above configurable threshold.
- Added configurable index writer memory and parallel threshold via env vars:
	- `NIX_SEARCH_INDEX_WRITER_BYTES`
	- `NIX_SEARCH_INDEX_PARALLEL_DOC_THRESHOLD`
- Added determinism stress test for large fixture doc preparation.
- Validation:
	- `cargo test --lib cache::index::tests::prepare_documents_is_deterministic_for_large_fixture` passed.
	- `cargo check` passed.
	- Fresh-cache release runtime comparison vs stage-01 baseline sample:
	  - baseline (`tmp/bench/perf-size-smoke2`): `2.84s`
	  - stage-02 fresh run: `2.61s`
	  - delta: `-0.23s` (`-8.1%`)
- Subagent conformance review: pass with follow-up recommendations.
- Stage status: complete.
