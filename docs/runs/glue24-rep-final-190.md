tag: glue24-rep-final-190
run: 190
date: 2026-10-20
lane: glue24
dll: FreeCity4-glue24
pacing: turbo
verdict: exited
switches: `enable = netmgr,netlot` (both replacements), turbo to 2032-01-01 on Konradshohe, queue request `glue24-rep-final`
result: **exited (code -1) without a verdict, 117 s wall, at about tick 5500 (2031-07-03, emergency pause set, `main: outside FreeCity4 (last OnTick 15 ms ago)`).** The same debug CRT int3 as run 186 (`FreeCity4.dll+0xB320CD`, esp 0x001AE8E4 identical), the second of the build family. The heartbeat line before it: +6368155 deliveries, 0 foreign exceptions, 0 x87 leftovers, 791 of 2047 MiB.
summary: rep-final hit the same tick-5500 int3; second occurrence
