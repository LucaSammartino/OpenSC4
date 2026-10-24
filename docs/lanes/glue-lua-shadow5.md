| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lua-shadow5: Lua host record/replay shadow | Codex; glue-lua-shadow5 | in progress | Inspect documented interception contracts; implement and measure supported shadow work. |

## Handoff
- Goal: default-off Lua host shadow with real-city validation through 2032.
- Done: reserved detached worktree reused; inspected Lua host sections 1, 3, 5, 6.1, 6.2, 8, 13-16 and advisors 8.2 (collector layout). Filed LH6-LH7 for full interception and nondeterminism contracts.
- Done: 6c5fad877 records LH6-LH7. Implemented default-off lua_shadow with distinct lua-registry label, collector capture and core register_api replay; no public DLL exports added.
- Focused checks: cargo test -p fc4-capi lua_registry_shadow: 4 passed (1m34s); owned-file qbuild fmt passed; git diff --check passed.
- Next: run stable-source qbuild all; native verification must use queued qbuild verify; then real-city turbo to 2032.
- No game request outstanding. Full gate not yet run. Bash q.py focused-test attempt failed CreateFileMapping (build-runs/q/20261024-012649-7620.log); direct PowerShell cargo test succeeded.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-lua-shadow5-r1-manifest.json.
- Plan: lua_shadow default off; distinct lua-registry label; all 47 names/order/flags, native callback bindings and scratch alias checked. No excluded nondeterministic fields.
