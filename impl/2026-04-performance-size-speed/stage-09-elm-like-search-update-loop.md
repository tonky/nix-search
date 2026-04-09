# Stage 09: Elm-like Search Update Loop

## Objective
Adopt a reducer/message-queue flow for search interactions so input handling and search computation are decoupled from synchronous UI refresh.

## Scope

1. Introduce a message enum for search events:
   - input changed
   - debounced query commit
   - search run request
   - search compute finished
2. Add a queue-backed dispatcher that drains messages asynchronously (next tick).
3. Move search state transitions into reducer-style message handling.
4. Preserve existing search ranking behavior and UI semantics.

## Proposed Implementation

1. `lib.rs`:
   - add `SearchMsg` enum and dispatcher with `Rc<RefCell<VecDeque<SearchMsg>>>` queue.
   - use generation/epoch checks to drop stale search-compute completions.
   - wire input and debounce effects through dispatcher instead of direct signal mutation for search query.
2. Keep refresh/startup flows unchanged in this stage to limit risk.
3. Validate with unit/build/E2E and latency probe.

## Verification

- `cargo check`
- `cargo test -p nix-search-web`
- `just e2e-test`
- `just latency-probe-latest` and compare search p95 against stage-08 sample
