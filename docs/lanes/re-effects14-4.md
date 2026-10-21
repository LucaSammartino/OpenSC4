| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-effects14-4: effects spec 7.1.1, EF14 (a-e) answered (plain emitter body, +0xC0 precedence, period floor, particle life, curve edges) | `re-effects14-4` / `.claude/worktrees/re-effects14-4` | ready, docs only, no ABI change; no game runs; no open effects questions | 1. core lane: rewrite the particles.rs plain (`0x04`) emitter as a top-up to ceil(curve[0]): no m/+0xC0 scale, no slot-20 pre-age, `n`/`w`/`c` untouched. 2. core lane: draw particle life uniformly from [D+0x0C, D+0x10] in init (not passed by the caller); slot 20 does age += t and fails at age >= life. 3. Keep the clamp and the empty-curve 0 as [choice]; the original does not guard either. |

## Report
Changes (docs only): effects.md 7.1.1 gains a "Plain emitter" block and an "Emitter details" block, and the rate
emitter's step 2 is clarified. EF14 moved to answered/effects.md and QUESTIONS.md was regenerated.
Findings, all from instruction bytes (re_query asm; REA not needed):
- (a) `0x005B3CF0`: `P += e·ρ`. Once `P > n` (with `n` = 0), it computes `T = ceil(curve[0])` and creates `T − live`
  particles through CreateParticle (age 0, new life) and AddParticle. There is no slot 20, no bit-31 birth hook and no
  `m` or `+0xC0` scale, and `w`, `c` and `n` are untouched. If `live > T`, the unsigned count underflows (about 2^32
  iterations); creating nothing in that case is [choice].
- (b) Bits 19, 20 and 21 are tested in that order and the first set bit wins (k, k², k³); with none set there is no
  factor (`0x005B87BD`–`0x005B8828`). The core matches.
- (c) The `1e-6` floor is a local value; `D+0x14` is never written (`0x005B7CF3`). The core matches.
- (d) InitEffectsParticle `0x005B7580` sets life to a uniform value in [D+0x0C, D+0x10] and age to 0. Slot 20
  `0x005B9BA0` does `age += t` and returns false when age >= life. The Extra path (flags A & 0x18000D00) adds
  water/terrain kills (`0x005B8DC0`, D+0x154/D+0x150).
- (e) The curve lookup has no clamp: when `P − n > 1` it reads past the end (reachable when ρ > 15). Empty curves are
  not guarded. The clamp and the 0 for an empty curve stay [choice].
Questions: EF14 answered; none open. No game runs; the shadow is not affected (spec only).
