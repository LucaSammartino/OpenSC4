# Lane apply15c

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| apply15c | `.claude/worktrees/apply15c` (branch `apply15c`) | **ready** — the four re-answers25 engine follow-ups are done and verified: PL12 (lot side `1 + floor(w/16)`), PL13 (`FixLotConfiguration` footprint around the record offset, code 4 moves nothing, centre moved to the footprint centre), TR20 (`IsNavigable` pole test blocks only its own cell), TU5 (tutorial `Shutdown` teardown). Gate green: cargo test 2599 passed / 0 failed (baseline 2595), clippy `-D warnings` clean, rustfmt clean on every touched file, plugin builds, ctest 57/57 including `ret_channel_check` (0 flagged, 7 reviewed). Game run `apply15c-terrain-1` on Konradshohe: **clean**, `ref_warnings=0`, terrain shadow 92/92 exact, `navigable 36742/36742`. Merge-queue entry written; no merge lock taken. | Merge `apply15c` into main with `--no-ff` (`build-runs/merge-queue/apply15c.json`). Two follow-ups for the coordinator: (1) confirm the §4.2 / TU5 window-reference reading in question 1 below; (2) `build-runs/pending-abi/re-answers25-rows.tsv` is untouched by this lane — `docs/abi/` was not edited and `FC4_ABI_VERSION` was not bumped. |

## What changed

| Id | File(s) | Change |
|---|---|---|
| PL12 | `engine/core/src/lot_config_manager/manager.rs`, `mod.rs` | `CreateGenericLotConfiguration` sizes the lot with `1 − trunc(w × −0.0625)` = `1 + floor(w / 16)`, not `ceil(w / 16)` |
| PL13 | `engine/core/src/lot_config_manager/manager.rs`, `mod.rs` | `FixLotConfiguration`: rebuild gated on the size and centred on the record offset; out-of-lot only sets code 4 and moves nothing; offset outside its footprint moves to the footprint centre. Lot-bounds constants pinned to the stored bit patterns |
| TR20 | `engine/plugin/FC4TerrainShadow.cpp`, `engine/core/src/terrain/water.rs`, `tests.rs`, `engine/conformance/vectors/tr00_navigability_and_bodies.json` | `HostPowerPole` queries `cISC4PowerPoleOccupant` and compares `GetCell` with the visited cell instead of treating any occupant-manager list entry as a blocker |
| TU5 | `engine/core/src/tutorial/{system,mod,tests}.rs`, `engine/capi/src/tutorial_tests.rs`, `engine/conformance/src/tutorial.rs`, `vectors/tu*.json` | `Shutdown`: cheat (no slot-21 test) → window close and double release → unsubscribe `0xAA3840E6`, `0xEA8AE29A`, `0x26C63343` |

## Vectors added or updated

- `lot_config_manager::tests::t14_generic_lot_side` (PL12, §T 14)
- `lot_config_manager::tests::t15_fix_lot_configuration` (PL13, §T 15) and the rewritten `fix_codes` cases 3–5
- `terrain::tests::is_navigable_rules` and `conformance/vectors/tr00_navigability_and_bodies.json` (TR20, §T 25)
- `tutorial::tests::t7_shutdown` (TU5, §T 7) and `conformance/vectors/tu8_07_shutdown.json`
- `conformance/vectors/tu00_messages.json` (unsubscribe order), `tu8_05_end_without_registry_then_save.json` (`window_refs` 2)

## Numbered questions

See the lane report. Summary: (1) §4.2's `endTutorial` summary versus TU5's "same teardown"; (2) the
task brief's "an offset outside the lot" versus PL13's "outside its footprint"; (3) `GetCell`'s return
channel; (4) `1 − trunc` for a negative footprint; (5) whether `create_window` carries a reference.
