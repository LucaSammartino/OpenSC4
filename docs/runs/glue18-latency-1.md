tag: glue18-latency-1
run: 165
date: 2026-10-20
lane: 
verdict: clean
switches: adopted by the merge train (the lane's own run)
result: **clean.** ref_warnings=0; 79.08 days/s.
summary: regress glue18-latency-1, clean (the lane's pre-train run, adopted by the train)
# glue18-latency-1

Terrain shadow with a script that drives the height setters
(`terrain_shadow = 1` + `build-runs/glue18-terrain-script.txt`): measures how
late the queued terrain messages arrive under turbo. Turbo to 2032-01-01 on
Konradshohe.

```
AUTOTEST VERDICT: clean (foreign_exceptions=0, watchdog_stalls=0, rtc=0,
  overruns=0, ref_warnings=0, julian=2463240, date=2032-01-08,
  real_seconds=27.766, days_per_second=79.08)
```

## The scripted setters (through `cISTETerrain` slot 49)

```
script: line 6 2026-02-01 terrain 40 40 48 48 310.0 1 -> SetAltitude(40,40..48,48, 310, true, 1): vertex (40,40) 275.028595 -> 310
script: line 7 2026-03-01 terrain 60 40 68 48 250.0 3 -> SetAltitude(60,40..68,48, 250, true, 3): vertex (60,40) 198.90419 -> 250
script: line 8 2026-04-01 terrain 40 60 48 68 20.0 5  -> SetAltitude(40,60..48,68, 20, true, 5): vertex (40,60) 274.149628 -> 20
script: line 10 2028-01-01 terrain 80 80 88 88 320.0 4 -> SetAltitude(80,80..88,88, 320, true, 4): vertex (80,80) 272.588776 -> 320
script: all 8 action(s) ... have run
```

Each is mirrored at once by the shadow (correct: the setter is synchronous):

```
terrain shadow script 2026-02-01 SetAltitude rect=40,40..48,48 h=310 kind=1: vertices 81/81; cache 192.048019/310; refused=0; mismatches=0
terrain shadow script 2026-03-01 SetAltitude rect=60,40..68,48 h=250 kind=3: vertices 81/81; cache 192.048019/310; refused=0; mismatches=0
```

## Message latency (the observation this run is for)

The `0x278E6003` post for a change is a **queued** post and arrives long after
the edit. The `2026-03-01` setter's change is followed on **`2026-04-10`**:

```
terrain shadow change 2026-04-10 terrain kind=3 rect=60,40..68,48: vertices=81 moved=0 wet=0
  depth rules slot50/raiselower/reset/none=0/0/0/0; message 1/1;
  changed elsewhere 2 (rect=21,92..22,92; cells touching them skipped 0); ...
```

That is about **40 game days** from the change to its message, the same order
glue16 measured (posted 2026-02-10, delivered 2026-03-20). A terrain
replacement consumer of `0x278E6003` must therefore **not** assume prompt
delivery: the rectangle it is told about is already stale, and anything that
reacts to it (the network manager's terrain handler, the landfill, the BDS)
sees the change up to weeks late. The shadow handles this by keeping moved
vertices "ahead" of their message and reconciling when it lands; a replacement
that only reacted to the message would be wrong for that whole window.

## Terrain shadow summary

```
terrain shadow: city shutdown: 100 lines, 100 with mismatches=0;
  totals queries altitude 25082/25082 normal 23292/23292 water 37886/37886
    land 37886/37886 depth 37886/37886 type 37886/37886 navigable 37886/37886;
  other checks 17301, mismatched 0;
  unfollowed vertices 0, followed late 19, pending at shutdown 0;
  depth rules by kind (slot50/raiselower/reset/none) 5:0/23/0/0
```

- **exact/total:** all seven queries exact (25082 / 23292 / 37886 each).
- **`unf` 0, `late` 19, `pend` 0.**
- **refused=0** (TR18).

Verdict `clean`, `ref_warnings=0`.
