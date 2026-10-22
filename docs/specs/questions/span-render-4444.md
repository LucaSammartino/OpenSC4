# span-render-4444: open questions
<!-- Questions about [span-render-4444.md](../span-render-4444.md). Append new entries at the END; when one is answered, move it to answered/span-render-4444.md. Rules: README.md. -->

<!-- from: core-span4444-4 (2026-10-21) -->
- **SP1 - section 6 / section 4:** RenderAlphaGradient with an alpha ramp outside 0..255 gives `a = acc >> 27` in
  17..31 (or a wrapped value). The packed R|B register form `((sRB - dRB)*a + 16*dRB + 0x808) >> 4` can then borrow
  or carry between fields. What is the exact bit result for a > 16 (masking after the shift, field placement)?
  [core evaluates each channel separately in wrapping 32-bit maths and keeps its low nibble; alpha 0xF]
- **SP2 - section 2:** a span record whose `end - start` exceeds the pixels left in the row (or whose `end` is below
  `start`, wrapping to a huge width) writes linearly past the row end in the original. Do callers ever pass such
  records, or may a reimplementation require `x + width <= 65536` on one row? [core walks linearly from (x, y) and
  rejects any byte outside the supplied buffer before writing anything]
<!-- from: re-spanbridge4444-4 (2026-10-21) -->
- **SP4 - section 1.2 (type-7 producer; narrowed 2026-10-21 by re-spanbridge4444-4 round 2):** the static sweep in
  section 1.2 finds one shipped type-7 buffer, the screen buffer in a 16-bit screen mode, and it gets a SimGL context,
  never a `cGZDrawContextRenderToBuffer`. Colour type 1 stays 1 and is rejected. One question is still open and needs
  a runtime breakpoint on `0x008307A8` (the type-7 Init branch) in a scripted 16-bit session, or a hit count of 0 from
  it: does any `cIGZDrawSystem` slot-4 caller (`0x0082BAB0`) pass a buffer whose type came from data (a buffer
  resource v1/v2 header with type 7, or a Gimex load whose target type is not a literal)?
  [no native 4444 inputs captured; the core stays unit-tested only]
<!-- from: glue-span-context-final4 (2026-10-22) -->
- **SP6 - section 1.3 (fixture inputs and counting):** (a) slot 13 SetBltSource `(src, x, y, sx, sy, rect*)` and
  slot 15 SetBltTiling `(rect*)`: what do x, y, sx, sy mean (source origin and scale, or destination-to-source steps),
  and is the 16-byte rect `(x, y, w, h)` or `(left, top, right, bottom)`? (b) "count by the return address" with the
  call sites of span-render-565.md section 1.1: is each listed address the CALL instruction (return address = site +
  instruction length) or the return address itself? [the fixture uses rects at the origin, which read the same both
  ways, x = y = 0 and sx = sy in {1, 2, 0.5/0.75}; the observed site matched the expected path in all 378 cases;
  return addresses 1..7 bytes after a listed site are counted for it]. In-game evidence for SP1: through the native
  context, 4 of 6 type-7 alpha-gradient cases with alpha outside 0..255 differ (1-2 bytes, R nibble), every in-range
  case is exact (run gsctx4-r2-union2032).
