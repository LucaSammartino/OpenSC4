tag: glue23-shadow-1
date: 2026-10-16
lane: glue23
dll: FreeCity4
pacing: turbo
verdict: clean
switches: demolition_shadow = 1, script = build-runs/glue23-demolish.script (zone / dezone blocks 2027-2029), turbo to 2032-01-01 on Konradshohe
result: **clean**, ref_warnings=0, rtc=0, overruns=0, 70.53 days/s. demolition shadow **73 lines, 73 with every mismatch count 0** (record=2 bytes roundtrip=same every event). glue17's exactness is not regressed: its glue17-shadows-4 / -5 runs were 73/73 on the same shape.
summary: glue23 shadow regression check, clean, demolition_shadow 73/73 exact
