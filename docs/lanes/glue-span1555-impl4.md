| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-span1555-impl4: native bridge for cSpanRender1555 (enable=span1555) | `glue-span1555-impl4` / engine lane | in progress | see Handoff |

## Handoff
Goal: patch vtable 0x00AC182C slots 1-5 (spec span-render-1555 §1.1) with thiscall forwarders that compare original vs ours
(fc4_span1555_* CAPI over core span_render1555) and a constructed COM cGZBuffer (CLSID C470D325, type 4/6) fixture.
Done: nothing yet.
Next: CAPI module engine/capi/src/span1555.rs + fc4.h decls; plugin FC4Span1555Shadow.cpp; gate; game run.
Facts: draw system sysserv 0x4B2EA3B3 iid 0xEB2EA3CB (SDK GZServPtrs); slot 4 = 0x0082BAB0 (verify at runtime).
