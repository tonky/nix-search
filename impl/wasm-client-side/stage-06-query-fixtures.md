# Stage 06 Regression Query Fixtures

Use these manual queries in the web UI to check ranking and section behavior.

## Ranking/Fuzzy

1. `claude code`
2. `claud cod`
3. `cld cod`
4. `rip`
5. `rust analyzer`

Expected:
- Claude-related typo queries should rank `claude-code` near top when available.
- Partial query `rip` should rank `ripgrep` near top.

## Section Split

With `all platforms` disabled:
- query: `claude code`
- matched section should include packages containing selected platform.
- others section should include packages missing selected platform.

With `all platforms` enabled:
- others section should disappear.
- result set should be a single matched section.

## Empty State

Query unlikely to match:
- `zzzz-not-a-real-package`

Expected:
- Left pane shows explicit empty state message.
- Right pane shows no selection state.
