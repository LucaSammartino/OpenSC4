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
<!-- from: glue-span4444-impl4 (2026-10-21) -->
- **SP5 - section 1.2 "What a game-pixel test can use":** the pixel test through a COM-created `cGZBuffer`
  (CLSID `0xC470D325`, IID `0x20732180`) needs contracts no spec or docs/abi row publishes. Please give, for
  GOG 1.1.641: (a) the `cIGZBuffer` slot of `Init` (`0x008269B0`), its argument list (width, height, the colour-type
  pair: by value or by pointer?), callee pop count and return; (b) how a caller gets the pixel base and row stride of
  an Init-ed buffer (slot, lock/unlock pairing, arguments, pops); (c) how a plug-in obtains `cIGZDrawSystem`
  (`cRZSysServPtr<cIGZDrawSystem, 0x4B2EA3B3, 0xEB2EA3CB>` in widget-toolkit.md: is `0x4B2EA3B3` the service ID
  and `0xEB2EA3CB` the IID?) and the reference rule of slot 4's `out`; (d) the context's slot 3 `Init(buffer)`
  pop count and return, and the public context slots that set `+0x74`, `+0x41`, `+0xC4`, `+0xB4`, `+0x44` before
  slot 22 `DrawPreclippedSpanList`. [glue-span4444-impl4 creates the buffer and releases it unused; its
  original-versus-ours pixel comparison calls the five original slot functions directly on a constructed,
  plug-in-owned fixture with the published section 1.1 contract]
