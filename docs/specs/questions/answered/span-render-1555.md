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
