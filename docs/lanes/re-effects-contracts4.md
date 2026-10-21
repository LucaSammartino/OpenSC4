| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-effects-contracts4: effects spec, EF11-EF13 answered (dilation default, DoEffectsTick/select rebuild test, emitter contract) | `re-effects-contracts4` / `.claude/worktrees/re-effects-contracts4` | ready, docs only, no ABI change; no game runs; no open effects questions | 1. core lane: `effects_tick` must not activate or rebuild on an ordinary tick (only on a queued-file reload); activation happens in create / draw sweep / UpdateActiveVisualEffects. 2. core lane: select-group draw only for members entering the range (L vs L_prev), new build or reload (effects.md 5.3). 3. core lane: particles.rs emitter per effects.md 7.1.1 (start countdown seeding, ceil burst, rate emitter loops + carry, pre-roll on Start(1)). |

## Report

Answered all three open effects questions from the bytes (re_query.py asm/func, decomp/; REA not needed).

- **EF11 (MATCHES)** - effects.md §6 step 1: the dilation getter `0x007026F0` maps speed 1 -> `[2]`, 2 -> `[3]`,
  every other `u32` (0, 3+, "negative") -> `[1]` = 1.0. No clamp.
- **EF12 (DIFFERS)** - §6.1 correction: `DoEffectsTick` `0x00593D00` calls `UpdateActiveVisualEffects` only
  when the `+0xC78` object reports queued effect files (reload with `+0xB8C` = 1), then the 25-December snow
  line. `UpdateActiveVisualEffects` is a no-op until PostAppServicesInitComplete sets `+0x1AD`. Activation
  callers listed. §5.3 new "rebuild test": no per-effect flag; a description takes part in a select draw only
  if (in range at L and not at L_prev) or new build or reload; quirks (out-of-range members count in new-build
  draws; weights past 0xFFFF create a second member).
- **EF13 (PARTLY)** - new §7.1.1 emitter contract: SetDescription rate `1/max(D+0x14,1e-6)`; Start refusal for
  finite emitters on `Start(1)`, field resets, countdown seeding from start-delay then interval range (global
  cRZRandom `0x00B22DF8`), pre-roll `D+0x1C` on `Start(1)`; update ages particles first then emits; dispatch
  skips during a delayed stop, overshoot-only emit time, pre-roll clamp; loop-budget burst (ceil count, age,
  max-age gate, Stop(0) at count, interval gap); rate emitter (1/15 s sub-steps, curve interpolation, float
  carry `+0x138`, floor count, hold with flag `0x08`). §7.1 item 2 wording fixed; §14 two RNG rows.
- Core hints in the question brackets that the bytes contradict: EF12 "only activates the new list" (it does
  not activate per tick at all), EF13 (a) "at once", (b) "caller-given count", (c) "caller-given seconds".

Commits: `57d4db01` (EF11/EF12 spec), `58efba2e` (EF13, answered moves, QUESTIONS.md), plus this report.
Not resolved: the identity of the `+0xC78` object (created from GZCOM ids `0x5CF9459A`/`0x1CF94583`) and of the
`+0x21A` "emission suppressed" setter `0x005B2230` (no direct refs).
