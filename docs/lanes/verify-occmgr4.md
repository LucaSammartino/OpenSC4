| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-occmgr4 (verify the occupant-manager shadow, native original + occmgr_shadow=1) | `verify-occmgr4` / (engine lane) | ready: occmgr shadow EXACT in both runs, resyncs=0; one plugin fix (occmgr_shadow keys registered in FC4Config, 61e000c7). Gate: cargo 3930/0, clippy 0, ctest 63/63. Game: vom4-orig-shadow-2032a (turbo) 72/72 months exact, vom4-orig-shadow-normal-b (300 s) 188/188 exact, ref_warnings=0 | add occmgr_shadow = 1 to the fullhouse/regress shadows run (coordinator tools); no OM questions |

## Report

**Goal.** Verify the existing FC4OccMgrShadow (original cISC4OccupantManager vs fc4_occmgr core, same process),
with the occmgr replacement OFF (`disable = occmgr`; it is also off by default), `occmgr_shadow = 1`,
`occmgr_shadow_probe = 1`, default samples (4). Sampling, probe and comparison code untouched.

**Change.** `engine/plugin/FC4Config.cpp` (61e000c7): `occmgr_shadow`, `occmgr_shadow_samples`,
`occmgr_shadow_probe` added at the end of `kOtherKeys`. The shadow read them, but every config load logged
`config: WARNING unknown key 'occmgr_shadow' ... (ignored)` (the shadow still ran: `occmgr shadow: ON`). After the
fix the normal run has 0 such lines. No behaviour, core, ABI or test change.

**Game results** (Konradshohe; both verdict clean, ref_warnings=0, foreign_exceptions=0):

| | vom4-orig-shadow-2032a (turbo, to 2032-01-01, DLL a041aa9e = main) | vom4-orig-shadow-normal-b (300 s, untimed pacing 19 days/s, to 2041-09, DLL 54fce078) |
|---|---|---|
| month lines / mismatches=0 | 72 / 72 | 188 / 188 |
| followed= | yes 72 (no first/resynced) | yes 188 |
| resyncs (shutdown line) | 0 | 0 |
| record cell-by-cell | exact 72/72; 23 months set aside some, 311 occupants total | exact 188/188; 30 months, 325 total |
| queries on followed core | 4312/4312 (49 months) | 13904/13904 (158 months) |
| queries on fresh core (months with set-aside) | 2024/2024 (23 months) | 2640/2640 (30 months) |
| per slot (25, 27, 28-47) | 288/288 each | all exact |
| messages ins/rem/move | 13842/13993/0, silent moves 0 | 28383/28408/2, silent moves 1 |
| boxes after Insert/Move | 13842/13842 | 28385/28385 |
| msgorder | 27835/27835 exact | 56793/56793 exact |
| refused / off-thread / stray releases | 0 / 0 / 0 | 0 / 0 / 0 |
| probe other-thread calls / overlaps / slot12 | 0 / 0 / 0 | 0 / 0 / 0 |

Exact credit caveats (honest scope): no resync, no forced reset, no dropped or adopted comparisons. The months that
set aside occupants (pending Insert/Remove messages, by flag 1) ran their query samples on a fresh core read from that
month's record, so those 2024 + 2640 query results do not credit the followed core; the followed record itself was
compared cell by cell (minus the set-aside occupants) and was exact in every month, and the next month compares the
set-aside occupants again. Move coverage is thin: 0 Moves in the turbo run, 2 Moves + 1 silent move at normal pacing.
The thread probe saw no second thread (model loader slot 40 never called off the main thread; the game window is
minimized), so OM1 contention is not exercised by these runs.

**Questions.** None filed (no difference found).

**Next.** 1. Coordinator: put `occmgr_shadow = 1` into the fullhouse/regress shadows run (tools, not this lane).
2. A scripted run with relocations (Move-heavy, e.g. plopping/moving props) and a visible window to exercise the
second-thread probe.
