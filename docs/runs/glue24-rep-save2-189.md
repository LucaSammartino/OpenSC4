tag: glue24-rep-save2-189
run: 189
date: 2026-10-20
lane: glue24
dll: FreeCity4-glue24
pacing: turbo
verdict: clean
switches: `enable = netmgr,netlot` (both replacements) with the save script, turbo to 2027-01-01 on Konradshohe, queue request `glue24-rep-save2`
result: **clean (foreign_exceptions=0, ref_warnings=0), 25 s wall, 21.96 days/s, reached 2027-01-10.** The component 53 / 11 save round trip with the replacements on: `netmgr Write -> 1 (6 bytes, record53 round trip same)`; `netlot Write -> 1 (3160 bytes, record11 round trip DIFFERENT)` (the same one clock-seeded configuration draw the shadow save run recorded, counted, not a mismatch); `script save -> ok=1 (SaveCity(full) in 10844 ms)`; both digests ran; `netmgr Shutdown` / `netlot Shutdown` and both destructors logged clean at quit.
summary: the save round trip with the replacements on is clean; record53 same, record11 differs by the known clock-seeded draw
