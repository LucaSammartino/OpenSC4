| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-models4 (RE spec: exact cSC43DRender owner, docs/specs/render3d.md) | `re-models4` / (RE spec lane) | in progress | see Handoff |

## Handoff
- Goal: docs/specs/render3d.md behaviour spec for cSC43DRender owner methods not covered by view3d.md §3/§4 (lifecycle, QI, serialize, camera/viewport, model queue, dirty rects, backing store, shake, fog/lighting, picking).
- Done: render3d.md §1-6 (QI/refs/ctor defaults, serialize, accessors, camera mode, fog/lighting, full redraw, model queue, dirty-rect entry) + §7 remaining list + §8 cases; questions R3D-1..3 (e9b05d3a); §6 dirty-rect queue/process/clear/cleanup/partial update.
- Next: shake group (0x007C6BC0, 0x007CACD0, 0x007C8630, 0x007C86D0) or Init/Shutdown; then finish report.
- Facts: view3d.md §3.1 layout, §3.2 slot table, §3.4 DoMessage, §4 Draw already proven; reuse by reference.
