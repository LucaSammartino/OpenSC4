# Map colour buffer fill (`MakeMapColorBuffer`) (behaviour spec)

Scope: the two instantiations of the anonymous-namespace template `MakeMapColorBuffer` (unsigned bytes `0x0079ED90`,
signed bytes `0x0079F230`), which the map view uses to paint a byte grid into a 32-bit colour buffer through a
256-entry palette (map-view.md §4.3: the palette ramp, its premultiplied copy and the resampling that precedes this
step; grid-interpolation.md: the resampling bodies). Callers: the map view at `0x007A2628` and `0x007A29F5`.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Both bodies were read in the disassembly; the rotation jump table `0x0079EFC0` was read in
the PE. Written 2026-10-25 (re-mapcolor8).

## 1. Signature *[compat]*

`MakeMapColorBuffer(G, nOuter, nInner, B, W, H, palette, rotation)`, cdecl, no result:

- `G`: a vector of rows (12-byte vector records), row i holding the cells `G[i][0…]` as bytes;
- `nOuter`, `nInner`: how many rows and how many cells per row are painted;
- `B`: the destination buffer (`cIGZBuffer`), 32 bits per pixel;
- `W`, `H`: the buffer's size in pixels along the outer and the inner axis;
- `palette`: 256 packed colours;
- `rotation`: 0–3, the view rotation.

## 2. Algorithm *[compat]*

1. **Block size.** `sx := 1`, `w := W`; while `w > nOuter`: `w >>= 1` (arithmetic), `sx <<= 1`. Likewise `sy` and `h`
   from `H` and `nInner`. So `sx`, `sy` are the powers of two that bring the buffer down to at most one block per cell.
2. **Placement by rotation** (pixels for x, lines for y): the start `(x0, y0)`, the step per row `(dx, dy)` and the step
   per cell `(ex, ey)`:

| rotation | start | per row | per cell |
|---|---|---|---|
| 0 | (0, 0) | (sx, 0) | (0, sy) |
| 1 | (0, (h − 1)·sy) | (0, −sx) | (sy, 0) |
| 2 | ((w − 1)·sx, (h − 1)·sy) | (−sx, 0) | (0, −sy) |
| 3 | ((w − 1)·sx, 0) | (0, sx) | (−sy, 0) |

   Any other rotation leaves the steps and the start at the raw values of other arguments *[quirk]*.
3. **Lock** the buffer (`B` slot 6 with `0x8040`); when that fails, return. `p0 :=` its bits (slot 34), `pitch :=` its
   pitch in bytes (slot 35).
4. **Paint.** For row i = 0…nOuter−1 and cell j = 0…nInner−1: the index k is the cell byte (unsigned instantiation)
   or `(cell − 128) & 0xFF` (signed instantiation, so −128 → 0 and 127 → 255); the colour `palette[k]`; the block's
   first pixel is at `p0 + (y0 + i·dy + j·ey)·pitch + 4·(x0 + i·dx + j·ex)`; fill `sy` lines of `sx` pixels with the
   colour (`memset32` per line).
5. **Unlock** (slot 7 with `0x8040`).

For rotations 1 and 3 the row step uses `sx` along y and the cell step `sy` along x while each block stays `sx` wide
and `sy` tall, so unequal block sizes overlap or leave gaps *[quirk]*. Nothing is clipped: the caller sizes the grid
and the buffer.

## 3. Conformance tests

1. `W = H = 256`, a 64 × 64 grid: `sx = sy = 4`; with rotation 0 cell (i, j) fills pixels `4i…4i+3` on lines
   `4j…4j+3`.
2. The same with rotation 2: cell (0, 0) fills pixels 252–255 on lines 252–255.
3. Signed grid value −128 paints `palette[0]`, 0 paints `palette[128]`, 127 paints `palette[255]`.
