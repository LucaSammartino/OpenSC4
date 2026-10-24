tag: split11-combined-624
run: 624
date: 2026-10-24
lane: tool-regress-split11
dll: main ffb8f38c2 engine, rebuilt by qbuild verify; sha256 0a67d38e6a8c1fbf969f7e483132fa17ead29076f3a0d248d8ddff2037ede82c
pacing: turbo target 2032-01-01, queue, ShadowSplit 1; waited for train198 before submission
verdict: FAIL; watchdog stall at 2029-02-08; incomplete, not a valid 2032 parity baseline
switches: unchanged house shadows profile and standard replacements, same DLL snapshot as run 612
result: regress-split11-c1 wall 455 s, shadows real_seconds=421.172, days_per_second=2.62, watchdog_stalls=1, foreign_exceptions=0, ref_warnings=0. Replacement clean after 132 s. The last heartbeat reported outside FreeCity4, last OnTick 6281 ms ago. Traffic 36/38 has mismatch total 70; other labels have zero mismatch totals up to the early stop. read_regress parsed all 52 labels; the existing regress delta also parsed the aggregate baseline (the validator used the failed initial split11-s3 baseline, not the corrected split). The 455 s duration is truncated and cannot be used for a speedup claim. A fresh completed combined run is needed before asserting strict split/combined count and mismatch parity.
summary: Incomplete combined attempt retained separately; no exactness or performance credit claimed.

Counts from `tools/coverage.py read_regress(build-runs/regress-split11-c1.out)` (only through 2029-02-08):

```text
advisors 38/38
aura 36/36
automata 37/37
automataowners 37/37
budget 37/37
citydetail 1/1
civic 37/37
clock 38/38
commercial 18/18
crime 37/37
dbpf 38/38
demand 1145/1145
development 458/458
feature 38/38
flammability 27/27
flora 37/37
history 41/41
landfill 37/37
landvalue 37/37
lotcfg 153/153
lotconf 38/38
lotdev 38/38
lotrec 38/38
lua 38/38
mysims 38/38
neighbours 37/37
netmgr 38/38
netocc 37/37
netrules 37/37
occmgr 37/37
ordinance 37/37
overlayrecords 152/152
overlayreplay 152/152
overlays 38/38
parks 37/37
plumbing 75/75
pollution 37/37
power 249/249
propmgr 115/115
protection 148/148
residential 582/582
scres 79/79
simgrid 38/38
strings 37/37
terrain 313/313
tract 75/75
traffic 36/38
tutorial 76/76
variant 1/1
weather 38/38
zonedev 190/190
zones 152/152
```
