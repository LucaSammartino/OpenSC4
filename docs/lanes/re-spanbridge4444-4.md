| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-spanbridge4444-4 | workflow / `re-spanbridge4444-4` | in progress (round 2: SP4). Round 1: SP3 answered in `docs/specs/span-render-4444.md` §1.1 (native call boundary of `cSpanRender4444`); SP4 filed (type-7 producer) | SP4 needs a runtime breakpoint on `0x008307A8`; reconcile the proposed vtable rows |

## Handoff
- Goal (round 2): answer or sharpen SP4 (which shipped path builds a colour-type-7 RenderToBuffer target).
- Facts so far (bytes): Init 0x00830710 fails (releases buffer, returns 0) for types outside 2..9 (default 0x0083087A).
  cGZBuffer (CLSID 0xC470D325, creator 0x008298F0, ctor 0x008272E0 default type 3/16bpp, vtable 0x00AC1400) Init
  slot 3 0x008269B0 stores the colour type unchanged. Draw-system CreateBuffer 0x0082B980 forwards caller type.
  Gimex factory (IID 0x02FFBB60) LoadFromFile slot 4 arg3 = target type; 13 IID sites; seen literal targets 1,2,9.
  Buffer resources v1/v2 carry the colour type in the file header (persistence.md 6.2); factory defaults 3 / 1.
  FUN_00604920 (lot editor) maps textures to type 8/9. REA session unavailable (no app open): bytes only.
- FOUND producer: cGZGraphicSystemW32::OnCanvasInit 0x0098B3C7..0x0098B40F: if object+0x41 (ctor default 1)
  type = 7 when screen depth (GDriver slot 0x100 struct +0xC, default 32) <= 16 else 9; calls cGZScreenBuffer
  (+0x100, ctor 0x00991DA0, iface +0x4C vtable 0x00ADB388 slot 4) Init 0x00991EC0 -> type stored at buffer +0xC;
  also a 4x4 aux buffer (+0x30) same type. ScreenBuffer Init registers cGZDrawContextRenderToSimGLFactory (0x00991E50)
  with draw system slot 3, creates 0xAB300B20. Draw-system slot 4 0x0082BAB0 = first factory accepting the buffer.
- Next: does SimGL factory claim the screen buffer before RenderToBuffer (list order, accept rule)? then spec 1.2.
