| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-propmgr-race13: remove unsafe query probe | `Codex` / `glue-propmgr-race13` (engine lane) | in progress; main merged; prior probe failed train207 with native AV | Remove query vtable patches; gate and two house runs |

## Handoff
- Goal: remove LotQueryProbe shared-slot patches and retain exact default-off comparisons.
- Main merged on detached HEAD as requested; do not switch refs.
- Prior tip 39b9ee63d reported exact runs but shared slot patches crashed train207; those results do not validate safety.
- Probe patches OM slot40, occupant slot8, city slot96 in FC4PropLotShadow.cpp.
- Existing PL24 records native-only props exactly on upper boundaries; retain core half-open contract.
- Done: 424731910 removed all three shared-slot patches and all capture/replay state; comparison remains strict.
- Diagnostic verify qbuild-20261024-203951-build-ctest PASS: build 168/168, CTest 81/81. This was to build unpatched diagnostic DLL, not final all gate.
- Done: runs 735/736 both clean through 2032, propmgr 220/220, zero resyncs/reference warnings; DLL 39ab9efd. PL24 updated to retract unsafe capture assumptions.
- First all gate collected to terminal FAIL: capi 595/1, clippy clean, native 214/214 and CTest 81/81 (qbuild-20261024-221430-build-ctest). Failed evidence preserved as build-runs/qbuild-r3-parallel-test.log and propmgr-r3-parallel-gate-summary.txt.
- Next: collect corrected all gate with RUST_TEST_THREADS=1; finish final report and ready queue entry. No source changes for retry.
- Current all Rust stage FAILED: capi 595 passed/1 failed, slot44_holds_the_lock_across_threads (lock_waits 0/1). No worker-start handshake before 300 ms timeout (occupant_manager_tests.rs:686-714). Other stages still running; collect this operation to terminal before repeating.
- Corrected input for the permitted retry: RUST_TEST_THREADS=1 (also used by prior lane gate). Preserve current qbuild-test.log and other failed-gate logs before retry; do not edit unrelated owner/test code.
- No game request outstanding. No new owners or questions this round. Earlier mismatch cause remains unproven; do not claim a general boundary fix.
- Corrected all gate terminal PASS: cargo 7464/0 (51 result lines), clippy 0 diagnostics, native 164/164 and CTest 81/81; qbuild-20261024-224215-build-ctest. No engine changes since 424731910.
- Final gate DLL SHA256 b65c4f6dcae1e35a62aab54847bafc7f601043149396ff720a05ce124c3701f8 differs from earlier game artifact 39ab9efd; verifying final artifact before ready. Outstanding unique game label propmgr-race13-r3-final-a (turbo through 2032); next normal-speed run, then final report/queue. No further gate required absent source/input changes.

## Previous round (superseded safety evidence)

The remaining independently sampled prune input was the world bounds: after capturing the native slot-40 list and slot-8 positions (`88730ab68`), Rust still made later city slot-96 calls. Source `c5d532838` now captures both native slot-96 conversions and replays those exact dependency returns. It validates corner count, order and cell arguments, alongside the existing list rectangle, filter, position and result checks. TLS limits observation to our native lot query; other calls forward, and the scoped probe restores the city slot. Mismatch diagnostics print actual native bounds. No returned prop list is imported, no comparison is skipped or tolerated, and the core retains A7.2's literal half-open test.

Both house runs with complete dependency replay were exact. These counts include every propmgr observation, rather than only query lines:

```text
propmgr-race13-house2032-c-bounds 220/220
propmgr-race13-house2032-d-bounds-normal 220/220
```

Run records: [733, turbo](../runs/propmgr-race13-house2032-c-bounds-733.md), [734, normal](../runs/propmgr-race13-house2032-d-bounds-normal-734.md). Both ordinary Konradshohe runs used `propmgr_shadow=1`, cells=32, and DLL SHA prefix `6da5598c` from source `c5d532838`. Turbo reached 2032-01-04 in 28.203 s; normal reached 2032-01-01 in 108.891 s. Both verdicts are clean, with ref_warnings=0, zero input-error lines and zero propmgr resync lines. Logs/verdicts are in `build-runs/game-results/`. No game request remains outstanding.

Gate on stable final engine source: `qbuild.py --wt <WT> all`, `RUST_TEST_THREADS=1`, cargo **7389 passed / 0 failed** summed across all result lines, clippy **0 diagnostics**. Initial native build passed **161/161 edges**, CTest **80/81**: `plugin_autotest_test` failed its internal `ms < 15000` assertion with the end-date quit child at **15656 ms**. The same built executable and DLL passed focused `--autotest`, with that child at **172 ms**. After this timing diagnostic, queued `qbuild.py --wt <WT> verify` passed **46/46 build edges and CTest 81/81**, 89.17 s CTest time. Both queue operations were collected to terminal verdicts. The rerun noted tree changes; all intervening commits changed documentation only (`git diff c5d532838..HEAD -- engine` empty). No Rust files changed; no Rust formatting operation needed. C ABI exports unchanged.

Failed evidence is preserved in `build-runs/fc4verify-r2-first-{build,ctest}.log` and the unique failed verdict `build-results/qbuild-20261024-184634-build-ctest.verdict`; focused evidence is `propmgr-r2-autotest-focused.log`. Final native verdict is `build-results/qbuild-20261024-190738-build-ctest.verdict`; final logs use `fc4verify-{build,ctest}.log`. Earlier round failures and runs 731/732 remain preserved separately.

[PL24](../specs/questions/prop-manager.md) was updated, not duplicated. The prior 219/220 runs did not capture native world bounds, so their calculated upper bounds do not prove a general inclusive native comparison rule. Two exact runs validate complete input replay; the general boundary/NaN contract remains open for specification review. No guessed ordering rule was implemented. Shadows remain default-off except when enabled through the ini key. Coverage tables and regress scripts were not edited; coordinator review/credit and any profile integration remain with the coordinator, as does question-index regeneration.
