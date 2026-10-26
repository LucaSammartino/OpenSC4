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

## EF25 - Effects-resource creation and complete primary interface (replace-effectsres28)

The requested `effectsres` factory would replace CLSID `2A5118AD` while leaving
the original manager usable. Sections 18.7.3 and 18.8.1 specify resource IID
`CA51189B`, collection IID `C97CF5CE` at resource+4, publication at primary slot 5,
and stream Read at primary slot 7. The SDK supplies the class ID but no resource
interface declaration. `docs/abi/vtables.tsv` supplies the collection vtable
`00AA0A60`, but not the primary vtable `00AA0AC0`.

Please specify the complete primary slot declarations (argument/return types,
stream interfaces and calling conventions), all QI identities/sub-object offsets
used by the existing resource factory/serializer, and reference-count/destruction
contracts. In particular, what are primary slots 3, 4, 6 and the slots after 7;
does QI expose `cIGZSerializable` and/or `cIGZPersistResource`, and at what offsets?
Also specify the writer's supported root versions, output/result contract and
failure-state behavior for a partially read resource. The existing Rust directory
writer explicitly emits only its simulation sections, so its output alone cannot
establish full-resource serialization compatibility.

The lane stopped native resource registration rather than invent these ABI facts.
It separately fixed the specified rejection of root versions outside 3..4
(section 18.8.1), which both Rust directory entry points previously accepted.
