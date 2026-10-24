| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lotconf-sim-shadow6: lot configuration and simulator shadows | Codex, `glue-lotconf-sim-shadow6` (reserved detached worktree) | ready; final full gate PASS; native 2032 run clean, lotconf 73/73 and simulator 73/73 | Coordinator review/credit measured fields, append shadow keys to current regress profile; answer GLUE22 occupancy/cost contracts |

## Changes

Added default-off `lotconf_shadow` and `simulator_shadow` ini keys, registered in KnownKey and wired through the existing observer's city-load/month/save callbacks. They refuse replaced owners; neither modifies original simulation state. All borrowed configuration/simulator pointers remain borrowed. No ABI version bump; one additive C export, `fc4_lot_configuration_project` (ABI exports changed: yes).

Rust `lot_config_manager/configuration.rs` independently derives size, zone/wealth/purpose masks, growth stage, water/construction flags, slopes and the complete object array from effective exemplar properties. It implements raw byte-vector semantics, detail filtering, position correction and the appended foundation. Expected results are cached from resource inputs at city load/detail change; each load/month compares every native loaded configuration's getters and object/ID lists. Capacity/cost comparisons remain unmeasured pending GLUE22. The native configuration interface has no specified cost getter; copying native occupancy outputs into expected values would not measure derivation.

The simulator shadow captures the ORIGINAL simulator's Write through its public serializable interface, loads an isolated Rust `simulator.rs` instance and compares saved bookkeeping (all 11 counter/fired pairs, pause, speed/time, budget, date) and native date/speed/time/pause getters. This is record/getter agreement: `record_bootstrap=1`, `independent_evolution=0`. It does not establish independent scheduling, pacing or timer evolution. The spec says Read ignores inTick, so only that byte is excluded from record equivalence. `disable = simulator` is required for native comparison; turbo would install our owner, so both measured runs used normal speed.

## Validation

- Focused Rust derivation tests initially 2/2; added a C ABI layout/null/failure test. Final workspace total: **6879 passed, 0 failed, 0 ignored**; **clippy -D warnings PASS, 0 diagnostics**; owned Rust formatting 4/4 and git diff --check clean.
- Final `qbuild all` PASS on stable final implementation, session 8214 terminal exit 0. Queued native request `qbuild-20261024-025907-build-ctest`: build **47/47 edges**, 2.9s; **CTest 72/72**, 90.1s; verify PASS, 94.9s. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`; unique queue evidence in `build-runs/build-results/`.
- Initial full gate failed solely on two unnecessary u8 casts despite 6879 tests and 72 CTests passing; corrected in `3f34d9bcc`. Failed evidence: `build-runs/gate-initial-failed.md` and native queue `qbuild-20261024-023705-build-ctest`. Corrected full gate also passed before first game run (`qbuild-20261024-024520-build-ctest`).
- Implementation milestone `ada4367a1`; first measured game and ini-key registration `85abf300f`; final gate checkpoint `3fdb7d8e8`.

## Real-city measurements

Final label **lotconf-sim-shadow6-native-2032-20261024b**, [run 375](../runs/lotconf-sim-shadow6-native-2032-20261024b-387.md): Konradshohe, normal speed, end date **2032-01-01**, DLL SHA256 prefix **b5bb8553**. Verdict **clean**, 126.328s; watchdog stalls, foreign exceptions, reference warnings, RTC, overruns, panics, structured exceptions, incomplete shadow lines and other warnings **all 0**. Independent log counts:

```
lotconf 73/73
simulator 73/73
```

Every dated shadow line has `mismatches=0`. Each lotconf line: **1909/1909 configurations**, **97752 pairs**, **25210 objects**, detail **1**, refused **0**. Every simulator line: **89 pairs**, **79-byte record**, **11 counter/fired pairs**. Both final dated lines are **2032-01-01**. Raw appended-foundation tags differ for **1908 objects per line**: these are uninitialised native values explicitly excluded by the existing numbered **PL23** contract, separately logged as `raw_foundation_tags=1908`. Defined fields are exact; whole-array raw identity is not claimed.

First label **lotconf-sim-shadow6-native-2032-20261024a**, [run 374](../runs/lotconf-sim-shadow6-native-2032-20261024a-386.md): also clean through 2032, ref_warnings=0, lotconf **73/73**, simulator **73/73**. It diagnosed six unknown-key warnings, fixed by registering both keys. One watchdog sample followed the clean verdict during shutdown. Final run has neither issue. Full logs/verdicts remain at `build-runs/game-results/<label>.{log,verdict}`. No game/native requests are outstanding.

## Open work for the coordinator

- **GLUE22**, `docs/specs/questions/lot-configuration.md`: provide native building-capacity input lookup contract and clarify any configuration cost getter. Occupancy/cost remain unmeasured; this does not block the measured default-off shadows. Regenerate the question index centrally.
- Review/credit the measured scopes and PL23 raw-tag distinction. Append `lotconf_shadow` and `simulator_shadow` to the current single regress shadows profile after integration, with native simulator disabled for its comparison. This lane did not read/edit tools/, merge main or move refs, per clean-room/detached-worktree restrictions.
