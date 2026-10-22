| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-construction-contractfix4 (apply OP13-OP16 answers, lr §9.1/§9.2, ld §6.7.1/6.7.6/6.7.7) | `core-construction-contractfix4` / (engine lane) | ready + report: record re-keyed to object-frame roles, load resumes schedule, refresh after load, 4 subscriptions + base Init + AddOccupantGroup(0x5007), x/z spark extent, crane T/k/round_f32; core only, game=false; gate cargo 4105/0, clippy 0, ctest 65/65; question OP17 | 1. OP17 (who writes world box #6). 2. Wire into the lot developer's `start_construction_occupant` / plugin. 3. Crane fit, DivideLot, props, crew still host-delegated (ld §6.7.10). |

## Report

**What changed** (`engine/core/src/lot_records/construction.rs`, `tests_construction.rs`; commit c4cc91d7):
- `ConstructionScalars` now carries the lr §9.1 roles (state, T0, unused_148, nextDay, N, counter, span, startY,
  A/B/C/E, msAcc, crane state, swing min/max, unused_180, crane acc/duration, target/previous, crane type). The
  occupant keeps no unsaved copies: lot/building (#3/#4 live refs), world box #6, scaffold box #7, crane position,
  effect vector (#12, returned by `create_sparks`) and #14 are the live state, so a loaded occupant resumes.
- #8: written from the live crane flag (`+0x104`), read into `loaded_has_crane` (`+0xD8`); Read sets
  `refresh_pending` (`+0xD9`). New `refresh()` on `0x26D31EC1`: RebuildModel, CreateCrane(x, z, type) if loaded flag,
  AddToRenderer, DoAnimationTick(0) with state 2 shown as 1, clear pending.
- `init`: base Init (host) unconditionally, 4 subscriptions (tick not subscribed), tick 0x80,
  `add_occupant_group(0x5007)` on property 0xAA1DD396 (append + post 0xEBFF18A3 only if absent).
- `spark_count`: max of x / z extent. `animate_crane`/`crane_duration`: T = tunable crane_rotation_ms, k = f32
  0x3E22F983, round_f32 via +12582912.0 bits; previous := old target. SetLot crane init: target min, previous max,
  acc 1, duration 0, state 2, `pose_crane(min)`; `CraneFit` from `attempt_add_crane` stores type/position/limits.
- v4 56-byte legacy transform path untouched (tests still pass).
- Host trait additions: `occupant_base_init`, `post_occupant_group_added`, `pose_crane`, `create_crane`;
  `attempt_add_crane` returns `Option<CraneFit>`, `create_sparks` returns refs; `crane_swing` removed.

**Tests** (new): crane duration/T/k, SetLot crane initial values + RebuildModel saved values, Init group once,
save/load mid-schedule resumes the identical schedule, refresh recreates crane once (Write before refresh saves #8
false), live refs released (Rc counts) on SetLot/Shutdown, sparks y excluded.

**Gate**: qbuild all PASS: cargo test 4105/0, clippy 0, build, ctest 65/65. No plugin/CAPI/CMake/ABI change, no game run
(core-only owner, not wired into the plugin).

**Questions**: OP17 appended to `docs/specs/questions/occupants-props.md` (world box #6 writer); QUESTIONS.md needs
regenerating by the coordinator (tools/ not run). Unproved, left host-delegated: FUN_005CBE10 beyond posing the boom,
0xA6B79602 / 0x06CDB65B visual handlers, scaffold/completion interpolation.
