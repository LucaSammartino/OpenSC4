# Span renderer A4R4G4B4 (`nGZDrawSoftRender::cSpanRender4444`)

Status: RE spec, lane re-span4444-4, 2026-10-21. Source: SimCity 4 Deluxe GOG 1.1.641 Windows image, Ghidra export
(`re_query.py asm`) and raw table reads from the image. Every numeric rule below was read from the instructions at the
cited addresses; nothing is inferred from "usual" RGBA maths.

Scope: the owner range `0x00847CE0`-`0x00854E10` (172 functions, 51,809 bytes, `tools/coverage-ranges.tsv:21`): the
five vtable methods, the 166 blit kernels of its `RenderBlt` table that live in the range, and one helper.
Out of scope (other owners): `cGZDrawContextRenderToBuffer` itself (view3d.md section 7), the 1555/565/8888/8 span
renderers, fonts, base widgets, the DX7 driver.

## 1. Class and creation

- vtable `0x00AC185C`, 6 slots: 0 deleting destructor `0x0082FF00` (shared by all five span renderers; resets the
  vtable to `cISpanRender` `0x00AC17AC`, frees when flag bit 0 is set), 1 `RenderSolid` `0x00847CE0`,
  2 `RenderGradient` `0x00847D30`, 3 `RenderAlpha` `0x00847E40`, 4 `RenderAlphaGradient` `0x00847F10`,
  5 `RenderBlt` `0x008480C0` (Mac slot order, `re/symbols/gzdraw-softrender.tsv`).
- Created by `cGZDrawContextRenderToBuffer::Init` (`0x00830710`) when the target buffer's colour type is 7
  (A4R4G4B4), stored at context `+0xB0`. The object has no data members the methods read: every method takes all
  its inputs as arguments (`this` is never dereferenced in slots 1-5).

## 2. Shared conventions

**Pixel.** 16 bits, A in bits 12-15, R 8-11, G 4-7, B 0-3 (masks `0xF000/0x0F00/0x00F0/0x000F`).

**Span record** (8 bytes). Two dwords `start`, `end`, each packed as `(y << 16) | x` with unsigned 16-bit x and y.
Width is `end - start` computed on the whole dwords, so a span is one row: `end` is `start` plus the pixel count
(exclusive end). A span with width 0 writes nothing.

**Target descriptor** (argument `buffer`): dword `+0` = base address of pixel (0,0), dword `+4` = row stride in
bytes. Pixel (x, y) is at `base + y*stride + 2*x`. No clipping is done in this class: spans are assumed
pre-clipped by the caller (`DrawPreclippedSpanList`, view3d.md 7.1).

**Span list loop.** Slots 1-4 take `(spans, count, buffer, ...)` and process `count` spans in order with a
decrement-then-test loop: `count` must be at least 1 (a count of 0 is not guarded; see U1).

## 3. RenderSolid (`0x00847CE0`)

Arguments `(spans, count, buffer, colour)`; `colour` is already a 4444 pixel (low 16 bits used). For each span,
fills `width` pixels from (x, y) with `colour` (helper `0x0090FFC8`, a 16-bit fill: one leading word if the
address is not 4-byte aligned, then dword pairs, then one trailing word). The pixel is stored verbatim, alpha
nibble included.

## 4. RenderAlpha (`0x00847E40`) - constant colour over the destination

Arguments `(spans, count, buffer, colour, alpha)`.
- Weight: `a = ((alpha >> 27) + 1) >> 1` on the unsigned 32-bit `alpha` (only its top 5 bits matter), so
  `a` is 0..16. With the alpha byte in bits 24-31: `a = ((A >> 3) + 1) >> 1`; A = 0x00..0x07 gives 0,
  0xF8..0xFF gives 16.
- Per pixel, for each of R, G, B (4-bit `s` from `colour`, 4-bit `d` from the destination):
  `out = floor((16*d + a*(s - d) + 8) / 16)`. (R and B are computed together in one register as
  `((sRB - dRB)*a + 16*dRB + 0x808) >> 4`, G alone as `((sG - dG)*a + 16*dG + 128) >> 4` on the in-place
  fields; the low field never borrows into the high one because `16*d + a*(s-d) + 8` stays in 8..248.)
