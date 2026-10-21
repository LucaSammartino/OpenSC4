| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-construction4 (construction occupant record + life cycle, lot-records §9, lot-developer §6.7) | `core-construction4` / (engine lane) | ready + report (round 3: v4 record via the NO15 56-byte legacy transform, `Xform::read_into`/`legacy_load_into`): fc4-core `lot_records::construction` (record Write/Read incl. v4, tuning, geometry/timing, SetLot, DoDailySim, completion, Shutdown, lot-texture/spark/crane draws) through a typed `ConstructionHost`; partial owner (visual steps host-delegated); no C ABI / plugin / CMake change, game=false; gate cargo 3772/0, clippy 0, build 54/54, ctest 63/63; questions OP13-OP16 | 1. After OP13: map the life-cycle values into the saved scalars so a loaded construction resumes. 2. After OP14/OP15: saved-ref fill, #8 source, subscription set. 3. Crane fit, DivideLot, ground/roof props, crew (ld §6.7.10 open). 4. Wire into the lot developer's `start_construction_occupant`. |

## Report

**What changed** (`engine/core/src/lot_records/`):

- `construction.rs` (new): `ConstructionRecord` (lr §9 #1-#15): v5 Write; Read rejects < 4 before the base,
  reads > 5 as 5 *[quirk]*, v4 reads the v5 layout with the 56-byte legacy transform (round 3, NO15 answered); bools read as byte != 0, written 0/1; ref lists replaced (not appended), no pre-allocation from the count;
  the 19 #14 scalars kept as raw typed data (`ConstructionScalars`). `ConstructionTuning` (ld §6.7.2, code
  defaults, `r24(1/v)` for texture scales and grow/remove). `rebuild_model` (§6.7.3: ground with NaN rule, startY,
  f, floor idiom, N, scaffold box), `timing` (§6.7.4: D 30/10 rules, A/B/C with the `min(...,1)` quirk, E signed max
  then unsigned divide). `ConstructionOccupant` life cycle: `init` (5 subscriptions + tick at 0x80), `set_lot`
  (§6.7.4 order), `do_daily_sim` (§6.7.5 state machine, sim date not message data), completion order
  (completion 1.0, renderer remove, occupant-manager remove, lot `SetState(3,0,0)`, Shutdown), `shutdown`
  (0xC9E7B1E8(0) first, unsubscribe, untick, release), `set_building_completion` (posts only on change),
  `do_message`, `do_animation_tick` (msAcc + crane), `position`. Draws: `lot_texture` (step 6), `spark_count`,
  `reset_iterator` (table mode, n-1 `rand()`), `scatter_sparks`, `CraneAnim::animate` (0→1 pause draw, 1→2, 2→0
  swing with caller-supplied limits and `k`).
- `mod.rs`: `pub mod construction`, factory entry for `0xA97F909E`; `tests.rs` helpers made `pub(super)`.
- `tests_construction.rs`: 14 tests.

**Owner status**: exact for the record Write/Read on v5 and later, the tuning, geometry, timing, daily schedule,
completion and Shutdown order, and the draws of tests 23-25. **Partial**: `AttemptAddCrane` fit,
`DivideLot`, ground/roof props, crew, dust, scaffold model and the tick's interpolation are host callbacks (ld
§6.7.10 open); the visual messages (0x06CDB65B, 0xA6B79602, 0x26D31EC1) are not handled; the life cycle is not
persisted in the record (OP13), so a loaded construction does not resume. Not registered in the plugin.

**Measured**: ld §16 test 20 (all 9 rows incl. f bits `0x40533335`, `0x419FBBBC`, startY `0x42C83333`), test 21 (all
completion days, segment days 1/8/15/22, removal 29, day-0 variant 29, D=0 segments 1/3/5/7), test 23 (textures
C100/C400/C200/C400), test 24 (x `0x41387420`, y `0x42CCC8DD`, 3 `rand()`), test 25 (1048 ms). Record: byte round
trip, v9 read as 5 and written as 5, v3 consumes 2 bytes, short payload false. Round 3: v4 record (56-byte legacy transform: mask 5 at scale 1.0, 7 at 2.0 / NaN, revision = low byte 0x78 of 0x12345678, written back as v5 mask 5 + 48 bytes, rest identical; truncations at +1/+52/+55/end false), xform NO15 rules (13-byte and 53-byte worked examples, mask 7 scale 1.0 reads 5, reset+counter on live mask, omitted part identity after reset / untouched at live mask 0, legacy keeps mask bits).
Gate (`qbuild all`, round 3, tip after 37c1cb69): cargo 3772 passed / 0 failed, clippy 0, fc4verify build 54/54, ctest 63/63. game=false.

**Questions** (docs/specs/questions/occupants-props.md): OP13 (life-cycle values in the saved scalars), OP14 (saved
refs vs live, record #8 / #14g source), OP15 (4 vs 5 subscriptions, tick data2 vs data3, base Init), OP16 (spark box
and axes, crane swing limits / `k` / initial target / 6000).

**Choices marked in code**: record defaults 0/null; target 0 before the first swing; 1→2 keeps the duration; the
"in renderer" bit (byte +0x37 bit 0x02) kept as an object field; tuning texture scales `None` when absent.

**Round 3 note**: `Xform::read` (network record) is unchanged (fresh-object form without the scale-bit test); the network lane may switch to `read_into`. The construction record keeps the unsaved revision counter as `transform_revision`. OP13-OP16 still open; no schedule / host-prop / crane behaviour added.
