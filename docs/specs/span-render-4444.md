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

Kernel arguments as the kernels read them (`p1`, `p2` and the three records):
- `p1` = span list, `p2` = span count (same span loop as section 2).
- `rectA` (24 bytes) = destination: `+0` base, `+4` stride (bytes). `+8..+0x14` not read by any examined kernel.
- `rectB` (24 bytes) = source image: `+0` base, `+4` stride, `+8` width, `+0x0C` height (the last two read only
  by tiled kernels). `+0x10`, `+0x14` not read by any examined kernel.
- `src` (32 bytes): `+0x00` u0, `+0x04` v0, `+0x08` du/dx, `+0x0C` dv/dy, `+0x10` source format (row), `+0x14`
  constant colour/alpha (ARGB 8888), `+0x18` colour key, `+0x1C` keyed flag (byte), other bytes unread.

## 8. Kernel table: rows and columns

**Rows** (`format*2 + keyed`). The source format is identified from each row's plain `Blt` kernel (how it loads
and splits the source pixel):

| format | source pixel | read as | Mac name (RGBFormat masks) |
|---|---|---|---|
| 0 | A4R4G4B4, 16-bit | copied verbatim | `61440,3840,240,15,2,4` |
| 1 | A1R5G5B5, 16-bit | A = bit 15 | `32768,31744,992,31,2,5` |
| 2 | X1R5G5B5, 16-bit | A forced opaque | `0,31744,992,31,2,5` |
| 3 | R5G6B5, 16-bit | A forced opaque | `0,63488,2016,31,2,5` |
| 4 | A8R8G8B8, 32-bit | A = bits 24-31 | `4278190080,16711680,65280,255,4,8` |
| 5 | X8R8G8B8, 32-bit | A forced opaque | `0,16711680,65280,255,4,8` |
| 6 | A8, 8-bit (alpha only) | R = G = B = max | `255,0,0,0,1,0` |

**Columns** (`mode`): geometry x operation, `mode = 4*geometry + op`.
- geometry 0 = direct (source pixel at the same (x, y) as the destination pixel), 1 = stretch, 2 = tiled.
- op 0 = copy, 1 = constant alpha, 2 = modulate, 3 = modulate + alpha.
- Evidence: every kernel in columns 4-7 carries the extra 16.16 shifts of the stretch walk and every kernel in
  columns 8-11 a `DIV` (tile wrap); columns 1/5/9 take the alpha nibble of `src+0x14`, columns 2/3/6/7/10/11
  use the MMX multiply, columns 3/7/11 additionally blend with the destination. This matches, one for one, the
  12 Mac method names `Blt, BltConstantAlpha, BltModulate, BltModulateAlpha` and their `Stretch`/`Tiled` forms
  (Mac `cBlitter<Dst, Src, keyed>`); the Mac `BltRect` has no Windows column.
- Two table entries are outside this owner range because identical code was folded: row 0 column 4
  (`0x00833730`, in the 1555 range) and row 0 column 8 (`0x00842C50`, in the 565 range); both are a raw 16-bit
  copy that does not depend on the pixel format. So this range holds 166 kernels + 5 methods + the helper
  `0x0084BD40` = 172 functions.

## 9. Kernel semantics

### 9.1 Geometry (source texel for destination (x, y))
- Direct: source pixel `(x, y)` of `rectB` (the caller pre-offsets `rectB.base`). Verified `0x00848160`.
- Stretch: per span, `v = (dvdy*y + v0) >> 16` (unsigned), source row `base + v*stride`; `u = dudx*x + u0`, each
  pixel reads texel `u >> 16` (unsigned) then `u += dudx`. x and y are the absolute destination coordinates; no
  clamping. Verified `0x00833730`.
- Tiled: per span, `tx = (x + u0) mod width`, `ty = (y + v0) mod height` (unsigned 32-bit division; here u0, v0
  are integer pixel offsets, not 16.16); each pixel reads `(tx, ty)`, then `tx += 1` and wraps to 0 on reaching
  `width`. Verified `0x00842C50`.

### 9.2 Conversion to 4444 (copy and constant-alpha ops)
Each channel keeps the top 4 bits of the source channel (truncation, no rounding, no dither):
A1R5G5B5: A = 0xF if bit 15 else 0, R = bits 11-14, G = bits 6-9, B = bits 1-4. X1R5G5B5: same with A = 0xF.
R5G6B5: R = bits 12-15, G = bits 7-10, B = bits 1-4, A = 0xF. A8R8G8B8: A = bits 28-31, R 20-23, G 12-15,
B 4-7. X8R8G8B8: same with A = 0xF. A8: A = bits 4-7 of the byte, RGB = `0xFFF`. 4444: unchanged.
Verified at the plain `Blt` kernels `0x00849C50`, `0x0084BBD0`, `0x0084DAB0`, `0x0084F9E0`, `0x00851700`,
`0x008533A0`.

