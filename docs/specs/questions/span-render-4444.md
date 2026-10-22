# span-render-4444: open questions
<!-- Questions about [span-render-4444.md](../span-render-4444.md). Append new entries at the END; when one is answered, move it to answered/span-render-4444.md. Rules: README.md. -->

<!-- from: core-span4444-4 (2026-10-21) -->
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

<!-- from: glue-span4444-context4 (2026-10-22) -->
- **SP7 - sections 1.3.1, 7 and 9.1 (source/destination alias):** may a native context's texture allocation
  overlap the destination, and if so what read/write ordering is required for self-blits, including keyed,
  stretched and tiled paths? The specified plain pointer bias can precede the texture allocation, so the bridge
  stages only the sampled texture texels instead of presenting `stride*height` bytes from that biased pointer
  as an allocation. [Nonoverlapping textures supported; overlapping source/target regions explicitly refused
  with counted original fallback pending this contract. No full-class/default replacement credit claimed.]
