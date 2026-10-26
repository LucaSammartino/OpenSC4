| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| automata-actions30: construction publication and terrain action boundaries (engine) | Codex / automata-actions30 / detached HEAD; reserved branch automata-actions30; code 49bab4287, f7b408f9b, c893a0eb3, 8c6c55931; ABI false; seam 0x4E540001 | ready; Rust 8075/clippy 0/CTest 135/135; s1/s2 clean 216/216; shadow exact 73/73; AP20 not reproduced | 1. Supervisor publishes actual HEAD to reserved branch. 2. Land ready engine entry. |

Merge criterion PASS: automata-actions29 already contained; main merged as 5634aba3b, preserving effects targets and terrain StaticInit in matching compatibility tables. Detached HEAD retained.
Construction fix (49bab4287): require lot slot 48's building before creating/initializing the native scene; complete SetLot before InsertOccupant. Missing-building recovery is marked SPEC-GAP; the native dust reference remains owned until Shutdown.
Carried fixes: texture prerequisites (f7b408f9b), intact packed terrain-message payloads (c893a0eb3), borrowed network AsOccupant receivers and synchronous sender lifetime (8c6c55931).
Focused tests PASS 3/3: plugin_ini_terrain_shadow_test, construction_identity_test (including rejected creation and dust lifetime), automata_construction_test.
Final gate PASS, run once this round: `build-runs/qbuild-test.log` 8075 passed/0 failed/0 ignored; `build-runs/qbuild-clippy.log` exit 0/0 diagnostics.
Native gate PASS: `build-runs/fc4verify-build.log` 31/31 edges; `build-runs/fc4verify-ctest.log` 135/135, 0 failed. No owned Rust files changed.
All final logs/verdicts below are in main's `build-runs/game-results/`; shared requests are in main's `build-runs/game-queue/done/`; house replace list unchanged.
Same final DLL for all f runs: SHA256 `4ad705342b7b996759305479c49f7700b013f27f44cc4b33cb16825232935232`; no subsequent build or code edit.
s2 PASS: `aa30-final-house-s2-f.log:15484` reports 216/216 actions; line 67842 and matching `.verdict` clean at 2028-01-28; foreign exceptions/watchdog/RTC/overruns/ref warnings all 0.
s2 construction evidence: same log line 8758 skips an unavailable-building scene before publication; line 12875 publishes a configured scene; native shutdown completes cleanly.
s1 PASS: `aa30-final-house-s1-f.log:15556` reports 216/216 actions; line 66260 and matching `.verdict` clean at 2028-01-28; foreign exceptions/watchdog/RTC/overruns/ref warnings all 0.
Shadow PASS: `aa30-final-shadow-f.log:11416` reports 73/73 exact, 438 components, 729139 bytes, 0 mismatches; line 11642 / `.verdict` clean at 2032-01-04, ref warnings 0.
AP20 remains the existing runtime question in `docs/specs/questions/application.md`; the old s1 native wait did not reproduce in final acceptance. Checked re-ap30's AP19 delivery/save contract; no application-loop or wait/wakeup fix is claimed.
NM20-A remains an uncollected runtime observation (original control, placement result, first pointer/cache write); passing actions do not supply that capture. No new questions filed.
Earlier-source e results remain distinct: s2 reached 216/216 then corrupt dust shutdown; s1 stalled 27/216. They are superseded for acceptance by the final f artifact above.
Renderer limits remain: mirrored normals are core face normals; texture-layer population is incomplete. No terrain/draw bypass or C ABI export change.
Ready for kind-engine landing; ignored merge-queue entry records the full final HEAD. Coordinator retains coverage-credit ownership.
