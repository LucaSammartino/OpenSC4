# span-render-565: open questions
<!-- Questions about [span-render-565.md](../span-render-565.md). Append new entries at the END; when one is answered, move it to answered/span-render-565.md. Rules: README.md. -->

<!-- from: re-span565-4 (2026-10-21) -->
- **SR565-1 - section 4.1 / 4.2:** does any shipped code path draw a gradient (`+0x74` set) into a colour-type 5
  buffer? `SetGradientTransform` (`0x0082FDB0`) and its base forwarder `0x0082C360` have no direct caller in the
  export. If a path exists, an in-game capture would show whether the B-channel drift of section 4.2 (start shifted
  right by 3, step with its low 3 bits dropped) is visible.

<!-- from: core-span565-4 (2026-10-21) -->
- **SR565-2 - section 7.3 (Modulate / ModulateAlpha, A1555 source):** the 8-bit expansion of the 1-bit alpha
  of an A1555 texel is not spelled out ("expand the texel to 8 bits per channel"). The core uses bit 15 clear → 0,
  set → 255 (`engine/core/src/span_render565/blit.rs`, `texel_to_8888`). Which value does the A1555 ModulateAlpha
  body (row 2/3, column 3/7/11) use for `A`?
- **SR565-3 - section 7 / 7.3 (ConstantAlpha, A-format rows):** rows 2, 3, 8, 9 have ConstantAlpha bodies of their
  own (columns 1, 5, 9: `0x0083E2B0`, `0x0083E360`, `0x0083E480`, ...) distinct from the X-format rows 4, 5, 10, 11,
  yet §7.3 gives `B(T, D, ca >> 3)` with no use of the texel alpha. Does the A1555 / A8888 ConstantAlpha weight
  involve the texel alpha (or a texel-alpha test), or do the bodies differ only in code? The core follows §7.3
  literally (texel alpha ignored).
- **SR565-4 - section 6.1 / 7.2 (X888 texel size):** the core reads an `f = 5` (colour type 8, R8G8B8) texel as
  4 bytes, inferred from rows 10/11 sharing every non-alpha body with A8888 and from the 32-bit key mask and
  example. Please confirm the bytes per texel the f = 5 bodies step by (plain `x·bpp`, stretch, tiled).

<!-- from: re-span565-bridge4 (2026-10-21) -->
- **SR565-5 - section 1.2 (shipped producer):** in a normal session (startup, region view, city load, 16-bit and
  32-bit display modes), is either of these addresses ever executed: `0x00826472` (cGZBuffer type resolver
  falling back to type 5 because `RZ::GS` returned null) and `0x008307A0` (RenderToBuffer::Init storing the 565
  vtable)? Count hits per address with a breakpoint/counter; also count the five method entries `0x0083DE50`,
  `0x0083DEA0`, `0x0083DFB0`, `0x0083E070`, `0x0083E210`. Zero everywhere confirms that only constructed fixtures
  reach this class.
- **SR565-6 - section 1.1 (destination lock leak):** RenderToBuffer::Init locks the buffer with `0x8040`
  (`0x00830750`) but its type-3/other exit (`0x0083087A`) releases it without unlocking, and the context's final
  Release (`0x008306C0`) releases it without unlocking when Shutdown was not called first. In both cases a
  cGZBuffer keeps flags `0x8040` in `+0x44` and its lock word `+0x38` stays incremented. Must an implementation
  reproduce the leaked lock (observable through a later lock count / flag read), or may it unlock on those exits?

<!-- from: glue-span565-impl4 (2026-10-21) -->
- **SR565-7 - section 1.2 (fixture conventions and setters):** the constructed-buffer route names draw system slot 4
  `(buffer, out)`, context slot 4 `Shutdown` and "the context's public setters" without their convention or slots.
  The bridge's trace takes the literal reading: draw system slot 4 thiscall, callee pops 8, bool result; Shutdown
  thiscall with no argument; and, for want of published setters, it writes the documented fields `+0x41`, `+0x74`,
  `+0x44..+0x70`, `+0xB4`, `+0xC4` directly. Please publish (a) the convention/return of `0x0082BAB0` and
  `0x0082FF20`, (b) the setter slots for alpha, colour, ARGB, gradient (`0x0082FDB0`'s slot) and texture.
- **SR565-8 - section 6.1 (texture record):** which context fields hold the RenderBlt `src` record that
  `DrawPreclippedSpanList` copies ("local copy of the texture record") and which setter fills `+0x24`, `+0x28/+0x2C`,
  `+0x38/+0x3C`, `+0x40`, `+0x94..+0xA0`, `+0xA4`? Until then the bridge compares RenderBlt by direct slot-5 calls on
  constructed buffers, not through `DrawPreclippedSpanList`.
