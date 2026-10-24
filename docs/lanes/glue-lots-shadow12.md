| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lots-shadow12 (lots and lot records verification) | `Codex` / `glue-lots-shadow12` (engine lane) | in progress | Inspect permitted owner contracts and existing shadows; extend measurable coverage and run through 2032. |

## Handoff
- Goal: verify all measurable lot-record owners with default-off shadows in a real city through 2032.
- Worktree is reserved and detached; commit HEAD normally, never update refs.
- Done: clean worktree confirmed; coverage report says 73/73, verified part.
- Done: implementation committed as 8089668c3; lottrip_shadow shares LR12 master capture, counts decoded inline trips, credits only fully exact containing lots.
- Focused Rust tests: 2 passed (build-runs/focused-trip-tests.log); fmt explicit owned files passed.
- LR15 filed for excluded business-unit/base-texture/wall inventory and reference ownership; no speculative native calls.
- Gate: qbuild all PASS; cargo 7214/0, clippy 0 diagnostics, coordinator native build 262/262, CTest 81/81.
- Gate evidence: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log.
- Native queue operation qbuild-20261024-144300-build-ctest collected to terminal PASS; no second verify queued.
- Run 511: glue-lots-shadow12-house2032-r1 stalled at 2030-11-28 (native/outside FreeCity4 heartbeat); ref_warnings=0; lotrec 59/59 and lottrip 59/59 exact. Failed evidence retained.
- Next: submit glue-lots-shadow12-house2032-normal-r2 with both keys at normal speed through 2032-01-01; use build-runs/wait-verdict.ps1 (event-based, not qwait).
- No game request outstanding; next available local run number 512.
- Clean-room prohibition on tools/ overrides task requests to inspect or edit its coverage/profile files; coordinator must handle those files.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-lots-shadow12-r1-manifest.json.
