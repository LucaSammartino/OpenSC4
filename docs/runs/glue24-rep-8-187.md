tag: glue24-rep-8-187
run: 187
date: 2026-10-20
lane: glue24
dll: FreeCity4-glue24
pacing: turbo
verdict: clean
switches: `enable = netmgr,netlot` (both replacements), turbo to 2032-01-01 on Konradshohe, queue request `glue24-rep-8`
result: **clean (foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0), 46 s wall, 62.61 days/s, reached 2032-01-12 and quit at the end date.** `netmgr Init: ok=1 managed=0 pending=0 retrofit=0 crews=0 tools=13` and `netlot Init: ok=1 lots=525 potholes=0 configs=543 pending=0` - the stand-in tools load (13) and the 543 type-0x21 network lot configurations attach, so the 525 lots of component 11 resolve. Both replacements' `Shutdown` lines and both destructors logged (`netmgr Shutdown: managed=0 pending=0`, `netlot Shutdown: lots=0 potholes=0`, `cFC4NetworkLotManager destroyed`, `cFC4NetworkManager destroyed`), then the autotest verdict at PostSystemServiceShutdown. The enable = netmgr,netlot run to 2032 the lane row asked for, on the build with the owner-block host user and the single-storage configuration records.
summary: replacements clean to 2032 with the full shutdown path; tools=13, configs=543
