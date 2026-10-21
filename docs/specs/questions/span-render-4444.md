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
- **SP4 - section 1.1 (type-7 producer):** which shipped code path, if any, initialises a `cGZDrawContextRenderToBuffer`
  on a colour-type 7 (A4R4G4B4) buffer in GOG 1.1.641 under the default DX7 driver? Static sweeps found no literal 7
  at the 34 calls of the colour-pair constructor `0x0044A490` nor at `cIGZDrawSystem::CreateBuffer` (`0x0082B980`)
  calls; open are (a) the data-driven types (buffer resources, `cGZGimexFactory::LoadWithReader` `0x0090C4FF`,
  `cSLBuffer::Load` `0x00602999`) - can any of them yield type 7 and then get a draw context via `cIGZDrawSystem`
  slot 4 (`0x0082BAB0`)? (b) what colour type 1 (passed by `cGZWin::create_private_buffer` `0x0099D147`, `cSC4App::Init`)
  resolves to. A runtime breakpoint on `0x008307A8` during a scripted UI session would settle it.
  [no native 4444 inputs captured; the core stays unit-tested only]
