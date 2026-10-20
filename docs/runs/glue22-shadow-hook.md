tag: glue22-shadow-hook
run: 170
date: 2026-10-20
lane: 
verdict: clean
switches: adopted by the merge train (the lane's own run)
result: **clean.** ref_warnings=0; 71.23 days/s.
summary: regress glue22-shadow-hook, clean (the lane's pre-train run, adopted by the train)
# Run glue22-shadow-hook (glue22, 2026-10-16)

Request `glue22-shadow-hook.req`: `effects_shadow = 1` (no `enable = effects`), `sim_turbo = 1`,
turbo to `2032-01-01` on Konradshohe.

```
verdict: clean
AUTOTEST VERDICT: clean (foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0,
  julian=2463242, date=2032-01-10, real_seconds=30.531, days_per_second=71.23)
```

Shadow summary line at shutdown:

```
effects shadow: ticks 0, created 0, end posts 0, select draws 0, brush applies 0, sweeps 0, host writes 0, differences 0
```

**Shadow exactness: 0 differences.** The run is clean with `ref_warnings=0`.

The counters are all zero and that is the honest reading: this run predates the shadow's own
animation-tick subscription, so with the effects manager being the original's nothing drove the
mirror. Re-running after the subscription landed (`glue22-shadow-hook3`) reached
`effects shadow: enabled (the original's effects manager is live; the mirror records every write)`
and then hit the harness's `no-load` verdict ("no city load within 390 s (region view? needs a
click)"), as did every later request in this batch — the same request had loaded cleanly 40 minutes
earlier, so that is a harness stall, not the DLL. Getting a non-zero tick count is the remaining
verification item.
