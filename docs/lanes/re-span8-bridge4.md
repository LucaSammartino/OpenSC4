| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-span8-bridge4 | workflow / `re-span8-bridge4` | in progress | see Handoff |

## Handoff
- Goal: span-render-8.md bridge section: vtable 0x00AC188C methods (callee-pop, return, lifetime), Init factory
  selection for colour type 2, type-2 buffer format/Init/lock, DrawPreclippedSpanList dispatch, real pixel route
  (fixture vs shipped producers); answer SR8-1/SR8-2.
- Done: findings gathered (not yet in spec): Init jump table 0x008308B0, ctor 0x00830690, factory 0x0082B910
  (vtable 0xAC162C, registered by cGZDrawSystem::Init 0x0082BC90), CreateDrawContext 0x0082BAB0 (+0x10 of 0xAC1668),
  cGZBuffer vtable 0xAC1400 (+0xC Init 0x008269B0, +0x18 Lock 0x00826AA0, +0x1C Unlock 0x00826490, +0x78 0x00825DA0
  returns 0 for type 2), storage 0x00826A50 stride=(w*bpp/8+3)&~3; buffer vtable 0xAC1550 +0x78 0x0082AD30 also 0.
- Next: write span-render-8.md section 10 Bridge; answer SR8-1 (solid byte = 0); SR8-2 narrowed (only Init(..,2,8)
  caller is cSC4WinRegionScreen::MakeWholeRegion 0x007AEDD0 heightfield, pixels set directly, no context).
- Helpers: build-runs/dw.py <va> <n> (dwords from exe), find.py <hexbytes>, calls.py <va> (E8/E9 callers + refs).
