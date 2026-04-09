# Stage 03: prep-web Throughput and Payload Optimization

## Objective
Reduce `prep-web` execution time and shrink delivered payload cost.

## Proposed Changes

1. Instrument `prep-web` sub-phases (fetch, decode, parse, filter, serialize, write).
2. Parallelize/filter efficiently for target platforms.
3. Introduce payload strategy improvements:
   - keep current JSON artifact as baseline-compatible output
   - add optional compressed artifact output (`.json.br` and/or `.json.gz`) for transport/storage
   - include manifest fields for compressed variants and sizes
4. Ensure deterministic version/checksum semantics remain clear and documented.
5. Publish stage handoff contract for downstream consumers (stage 04/05):
   - artifact naming/versioning rules
   - compressed/uncompressed availability guarantees
   - manifest compatibility expectations

## Verification

- `prep-web` timing improvements vs stage-01 baseline.
- Manifest + artifact validation tests updated.
- Existing web flow remains functional with uncompressed fallback.
- End-to-end check confirms stage-04/05 code paths can consume stage-03 outputs.

## Estimated Effort
5-6 hours
