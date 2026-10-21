| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-span-context4 (RenderToBuffer draw-context ABI: SP5, SR1555-7) | workflow / `re-span-context4` (spec lane) | in progress | see Handoff |

## Handoff

- Goal: publish exact cGZDrawContextRenderToBuffer (vtable 0x00AC17D0) creation/Init/state/setters/Lock/Unlock and
  DrawPreclippedSpanList ABI for buffer types 4/6/7; answer SP5 (span-render-4444) and SR1555-7 (span-render-1555);
  specify a native-context fixture route (constructed vs shipped call counts).
- Done: static RE complete (REA: no app open -> static fallback re_query/decomp). Findings: ctx vtable 0x00AC17D0 slots
  0-23 read raw; ctor 0x00830690; Init RET4; setters slot7 SetColorTable(i,n,argb*) RET0xC, 8 SetAlphaBlend(int,SETG) RET4,
  9 SetBltModulate(byte) RET4, 12 SetGradientTransform(m*,x0,y0) RET0xC, 13 SetBltSource 6 args RET0x18, 14/15 tiling;
  17 Suspend/18 Resume Unlock/Lock 0x8040; buffer Init slot3 (w,h,type,bits) RET0x10; pack +0x78 table 0x00825E60;
  GetSystemService(0xEB2EA3CB svc, 0x4B2EA3B3 iid); factory order: RenderToBuffer first, SimGL needs IID 0xE3094DC8.
- Next: write docs/specs/draw-context-buffer.md? NO: add section to span-render-4444.md §1.3 + 1555 §1.3, answer SP5, SR1555-7.
- Facts: span-render-565.md §1.1 already gives Init/Shutdown/Release order (Shutdown unlocks 0x8040; Release does not).
