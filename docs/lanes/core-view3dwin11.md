| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-view3dwin11: cSC4View3DWin core | Codex / core-view3dwin11 | in progress | Implement owner, conformance tests, closure table and full gate |

## Handoff
- Goal: implement cSC4View3DWin from view3d-window.md and view3d.md in core only; no ABI/plugin changes.
- Worktree is reserved and detached; commit normally without changing refs.
- Done: 12a116a6c establishes lane record. Existing owner audited against resolved VW10–VW18.
- Source now corrects paint/tail, zoom bounds, cursor/fade, CRT grammar, bucket-ordered triggers, pre-city teardown, borrowed mission payload and background latch. Added registry and coverage helper.
- Next: collect running focused compile session 39100 (build-runs/focused-1.log), all session 46408 (build-runs/all-final.log), verify session 42276 (queue qbuild-20261024-131859-build-ctest). Never launch duplicates.
- Focused compile was launched before the last source/test additions, so its eventual result is diagnostic evidence, not final-source verification.
- New question VW19: trigger load reconstruction; literal insertion in stream order.
- c9dc50ae0: implementation milestone. README now has authoritative per-section Owner closure table and explicit remaining implementation gaps; owner closure is partial.
- Known likely test diagnostic: new VW-T21 uses Host default changed=true, which logs pause/restart beyond its exact expected zoom list; collect the running result before correcting it.
- Formatting checked via qbuild fmt: relative paths unsupported; absolute paths exposed pre-existing recursive child diffs. Scoped skip_children rustfmt differences were applied using generated apply_patch hunks (never stdout-to-source).
- Main: C:/Users/tlori/FreeCity4; WT: C:/Users/tlori/FreeCity4/.claude/worktrees/core-view3dwin11.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-view3dwin11-r1-manifest.json.
- No game requests outstanding; core-only task.
