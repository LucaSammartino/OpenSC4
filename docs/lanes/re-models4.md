| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-models4 (RE spec: exact cSC43DRender owner, docs/specs/render3d.md) | `re-models4` / (RE spec lane) | in progress | see Handoff |

## Handoff
- Goal: docs/specs/render3d.md behaviour spec for cSC43DRender owner methods not covered by view3d.md §3/§4 (lifecycle, QI, serialize, camera/viewport, model queue, dirty rects, backing store, shake, fog/lighting, picking).
- Done: worktree created; function list in build-runs/funcs.txt (130 owner funcs).
- Next: read decomp/cSC43DRender.c per group, write render3d.md sections.
- Facts: view3d.md §3.1 layout, §3.2 slot table, §3.4 DoMessage, §4 Draw already proven; reuse by reference.
