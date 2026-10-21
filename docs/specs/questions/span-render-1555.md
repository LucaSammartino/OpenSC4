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

