| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-effects4: effects core, re-answers34 integration (dilation table, particle self-stop, DoEffectsTick activation) | `core-effects4` / (engine lane) | in progress | see Handoff |

## Handoff

- Goal: bring engine/core/src/effects up to docs/specs/effects.md after re-answers34 (EF1, EF4, EF5): §6 dilation
  table (test 17), §7.1 particle emitter self-stop model (test 16), §6.1 DoEffectsTick activation; tests; questions.
- Existing core (cores19, train6) already covers manager, directory (resource/parser of the binary record),
  brush, game (scrubber), shipped tables. Coverage map row "effects" has core "-": propose `core/src/effects`.
- Done: (none yet)
- Next: baseline `qbuild all`, then add effects/dilation.rs and effects/particles.rs, manager `effects_tick`.
- Facts: no game runs (core lane, game=false).
