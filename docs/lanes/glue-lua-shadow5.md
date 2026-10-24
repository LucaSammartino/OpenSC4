| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lua-shadow5: Lua host record/replay shadow | Codex; glue-lua-shadow5 | in progress | Inspect documented interception contracts; implement and measure supported shadow work. |

## Handoff
- Goal: default-off Lua host shadow with real-city validation through 2032.
- Done: reserved detached worktree reused; inspected Lua host sections 1, 3, 5, 6.1, 6.2, 8, 13-16 and advisors 8.2 (collector layout). Filed LH6-LH7 for full interception and nondeterminism contracts.
- Done: 6c5fad877 records LH6-LH7. Implemented default-off lua_shadow with distinct lua-registry label, collector capture and core register_api replay; no public DLL exports added.
- Focused checks: cargo test -p fc4-capi lua_registry_shadow: 4 passed (1m34s); owned-file qbuild fmt passed; git diff --check passed.
- Done: implementation commit 33cacb5aa. Stable-source qbuild all Rust workspace 6807 passed, 0 failed, 0 ignored; clippy exit 0, diagnostics 0.
- Done: initial qbuild all terminal FAIL: native build PASS 224/224 edges; CTest 69/70 (ret_channel_check flagged missing citation for IsCFunction). Added lua-host §5 slot 16 AL bool citation at call.
- Next: focused ret_channel_check, then rerun all after this diagnostic source correction; then real-city turbo to 2032. Initial operation 94445 is complete; queue qbuild-20261024-013309-build-ctest verdict retained.
- No game request outstanding. Bash q.py focused-test attempt failed CreateFileMapping (build-runs/q/20261024-012649-7620.log); direct PowerShell cargo test succeeded.
- Initial gate logs preserved as build-runs/initial-{fc4verify-build,fc4verify-ctest,qbuild-test,qbuild-clippy}.log. Game wait helper build-runs/wait-verdict.py uses blocking Windows directory notifications (no qwait/polling).
- Highest existing run number 348 at inspection; recheck before reserving run record.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-lua-shadow5-r1-manifest.json.
- Plan: lua_shadow default off; distinct lua-registry label; all 47 names/order/flags, native callback bindings and scratch alias checked. No excluded nondeterministic fields.
