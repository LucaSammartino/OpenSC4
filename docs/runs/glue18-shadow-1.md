# glue18-shadow-1

Terrain shadow alone (`terrain_shadow = 1`, no replacement): the shadow's
behaviour is unchanged by this lane. Turbo to 2032-01-01 on Konradshohe.

```
AUTOTEST VERDICT: clean (foreign_exceptions=0, watchdog_stalls=0, rtc=0,
  overruns=0, ref_warnings=0, julian=2463252, date=2032-01-20,
  real_seconds=26.781, days_per_second=82.48)
```

Terrain shadow summary:

```
terrain shadow load 2026-01-27: cells=128x128 sea=250 water cells=3009;
  depths rebuilt from the sea level 16641/16641;
  cache orig 192.048019/282.732513, fresh 192.048019/282.732513 (kept: orig, §4.6);
  allowed heights 2/2;
  queries altitude 1024/1024 normal 1024/1024 water 17408/17408 land 17408/17408
    depth 17408/17408 type 17408/17408 navigable 17408/17408;
  refused=0; mismatches=0
terrain shadow: city shutdown: 98 lines, 98 with mismatches=0;
  totals queries altitude 22532/22532 normal 21592/21592 water 37036/37036
    land 37036/37036 depth 37036/37036 type 37036/37036 navigable 37036/37036;
  other checks 16940, mismatched 0;
  unfollowed vertices 0, followed late 18, pending at shutdown 0;
  depth rules by kind (slot50/raiselower/reset/none) 5:0/22/0/0
```

- **exact/total per query:** altitude 22532/22532, normal 21592/21592, water
  37036/37036, land 37036/37036, depth 37036/37036, type 37036/37036,
  navigable 37036/37036. Every one exact.
- **`unf` 0, `late` 18, `pend` 0.** The 18 followed-late vertices are the queued
  `0x278E6003` posts arriving after the change (see glue18-latency-1).
- **refused=0** everywhere: the navigability host (bridges, power poles) never
  called back into the terrain handle (TR18), on either side.

Verdict `clean`, `ref_warnings=0`.
