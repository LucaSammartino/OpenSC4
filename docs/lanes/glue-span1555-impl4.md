| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-span1555-impl4: native bridge for cSpanRender1555 (enable=span1555) | `glue-span1555-impl4` / engine lane | in progress | see Handoff |

## Handoff
Goal: patch vtable 0x00AC182C slots 1-5 (spec span-render-1555 §1.1) with thiscall forwarders that compare original vs ours
(fc4_span1555_* CAPI over core span_render1555) and a constructed COM cGZBuffer (CLSID C470D325, type 4/6) fixture.
Done: CAPI fc4_span1555_* (06cf2cf0); plugin FC4Span1555Shadow.cpp + compat Span1555SlotEntry + wiring (quick PASS).
Next: game run gs1555i4-r2 (r1: clean, fixture 158/158 exact, but ref_warnings=16 from missing FC4_ACQUIRED; fixed); then qbuild all; question on context ctor state.
Facts: draw system sysserv 0x4B2EA3B3 iid 0xEB2EA3CB (SDK GZServPtrs); slot 4 = 0x0082BAB0 (verify at runtime).
