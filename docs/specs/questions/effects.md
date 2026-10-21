# effects: open questions
<!-- Questions about [effects.md](../effects.md). Append new entries at the END; when one is answered, move it to answered/effects.md. Rules: README.md. -->

<!-- re-answers34 (2026-10-20) answered EF1, EF4, EF5, EF6, EF7, EF8, EF9 and EF10; they moved to answered/effects.md.
     EF2 (closed re-integrate1) and EF3 (answered re-integrate1) were archived earlier. No open questions remain. -->

<!-- re-effects-contracts4 (2026-10-21) answered EF11, EF12 and EF13; they moved to answered/effects.md. No open questions remain. -->

- **EF14 - §7.1.1 emitter gaps** (core-effects-fix4, 2026-10-21): (a) the plain emitter `0x005B3CF0` (flag A `0x04`)
  has no body in the spec; the core uses the rate emitter's integral over the whole step without loop counting,
  gaps or stop - what does it do (sub-steps, phase wrap, ages, does it touch `w`)? (b) the rate scale `+0xC0` for
  flags A bits 19/20/21: which wins when several are set, and is `m'` scaled at all when none is? (core: lowest
  bit wins; none -> not applied). (c) `SetDescription`'s `1e-6` floor for `D+0x14`: is it written back into the
  description (so the burst age and the gap use it too) or used for `ρ` only? (core: `ρ` only). (d) the
  particle init/update (main slot 20): when does init with age `a` fail, and when does an update return false?
  (core: a caller-given life; init fails when `a >= life`, a particle dies when its remaining life reaches 0).
  (e) the curve interpolation when `P − n` is outside `[0, 1]` and for an empty curve (core: clamped, 0).
  [all marked `[choice, EF14]` / `[choice]` in engine/core/src/effects/particles.rs]
