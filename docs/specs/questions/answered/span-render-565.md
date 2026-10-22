# span-render-565: answered questions

<!-- from: glue-span565-impl4 (2026-10-21) -->
- ~~**SR565-7 - section 1.2 (fixture conventions and setters):** the constructed-buffer route names draw system slot 4
  `(buffer, out)`, context slot 4 `Shutdown` and "the context's public setters" without their convention or slots.
  The bridge's trace takes the literal reading: draw system slot 4 thiscall, callee pops 8, bool result; Shutdown
  thiscall with no argument; and, for want of published setters, it writes the documented fields `+0x41`, `+0x74`,
  `+0x44..+0x70`, `+0xB4`, `+0xC4` directly. Please publish (a) the convention/return of `0x0082BAB0` and
  `0x0082FF20`, (b) the setter slots for alpha, colour, ARGB, gradient (`0x0082FDB0`'s slot) and texture.~~
  **ANSWERED 2026-10-22 (re-span565-context4):** MATCHES — the bridge's stated conventions: draw-system slot 4
  is thiscall `(buffer,out)`, pops 8, returns byte AL (RET `0x0082BADF`/`0x0082BAE8`); Shutdown is thiscall,
  no arguments/pops, void (RET `0x0082FF6A`, or tail-jump `0x0082FF66`). Spec §1.3 publishes setter slots 7
  (ARGB plus packed colour, pops 0xC), 8 (signed-positive alpha enable, pops 4), 12 (gradient, pops 0xC), 13
  (texture, pops 0x18), 14/15 (tiling, pops 0/4), with field provenance and vectors. Image entries
  `0x00AC17EC`, `0x00AC17F0`, `0x00AC1800`–`0x00AC180C` fix these slots. Type-5 packing is `0x00825DE7`:
  `0x80FF8040` → `+0xB4=0x80FF8040`, `+0xC4=0xFC08`; alpha input -1 disables blending. Shared 2D context
  applicability is proved by type-5 Init at `0x008307A0`/`0x008307EE`; the separate 3D spec is inapplicable.

<!-- from: glue-span565-impl4 (2026-10-21) -->
- ~~**SR565-8 - section 6.1 (texture record):** which context fields hold the RenderBlt `src` record that
  `DrawPreclippedSpanList` copies ("local copy of the texture record") and which setter fills `+0x24`, `+0x28/+0x2C`,
  `+0x38/+0x3C`, `+0x40`, `+0x94..+0xA0`, `+0xA4`? Until then the bridge compares RenderBlt by direct slot-5 calls on
  constructed buffers, not through `DrawPreclippedSpanList`.~~
  **ANSWERED 2026-10-22 (re-span565-context4):** DIFFERS — the premise of a persistent context `src` record:
  slot 13 holds a buffer interface in `+0x24`, writes placement/scales `+0x28..+0x3C` and mode byte `+0x40`;
  slot 15 fills `+0x94..+0xA0` and enables `+0xA4`, slot 14/Init clear that byte. Slot 22 builds the six-dword
  `src` on its stack from source buffer vfuncs after Lock(`0x800`), `0x00830306`–`0x00830357`, then adjusts it
  for plain/stretch/tiled mode before calling RenderBlt. Spec §6.2 maps every field and specifies sampling and
  failure vectors. Texture QI failure still returns AL=1 (`0x00830008` → `0x00830270`), with no installed texture
  or output-rect write; null source returns AL=0 (`0x00830290`). The public setter/slot-22 route is statically
  established; shipped-path execution remains SR565-5, and the new vectors have not been run.
