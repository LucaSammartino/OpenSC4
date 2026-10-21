| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-lotrecords-final4 (lot records: cSC4Lot, cSC4BusinessUnit, cSC4LotRetainingWallOccupant completed per LR1-LR5 and §7.1/§7.2) | `core-lotrecords-final4` / (engine lane) | ready at report commit (code `a8a37215`, comments after `b3d1ed2f`): core only, no C ABI / plugin / CMake change, game n/a; gate cargo 4116/0, clippy 0, build 19/19, ctest 65/65; questions LR6, LR7 | wire `lot_records::walls` into a lot developer host (AddLotFoundation / ClearLot callers, lot-developer.md) in a glue lane; answer LR6/LR7 |

## Report

**What changed** (`engine/core/src/lot_records/`, core Rust only; no construction, scheduler, visual, plugin, CAPI,
tools or ABI edits):

- `lifecycle.rs` (LR1, LR2, LR3): `LotHost` now asks for `terrain_altitude(x, z)` (terrain slot 13),
  `lot_slope(rect)` (GetLotSlope / GetAreaSlope(rect, false)) and `has_terrain_view()`, replacing the old
  `center_altitude_and_slope`. New `lot_center` (GetCenter, exact), `lot_set_altitude` (acts only when
  |a - altitude| > double 0.001, NaN/equal no change; stores a, then both slope fields := f32(slope)),
  `lot_update_slope` (field 8 only). `Init` step 1 = GetCenter -> GetAltitude -> SetAltitude. `SetFlags` stores
  first, then returns without a terrain view. `business_unit_remove` erases every equal node, always SetUnit(0)
  then SetTravelDesignate(0), and returns nothing (no defined AL).
- `lot.rs` (LR4): `Write` field 2 = held configuration object's ID or 0; the ID read is never written.
- `walls.rs` (new, §7.1/§7.2, LR5): `WallHost` trait (lot manager slots 6/9/11/12, terrain slot 14, wall type
  draw, BDS slot 29, occupant manager slots 20/21/33, visual model add/remove); `add_retaining_wall_occupant`
  (AL always 0; mask from non-zero hashes, rect-centre position, size (16w, top - a, 16d), lot ref, type t, insert,
  flag bit 2), `remove_retaining_wall_occupant` (first wall at location cell, always clears bit 2, AL 0/1),
  `lot_wall_edge_flags` (need/bits rules, 33rd-cell bit reuse, round-to-even via 12582912 trick, wrapping hash,
  top), `add_retaining_wall_model` (removal leaves the mask; build ok sets, failure clears, early return
  untouched), `update_retaining_walls` (per-edge 32-bit compare, remove iff old != 0, add iff new != 0,
  SetEdgeHashes once if changed, mask untouched by it, all-zero walls kept, no null guard),
  `update_surrounding_retaining_walls` (host ring order, never the lot itself, refs released).
- Tests: `lifecycle_tests.rs` T17, T18, T19, T20 + SetAltitude threshold/NaN/UpdateSlope; `walls_tests.rs` (13):
  T21 (both halves, hash 43,282, top 103.6, size y 3.6f, AL 0), props failure / t = 0, need/bits rules, bit wrap,
  removal paths, T22 (call order, mask 0b1111), T23, all-zero update, T24 (+ save/load mask repair), null-guard
  panic, T25 (order, not self, strong counts unchanged).

**Not claimed:** the 949 saved mask-less walls are not explained by this code; §7.2 calls a failed build only one
possible cause *[inferred]*, and nothing here relies on that.

**Measured**: baseline (pre-edit) cargo 4099/0; final `qbuild all`: cargo 4116 passed / 0 failed, clippy 0,
fc4verify build 19/19, ctest 65/65. fmt clean on touched files. No game run (core only, no plugin change).

**Questions** (docs/specs/questions/lot-records.md): LR6 vertex range when slot 11 returns false; LR7 slot 33 true
with null wall on the update path (core panics, no synthesised guard).

**Owners**: `cSC4Lot` (record, Read/Write, Init/SetAltitude/GetCenter/UpdateSlope/SetFlags/PostCityInit/Shutdown),
`cSC4BusinessUnit` (record, Add/Remove/GetMembers) and `cSC4LotRetainingWallOccupant` (record, Read mask repair,
build/remove/update/surrounding) are complete in core; base-texture owner unchanged. Host wiring into the game's
lot developer is a separate glue step.
