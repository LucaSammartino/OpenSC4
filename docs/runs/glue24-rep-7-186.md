tag: glue24-rep-7-186
run: 186
date: 2026-10-20
lane: glue24
dll: FreeCity4-glue24
pacing: turbo
verdict: exited
switches: `enable = netmgr,netlot` (both replacements), turbo to 2032-01-01 on Konradshohe, queue request `glue24-rep-7`
result: **exited (code -1) without a verdict, 117 s wall, at about tick 5500 (2031-01-03, emergency pause set).** `first-chance exception in FreeCity4.dll: code 0x80000003 BREAKPOINT at FreeCity4.dll+0xB320CD` - our statically-linked debug CRT's `failwithmessage` - and the vectored handler's report died before its register line, so the assert site is unknown. `netmgr Init ... tools=13` / `netlot Init: lots=525 configs=543` fine; the run played 100 s before it. The same signature as `glue24-rep-final` and `glue24-rep-final2` (about tick 5500, the same esp 0x001AE8E4); two other runs of the same build family (`glue24-rep-8`, `glue24-rep-final3`) ran clean to 2032 and through the full shutdown. Open defect, one run in two.
summary: rep-7 hit the debug CRT int3 at tick 5500; site unknown (no report survived)
