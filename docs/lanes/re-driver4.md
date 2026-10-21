| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-driver4: RE spec of the DX7 3D driver (`nSGLDX7::cGDriver`, lifecycle, state, video modes) | `re-driver4` / (RE lane) | ready: new docs/specs/renderer-driver.md §1-§6 + questions RD1-RD5 (docs/specs/questions/renderer-driver.md); no engine, no ABI change, no game runs | 1. answer RD2 (SetDefaultD3DDeviceState 0x008872A0) and RD5 (texture/draw slots, cTextureManager, cPrimitiveManager) in a follow-up RE lane; 2. coordinator: consider coverage-map row below |

## Report

What changed (spec only):
- docs/specs/renderer-driver.md (new). §1 scope: the three 83-slot graphics-driver tables; only 0x00AC4660 (DX7,
  interface at object +0xC) is covered; 0x00AC36D0 (OpenGL driver, inferred: its bodies call gl* imports, some
  misfiled under nSGLDX7::cGDriver) and 0x00ACC4D8 (third driver) are not. §2 construction. §3 sticky error
  channel (codes 0/1/2/3/6, the "surface lost clears the code" quirk). §4 lifecycle: Init/Shutdown, SetVideoMode
  (window styles, failure paths), 3D environment set-up (failure message boxes, initial pixel-space ortho
  projection checked against bytes: 2/W, -2/H, constants 0x00A8825C / 0x00ABD2D4), BeginScene, Flush as frame
  boundary and the device-lost / restore cycle (messages 0xAACB92A3, 0x8ACAA95F, 0x6ACB92A9), teardown.
  §5 state slots with enum tables read from exe data (compare 0x00AC44EC, stencil op 0x00AC450C, shade
  0x00AC4524, blend 0x00AC452C, caps 0x00AC45DC), Clear mask bits, Enable/Disable/IsEnabled cap table,
  colour/alpha quantisation (single-precision +49152.0 trick, verified in asm 0x00881AB0, 0x00882F60).
  §6 video-mode list (two records per display mode: full screen then windowed), record fields, getters.
- docs/specs/questions/renderer-driver.md: RD1-RD5 filed; docs/specs/QUESTIONS.md re-indexed.

Owners covered (honest): nSGLDX7::cGDriver lifecycle/state/video-mode bodies 0x00881DF0-0x00889260 (about 40 of
83 slots plus the private lifecycle helpers) and nSGLDX7::TranslateColorToD3D. Not covered: texture, draw,
blit, viewport, matrix and texture-stage slots; cTextureManager, cPrimitiveManager, cTextureStageData,
cVertexBuffer, D3DLight, nRZSimGL, cGZBufferSimGLCache, cSC4GDriversCOMDirector.

Proposed coverage-map row (coordinator; insert before `nSGLDX7*`, partial spec, so judge against the
"covers one method does not count" rule):
`nSGLDX7::cGDriver	3D driver (DirectX 7)	renderer	renderer-driver.md	-	-	-`

Tooling notes: REA was unavailable ("No app is open"); all facts are from tools/re_query.py, decomp/ and exe data
bytes (helpers build-runs/fn.py, build-runs/rd.py, not committed).
