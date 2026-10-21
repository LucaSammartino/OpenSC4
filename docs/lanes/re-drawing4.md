| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-drawing4: 2D drawing and fonts spec (cGZDraw, nGZDrawSoftRender, cGZAlphaBlt, cGZCanvas, cGZFont) | `re-drawing4` / (RE spec lane) | in progress | - |

## Handoff
- goal: write docs/specs/drawing-fonts.md for the largest coherent renderer2D/font owner family (not DX7 driver, not widgets).
- done: lane file created; family chosen = fonts (cGZFont 16187 B, cGZFontSys 8358, FontFixedInfo 105, CachedBitmapData 56, cFontCOMDirector 220, cGZDrawFontRenderer 4444; bytes from tools/coverage.py --owners). Span renderers (5 classes, ~203 KB, 840 kernel slots, no decomp/) judged infeasible exactly in one round.
- next: finish reading decomp/cGZFont.c 2740-3050 (GetStringVisibleWidth, DrawTextLine/Paragraph), then cGZFontSys.c, cFontCOMDirector.c, cGZDrawFontRenderer.c; write docs/specs/drawing-fonts.md.
- facts: helper build-runs/compact.py <file> <a> <b> strips decomp headers. cGZFont vtable 00ac55d8 (64 slots), FreeType statically linked (FT_Get_Char_Index); "_lua_gettop" in decomp = ftol misname. Thai shaping (MakeSubstitutedString 00899665) and AA LUT (008993bb) need asm verification.
