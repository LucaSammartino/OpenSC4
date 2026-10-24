| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lua-shadow5: Lua host record/replay shadow | Codex; glue-lua-shadow5 | in progress | Inspect documented interception contracts; implement and measure supported shadow work. |

## Handoff
- Goal: default-off Lua host shadow with real-city validation through 2032.
- Done: reserved detached worktree reused; inspected Lua host sections 1, 3, 5, 6.1, 6.2, 8, 13-16 and advisors 8.2 (collector layout). Filed LH6-LH7 for full interception and nondeterminism contracts.
- Done: 6c5fad877 records LH6-LH7. Implemented default-off lua_shadow with distinct lua-registry label, collector capture and core register_api replay; no public DLL exports added.
- Focused checks: cargo test -p fc4-capi lua_registry_shadow: 4 passed (1m34s); owned-file qbuild fmt passed; git diff --check passed.
- Done: implementation commit 33cacb5aa. Stable-source qbuild all Rust workspace 6807 passed, 0 failed, 0 ignored; clippy exit 0, diagnostics 0.
- Next: collect SAME qbuild all operation (exec session 94445) to terminal native verdict, then real-city turbo to 2032. Native queue label qbuild-20261024-013309-build-ctest is already queued by all; do not start duplicate verify.
- No game request outstanding. Native gate outstanding as above. Bash q.py focused-test attempt failed CreateFileMapping (build-runs/q/20261024-012649-7620.log); direct PowerShell cargo test succeeded.
- Gate logs: build-runs/qbuild-test.log and qbuild-clippy.log. Game wait helper build-runs/wait-verdict.py uses blocking Windows directory notifications (no qwait/polling).
- Highest existing run number 348 at inspection; recheck before reserving run record.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-lua-shadow5-r1-manifest.json.
- Plan: lua_shadow default off; distinct lua-registry label; all 47 names/order/flags, native callback bindings and scratch alias checked. No excluded nondeterministic fields.
