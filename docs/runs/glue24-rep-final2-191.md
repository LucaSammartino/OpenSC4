tag: glue24-rep-final2-191
run: 191
date: 2026-10-20
lane: glue24
dll: FreeCity4-glue24
pacing: turbo
verdict: exited
switches: `enable = netmgr,netlot` (both replacements), turbo to 2032-01-01 on Konradshohe, queue request `glue24-rep-final2`
result: **exited (code -1) without a verdict, 123 s wall, at about tick 5500 - the third int3 (`FreeCity4.dll+0xB320CD`, esp 0x001AE8E4).** This build had the new `_CrtSetReportHookW2` hook (FC4FailFast.cpp), which logs every CRT report with the last interface calls: it did **not** fire for this int3, so the site is one the debug CRT reaches without `_CrtDbgReport` and without the invalid-parameter handler (`failwithmessage` itself). No Rust panic line either. Open defect, three of five runs of this build family.
summary: rep-final2 hit the int3 again; the CRT report hook did not fire, so the site bypasses _CrtDbgReport
