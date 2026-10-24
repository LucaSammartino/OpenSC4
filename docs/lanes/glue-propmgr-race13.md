| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-propmgr-race13: exact native prune-input capture | `Codex` / `glue-propmgr-race13` (engine lane) | ready: source c5d532838; cargo 7389/0, clippy clean, CTest 81/81; runs 733/734 clean, propmgr 220/220 each, zero resyncs; ABI unchanged; PL24 updated | (1) Coordinator review and credit exact runs; (2) resolve general boundary/NaN contract PL24 |

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
