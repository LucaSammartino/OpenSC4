tag: glue24fix2-save-195
run: 212
date: 2026-10-20
lane: glue24fix2
dll: FreeCity4-glue24fix2
pacing: turbo
verdict: clean
switches: `enable = netmgr,netlot` (both replacements) with the save script (mark/digest/save/mark/digest at 2031-03-01/02), turbo to 2032-01-01 on Konradshohe, queue request `glue24fix2-save`
result: **clean (foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0), 62 s wall, 44.83 days/s, reached 2032-01-03.** The save round trip with the replacements on and the always-on RTC handler: `netmgr Write -> 1 (6 bytes, record53 round trip same)`; `netlot Write -> 1 (3160 bytes, record11 round trip DIFFERENT)` (the same one clock-seeded configuration draw the glue24-rep-save2 shadow run recorded, counted, not a mismatch); `script save -> ok=1 (SaveCity(full) in 10906 ms)`; all 5 script actions ran; both digests; verdict at the end date through the full shutdown.
summary: save round trip clean with the always-on RTC handler; record53 same, record11 the known clock-seeded draw
