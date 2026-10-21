# span-render-1555: open questions
<!-- Questions about [span-render-1555.md](../span-render-1555.md). Append new entries at the END; when one is answered, move it to answered/span-render-1555.md. Rules: README.md. -->

<!-- from: re-span1555-4 (2026-10-21) -->
- **SR1555-2 - section 4.2:** does any shipped code path draw a gradient (`+0x74` set, `+0x41` clear) into a
  colour-type 4 or 6 buffer? RenderGradient takes B from the low 5 bits of the 8-bit value and does not scale the
  G/R start values, unlike the 565 sibling `0x0083DEA0`; if a path exists, an in-game capture of such a gradient
  would confirm the quirk is visible.

<!-- from: re-span1555-bridge4 (2026-10-21) -->
- **SR1555-6 - section 1.2:** does any shipped path create a colour-type 4 or 6 buffer that gets a
  `cGZDrawContextRenderToBuffer` (vtable store `0x0083078A` executes), and does `DrawPreclippedSpanList` then call
  `0x008310B0`/`0x00831100`/`0x00831200`/`0x008312D0`/`0x00831470` in a normal session (title screen, city load,
  menus, region view)? The static sweep of section 1.2 found none. A runtime hit counter on these six addresses
  decides it; until then only the constructed-buffer test of section 1.2 exercises the class.

<!-- from: glue-span1555-impl4 (2026-10-21) -->
- **SR1555-7 - section 1.1 / 6.1:** to drive the renderer through its only native caller
  (`DrawPreclippedSpanList`, context vtable `0x00AC17D0` slot 22) on a constructed type-4/6 buffer, the bridge
  needs, for the context the RenderToBuffer factory (`0x0082B910`) constructs (`0x00830690`): (a) the values of
  `+0x24` (texture), `+0x41`, `+0x42`, `+0x74`, `+0xA4`, `+0xB4`, `+0xC4` right after construction and after
  `Init` (slot 3); (b) the context vtable slots (offset, arguments, callee pop, return) of SetAlphaBlend
  `0x0082FEA0`, SetGradientTransform `0x0082FDB0` (slot 12 is given), SetBltSource `0x0082FFD0`, SetBltModulate
  `0x0082FEB0`, SetBltTiling `0x0082FE70`, ClearBltTiling `0x0082FE60` and the draw-colour setter that fills
  `+0xC4`/`+0xB4`; (c) whether `Init`'s buffer lock (`vfunc +0x18`, `0x8040`) is undone by `Shutdown`/`Release`
  of the context, and with which Unlock argument; (d) the Unlock (`vfunc +0x1C`) argument the textured path passes.
  Until then the bridge's fixture calls the five vtable entries directly (section 1.1 calling convention) and
  unlocks with the flags it locked with (literal reading, marked in `FC4Span1555Shadow.cpp`).
