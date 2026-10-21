| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-lotrecords4 (lot records: lot life cycle, business unit membership, trip data, base texture, retaining wall) | `core-lotrecords4` / (engine lane) | ready at `e5421918` (code `38f2d8d5`) + report: `lot_records::lifecycle` (lr §3.5, §3.6, §4 + lot-developer §5.7 Add/Remove), core only, no C ABI / plugin / CMake change, game=false; gate cargo 3817/0, clippy 0, build 56/56, ctest 63/63; questions LR1-LR5 | 1. Answer LR1-LR5 (LR5 blocks building retaining walls). 2. Regenerate docs/specs/QUESTIONS.md (`tools/questions.py index`, not run: tools/ off-limits to engine lanes). 3. Wire `lifecycle` into the zone developer core's Init / PostCityInit walk. |

## Report

**What changed** (`engine/core/src/lot_records/`, core Rust only):

- `lifecycle.rs` (new): `LotHost` trait (terrain altitude+slope at the lot centre, lot RNG `RandomUint32Uniform`,
  configuration lookup, terrain cell lock / unlock); `lot_init` (§3.5 Init: new-lot altitude/slopes/travel jobs,
  one draw for strategy `0xFF`, configuration lookup then ID := 0), `lot_set_flags` (bit-0 change locks/unlocks the
  rect), `lot_post_city_init` (`f := flags; flags := 0; SetFlags(f)`), `lot_shutdown` (clear populations, travel
  map, designate, configuration, `unit.Remove(lot)`); business unit `Add` (no duplicate, always SetUnit),
  `Remove` (unlink, SetUnit(0), SetTravelDesignate(0)), `GetMembers`; §3.6 lot property ID constants.
- `lot.rs`: `Lot.configuration` (held configuration ID, unsaved); `Write` saves it as field 2; `Lot::constructed()`
  (strategy `0xFF`).
- `lifecycle_tests.rs` (new, 14 tests): T13 (no draw, altitude kept, T1 bytes unchanged after Init; `0xFF` draws
  exactly once), T14 (`0x5B` locks all rect cells at the altitude, `0x50` none), T9 + Init (altitude stays 0),
  configuration found / missing (missing saves 0), unlock on bit-0 clear, BU add/remove order and idempotence,
  shutdown, §3.3 uninitialised trip fields (`0xCCCC`, `0xCCCCCCCC`) byte-exact.
- `tests.rs`: helpers made `pub(super)` for the new test file.

**Owners** (whole-owner credit): `cSC4Lot` record + §3.5 life cycle complete apart from the LR1/LR4 choices;
`cSC4BusinessUnit` record + Add/Remove/GetMembers complete (LR3 return value); `cSC4TripData` complete (record in
`traffic_simulator::record`, T15/T16 there, §3.3 here); `cSC4LotBaseTextureOccupant` complete (core-occprops4).
`cSC4LotRetainingWallOccupant`: record Write/Read and T7 only; `AddRetainingWallOccupant` /
`UpdateRetainingWalls` are not specified (LR5), so this owner stays partial.

**Measured**: `qbuild all`: cargo 3817 passed / 0 failed, clippy 0 diagnostics, fc4verify build 56/56, ctest 63/63.
No game run (core only, no plugin change).

**Questions** (docs/specs/questions/lot-records.md, new prefix LR): LR1 GetCenter / SetAltitude slopes, LR2 lock
order, LR3 Remove return, LR4 config ID of a read-but-not-Init lot, LR5 retaining-wall build path.
