| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-view3dwin11: cSC4View3DWin core | Codex / core-view3dwin11 | in progress | Collect focused diagnostic, run final all/verify, report partial closure and enqueue READY |

## Handoff
- Goal: cSC4View3DWin core only; partial closure explicitly authorized. No plugin/capi/ABI changes.
- Worktree reserved and detached; do not change branches or refs. Previous tip 7f31a3a02.
- Implemented column: engine/core/src/ui_windows/view3d.rs and view3d/. README has section-level owner closure and explicit remaining work.
- Fixed shutdown_tail to release each queued notification reference; regression covers acquisition, duplicate suppression, shutdown and repeated shutdown.
- Added safe/disabled modal-tail reference/drop vector, locale SizeOf vector, VW-T24 date boundaries and VW-T39 graph/conditional-stop vectors.
- Prior VW-T21 fixture correction (b4b0756a4) is present; final verification must confirm it.
- Current focused diagnostic: exec session 27274, build-runs/focused-resume.log. Launched before the last test additions: diagnostic only. Collect the same operation to termination.
- Sessions 46408/42276 from previous round are gone per coordinator; do not wait on them. Earlier evidence remains in all-final.log, focused-1.log and build-queue/.
- Next: collect focused diagnostic, run final qbuild all and qbuild verify once on stable source; log to distinct final-resume files and read tails/failures only.
- qbuild fmt passes for lifecycle.rs, owner_tests.rs and resolved_tests.rs (3 files, zero differences).
- VW19 already filed in docs/specs/questions/view3d-window.md: unspecified trigger reconstruction; literal stream insertion documented in code.
- Remaining work is not relabeled Unsupported: see README, especially GUI/fallbacks, dormant helper/panel extras, mission/advice/background vectors, coverage/graph owner integration.
- No game requests or game measurements; core-only scope.
- Main C:/Users/tlori/FreeCity4; WT C:/Users/tlori/FreeCity4/.claude/worktrees/core-view3dwin11.
- Manifest C:/Users/tlori/FreeCity4/build-runs/run4-core-view3dwin11-r2-manifest.json.
