| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-models4 (RE spec: exact cSC43DRender owner, docs/specs/render3d.md) | `re-models4` / (RE spec lane) | in progress | see Handoff |

## Handoff
- Goal: docs/specs/render3d.md behaviour spec for cSC43DRender owner methods not covered by view3d.md §3/§4 (lifecycle, QI, serialize, camera/viewport, model queue, dirty rects, backing store, shake, fog/lighting, picking).
- Done: render3d.md §1-6 (QI/refs/ctor defaults, serialize, accessors, camera mode, fog/lighting, full redraw, model queue, dirty-rect entry) + §7 remaining list + §8 cases; questions R3D-1..3.
- Next: specify a §7 group (dirty-rect internals 0x007C8F30/0x007CAE00/0x007C6D80, then shake) and move it out of §7.
- Facts: view3d.md §3.1 layout, §3.2 slot table, §3.4 DoMessage, §4 Draw already proven; reuse by reference.
