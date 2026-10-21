| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-occprops4 (lot-growth save records: lot, business unit, building, base texture, retaining wall, foundation, bare occupant) | `core-occprops4` / (engine lane) | ready at `2f459ed5` + report: new fc4-core module `lot_records` (lot-records §2.4, §3.2-§3.4, §4-§8, §12), no C ABI / plugin / CMake change, game=false; gate cargo 3613/0, clippy 0, build 183/183, ctest 62/62; T1, T2, T5, T6 byte-exact with entry CRCs; questions OP11, OP12 (occupants-props.md) | 1. Lot after-load work (lr §3.5 Init / PostCityInit). 2. Construction occupant (lr §9), signpost / text decal (lr §10.3, §10.4) after OP1/OP2/OP7. 3. Wire `lot_records::create` into a core load factory when a core owner (zone / lot developer) holds the lots. |

## Report

**What changed** (`engine/core/src/lot_records/`, `lib.rs` one additive line):

- `lot.rs`: `LotRecord` (27 fields of §3.2), `Lot` store object with the unsaved `initialised` flag set before the
  version check (§3.4 step 1); `Read` accepts ≤ 8, legacy (< 6) s32 rect/location/size and the four legacy bools
  as flag bits 6, 3, 4, 5; business unit / designate cleared then read with IIDs `0x6A233069` / `0xE818FCED`;
  population maps resized to P and inserted into (`operator[]`), travel map inserted into; holder result ignored;
  trips (> 6) appended, a trip failure returns false; strategy (> 7). `Write` is version 8, maps in key order,
  stops before field 27 on a trip write failure. Trips reuse `traffic_simulator::{read_trip, write_trip}`.
  `BusinessUnitRecord` / `BusinessUnit` (§4): version 1 only, members appended.
- `mod.rs`: `OccupantRecord` trait + generic `RecordObject<R>` store object; `BuildingRecord` (§5),
  `BaseTextureRecord` + `TextureRecord` (§6, with `set_textures` = `SetTextureSpecification` frame draws through a
  caller-supplied frame count and RNG draw), `RetainingWallRecord` (§7: old lot released first, mask ORed with
  every non-zero hash bit), `FoundationRecord` (§8), `BareOccupantRecord` (§12); `occupant_init` (§2.4:
  AlreadyInitialised / PostCreateOccupant / HighlightCleared); `create(class_id)` factory for the 7 classes.
  Base block reuses `automata::OccupantBase`.

**Exact owners vs partial** (for the coverage map): full record owners: `cSC4Lot` Write/Read (§3.2-§3.4),
`cSC4BusinessUnit` (§4), `cSC4BuildingOccupant` (§5), `cSC4LotBaseTextureOccupant` (§6, incl. SetTextureSpecification
frame rule), `cSC4LotRetainingWallOccupant` Write/Read (§7), `cSC4FoundationOccupant` (§8), bare `cSC4Occupant`
(§12) and occupant Init (§2.4). Partial / not done: lot §3.5 after-load (Init altitude/RNG strategy draw,
PostCityInit terrain lock), §3.6 lot properties beyond the plain holder, retaining-wall `AddRetainingWallOccupant`,
construction (§9), signpost / text decal (§10.3/§10.4), cell-rect rule (§16 Q1). No class registration in the plugin.

**Measured**: 22 new tests. T1 lot (118 B) and T2 business unit byte-exact, entry CRCs `0x1C85D610` / `0x73D2BA7A`
reproduced. T5 building and T6 base texture rebuilt from the decoded values: the spec's 3-decimal bounds were
resolved to exact f32 bits by matching the entry CRCs `0x9175D64A` / `0xB4A11718` (build-runs/crcsolve.py, meet in
the middle over ±0.0005); this also confirms base flags 5 and the empty holder. T7 mask 9 → 15 at payload offset
40; T8 foundation layout (70 B); T9 (version 9: 2 bytes, initialised); T10 (trips appended, maps merged).
Gate (`qbuild all`): cargo 3613 passed / 0 failed, clippy 0, fc4verify build 183/183, ctest 62/62.

**Questions filed** (docs/specs/questions/occupants-props.md): OP11 (u8 map count > 255: are all entries written),
OP12 (base-texture short read: defaults of padded records).

**Choices marked in code**: texture record defaults other than the five `0xFF` bytes are 0; short base-texture read
pads at most 65,536 records; legacy lot setters applied as plain flag bits; retaining-wall Read releases the lot
before the version check (literal order).
