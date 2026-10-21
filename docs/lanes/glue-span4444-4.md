| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-span4444-4: plugin glue for span render 4444 (nGZDrawSoftRender::cSpanRender4444) | `glue-span4444-4` / (engine lane) | in progress | see Handoff |

## Handoff
- Goal: wire core span_render4444 into the plugin only through a supported compat/GZ boundary.
- Found: no supported boundary. Spec section 1: class internal to cGZDrawContextRenderToBuffer (+0xB0), no GZCOM
  interface/CLSID, no docs/abi rows, no calling convention/pop counts for slots 1-5, no known in-game path that
  makes a colour-type-7 context. Compat policy (engine/plugin/compat/FC4Compat.h) needs entries from specs/abi.
- Done: SP3 filed in docs/specs/questions/span-render-4444.md (a-d: convention, ABI row, GZ path, reach).
- Next: gate (qbuild all) on unchanged code, finish report, merge-queue state wip (blocked on SP3).
