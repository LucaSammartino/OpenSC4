# occupant-groups: open questions
<!-- Questions about [occupant-groups.md](../occupant-groups.md). Append new entries at the END; when one is answered, move it to answered/occupant-groups.md. Rules: README.md. -->

<!-- from: re-occgroups (2026-10-19) -->
- **OG1 - §3.2, §5.1:** the special filter groups (ID 0; 35 in the shipped scripts) are listed in creation order,
  which is Lua's `lua_next` order over `occupant_group`. Does any consumer of script slot 10
  `GetSpecialFilterOccupantGroups` depend on that order? The same order decides which of two groups with one ID
  can be found by ID, but the shipped data has no such collision.

<!-- from: re-occgroups (2026-10-19) -->
- **OG2 - §3.3:** the scripts say an empty key-effect `bone` means the same as `fx`, but the importer stores the
  empty string. Is the default applied when the effect starts (automaton slots `+0xB0`, `+0xB4`, `+0xBC`, called
  from `HandleKeyEffects` `0x00566A70`), or is an empty bone passed on as is?

<!-- from: core-automata-groups4 (2026-10-22) -->
- **OG3 - §§4.8,7, complete per-owner body inventory:** the implementation brief attributes 15 functions / 1662 bytes to cSC4OccupantGroupTemplate and tKeyEffect, but the published inventory names the group constructor, destructor, deleting destructor, seven nontrivial queries, the shared true filter body, key-effect copy and map-clear helpers (13 distinct bodies, counting the shared true body once). Please publish the exact 15-body-to-owner list and the missing bodies' observable contracts, including any separately counted tKeyEffect constructor/destructor. [Implement only the explicitly published bodies; Rust KeyEffect has explicit supplied state and ordinary string ownership, with no inferred native constructor. No 15/15 owner-credit claim.]

<!-- from: core-automata-groups4 (2026-10-22) -->
- **OG4 - §1 and occupant-manager §4.4, inherited base lifetime:** please state explicitly the group base's initial reference count, AddRef overflow behavior, and whether successful QueryInterface increments it (including IID 1 and the serializable view). What happens to the output/count on failed IID or null output? The shared-filter description's “reference count ... and the type ... initially 0” does not explicitly distinguish both initial values. [Literal initial count 0 and wrapping u32 AddRef; implement the published unsigned Release rule, but do not invent QueryInterface acquisition/failure side effects. Interface mapping can be represented independently.]
