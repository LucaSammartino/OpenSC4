| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-effects-fix4 | .claude/worktrees/core-effects-fix4 | in progress | fix EF12/EF13 DIFFERS in engine/core/src/effects |

## Handoff
- Goal: correct EF11-EF13 DIFFERS in effects Rust core (reload-only activation, selectdraw rebuild rule, emitter contract 7.1.1).
- Done: EF12 manager rebuild test + reload-only effects tick (44f9689b); EF13 emitter rewrite + EF14 filed (c7fa1fdf).
- Gate after c7fa1fdf: qbuild all PASS (cargo 3958/0, clippy 0, ctest 64/64).
- Game: r1, r2 (enable=effects, effects_shadow=1) both no-load: log stops after plugin init, ref_warnings 0, no panic; other lanes clean at the same time. Outstanding: r3 = control without enable=effects.
- Next: wait build-runs/game-results/core-effects-fix4-r1.verdict (qwait without --fresh), grep log for 'effects shadow:'; then finish report + merge-queue json.
- Facts: EF11 already matched (dilation.rs untouched). services_ready (+0x1AD) defaults true [choice] because CAPI has no PostAppServicesInitComplete route; shutdown keeps it.
