tag: glue24-rep-2-146
run: 200
date: 2026-10-16
lane: glue24
dll: FreeCity4-glue24
pacing: turbo
verdict: corrupt
switches: `enable = netmgr,netlot`, turbo to 2032-01-01 on Konradshohe, queue request `glue24-rep-2`
result: **corrupt, 15 s wall, julian=0.** The first null guard (a network lot whose occupant did not resolve is not handed out by `GetNetworkLot` / `IterateAllNetworkLots`) did not change the crash: the same `SimCity 4.exe+0x33293C` `ACCESS_VIOLATION (reading 0x00000000)` 7 ms after `PostCityInit: hidden pause released`, `eax=0`. Between `glue24-rep-1` and `glue24-rep-3`.
summary: replacement run 2, first null guard did not help
