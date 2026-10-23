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
- Milestone 6fa120b4: executable conformance and owner tables.
- Focused final semantic checks: 24 dispatch tests plus the new autonomous-resolution test passed; 9 conformance integration tests passed.
- Corrected one conformance test's embedded-NUL GZStr data; serializer unchanged. Evidence: build-runs/focused-results.md.
- Milestone 9eb1b47d: all focused semantic checks and formatting passed (25 dispatch tests total, 9 conformance tests).
- Round 2: session 59061 is unavailable (Unknown process id); process inspection finds no original qbuild/CMake process for this worktree. Prior gate is incomplete, not PASS: Rust 5941 passed, clippy passed; native stopped at compiler ABI probe, CTest never reached.
- Preserved evidence under build-runs/r1-{qbuild-test,qbuild-clippy,fc4verify-configure}.log before rerunning corrected source.
- Implemented §3.3 explicit DispatchHost::has_owner and centralized notify_owner; every delivery path uses it, including flags, job, arrival and MySim state notifications.
- Tests verify absent owners suppress calls without suppressing changes/counters, and recheck ownership between synchronous callbacks.
- Round 2 focused check: 27 dispatch tests passed, 0 failed; fmt four changed Rust files passed, diff_lines=0.
- Next: final qbuild all on corrected source, then report/commit/queue. No game request outstanding.
- Formatting first qbuild fmt used relative paths (resolved from engine, failed file lookup); retry uses absolute paths.
- Required final gate: qbuild.py all on stable source. No game request outstanding.
