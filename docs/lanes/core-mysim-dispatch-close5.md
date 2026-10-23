| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-mysim-dispatch-close5: dispatch owner closure | Codex / core-mysim-dispatch-close5 | in progress | Implement settled MS17 contracts and verify |

## Handoff
- Goal: close cSC4MySimDispatch and cLocation against mysims.md; only MS18 may remain open.
- Worktree reserved, detached HEAD; do not change refs. No plugin/C ABI changes.
- Done: settled trip/queue/clock/tuning/player APIs and full dispatch/location record implemented.
- Focused cargo test: 22 dispatch tests pass (including nine new trip tests). First compile failed on ByteWriter accessor, corrected.
- Shared in-memory dispatch fixture extracted for reuse by conformance; no plugin/C ABI edits.
- Filed MS19 for missing identity/cell equality precedence and TeleportToCommuteLocation effects; state-1 request is UnsupportedMs19.
- Next: executable MS17 conformance vectors, additional branch/record tests, owner table, formatting and final full gate.
- Required final gate: qbuild.py all on stable source. No game request outstanding.