### 9.3 Conversion to 8888 (modulate ops)
Each channel is widened by bit replication: 4-bit `n` -> `n*17`; 5-bit `v` -> `(v << 3) | (v >> 2)`; 6-bit
`g` -> `(g << 2) | (g >> 4)`; 1-bit alpha -> 0xFF or 0; formats without alpha -> A = 0xFF; A8 -> A = byte,
R = G = B = 0xFF; 8888 unchanged. Verified at column 2 of rows 0, 2, 4, 6, 8, 10, 12
(`0x00848260`, `0x00849DE0`, `0x0084BDB0`, `0x0084DC20`, `0x0084FB60`, `0x00851860`, `0x008534C0`).

### 9.4 Operations
- **Copy** (op 0): write the 9.2 conversion. The destination is not read. Alpha comes from the source.
- **Constant alpha** (op 1): `a = src.colour >> 28` (top nibble of the alpha byte of `src+0x14`, 0..15). The
  source is converted per 9.2, its alpha ignored, then the helper `0x0084BD40` blends each of R, G, B:
  `out = floor((16*d + a*(s - d) + 8) / 16)`, output alpha `0xF`. Because `a` tops out at 15, a fully opaque
  constant alpha does not reproduce the source: s = 15 over d = 0 gives floor(233/16) = 14. (Contrast
  RenderAlpha, whose weight reaches 16.)
- **Modulate** (op 2): widen per 9.3 to `C`, then per byte channel (A, R, G, B)
  `M = (C * K + 128) >> 8` with `K` the matching byte of `src.colour` (MMX word multiply, rounding constant
  `0x0080` from `0x00AC1B80`); write the top nibble of each M byte (A from M too). The destination is not read.
- **Modulate + alpha** (op 3): M as above, then blend R, G, B (top nibbles of M) over the destination with
  `a = M >> 28` by the op-1 formula; output alpha `0xF`. Verified `0x00848380`.
- **Keyed rows** (odd rows): before anything else each source texel is compared with `src+0x18`; if
  `(texel & rgbmask) == key` the destination pixel is left untouched. `rgbmask` is the format's colour bits:
  4444 `0x0FFF`, 1555 and X555 `0x7FFF`, 565 `0xFFFF`, 8888 `0xFFFFFF` (`0x00848DB0`, `0x0084AB70`,
  `0x0084CAD0`, `0x0084E990`, `0x008507C0`). For A8 the mask is empty, so the test degenerates to "key == 0":
  with a zero key **nothing** is drawn, with any other key every texel is drawn (`0x00854070`, `0x00854190`).
  The key compare is present in every column of the keyed rows sampled (rows 1, 3, 13; columns 0-3, 5, 9).

### 9.5 Verification level
Fully read: the five methods, the helper, row 0 columns 0-4 and 8, the Blt kernel of every format, the key test
of rows 1, 3, 9, 13, the 8888 widening of every unkeyed row. The remaining kernels are covered by the feature
sweep only (each kernel's column signature - stretch shifts, DIV, MMX multiply, alpha nibble shift - matches its
column in all 14 rows), not by a line-by-line read; see U4.

## 10. Unknowns

- U1. Count 0 in slots 1-4: the loop decrements before testing, so 0 would run 2^32 iterations; whether any
  caller passes 0 is not checked.
- U2. RenderGradient with R outside 0..255 along a span corrupts the alpha nibble (section 5); whether callers
  ever produce such ramps is not checked.
- U3. RenderBlt does not validate `src.format`, `src.flag` or `mode`.
- U4. Kernels not read line by line (section 9.5): stretch/tiled forms of ops 1-3 for every row, and ops 1-3
  for rows 3, 5, 7, 9, 11 beyond the key test. They are expected to compose 9.1 with 9.2-9.4 but that is not
  proven instruction by instruction.
- U5. Whether the stretch walk ever lets `u >> 16` or `v` run past the source (no clamp in the kernel) depends
  on the caller's u0/du; not checked here.
- U6. Fields `rectA+8..+0x14`, `rectB+0x10..+0x14` and the bytes of `src` beyond those listed are unread by the
  examined kernels; their meaning belongs to the caller (`cGZDrawContextRenderToBuffer`).

## 11. Coverage proposal

- specified: `nGZDrawSoftRender::cSpanRender4444` (whole owner range, 172 functions, 51,809 bytes), with U4 as
  the explicit gap: 5 methods + helper + 22 kernels read in full, 144 kernels by signature sweep.
- No ABI rows proposed: the class is internal to `cGZDrawContextRenderToBuffer` and has no `docs/abi` rows.
