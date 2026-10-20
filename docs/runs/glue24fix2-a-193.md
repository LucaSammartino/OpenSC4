tag: glue24fix2-a-193
run: 193
date: 2026-10-20
lane: glue24fix2
dll: FreeCity4-glue24fix2
pacing: turbo
verdict: clean
switches: `enable = netmgr,netlot` (both replacements), turbo to 2032-01-01 on Konradshohe, queue request `glue24fix2-a`
result: **clean (foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0), 52 s wall, 60.91 days/s, reached 2032-01-06 and quit at the end date.** First run on the build with the RTC failure handler installed in every build (commit `c1afaa18`): a Debug configuration compiles /RTC1 into the plugin even with FC4_CHECKED off, and until now a failing check reached the UCRT's `failwithmessage` with no error func installed and ended the process at its silent int3 (`FreeCity4.dll+0xB320CD`, the glue24-rep-7/-final/-final2 deaths). The handler now logs instead. The run passed through the exact death window (2031-01-03, tick ~5500) without the int3 and without any RTC error (rtc=0 — the rare site did not fire this time; if it fires, the next run logs `RTC ERROR #N` with the call site instead of dying). Full Shutdown path and verdict at PostSystemServiceShutdown.
summary: first clean run with the always-on RTC handler; passed the tick-5500 death window
