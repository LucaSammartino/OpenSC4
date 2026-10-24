| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-pedestrians12: pedestrian owners | `Codex` / `core-pedestrians12` (engine lane) | ready; final-round cargo 7383/0/0, clippy clean, all and verify PASS, CTest 81/81; PN1 consumer open, prior PN3 filed; ABI unchanged | (1) Coordinator review and land; (2) spec writers resolve PN1 and PN3 |

## Report

Implemented column: `engine/core/src/pedestrians`. The module implements `cSC4Pedestrian` and `cSC4PedestrianOneShot`, reusing `automata::AutomatonBase`, `PedestrianRecord`, shared stacks/serialization and `vehicles` traffic movement. Its README contains the per-owner, per-section Owner closure table.

Implemented construction and initialization, first parameter draw, animation transitions, state entry/dispatch, collision cadence and movement, crowd reaction/push consumption, queue/strike/riot/ignite behavior, one-shot instance parameters and play-count draw, and inherited record read/write. Ten module tests cover those behaviors plus empty paths, route installation, unsupported record versions and one-shot record inheritance. Shared occupant Read's documented status-2 transformation is included in round-trip expectations.

PN1 remains open only for the one-shot play-count consumer (pedestrians sections 3 and 7); `Unsupported::PlayCountConsumerPn1` is the module's sole explicit Unsupported behavior. The first-Init operation is implemented under the spec's PR clarification. PN2 concerns fire crew sections 4.2–4.4, outside these owners. PN3 was filed in `docs/specs/questions/pedestrians.md` for summary-level cadence, crowd selection, strike inset and riot randomness; those paths execute literal implementations and are documented in code and the README. Inherited vehicles VH limitations remain owned by the shared traffic module.

## Validation

- Final-round `qbuild all`: PASS; workspace totals 7,383 passed, 0 failed, 0 ignored, including all ten pedestrian tests; clippy exit 0, diagnostics 0. Native build PASS 46/46 edges (2.2 s); CTest PASS 81/81 (91.1 s); native total 95.4 s. Queue label `qbuild-20261024-191506-build-ctest`. Console: `build-runs/pedestrians-all-final-console.log`. Inputs: `RUST_TEST_THREADS=1`, `CARGO_BUILD_JOBS=1`, matching the previous successful diagnostic. Result committed at c89d527ff.
- Final-round explicit `qbuild verify`: PASS; native build 161/161 edges (9.9 s), CTest 81/81 (159.9 s), total 172.2 s. Queue label `qbuild-20261024-191705-build-ctest`. Console: `build-runs/pedestrians-verify-final-console.log`. Both operations were collected to their terminal results; neither was duplicated. Standard logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.
- Previous-round `qbuild all`: PASS; workspace totals 7,383 passed, 0 failed, 0 ignored, including all ten pedestrian tests; clippy exit 0, diagnostics 0. Native build PASS 161/161 edges; CTest PASS 81/81 (111.4 s). Queue label `qbuild-20261024-183724-build-ctest`. Console: `build-runs/pedestrians-all-r2-serial-console.log`.
- Previous-round explicit `qbuild verify`: PASS; native build 46/46 edges (1.8 s), CTest 81/81 (129.3 s), total 131.2 s. Queue label `qbuild-20261024-185820-build-ctest`. Console: `build-runs/pedestrians-verify-r2-console.log`.
- Initial round-2 all: FAIL, cargo 591 passed / 1 failed, clippy clean, native build PASS 262/262 and CTest 81/81. The scheduling-sensitive capi `slot44_holds_the_lock_across_threads` assertion at line 714 observed lock_waits 0 instead of 1 after a 300 ms worker window. Failure evidence remains in `build-runs/pedestrians-all-r2-console.log` and commits `7301e48f1` / `c7e388658`. A diagnostic rerun with `RUST_TEST_THREADS=1` and `CARGO_BUILD_JOBS=1` passed the complete workspace without source changes.

No game requests or shadow measurements: this is an unwired portable core module. Plugin and capi wiring were deliberately excluded; C ABI exports and ABI version are unchanged. Round 2 changed only this report and gate inputs; implementation and Owner closure README were completed in round 1.

Final round resumed from 03a5edffd and changed only the lane report and validation evidence. No new owners, questions, implementation changes, or ABI changes were introduced. Both final-round gates passed once; no gate operation remains outstanding. The ready merge-queue entry names the final report commit with its full 40-character tip.

Next: coordinator review/landing; spec writers resolve PN1's consumer (pedestrians sections 3 and 7) and prior PN3's summary choices (sections 2.4–2.6 and automata 6.5). PN3 paths are implemented; only PN1's consumer remains Unsupported for these owners.
