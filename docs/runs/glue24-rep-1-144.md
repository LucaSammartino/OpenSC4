tag: glue24-rep-1-144
run: 198
date: 2026-10-16
lane: glue24
dll: FreeCity4-glue24
pacing: turbo
verdict: corrupt
switches: `enable = netmgr,netlot` (both replacements), turbo to 2032-01-01 on Konradshohe, queue request `glue24-rep-1`
result: **corrupt, 15 s wall, never reached the first day (julian=0).** `foreign_exceptions=1`, `ref_warnings=0`: `first-chance exception OUTSIDE FreeCity4.dll in SimCity 4.exe+0x33293C: code 0xC0000005 ACCESS_VIOLATION (reading address 0x00000000)`, right after `PostCityInit: hidden pause released`, registers `eax=0 ebp=0`, caller `SimCity 4.exe+0x8BC91`. Both classes were created and receiving messages (the reference audit shows `cFC4NetworkManager::DoMessage` and `cFC4NetworkLotManager::DoMessage` running). Cause: `GetNetworkLot` handed out a network lot whose occupant had not resolved, so `GetNetworkOccupant()` answered null and the game dereferenced it. Fixed in `cFC4NetworkLotManager.cpp` (a lot with occupant 0 is not handed out, and `IterateAllNetworkLots` skips it); see `glue24-rep-2`.
summary: replacement run 1 crashed (a null network lot occupant); fixed for run 145
