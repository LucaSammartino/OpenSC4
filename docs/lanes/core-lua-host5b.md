| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-lua-host5b: Lua host owner closure | Codex / core-lua-host5b | in progress | Implement settled LH4/LH5 contracts and verify full gate |

## Handoff
- Goal: close all settled cGZLua5, cGZLua5Thread and command-generator owner behavior; interpreter remains foreign.
- Done: implemented LH4 registry/thread lifecycle and LH5 setup, notifications, dispatch and cleanup; expanded owner table.
- Done: implementation milestone 021cc87e; latest focused 24/24 in build-runs/focused-20261023-211944.log; fmt 7 owned files PASS. Earlier diagnostic failures preserved separately.
- Gate in progress: workspace 6379 passed / 0 failed / 0 ignored; clippy 0 diagnostics. Native queued as qbuild-20261023-212359-build-ctest (game regress delays it).
- Outstanding operation: exec session 75902, qbuild all. Collect SAME operation to terminal, never duplicate. Logs build-runs/qbuild-test.log and qbuild-clippy.log.
- Next: collect native gate. Then fix destructor cleanup when failed main hook leaves context null but registry owner retained: destroy must shut down remaining registry after Close. Add focused regression; preserve first full-gate logs before final rerun.
- Thread views share synchronized state; explicit release/destroy methods handle foreign lifetime operations. No RefCell.
- Only pre-existing spec §15 Q1/Q2/Q5/Q8 remain unsettled; no new questions filed.
- No game requests; core-only task, no plugin/C ABI changes authorized.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-lua-host5b-r1-manifest.json.
