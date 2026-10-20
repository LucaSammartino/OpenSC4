# Lane glue24

Per-lane row for the regenerated `HANDOFF.md` 0 table (see `docs/merge-train.md`).

| Lane | Worktree | State at stop | Next steps |
|---|---|---|---|
| glue24 (network manager and network lot manager glue) | `.claude/worktrees/glue24` / `glue24` | **The replacements run to 2032 and through the save round trip (2 of 5 full runs clean, save round trip clean), but an intermittent debug-CRT int3 at about tick 5500 is unresolved (state wip).** (1) **The fix round found and fixed three defects in the host wiring** (commits `0d65ba28`, `e7a227b0`, `58f5f461` on top of `592a9c35`'s tools + 543 configurations): (a) the host `user` was the manager object while the shared read callbacks (`FC4NetMgrHost.cpp`) read it as a `fc4::NetMgrContext*`, so `City(u)` returned the manager's vtable pointer as the city and the game's city-detail pass crashed jumping into the ILT (rep-4's `eip=0x89ECD3E9` under `TrafficMap -> traffic_map_exists -> create_lot_objects -> redevelop_lot -> RedevelopLot`); the user is now `fc4::NetMgrOwner { NetMgrContext ctx; void* mgr; }` (context at offset 0, the manager reachable for the write callbacks; the shadow's bare-context use unchanged); (b) `MakeHost` first forgot to fill `owner->mgr` (rep-5 died on a null `this` in `Init`); (c) the configuration's object records were owned twice (a `std::vector` build storage plus an `SC4Vector` slot-5 array memmove-built from it, so each record's `data` was freed twice - rep-6 and rep-save crashed at teardown in `SC4Vector::deallocate`); one storage only, and `AddObject` builds each record in place from its fields (SC4Vector's push_back is a raw memmove, so a by-value AddObject shared the temp's data buffer). (2) **Run `glue24-rep-8` (turbo to 2032-01-01 on Konradshohe): clean**, `ref_warnings=0`, 62.61 days/s, reached 2032-01-12, `netmgr Init ... tools=13`, `netlot Init: lots=525 potholes=0 configs=543 pending=0`, both `Shutdown` lines and both destructors logged, verdict at PostSystemServiceShutdown; run `glue24-rep-final3` second clean full run (2032-01-05, 38.18 days/s). (3) **The save round trip with the replacements on**: run `glue24-rep-save2` clean, `ref_warnings=0`: `netmgr Write -> 1 (6 bytes, record53 round trip same)`, `netlot Write -> 1 (3160 bytes, record11 round trip DIFFERENT)` (the same one clock-seeded configuration draw the shadow save run recorded, counted, not a mismatch), `script save -> ok=1 (SaveCity(full) in 10844 ms)`. (4) **Open: the debug-CRT int3** (`FreeCity4.dll+0xB320CD` = `failwithmessage`, esp `0x001AE8E4` identical every time) at about tick 5500 (2031-01/07, emergency pause set): runs `glue24-rep-7`, `-rep-final`, `-rep-final2` died there, three of five of this build family; a `_CrtSetReportHookW2` hook installed to log the assert text (commit `d10b154b`) did **not** fire, so the site bypasses `_CrtDbgReport` and the invalid-parameter handler, and the VEH's own report dies before its register line - the site is still unknown. (5) Gate: cargo **3021/0** (the fix round's runs), clippy clean, ctest **58/58**, `abi_entries.inc` unchanged at 178 interfaces, `FC4_ABI_VERSION` not bumped. (6) `netmgr_shadow = 1` joined the regress shadows list (`tools/regress.ps1`, commit `f2715c65`; the lane run `glue24-sh-1` was exact 72/73, the delta NM10 plus a clock-seeded draw). (7) §8.6 street props and flora, the object bridge (`GetLotConfigurationObjectCount` / slot 5 answer the real records) and `object_ids` are wired from `592a9c35`; question **NM11** filed in `QUESTIONS.md` (the game's tool classes' identities, the configuration set's element type, the traffic map's slot 36) | (1) Pin the tick-5500 int3: get a stack (the VEH's report needs to survive its own logging, or a dump), then fix or answer it - it is the one blocker for state ready. (2) Rerun `enable = netmgr,netlot` to 2032 twice clean and the save round trip after that. (3) The lot-configuration object bridge of slot 6 (`GetNetworkLotConfigurationSet` inserts nothing; NM11 names the missing element type). (4) `update_intersection` stays NULL [choice] until the traffic map's slot 36 is specified (NM11) |

## Next steps detail

The three fixed defects are written up in the run records 184-189
(`docs/runs/glue24-rep-5-184.md` ... `glue24-rep-save2-189.md`); the int3 in runs 186,
190, 191. The build family census with the replacements on:

| Run | Build | Verdict | Where |
|---|---|---|---|
| `glue24-rep-5` (184) | owner block, mgr not assigned | exited | `NetworkManager::Init`, null `this` |
| `glue24-rep-6` (185) | owner block complete | corrupt | Shutdown, double-owned record data |
| `glue24-rep-7` (186) | single storage | exited | tick ~5500 int3 |
| `glue24-rep-8` (187) | single storage | **clean** | full run to 2032-01-12 |
| `glue24-rep-save` (188) | single storage | corrupt | Shutdown after `save ok=1` |
| `glue24-rep-save2` (189) | records in place | **clean** | save round trip clean |
| `glue24-rep-final` (190) | records in place | exited | tick ~5500 int3 |
| `glue24-rep-final2` (191) | + report hook, + null guard | exited | tick ~5500 int3, hook silent |
| `glue24-rep-final3` (192) | + report hook, + null guard | **clean** | full run to 2032-01-05 |

The rep-4 diagnosis that started the fix round (its log is `game-results/glue24-rep-4.log`
from the previous session, run not renumbered here): the stack resolved through the DLL's
PDB, `ecx = &cFC4NetworkLotManager::vftable` and `edx` = the ILT thunk of its
`QueryInterface` = vtable slot 0, so the "city" pointer was the vtable address itself.
