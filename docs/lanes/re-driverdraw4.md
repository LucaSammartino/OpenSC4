| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-driverdraw4: RE spec follow-up of the DX7 driver (device state, draw path, primitive manager, textures, stages, viewport) | `re-driverdraw4` / (RE lane) | partial ready for docs train; PARKED3/3, no fourth round; committed round3 vertex/matrix/writers, pending drafts NOT included; prior: docs/specs/renderer-driver.md §7-§8 (round 1), §9 textures, §10 texture stages, §11 viewport/blits/scene end (round 2); RD2 answered, RD3/RD4/RD5 narrowed; no engine, no ABI change, no game runs | 1. internal-format selection 0x0088F8D0 and pixel converters; 2. slots 6-10, 40-42 (vertex formats, matrices); 3. writers of bytes +0x209/+0x261/+0x263; 4. RD3 mode-record fields |

## Handoff (round 3, 2026-10-21)
Goal: final round 3/3: slots 6-10, 40-42; 0x0088F8D0 internal-format selection + converter table; writers of
+0x209, +0x261, +0x263; RD3 mode-record fields; RD4 per-frame context update and 0xAACB92A3 handler.
Done: rounds 1-2 (tip 1b8ad5ac, §7-§11). Round 3: fcc9230d §8.5 slots 6-10, §8.6 slots 40-42 (LoadMatrix
half-pixel/depth/handedness), §2 identity matrix 0xB542F8, +0x209 (=mode rec +0x29, const 1 at 0x00886E80),
+0x261 (device lost, Flush), +0x263 (env ready, SetupViewport/DestroyViewport) written into §10.2/§11/§9.5.
Next (exact): 1) `python build-runs/ins.py build-runs/sec96.md "## 10. Texture stages"` (sec96.md is §9.6
internal-format selection + converters, written and checked, NOT yet inserted; first append to its converter
table the line "(The decode targets are named from the destination key *[inferred]*.)"); commit.
2) §6.2 mode record table (renderer-driver.md ~line 262): +0x10..+0x1C = alpha,R,G,B masks of the render target
(+0x22C = device GetRenderTarget), set by GetDeviceProperties 0x008886E0 into list[cur]; +0x21 = StencilCaps!=0,
+0x24 dword = wMaxSimultaneousTextures, +0x28 = that >1, +0x29 = 1, +0x2A/+0x2D/+0x2E = 0, +0x2B = RasterCaps
bit 7 (FOGVERTEX), +0x2C = DXT formats seen, +0x2F = TextureCaps bit0 (PERSPECTIVE), +0x30 = 1 (all by
0x00886E30 from D3DDEVICEDESC7 at obj+0x20); +0x22 = 1 (enumerator); +0x34 = obj+0x274 copy, passed unused to
0x00888590; list stride 0x38, SetupViewport copies 13 dwords to obj+0x1E0. 0x00887D80 = vector grow-insert
(capacity size+max(size,n)) i.e. plain append. Then move RD3 to answered (MATCHES).
3) §4.4 Flush: per-frame update = context vt[10] 0x0088C400 ShowFrame(0): full screen (ctx+0x2C) primary
(ctx+0x18)->Flip(NULL,0); windowed (ctx+0x2E): client rect of hwnd ctx+0x28, empty → returns 0x887601C2 no blit,
else ClientToScreen, primary->Blt(screenRect, backbuffer ctx+0x1C, whole, ASYNC|WAIT 0x1000200), on failure
retry WAIT 0x1000000; result ignored by Flush. Restore failure path calls ctx vt[9] 0x0088C3A0 (releases ctx
surfaces +0x14,+0x1C,+0x18,+0x20). Context = 0x48-byte object, vtable 0xAC5060, made by 0x0088C5F0 in SetupD3DX
(+0x25F RD1 byte passed as its 3rd arg - lead for RD1). 0xAACB92A3: occurs once in the whole exe (the PUSH at
0x00884E25): no listener in SimCity4.exe. Then move RD4 to answered; narrow RD5 (slots 6-10/40-42, 0x0088F8D0
done) -> answered if nothing left; `python tools/questions.py index`; final report + merge-queue json.
REA was unavailable (no app open); used re_query/decomp/exe reads (build-runs/rd.py, conv.py, win.py).


## Report (round 2, 2026-10-21)

What changed (spec only, `docs/specs/renderer-driver.md`; §7-§8 from round 1 preserved, §8.2 step 2 corrected:
`0x00884500` sends the texture environment, not a texture transform; dangling §8.5/§8.6 references fixed):
- §9 `cTextureManager`: object layout, 48-byte texture record with defaults, LIFO name reuse, validity rule,
  HasAlpha (internal format not 0/1), SetPriority 0.95f marks dynamic, Init / name 0 (1x1 white), device loss and
  InitDevice re-creation, surface creation (DDSD 0x21007, caps 0x401008, caps2 0x20000/0x20004/0x20080, 1..4096),
  (format, type) → 15 source formats with bit counts (DXT1/3/5 inferred), LoadTextureLevel lock/copy rules;
  driver slots 24-26, 35-39, 48, 58-61 with error codes and quirks.
