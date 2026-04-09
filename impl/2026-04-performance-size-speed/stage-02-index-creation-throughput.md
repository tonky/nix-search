# Stage 02: Local Index Creation Throughput

## Objective
Speed up the `cache update` pipeline (fetch -> parse -> group/filter -> Tantivy index write).

## Proposed Changes

1. Instrument phase-level timings in `cache::update` and index build path.
2. Evaluate and apply safe parallelization:
   - parallel parse/group preparation for large package sets
   - controlled worker count (`available_parallelism`, capped)
3. Optimize Tantivy write settings:
   - reassess writer memory budget
   - profile commit behavior for current document shape
4. Keep deterministic output behavior for package ordering where required.

## Verification

- Compare `cache update` dev/release timings against stage-01 baseline.
- Validate index correctness with existing tests and spot-check query parity.
- Add at least one regression test for deterministic grouping/order assumptions.
- Add parallel determinism stress validation (large fixture, repeated runs) and assert stable output/checksum expectations.

## Guardrails

- No behavioral changes to search results ordering beyond existing scoring behavior.
- No unbounded memory growth.

## Estimated Effort
5-6 hours
