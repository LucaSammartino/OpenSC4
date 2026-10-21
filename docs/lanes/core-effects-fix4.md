| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-effects-fix4: EF12/EF13 DIFFERS fixed in the effects core (reload-only effects tick, L/L_prev rebuild test, 7.1.1 emitter contract) | `core-effects-fix4` / .claude/worktrees/core-effects-fix4 | wip: gate PASS; game blocked: `effects_shadow = 1` runs (r1, r2) no-load, hang right after plugin init; controls r3/r4 clean | attribute the effects_shadow startup hang (run main's DLL with effects_shadow=1); answer EF14; CAPI route for PostAppServicesInitComplete / queued reload |

## Report

**What changed** (engine/core/src/effects, engine/conformance; no plugin/CAPI/ABI edits):
- `manager.rs` (44f9689b, EF12): `UpdateVisualEffect(effect, newBuild)` in two passes — existing components survive only
  without a reload and with L in range; a description takes part in a select draw only if it entered the range
  (in at L, not at L_prev), or newBuild, or reload; non-participants leave the group state alone; group-0 resets the
  group; unsigned `sum >= r`, `r := 0x10000` after a pick (a group past 0xFFFF creates a second member).
  `L_prev` (`+0xC88`, -1 initially) set by `set_zoom`. `effects_tick` (DoEffectsTick) does nothing unless
  `queue_effect_files()` was called, then reloads (`+0xB8C`). `UpdateActiveVisualEffects` gated on `+0x1AD`
  (`set_services_ready`, `post_app_services_init_complete`); `render_properties_changed` reloads on a changed
  detail option; the sweep now ends in `ActivateNewVisualEffects`.
- `particles.rs` + `particles_tests.rs` (c7fa1fdf, EF13): `EmitterDesc` with the 7.1.1 fields; Start seeds `w`
  from start-delay, else interval, else 0 (global `RzRandom::range_f64`); Start(arg!=0) skips a finite emitter
  without flag 0x08 and arms the pre-roll; dispatch emits the overshoot, clamps to pre-roll-0.01, nothing after a
  delayed stop; loop-budget burst `ceil(curve[0])` (scaled when >20 and D+0x08 bit0 clear) with burst age,
  interval reseed, Stop(0) at the loop count; rate emitter 1/15 s sub-steps, float carry, oldest-first ages,
  loop counting, hold with 0x08; live particles aged before emission.
- Conformance: ef17_18 updated (tick activates nothing; the sweep does), new ef17_19 rebuild-test vector, ef17_16
  rewritten for the emitter contract; ops `queue_effect_files`, `render_properties_changed`; README rows.
- EF11 already matched (dilation untouched).

**Measured:** `qbuild all` after c7fa1fdf: PASS — cargo 3958 passed / 0 failed, clippy 0, CMake 152/152, ctest 64/64.

**Game:** r1, r2 (`enable = effects`, `effects_shadow = 1`, turbo): no-load, log stops right after
"effects shadow: enabled" during plugin init (before any city; ref_warnings 0, no panic). r3 (no effects) clean,
r4 (`enable = effects` only; AddCls skipped without the shadow) clean, ref_warnings 0. Not attributed: the diff
touches no plugin/CAPI code; a run of main's DLL with effects_shadow=1 would settle it. No shadow line obtained.

**Choices / questions:** EF14 filed (plain emitter body, rate-scale bits, 1e-6 period floor, slot-20 life model,
curve edge cases). `+0x1AD` defaults true and Shutdown keeps it ([choice]: the C ABI has no route for
PostAppServicesInitComplete); a negative L is in no range; every directory component counts as a valid type.

**Next:** attribute the shadow startup hang; CAPI routes for queued reload / services-ready / render-properties
(a CAPI lane); answer EF14.
