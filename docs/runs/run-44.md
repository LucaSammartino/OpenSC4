tag: run-44
run: 44
date: 2026-10-13
lane: run
dll: unknown
pacing: unknown
verdict: clean
switches: demand fix `5904E092…` (merged as `42f7649`), same config as 43
result: **clean, and demand is bit-identical everywhere:** 160 of 160 NewDay updates and **6 of 6** modifier checks, including 2001-10-01 (orig/ours `0.0832541287`, at the same neutral `8.99755573` that run 43 had 4 ulp off). Cause and fix: the exe (`0x006A6070`) multiplies `r24(maxRate·0.1f)` by a float32 reciprocal of neutral computed once per update, rather than dividing (demand.md §8.1, DQ1).
