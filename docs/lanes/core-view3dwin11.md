| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-view3dwin11: cSC4View3DWin core | Codex / core-view3dwin11 | in progress | Implement owner, conformance tests, closure table and full gate |

## Handoff
- Goal: implement cSC4View3DWin from view3d-window.md and view3d.md in core only; no ABI/plugin changes.
- Worktree is reserved and detached; commit normally without changing refs.
- Done: 12a116a6c establishes lane record. Existing owner audited against resolved VW10–VW18.
- Source now corrects paint/tail, zoom bounds, cursor/fade, CRT grammar, bucket-ordered triggers, pre-city teardown, borrowed mission payload and background latch. Added registry and coverage helper.
- Next: collect running focused compile session 39100 (build-runs/focused-1.log); fix diagnostics, then all and verify once on stable source.
- Focused compile was launched before the last source/test additions, so its eventual result is diagnostic evidence, not final-source verification.
- New question VW19: trigger load reconstruction; literal insertion in stream order.
- Full owner closure is not yet established; README must explicitly retain remaining implementation gaps.
- Main: C:/Users/tlori/FreeCity4; WT: C:/Users/tlori/FreeCity4/.claude/worktrees/core-view3dwin11.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-view3dwin11-r1-manifest.json.
- No game requests outstanding; core-only task.
