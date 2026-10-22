# Published owner checklist

Source: `docs/specs/occupant-groups.md` sections 1–4,7,10. This module owns
portable decisions/state and does not ask a foreign provider for group results.
Only the occupant property, random draw and opaque owned parameter blocks are
foreign primitives. RNG/seed/resource-key primitives are reused from core.

## cSC4OccupantGroupTemplate

| Published body/entry | Portable implementation | Status / discriminating checks |
|---|---|---|
| constructor `0x005206B0` | `new`, `GroupState::default` | All section 2 defaults; unspecified speed is `None`, not a guessed zero. Inherited initial refcount uses literal zero pending OG4. |
| destructor `0x00520410` | `Drop`, `GroupState::destroy` | Parameters, key effects, trailers, models, anims, nonowning controllers, ID cache freed in published order; tracked block drops and buffer capacity checks. |
| deleting destructor `0x00520780` | `deleting_destroy` | Bit 0 controls allocation retention; bits 1–31 do not; owned data destroyed in either case. No native allocator binding. |
| IsOccupantIncluded `0x0051C3D0` | `is_occupant_included`, injected-draw body | Own ID only, reads property even for ID zero; skip/draw counts, equality, f64 boundary, zero/negative/NaN/infinity, seed fixups and per-object temporary sequence. |
| IsOccupantTypeIncluded `0x004FC900` | `is_occupant_type_included` | Unconditional true. Shared native body with next entry. |
| IsPropertyHolderIncluded `0x004FC900` | `is_property_holder_included` | Unconditional true. |
| IsOccupantGroup `0x0051C380` | `is_occupant_group` | Self and ancestors, including zero. |
| GetClassID `0x0051C3B0` | `get_class_id` | First nonzero, self first, no fallback beyond first match. |
| GetSourceBuildingGroup `0x00505330` | `get_source_building_group` | First nonzero, self first. |
| GetGroupHierarchyDepth `0x0051C4A0` | `get_group_hierarchy_depth` | Root 1, null 0, chain 3. |
| GetGroupIDVariant `0x0051CD90` | `get_group_id_variant` | Cached u32-array snapshot, self first, zero omitted, duplicates kept, stale nonempty versus rebuilt empty cache. Native variant interface is outside this module. |
| GetKeyEffect `0x005204D0` | `get_key_effect` | Low command byte, all strings/flags copied, no parent fallback, failure leaves output unchanged. |
| key-effect map clear `0x0051FC80` | `clear_key_effects` | Clears owned entries; subsequent misses preserve output. |

## tKeyEffect

| Published body | Portable implementation | Status |
|---|---|---|
| key-effect copy `0x0051FD30` | `KeyEffect::copy_to` | Deep independent strings and entire stored flag word, including high bits. |
| Any separately counted constructor/destructor/other body | Not inferred | **Missing contract: OG3.** Explicit input state and Rust string ownership do not claim an unpublished native body. |

Counting the shared true body once gives **13 explicitly named distinct bodies**
across these rows, versus the brief's 15 / 1662 bytes. OG3 requests the actual
per-owner inventory. Do not assign a guessed body/byte count or complete-owner
credit on the basis of this checklist.

## Inherited shared-base operations and remaining contracts

- `add_ref` and `release` own the unsigned counter; Release destroys at 0 or 1,
  decrements above 1, and never treats high unsigned values as negative.
- `interface_view` implements only the published IID-to-view identity mapping.
  **Missing: OG4** covers QueryInterface acquisition/failure/output protocol and
  explicit initial-count/overflow confirmation. This is not a native QI adapter.
- `get_gzclsid`, `write_base_record`, `read_base_record` implement the inherited
  class ID and two-byte version-3 record; malformed/truncated versions fail and
  no imported group fields are serialized.
- `Weak` parents and integer controller handles do not acquire/release COM
  references. The manager must retain live parents. Acyclic live chains are the
  caller's precondition; no unpublished cycle recovery is introduced.
- State/cache synchronization permits getters and filtering on a second thread.
- Lua imports, TTT inheritance, name/ID maps, container references, three-pass
  linking and built-ins are script-manager bodies, not owner implementations.
  Existing `automata/script.rs` remains untouched; its provisional replacement
  policies are not promoted to compatibility claims here.
- OG1 (manager traversal order) and OG2 (effect playback bone default) remain
  outside these owner bodies. Empty stored bone strings are preserved exactly.
- No C ABI, plugin, shadow/replacement key, persistence registration or game
  binding is added. Three independent vectors exercise already-imported states;
  they do not claim importer conformance for section 10 items 1–5 or 9.
