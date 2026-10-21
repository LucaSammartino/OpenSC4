# span-render-1555: answered questions (archive)
<!-- Archive for [span-render-1555.md](../../span-render-1555.md). Lanes do not read this file. Open questions are in ../span-render-1555.md. -->

- **SR1555-1 - section 4.1:** who fills the 0x30-byte gradient record at context `+0x44`
  (`SetGradientTransform` `0x0082FDB0` is the candidate) and in what fixed-point scale? Section 5's
  conversion implies 8.16 with the 8-bit channel in bits 16-23; confirm, including channel order c0 = B.
  **ANSWERED 2026-10-21 (re-span1555-4):** MATCHES — section 4.1: SetGradientTransform 0x0082FDB0 fills the record from 12 floats scaled by 255*65536 (8.16, 8-bit channel in bits 16-23), start +0x8000 then shifted to pixel (0,0) via _ftol; record channel i = float index i (c0 feeds B). No direct caller of it or of the base forwarder 0x0082C360 (reachability stays SR1555-2).
- **SR1555-3 - sections 7.3, 7.4:** restate the remaining instantiations against their bytes: keyed rows other
  than A1555 (is the key compared on the raw texel masked per section 6.1?), ConstantAlpha/ModulateAlpha for
  A4444, A1555, R565, X888 and 8-bit sources (does any use the source alpha, or the A4444 4-bit alpha?), and the
  folded tiled body `0x00842C50`.
  **ANSWERED 2026-10-21 (re-span1555-4):** MATCHES — sections 7.3, 7.4, 7.7: every keyed row compares the raw texel masked as in 6.1 (row 13 uses an empty mask: key 0 hides all); ConstantAlpha never uses source alpha in any row; ModulateAlpha weights by the modulated source alpha (expansion table 7.4); folded 0x00842C50 is a raw tiled copy. All 168 slots verified by emulation against the bytes (3000 cases each).
- **SR1555-4 - section 7.5:** tiling (columns 8-11): the caller (`0x00830437`-`0x008304D1`) builds `b.+0x00`
  ... `+0x0C` from the tiling rectangle at context `+0x94`-`+0xA0` with `0x7FFFFFFF mod` terms; give the exact
  per-pixel source address and wrap rule, and the stretch caller's `b` values (`0x008304EA`, centre `>> 1`
  terms).
  **ANSWERED 2026-10-21 (re-span1555-4):** MATCHES — sections 7.5, 7.6, 6.1: tiled: pointer += L*bpp + T*stride, size = tile, u0 = (M - M mod TW - ox) mod TW, texel ((x+u0) mod TW, (y+v0) mod TH); stretch: unbiased pointer, b = {(du>>1)-ox, (dv>>1)-oy, du, dv} with ox, oy pre-scaled 16.16 by SetBltSource 0x0082FFD0.
- **SR1555-5 - sections 4.1 and 7.3/7.4:** two points the spec leaves implicit. (a) A4444 (colour type 7): the
  spec fixes A in bits 12-15 (key mask `0x0FFF`, "A = bit 15") but not the order of the other nibbles; the
  core assumes R bits 8-11, G 4-7, B 0-3 (A-R-G-B like A8R8G8B8). Please confirm. (b) SetGradientTransform's
  `c_i = trunc(s_i - (dx_i*x0 + dy_i*y0))` on the x87 stack: under which precision control (the game's 24-bit
  one of foundations section 18, or 53/64-bit)? The core rounds every operation to 24 bits; with 24-bit
  precision the subtraction loses low bits of `c_i` once `|s_i|` exceeds 2^24.
  **ANSWERED 2026-10-21 (re-span1555-bridge4):** MATCHES — (a) section 7.3: A4444 nibbles are bits 0-3 B, 4-7 G, 8-11 R, 12-15 alpha (masks 0x0F00/0x00F0/0x000F at 0x00831566-0x0083157A; 0xF8C3 -> 0xC726). (b) section 4.1: 0x0082FDB0 never sets the control word and runs at the caller's precision, 24-bit on the main thread: the product p*K, dx*x0, dy*y0, their sum and s-sum are each rounded to 24 bits (nearest-even), FILD loads exact, then _ftol truncates; worked examples in 4.1.
<!-- from: glue-span1555-impl4 (2026-10-21) -->
- ~~**SR1555-7 - section 1.1 / 6.1:** to drive the renderer through its only native caller
  (`DrawPreclippedSpanList`, context vtable `0x00AC17D0` slot 22) on a constructed type-4/6 buffer, the bridge
  needs, for the context the RenderToBuffer factory (`0x0082B910`) constructs (`0x00830690`): (a) the values of
  `+0x24` (texture), `+0x41`, `+0x42`, `+0x74`, `+0xA4`, `+0xB4`, `+0xC4` right after construction and after
  `Init` (slot 3); (b) the context vtable slots (offset, arguments, callee pop, return) of SetAlphaBlend
  `0x0082FEA0`, SetGradientTransform `0x0082FDB0` (slot 12 is given), SetBltSource `0x0082FFD0`, SetBltModulate
  `0x0082FEB0`, SetBltTiling `0x0082FE70`, ClearBltTiling `0x0082FE60` and the draw-colour setter that fills
  `+0xC4`/`+0xB4`; (c) whether `Init`'s buffer lock (`vfunc +0x18`, `0x8040`) is undone by `Shutdown`/`Release`
  of the context, and with which Unlock argument; (d) the Unlock (`vfunc +0x1C`) argument the textured path passes.
  Until then the bridge's fixture calls the five vtable entries directly (section 1.1 calling convention) and
  unlocks with the flags it locked with (literal reading, marked in `FC4Span1555Shadow.cpp`).~~
  **ANSWERED 2026-10-21 (re-span-context4):** MATCHES - span-render-1555.md section 1.3 and span-render-4444.md section 1.3: Init (slot 3, `(buffer)`, pops 4, byte) leaves `+0x42`/`+0xB4`/`+0xC4` indeterminate (constructor zeroes `+0x24`/`+0x41`/`+0x74`, Init sets `+0xA4` = 0); setters SetColorTable slot 7 (index, count, argb*) pops 0xC, SetAlphaBlend 8 (int) 4, SetBltModulate 9 (byte) 4, SetGradientTransform 12 (m*, x0, y0) 0xC, SetBltSource 13 (6 dwords) 0x18, ClearBltTiling 14 0, SetBltTiling 15 (rect*) 4; Shutdown unlocks with the lock flags `0x8040` (final Release does not unlock); textured path Lock/Unlock `0x800` both. The bridge's literal reading is correct.
