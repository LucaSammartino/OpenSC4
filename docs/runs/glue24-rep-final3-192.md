tag: glue24-rep-final3-192
run: 209
date: 2026-10-20
lane: glue24
dll: FreeCity4-glue24
pacing: turbo
verdict: clean
switches: `enable = netmgr,netlot` (both replacements), turbo to 2032-01-01 on Konradshohe, queue request `glue24-rep-final3`
result: **clean (foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0), 67 s wall, 38.18 days/s, reached 2032-01-05 and quit at the end date.** `netmgr Init ... tools=13`, `netlot Init: lots=525 potholes=0 configs=543 pending=0`; `netmgr Shutdown: managed=0 pending=0`, `netlot Shutdown: lots=0 potholes=0`, both destructors logged, the autotest verdict at PostSystemServiceShutdown. Second clean full run of this build family (with `glue24-rep-8`); the tick-5500 int3 did not reproduce here (two of five runs of the family hit it).
summary: rep-final3 clean to 2032 with the full shutdown; the int3 did not reproduce
