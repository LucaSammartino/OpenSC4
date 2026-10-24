| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-regionalcity-shadow13 (regional city shadow) | Codex / `glue-regionalcity-shadow13` | in progress | Inspect contracts and implement default-off shadow; gate and measure to 2032. |

## Handoff
Goal: compare native loaded-region regional-city records against the core model at load and monthly.
Done: implementation committed as 93cfd16e3; reviewed regional-city slots, record layout/Write, workforce address return and reference ownership.
Implementation: default-off regionalcity_shadow, load/month hooks; all distinct region records plus application's current record; getters and byte-exact core decode/write comparison of all saved fields.
Next: collect focused Rust check/format and final all (already running), then unique game request through 2032.
Facts: no game requests outstanding. Focused test operation still running (exec session 95871, build-runs/regionalcity-focused.log). First native verify qbuild-20261024-194017-build-ctest failed: unsupported cRZString.h; corrected to cRZBaseString. Failure evidence build-runs/regionalcity-native-include-failure.txt. Final all running (exec session 66943). Do not duplicate either check. Public C ABI unchanged; private static-link Rust symbol only.
Run records: highest observed run number 631; recheck before assigning. Game wait helper build-runs/wait-regionalcity.py waits once without qwait. Regress tools files are forbidden to this engine lane; coordinator must append regionalcity_shadow to the current shadows profile after merge and exact measurement.
