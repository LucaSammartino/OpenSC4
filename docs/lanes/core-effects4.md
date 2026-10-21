| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-effects4: effects core, re-answers34 integration (dilation table, particle self-stop, DoEffectsTick activation) | `core-effects4` / (engine lane) | ready, no ABI change; core commits `05532def`, `7e55e187`; gate cargo 3589/0, clippy 0, ctest 62/62; no game runs (core lane); EF11-EF13 filed | 1. coordinator: set the coverage-map `effects` row core path to `core/src/effects` (+ `conformance/src/effects.rs`). 2. regenerate QUESTIONS.md (tools/questions.py index) for EF11-EF13. 3. glue lane: host `simulator()` from `dilation::animation_time_dilation`, call `effects_tick` per animation tick. |

## Report

**What changed** (engine/core/src/effects, conformance only; no capi, no plugin, no ABI):
- `dilation.rs` (new): `SHIPPED_DILATION = [0.0, 1.0, 1.5, 2.0, 1.0]`, `SimTimeState`,
  `animation_time_dilation(_with)` (emergency pause / U-Drive-It driving → `[4]`, paused → `[0]` = 0.0, speed
  0/1/2 → `[1]/[2]/[3]`, hidden pause ignored; §6, EF5, test 17), `dilated_us` (the tick's `trunc(r24(µs × d))`,
  now used by `EffectsManager::tick`).
- `particles.rs` (new): `ParticleSimulator` = the particle component's start/stop protocol and the emitter
  self-stop (§7.1, EF1, test 16): component `Stop(0)` delayed, `Stop(1|2)` immediate, `CheckForStop` turns a
  delayed stop into the immediate one when the live count is 0; `EmitterKind::from_flags` (bit 2 plain, else
  bit 1 loop budget, else duration budget); loop/duration budgets call `Stop(0)` on themselves. Render data
  (loop count, interval, per-loop count, life, duration) is caller-given (`EmitterParams`).
- `manager.rs`: `EffectsManager::effects_tick` = DoEffectsTick `0x697D01ED` → `ActivateNewVisualEffects`
  (§6.1, EF4); tick doc fixed (paused = dilation 0, updates with dt 0).
- Tests: 9 new core unit tests (t16 x3, t17, paused tick, effects_tick, emitter choice, hard stop, volcano
  particle life); conformance ops `ef_dilation`, `ef_particles`, `ef_call effects_tick`; vectors
  `ef17_16_particle_self_stop.json`, `ef17_17_dilation_table.json`, `ef17_18_effects_tick_activation.json`.

**Measured**: baseline cargo 3577/0, ctest 62/62; at the tip cargo test --workspace 3589 passed / 0 failed,
clippy -D warnings 0 diagnostics, ctest 62/62. No game runs (core lane, game=false).

**Questions filed** (docs/specs/questions/effects.md): EF11 (speeds above 2 in the dilation table), EF12 (does
DoEffectsTick rebuild every active effect, and how a rebuild avoids a fresh select-group draw), EF13 (emitter
details: first loop timing, per-loop count, duration source, emission after a delayed stop, age/emit order).
QUESTIONS.md not regenerated (tools/ is off-limits to this lane).

**Owner classes, honestly**: the effects core (cores19, already on main, plus this lane) implements
`cSC4EffectsManager` (creation, priority filter, LOD, select groups, tick, finish sweep, end messages,
DoEffectsTick activation), `cSC4EffectsResource` (the binary directory read, `directory.rs`),
`cSC4BrushDescription` (+ brush effect), `cSC4ScrubberDescription` (+ game effect). **Not implemented**:
`cSC4EffectsParser` / `EffectsParser` (text effect files), `cSC4EffectsCollection` (BeginCollection /
EndCollection sub-object, §18), `EffectsBootstrap`, and the §18 interface sub-objects. The particle model in
`particles.rs` belongs to the separate "effects (particles, decals, visuals)" row; it covers only the stop
protocol, so claim no coverage there.

**Proposed coverage-map paths** for the `effects` row: core `core/src/effects` (mod.rs, manager.rs,
directory.rs, brush.rs, game.rs, shipped.rs, dilation.rs, particles.rs); conformance
`conformance/src/effects.rs`. Shadow and replace keys stay `-`.

**Next**: (1) the glue: the plugin host should answer `EffectsHost::simulator` with
`animation_time_dilation` of the real simulator state and call `effects_tick` once per animation tick;
(2) after EF12 is answered, revisit `set_zoom`'s rebuild (an unchosen select member redraws on rebuild).
