tag: glue24-rep-3-145
run: 145
date: 2026-10-16
lane: glue24
dll: FreeCity4-glue24
pacing: turbo
verdict: corrupt
switches: `enable = netmgr,netlot` (both replacements), turbo to 2032-01-01 on Konradshohe, queue request `glue24-rep-3`
result: **corrupt, 21 s wall, never reached the first day (julian=0).** Same signature as `glue24-rep-1` and `glue24-rep-2`: `first-chance exception OUTSIDE FreeCity4.dll in SimCity 4.exe+0x33293C: ACCESS_VIOLATION (reading 0x00000000)`, `eax=0 ebp=0`, caller `SimCity 4.exe+0x8BC91`, 7 ms after `PostCityInit: hidden pause released`. The two null-guard fixes (a network lot with an unresolved occupant is not handed out; `GetLotConfigurationObjectArray` returns a stable empty vector instead of null) did not change it. The log pins two defects in the replacement's host wiring: `netmgr Init: ok=1 managed=0 pending=0 retrofit=0 crews=0 **tools=0**` (the 13 tools are never created, so slot 5 `GetNetworkTool` answers null and the game dereferences it) and `netlot Init: ok=1 **lots=0 potholes=0 configs=0**` (the 543 network lot configurations of the exemplars are never fed to `fc4_netlot_add_configuration_props`, so nothing attaches and the 525 lots read from component 11 are dropped). `netmgr Read -> 1 (0 managed entries)` and `netlot Read -> 1 (525 lots, 0 potholes)` are both fine. Next: wire `create_tool` to the game's `cSC4NetworkTool` class through the GZCOM factory and load the type-0x21 exemplars into the lot core (the shadow's `FC4NetMgrShadow.cpp` already does both; copy its enumeration).
summary: replacement still crashes at PostCityInit; tools=0 and configs=0 pin the two host gaps
