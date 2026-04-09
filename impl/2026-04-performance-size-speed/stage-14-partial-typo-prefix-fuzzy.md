# Stage 14: Partial-Typo Prefix Fuzzy Matching

## Objective
Ensure abbreviated typo-like prefixes such as `ascin`, `ascine`, and `ascinem` still match `asciinema`.

## Scope

1. Expand candidate-admission fuzzy logic in web search runtime so near-prefix subsequences can pass initial filtering.
2. Keep scoring conservative to avoid broad noisy matches.
3. Add tests for exact user-provided query examples.

## Verification

- `cargo check`
- `cargo test -p nix-search-web`
- `just e2e-test`
- unit tests proving `ascin|ascine|ascinem` each return `asciinema`
