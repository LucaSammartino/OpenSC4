# span-render-8: open questions
<!-- Questions about [span-render-8.md](../span-render-8.md). Append new entries at the END; when one is answered, move it to answered/span-render-8.md. Rules: README.md. -->

<!-- from: re-span8-4 (2026-10-21) -->
- **SR8-2 - whole spec:** does any shipped path create a colour-type 2 render target (`Init` `0x00830710`, jump
  table `0x008308B0` entry type 2 → vtable `0x00AC188C`)? No caller analysis was done; if one exists, name the
  caller and what it draws (texture formats, kinds), so the reachable slots of section 7 can be listed.

<!-- from: core-span8-4 (2026-10-21) -->
- **SR8-3 - sections 7.3, 7.5:** do the row 13 `ConstantAlpha` bodies (plain `0x00864480`, stretch `0x008646C0`)
  read a texel? Section 7.5 says only that the tiled one (`0x00864940`) divides by W only, and section 7.4 that row 13
  compares no texel bits. The core (engine/core/src/span_render8, `body_reads_texel`) takes the literal reading that
  none of the three reads a texel, so no source bounds apply to them. Row 13 is unreachable, so this only matters
  for completeness; please confirm or correct. Also: for row 13 with key 0 (draws nothing), do the reading bodies
  still read their texels? The core checks their source addresses either way.

<!-- from: re-span8-bridge4 (2026-10-21) -->
- **SR8-4 - section 10.5 step 4:** with the antialias byte `ctx+0x43` set, `cGZDrawContextRenderToBuffer::Draw`
  (`0x00831000`) sends primitive types 6/7/8 to `FillAntialiasedPolygon` (`0x008308D0`), which writes the target
  through the buffer's set-pixel vfunc `+0x58` and bypasses the span renderer. What value does it write on a
  colour-type 2 (1 byte per pixel) target for full and partial coverage (the colour comes from `ctx+0xB4` bytes and
  possibly the `+0x78` mapping, which is 0 for type 2)? Belongs to the context spec, not to cSpanRender8.

<!-- from: glue-span8-impl4 (2026-10-22) -->
- **SR9 - section 10.1 / 10.5, reconciled native ABI prerequisite:** can the ABI owner publish the six
  `cSpanRender8` vtable `0x00AC188C` rows (slots 0..5, thiscall, callee pops 4/16/16/20/16/24;
  destructor returns this in EAX, render slots return void) and reconcile the native-context rows before this
  bridge is enabled? `docs/abi/interfaces.tsv` has no rows for this renderer, `cGZDrawContextRenderToBuffer`,
  `cGZDrawSystem` or `cGZBuffer`. The 17 rows in `docs/abi/proposals/re-span-context4.tsv` are explicitly
  unapplied per its adjacent report; in particular slot 22 still says `unknown` while section 10.5 says void.
  The constructed route needs draw-system slot 4 (pop 8, AL), context slots 3/4/7/8/9/12/13/14/15/22
  (pops 4/0/12/4/4/12/24/0/4/8), and buffer Init/Lock/Unlock plus the descriptor getters specified in
  section 10.3. [Stop this path under the lane's no-unreconciled-ABI instruction; no table edits, new native
  forwarders or fixture calls. Existing sibling infrastructure is unchanged.]
