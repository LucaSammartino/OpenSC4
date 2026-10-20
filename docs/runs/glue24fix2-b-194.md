tag: glue24fix2-b-194
run: 194
date: 2026-10-20
lane: glue24fix2
dll: FreeCity4-glue24fix2
pacing: turbo
verdict: clean
switches: `enable = netmgr,netlot` (both replacements), turbo to 2032-01-01 on Konradshohe, queue request `glue24fix2-b`
result: **clean (foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0), 46 s wall, 62.73 days/s, reached 2032-01-15 and quit at the end date.** Second clean full run with the always-on RTC failure handler: passed the 2031-01-03/04 death window of the three int3 deaths (glue24-rep-7/-final/-final2) with zero RTC errors and the full shutdown path. With the handler installed, any future failing check logs `RTC ERROR #N` with the call site instead of ending the process.
summary: second clean run through the previously fatal tick-5500 window; rtc=0
