# Run glue22-effects-shadow (glue22, 2026-10-16)

Request `glue22-effects-shadow.req`: `enable = effects`, `effects_shadow = 1`, `sim_turbo = 1`,
turbo to `2032-01-01` on Konradshohe. DLL `engine/build-x86/FreeCity4.dll`.

```
verdict: clean
AUTOTEST VERDICT: clean (foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0,
  julian=2463234, date=2032-01-02, real_seconds=34.406, days_per_second=62.98)
```

`ref_warnings=0`, no shadow INCOMPLETE lines, no WARNING lines (the expected 2 x87 start-up warnings
only), 0 Rust panics, 0 structured exceptions.

This was the first run of the lane. The log showed `subsystem 'effects': enabled (1 class replaced)`,
but the `effects` class table entry and the `EffectsShadowStartup` call had been dropped by a
CRLF-mismatched edit, so the class was never registered and nothing of the effects glue ran. Both
were fixed and the run re-sent (see `glue22-effects-shadow2/3`).
