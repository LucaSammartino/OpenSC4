# effects: open questions
<!-- Questions about [effects.md](../effects.md). Append new entries at the END; when one is answered, move it to answered/effects.md. Rules: README.md. -->

<!-- re-answers34 (2026-10-20) answered EF1, EF4, EF5, EF6, EF7, EF8, EF9 and EF10; they moved to answered/effects.md.
     EF2 (closed re-integrate1) and EF3 (answered re-integrate1) were archived earlier. No open questions remain. -->

<!-- re-effects-contracts4 (2026-10-21) answered EF11, EF12 and EF13; they moved to answered/effects.md. No open questions remain. -->

<!-- re-effects14-4 (2026-10-21) answered EF14; it moved to answered/effects.md. No open questions remain. -->
No open questions. EF15 and EF16 are archived in answered/effects.md.

<!-- re-effects-ef18 (2026-10-24) answered EF18 and EF19; moved to answered/effects.md.
     EF17 remains reserved by the coordinating brief. No open questions remain. -->

<!-- re-effects-ef20 (2026-10-24) copied EF20 from glue-effects-shadow7 and answered
     it in effects.md section 18.7; original question archived in answered/effects.md. -->
<!-- re-effects-ef21 (2026-10-24) answered EF21 from glue-effects-capture8 in
     effects.md section 18.8; original question moved to answered/effects.md. -->

<!-- re-effects-ef21 (2026-10-24) answered EF21 from glue-effects-capture8 in
     effects.md section 18.8; original question moved to answered/effects.md. -->

<!-- re-effects-ef24 (2026-10-24) answered EF23 in effects.md section 18.8.4;
     answered EF22 in sections 18.8.6–18.8.7; originals moved to answered/effects.md. -->

No open questions remain.

<!-- re-ef29 (2026-10-26) answered EF25 from replace-effectsres28 in effects.md
     section 18.9; original question moved to answered/effects.md. -->

- **EF26 - Typed renderer record codec dependencies for the replacement resource (replace-effectsres28):**
  EF25 settles the resource primary/persistence interface and the complete outer
  directory sequence. The replacement now has that physical interface layout,
  shared lifetime, collection publication and the empty-directory stream path.
  Full nonempty resource Read/Write still needs a clean interface to the separate
  renderer value codecs. Section 18.9.5 names particle writer `005AFFF0`, decal
  writer `005A3E00`, dynamic writer `00583620`, visual writer `005AB590` and
  camera writer `005AB340`; sections 3 and particles section 2 name associated
  readers. `docs/abi/functions.tsv` supplies calling convention/argument counts,
  but several entries have unnamed arguments, and e.g. particle reader A is
  thiscall while reader B is cdecl. Please specify each helper's ordered typed
  parameters, receiver (if any), return channel, section-version input (if any),
  and borrowed/owned argument contract. Alternatively, specify the complete
  binary record order/version masks for decal and dynamic descriptions, plus
  the native tractor/wiggle reconstruction from their two serialized vec3s.
  We can then retain the existing description classes as independent renderer
  dependencies without calling native cSC4EffectsResource Read/Write or guessing
  how to call a non-interface helper. Nonempty payload is currently refused;
  no game acceptance or replacement credit is claimed.
