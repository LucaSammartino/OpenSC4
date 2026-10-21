| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-driverdraw4: RE spec follow-up of the DX7 driver (default device state, draw path, primitive manager) | `re-driverdraw4` / (RE lane) | ready: docs/specs/renderer-driver.md new §7 (RD2 answered) and §8 (DrawArrays/DrawElements/InterleavedArrays, cPrimitiveManager, slots 30-34); RD5 narrowed; no engine, no ABI change, no game runs | 1. follow-up RE lane: cTextureManager 0x0088A380-0x0088B2C0 and texture slots 24-29, 35-39, 58-61; 2. stage records (0x00884320/0x00884500, slots 51-57, 62); 3. blit/viewport slots 68-76 (RD3) |

## Report

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
