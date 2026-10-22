| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-audio-inputfix4: conform the pure-Rust ambience manager to audio-ambience AA5-AA9 answers and §3.4, tests 19-24 | `core-audio-inputfix4` / (engine lane) | ready: code d496a154 + report; gate cargo 4105/0 (baseline 4099), clippy 0, ctest 65/65; no ABI/CAPI/plugin/CMake/tools change; no game run (core only, no glue); questions AA10-AA11 filed | 1. Coordinator: run `tools/questions.py index` for AA10-AA11 (lane may not run tools/). 2. A glue lane wires AmbienceHost (incl. the new lot_condition/lot_occupants/occupant_info providers) and a shadow. 3. Spec answers AA4, AA10, AA11. |

## Report

What changed (engine/core/src/audio/ambience.rs, ambience_tests.rs)
- AA5: `_AddAlarmOccupant` refuses occupants without flags 1+4 (`OccupantInfo::flags_1_and_4`) or with flag 0x10,
  sets `last` = current hour (unsigned minute/60), `on` = 0, appends at the tail.
- AA6: margins and layer ids are vectors; absent property leaves them unchanged (margins absent → first five
  70.0 [spec choice]), present gives exactly n values (margins padded to 5 with 70.0 [spec choice]). `zoom_index`
  is `Option` (no view global → previous rectangle kept).
- AA7: the block walk uses the shared prime-step `grid::CellIterator` (SetClipRect with inclusive halved bounds,
  x1/z1 raised, `Reset(0)` without rand; `current` carries over walks and cities). Worked example verified:
  20 blocks, step 7, first (2,1), last (0,0), second walk identical.
- AA8: move set-positions use the bare occupant address; code 9 uses `o + id` at the automaton position;
  `_PlayOccupantSound` starts with flag 1 on `A + 4` (`AutomatonEvent::base_occupant`), no preview/flag test;
  railway crossing instance = automaton address (`CrossingAutomaton`), flag 1, returns 1 also without sound
  system; on/off alarms go through `_AddOccupantSndInstance`/`_RemoveOccupantSndInstance` with the re-read value.
- `_AddOccupantSndInstance`/`_Remove...` in full: flag 1+4 and preview gate, automaton link/unlink, posts only
  when `cIGZSndSys` is held (`snd_sys`).
- AA9: focus → stage 1 / 4 (`Stage::Unfocused`, no OnTick case), `Pause(!f)`, result 1; master volume returns 0;
  `0x2A2C0384` without payload → 1, no load; city shutdown and `Shutdown` call each layer's `shutdown` (new
  `AmbienceLayer` slot 5) then drop it, alarms and tracked set kept.
- §3.4 `_UpdateLotOccupantsForConditionChange` implemented (was AA2-stubbed): typed providers `lot_condition`,
  `lot_occupants` (`LotOccupant`); rows 0/5/6, 2/3, 1/4, 1→0 R start; insert/init use U(lot, -1, -1).
- Host inputs are typed world/sound providers only (`occupant_info` replaces `occupant_position` and
  `on_off_sound_property`; `condition_update_unspecified` removed); no manager output is fed back as input.

Owner inventory: every manager method of §2-§7 now has a body (list in the module doc). Remainder: AA4 (third
init-walk type id, passed through only), AA10 (absent clock at alarm creation: [entry, last 0.0]), AA11 (move's
second set-position sound-system test: [tested]); the original's unchecked-global crashes are replaced by the
spec's [choice]s. The layer class (§8) is a separate owner. No glue or shadow exists, so no in-game evidence.

Tests: ambience 34 (was 28): §10.19 `alarm_created_at_current_hour`, §10.20 in `insert_move_remove_sounds`,
§10.21 `railway_crossing`, §10.22 `prime_step_block_order` + `iterator_current_carries_over`, §10.23 in
`focus_pauses_layers_and_exemplar_rereads_tuning`, §10.24 in `events_master_volume_and_fresh_player` and
`city_shutdown_stops_ticking_and_clears`; plus `alarm_creation_flags_and_order`, `snd_instance_needs_flags`,
`lot_condition_update_rows` (§10.7/§10.8).

Gate: `qbuild all` PASS: cargo 4105 passed / 0 failed (baseline 4099), clippy 0, fc4verify build 129/129,
ctest 65/65. Baseline ctest run was invalid (tree changed during the run). Questions: AA10, AA11 appended to
docs/specs/questions/audio-ambience.md; QUESTIONS.md index not regenerated (tools/ off limits for this lane).