- §10 stage records: 0x80-byte layout, slots 27-29, 51-57, 62; draw-time application `0x00884320` (filters: GL
  min-filter 5 falls through to 6, quirk from the jump table 0x008844DC) and `0x00884500` (env modes 0-5, combine
  ops via `0x00882100`/`0x00882300`, source table 0x00AC4604).
- §11 viewport (74-76, bottom-up rectangles, worked example), PolygonOffset (67), unsupported blits 68-73
  (error 3), scene end `0x00887260` (EndScene only, no present).

Questions: RD3, RD4, RD5 narrowed in place (none fully answered); QUESTIONS.md regenerated.
Still open: `0x0088F8D0` internal-format selection and converters, slots 6-10 and 40-42, writers of bytes
`+0x209`, `+0x261`, `+0x263`, the `0xAACB92A3` handler, mode-record fields. cGDriver is still partial (about 75 of
83 slots specified), not complete.

Sources: re_query.py asm (numbers checked: 0.95f at 0x00A9BEF8, jump tables 0x008844DC/0x008846AC/0x0088D088,
tables 0x00AC45F4/0x00AC4604 read from the exe), decomp/ for structure. REA not used.

Commits round 2: 4a6823ce (handoff), 8e755b89 (§9-§10), then §11 + this report.

## Report (round 1)

What changed (spec only, `docs/specs/renderer-driver.md`; §1-§6 kept as they were apart from the §1 scope list):
- §7 `SetDefaultD3DDeviceState` (0x008872A0), from the instructions: 26 render states in a fixed order (cull none,
  dither on, colour key off, specular off, blend off, perspective on, depth write on, depth func less-equal, alpha
  test off, fog off, fill solid, stencil off with fail keep / depth-fail **invert** / pass keep / func always /
  ref 1 / masks 1, texture factor 0xFFFFFFFF, ambient 0, lighting off, material sources ambient=colour1,
  diffuse=colour1, specular=material, colour vertex on); the four driver flags +0x2DC..+0x2DF; stage-0 texture
  states (select texture colour and alpha, wrap, coordinate index 0, point/point/no mip) and stage 1 disabled;
  stage-0 texture unbound; every 0x80-byte stage record and the stage set marked dirty. The +0x138 word (RD2) has
  no reader in 0x00881A00-0x0088B400 (write-only), and the BeginScene colour-clear flag +0x2DD is only ever
  written false.
- §8.1-§8.2 `InterleavedArrays` (slot 5), `DrawArrays` (3), `DrawElements` (4): stage application at draw time,
  the mode table 0x00AC44CC (8 modes; strips, fans, quads and quad strips become indexed triangle lists, with the
  index patterns), index-type handling and min/max vertex range for DrawElements.
- §8.3 `cPrimitiveManager`: one 64 KiB vertex buffer (capacity 65536/stride), write-only plus system memory unless
  the device has hardware T&L, FVF from the format (XYZ, DIFFUSE, TEX1/TEX2); queueing with wrap-and-discard,
  merging of consecutive list primitives, silent drop of oversized draws; strides over 32 drawn at once without
  flushing the queue (quirk); flush, index scratch, stop.
- §8.4 `ColorMultiplier`, `AlphaMultiplier` (constant 1.0 at 0x00A80AB0 read from the exe), `EnableVertexColors`,
  both `Fog` slots (vertex fog table 0x00AC45F8).

Questions: RD2 answered (MATCHES) and moved to docs/specs/questions/answered/renderer-driver.md; RD5 narrowed
in place (the draw and primitive-manager part is answered, texture manager / texture slots / stage records still
open); QUESTIONS.md regenerated. RD1, RD3, RD4 untouched.

Sources: tools/re_query.py asm, ghidra/export/windows/instructions.asm (field-reader scans), decomp/ pseudocode
for structure only. REA was not used. Numbers checked against instructions; the DrawElements expansion helpers
(0x008824F0-0x00882890) are marked [inferred].

Coverage: not credited as a full owner. `nSGLDX7::cPrimitiveManager` (7 functions, ~1.3 KB plus helpers
0x008891C0-0x00889A90) is now fully described; cGDriver is still partial (about 50 of 83 slots).

Commits: 89803b0d (lane file), 633d523d (§7), 594a88cb (§8, RD2), then this report.

## Coordinator disposition
Final guarded round returned at7dd9bea3 after context budget. Parked3/3. Queue only supported committed specs (including fcc9230d); drafts in ignored build-runs remain pending and RD3/RD4/RD5 are not marked answered. cGDriver partial, no whole-owner credit. No additional analysis round authorized.
