tag: rzones-97
run: 97
date: 2026-10-14
lane: rzones
dll: 69ac7ccb
pacing: unknown
verdict: PASS
switches: main `2b2420d` (the `zoneglue` merge; DLL `69ac7ccb`), `tools/regress.ps1 -Tag rzones` with zones added to both lists (sixteen replacements)
result: **PASS, both runs clean, 0 problems, `ref_warnings=0`.** Our zone manager loads the city's record (`zones Read -> 1`, counts 14257/3/1040/251/701/16/116 by zone type, grid 128×128, alpha 170) and runs to 2033-11 (94 month lines). The run zones nothing, so PlaceZone's commit path and the dezone handler did not run in game (`place=0 dezones=0`); every `undeveloped=` value reads 16384, worth a look against the spec. **Zones shadow, first in-game run:** counts, bookkeeping and tuning 92/92 exact; PlaceZone dry runs exact on 77 of 92 months. The other 15 differ in one call each: the 1×1 region with zone type 14 fails with error 3 on both sides, but the original writes a cost of 36–208 and ours 0 (ZG17, sent to re-answers8). Other shadows as run 96; power 366/457 and plumbing 175/183 remain (apply6b).
