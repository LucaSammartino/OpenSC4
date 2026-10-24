| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-regionalcity-shadow13 (regional city shadow) | Codex / `glue-regionalcity-shadow13` | in progress | Inspect contracts and implement default-off shadow; gate and measure to 2032. |

## Handoff
Goal: compare native loaded-region regional-city records against the core model at load and monthly.
Done: implementation committed as 93cfd16e3; reviewed regional-city slots, record layout/Write, workforce address return and reference ownership.
Implementation: default-off regionalcity_shadow, load/month hooks; all distinct region records plus application's current record; getters and byte-exact core decode/write comparison of all saved fields.
Checks: focused Rust comparison tests PASS 2/2 (build-runs/regionalcity-focused.log); owned-file fmt PASS. Final all cargo summaries: 7408 passed, 0 failed, 0 ignored across 51 summaries (build-runs/qbuild-test.log). SDK include fix committed f95ee4ee6. Production source stable since f95ee4ee6; owned test formatting f4edbbef1 was in place before the final C API test compilation.
Clippy: PASS, zero diagnostics with warnings denied. Source implementation is closed; only required native/game verification and reporting remain (context at ~101k/150k).
Next: repeat all on the corrected scoped-enum probes, then unique game request through 2032. The previous all operation has terminated.
Facts: no game requests outstanding. Native verify qbuild-20261024-194017-build-ctest failed on unsupported cRZString.h, fixed f95ee4ee6. Native part of all qbuild-20261024-213418-build-ctest then failed on two scoped enum-to-double assignments; fixed with explicit uint32_t casts. Both failures preserved in build-runs/regionalcity-native-*-failure.txt. Public C ABI unchanged; private static-link Rust symbol only.
Run records: highest observed run number 631; recheck before assigning. Game wait helper build-runs/wait-regionalcity.py waits once without qwait. Regress tools files are forbidden to this engine lane; coordinator must append regionalcity_shadow to the current shadows profile after merge and exact measurement.
