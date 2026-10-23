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
- Milestone 309413b1: settled contracts + core tests.
- Done: eight executable MS17 conformance integration tests pass; all dispatch portions covered, MySim-only state/sound remains with that owner.
- Added home/fallback/cancellation/player guard and complete opaque/reference record tests; expanded owner table including cLocation.
- Next: collect focused tests for additions, format explicit owned files, then qbuild all once on final source.
- Formatting first qbuild fmt used relative paths (resolved from engine, failed file lookup); retry uses absolute paths.
- Required final gate: qbuild.py all on stable source. No game request outstanding.
