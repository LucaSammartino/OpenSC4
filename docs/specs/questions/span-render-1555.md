# span-render-1555: open questions
<!-- Questions about [span-render-1555.md](../span-render-1555.md). Append new entries at the END; when one is answered, move it to answered/span-render-1555.md. Rules: README.md. -->

<!-- from: re-span1555-4 (2026-10-21) -->
- **SR1555-1 - section 4.1:** who fills the 0x30-byte gradient record at context `+0x44`
  (`SetGradientTransform` `0x0082FDB0` is the candidate) and in what fixed-point scale? Section 5's
  conversion implies 8.16 with the 8-bit channel in bits 16-23; confirm, including channel order c0 = B.
- **SR1555-2 - section 4.2:** does any shipped code path draw a gradient (`+0x74` set, `+0x41` clear) into a
  colour-type 4 or 6 buffer? RenderGradient takes B from the low 5 bits of the 8-bit value and does not scale the
  G/R start values, unlike the 565 sibling `0x0083DEA0`; if a path exists, an in-game capture of such a gradient
  would confirm the quirk is visible.
- **SR1555-3 - sections 7.3, 7.4:** restate the remaining instantiations against their bytes: keyed rows other
  than A1555 (is the key compared on the raw texel masked per section 6.1?), ConstantAlpha/ModulateAlpha for
  A4444, A1555, R565, X888 and 8-bit sources (does any use the source alpha, or the A4444 4-bit alpha?), and the
  folded tiled body `0x00842C50`.
- **SR1555-4 - section 7.5:** tiling (columns 8-11): the caller (`0x00830437`-`0x008304D1`) builds `b.+0x00`
  ... `+0x0C` from the tiling rectangle at context `+0x94`-`+0xA0` with `0x7FFFFFFF mod` terms; give the exact
  per-pixel source address and wrap rule, and the stretch caller's `b` values (`0x008304EA`, centre `>> 1`
  terms).
