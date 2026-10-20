# Run glue22-shadow-drawpass (glue22, 2026-10-16)

Request `glue22-shadow-drawpass.req`: `effects_shadow = 1`, `effects_finish = 1` (EF2 compat mode),
`sim_turbo = 1`, turbo to `2032-01-01` on Konradshohe.

```
verdict: clean
AUTOTEST VERDICT: clean (foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0,
  julian=2463237, date=2032-01-05, real_seconds=31.515, days_per_second=68.85)
```

Shadow summary line at shutdown:

```
effects shadow: ticks 0, created 0, end posts 0, select draws 0, brush applies 0, sweeps 0, host writes 0, differences 0
```

Same reading as `glue22-shadow-hook`: clean, `ref_warnings=0`, 0 differences, and the mirror was not
yet self-ticking so every counter is zero. The run's purpose was the EF2 comparison — with
`effects_finish = 1` the sweep is the draw pass's (`fc4_effects_draw_pass`) rather than the hook's,
and the run is clean either way. The counters that would separate the two modes (end posts, sweeps)
need a driven mirror; see docs/lanes/glue22.md.