- The output alpha nibble is always `0xF`, whatever the destination or `colour` alpha was.
- Worked example: d = `0x8000` (R=G=B=0), colour `0x0F84`, alpha `0x80000000` (a = 8):
  R = floor((0 + 8*15 + 8)/16) = 8, G = floor((64+8)/16) = 4, B = floor((32+8)/16) = 2, result `0xF842`.

## 5. RenderGradient (`0x00847D30`) - linear colour ramp, opaque

Arguments `(spans, count, buffer, grad)`. `grad` is 12 signed dwords: `+0x00..+0x0C` start values for channels
0-3, `+0x10..+0x1C` x steps, `+0x20..+0x2C` y steps. Channels 0, 1, 2 are B, G, R in 8.16 fixed point (8-bit
value in bits 16-23); channel 3 is alpha and is ignored here.
- At each span's (x, y) the start value is `c = start + dx*x + dy*y` (32-bit wrapping integer maths).
- B: the span start is then shifted right logically by 4 and the per-pixel step is `dx_B` shifted right
  arithmetically by 4, so B accumulates with the low 4 bits of its step dropped (B drifts versus the exact ramp).
- G: start and step unchanged.
- R: start shifted left 4, then OR `0xF0000000`; step `dx_R << 4`.
- Each pixel: the 16-bit pixel is bits 16-31 of `(Racc & 0xFF000000) + (Gacc & 0x00F00000) + (Bacc & 0x000F0000)`,
  i.e. each channel is the top 4 bits of its 8-bit integer part (truncation, no rounding, no dither), and the
  alpha nibble is the top nibble of the R accumulator, which is `0xF` while R stays within 0..255.99 along the
  span. If R leaves that range mid-span the carry/borrow lands in the alpha nibble (U2).
- Then each accumulator adds its step and the next pixel (x+1) is written.

## 6. RenderAlphaGradient (`0x00847F10`) - linear colour and alpha ramp over the destination

Arguments `(spans, count, buffer, grad)`, same 12-dword layout as section 5, all four channels used.
- B, G, R accumulators are set up exactly as in section 5 (including the B >> 4 drift; R uses `<< 4` but no OR).
- Alpha accumulator: `(startA + dxA*x + dyA*y) * 0x88`, step `dxA * 0x88` (32-bit wrapping). Per pixel
  `a = acc >> 27` (unsigned), so for an 8.16 alpha value A, `a = floor(A * 136 / 2048)` = floor(A*17/256):
  0..16 over A = 0..255. This is a different alpha-to-weight map from RenderAlpha (section 4); see the table in
  section 8.
- Source colour per pixel: R = bits 24-27 of the R accumulator, G = bits 20-23 of G, B = bits 16-19 of the
  shifted B accumulator (top nibbles of the 8-bit integer parts); then the blend of section 4 with this `a`.
  Output alpha nibble `0xF`. The R accumulator's top nibble is masked off, so out-of-range R does not reach
  alpha here.

## 7. RenderBlt (`0x008480C0`) - kernel dispatch

Arguments `(p1, p2, rectA, rectB, src, mode)`: `rectA`, `rectB` point to 24-byte records, `src` to a 32-byte
source descriptor, `mode` an int. It copies the three records by value and calls
`kernel(p1, p2, *rectA, *rectB, *src)` (cdecl, 88 bytes of arguments), where
`kernel = table[src.format * 2 + src.flag][mode]`: `src.format` is the dword at `src+0x10` (0..6),
`src.flag` the byte at `src+0x1C` (0/1). The row-pointer array is at `0x00AC2100` (14 entries); row `r` is the
12-entry array at `0x00AC20D0 - 0x30*r`, so the 168 entries occupy `0x00AC1E60`-`0x00AC20FF`. Neither index is
range-checked (U3).

(Kernel semantics: section 9, in progress.)

## 10. Unknowns

- U1. Count 0 in slots 1-4: the loop decrements before testing, so 0 would run 2^32 iterations; whether any
  caller passes 0 is not checked.
- U2. RenderGradient with R outside 0..255 along a span corrupts the alpha nibble (section 5); whether callers
  ever produce such ramps is not checked.
- U3. RenderBlt does not validate `src.format`, `src.flag` or `mode`.
