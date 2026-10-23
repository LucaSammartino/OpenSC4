| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-occfilters: the occupant filters, folded into occupant-manager.md §13.2 (RE spec lane) | `re-batch5` branch (outside contributor checkout, fork amarret/OpenSC4; on main `f01d1ce4`) | **ready** (docs and RE tables only). occupant-manager.md §13.2 rewritten from the instruction bytes and corrected in seven places; OP4, OP5, OP8, OP9 answered, OP6 and QV1 narrowed, OM9–OM10 filed; 100 names (65 new, 35 renamed); bulldozer-tool.md and zone-tool.md lot-slot corrections. Specified coverage +2,758 B / +49 functions (3,233,636 B, 57.9%) | OM9 (the occupant groups and properties the filters test by raw ID), OM10 (the cheat that installs a flagless realistic filter), OP6 (`cSC4DummyAutoOccupant`) |

## Report

Static RE only: the instruction bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`), read
with the local disassembler and the Ghidra export. This lane was written as a separate spec
(`occupant-filters.md`) on 2026-10-20 and held back when `main` gained its own §13.2; this is that work folded into
§13.2, checked against the bytes wherever the two disagreed.

### What changed

- **occupant-manager.md §13.2** rewritten as 13.2.1–13.2.8: the interface and the base class (layout, every slot,
  the record, the `+4` view); the ten classes the COM director registers, with sizes, factories and start values;
  the content filters (default, realistic, demolition, type list, hashed type set with the model maker's two sets,
  network, group); `SC4GetOccupantViewLevel` in its exact test order and the view-level filter; the string
  specification filter (fields, the word parser and its box, type and level words, three quirks, `GetOccupants`);
  the purpose filters (inside-lot, immovability, active fire, missing plugin packs, zoom and rotation, hover and
  query, demolish-by-networks, water, subway and the station test, blocking lot, surface networks, flora); the
  filters outside the family; and the data map view's filter for the 3D view (`0xCA0E3852`, new).
- **Corrections to the earlier §13.2**, each re-read in the bytes:
  - the default and realistic filters test occupant groups (occupant slot 22 `IsOccupantGroup`, `+0x58`, SDK
    order), not holder properties; the default's holder test is holder slot 3 `HasProperty` (`0x005E96EE`), not a QI;
  - the realistic filter keeps a lot base texture when `+0xC` is set and drops it only on a ready lot
    (`0x005E9A54`); the same shape for props and `+0xE` (`0x005E99E9`);
  - `AreModelsAvailableForZoomAndRotation` returns 0 when the resource is found (`0x0048C3F6`), 1 when not
    (`0x0048C411`), 2 with no instance; the missing-pack filter takes result 1, the occupants whose model is missing;
  - the hover test sends props down the flora path (`0x004C37DF`), so props need `0x8A416A99`;
  - the demolish-by-networks filter inverts the QI (`NEG`/`SBB`/`INC` at `0x0061F5B2`): it excludes `0xA9764C8C`;
  - `SetLot` builds the box from the lot's cell rectangle corners (`0x006BFAC0`, city slot 96 twice), not the centre;
  - class names from the exe's ID table: `0xE9C75ED1` `cSC4OccupantTypeFilter`, `0xE9C75EE8`
    `cSC4OccupantTypesFilter`, `0xAA25CB90` `cSC4OccupantTypesFilterHashed`, `0xEAC3A3D2` `cSC4NetworkOccupantFilter`;
  - the base's slot 6 is `0x005E9790`; `0x006C4A00` is a deleting destructor several subclasses share.
- §15 quirks 19–23 and §18 conformance tests 25–32 added.
- **map-view.md** §2: the four filter properties now say what they do (§13.2.8); `0x4A0B47F0` goes to the lot
  developer's `SetForceEmptyLotBaseTextures`.
- **bulldozer-tool.md**, **zone-tool.md**: lot slot 33 is `GetState()` (whole-lot rule: state past ready, not
  "larger than one cell"), slot 39 `IsInDemolishableState`, `0xA8C86142` is `cISC4PowerPoleOccupant`, slot 46
  `GetBuildingType(false)`; tests and BT1 / ZT1 narrowed to match.
- **re/symbols/occupant-filters.tsv**: 100 names (65 new; 35 replace auto names, among them `0x007CB920`
  `cSC4OccupantFilter::Release`, was a constructor, and `0x004696C0` `std::find`, was `vector::push_back`). The
  `names_windows.tsv` delta (−35 +100) is identical to `merge_names.py`'s.
- **tools/coverage-map.tsv**: `fn:SC4GetOccupantViewLevel` and `fn:GZCOM_CreatecSC4*Filter*` → occupant-manager.md
  (the classes are already owned by the `cSC4Occupant*` and `cSC4*OccupantFilter` rows once named).

### Measured

`tools/coverage.py` on the local (Steam) export, measurement only: `main` `f01d1ce4` specifies 3,230,878 bytes in
14,571 functions (57.9%); with this lane 3,233,636 bytes in 14,620 functions (+2,758 B, +49 functions; denominator
5,584,579 bytes unchanged). Most of the newly named functions were already owned by a specified neighbour; the
gain is the ones that were not. `docs/coverage-report.md` was not regenerated (the tool accepts only the GOG exe).

### Questions

- Answered and moved to questions/answered/occupants-props.md: OP4 (DIFFERS: the realistic flags), OP5 (the
  blocking-lot filter), OP8 (the types, hashed and `0xCA0E3852` filter bodies), OP9 (the automata types are the
  vehicle, pedestrian, aircraft and watercraft types).
- Narrowed in place: OP6 (only `cSC4DummyAutoOccupant` stays), map-view QV1 (only the terrain and zone displays).
- Filed in the new questions/occupant-manager.md: OM9 (the groups `0x4217`, `0x4208`, `0x421D`, `0x5001` and the
  properties tested by raw ID), OM10 (the cheat at `0x007F365D`).

### Not covered

- The data view filter's group list and two optional filters (`+0x18`, `+0x24`, `+0x28`): the map view never sets
  them, and no other writer was found.
- `cSC4DummyAutoOccupant` (OP6): only its constructor was read.
