| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-advisors4: GZCOM glue for the advisor system (enable=advisors), scoping round | glue-advisors4 / .claude/worktrees/glue-advisors4 | blocked: scoped, no plugin code; questions AD1-AD3 filed | answers to AD1-AD3, then a multi-round glue program (stages below) |

## Report (run4, one round, 2026-10-21)

**Checked first (as the task asked).** The advisor core is on main: `engine/core/src/advisors/` (9 files, 278152 bytes,
not the 71334 the task quoted; tests.rs alone is 68637) and a C ABI `engine/capi/src/advisors.rs` (2572 lines, 62
exports, host struct `fc4_advisor_host` with 64 fields). The core is one system: advisors, advices (base, reward,
city-situation, neighbour deal) and subjects are `Rc`s that share the system's state, so no advice class can be hosted
without the system. The plugin has **no** advisor glue: no `cFC4Advisor*` class, no `advisors` key in FC4Config (only
`development_advisor`, a different feature), nothing in FC4Subsystems.cpp. Enable key: none exists; `advisors` would be
new.

**Why no code this round.** The spec rules out a smaller coherent family: section 14.2 says the subclasses depend on
`cSC4Advisor`'s class vtable and members ("replace the whole hierarchy or nothing"), and a replacement system must also
replace every advisor, advice class and Lua extension, or write the private global. So the smallest owner family that
can run in the game is all five owner classes (cSC4AdvisorSystem, cSC4Advisor + 7 subclasses, cSC4Advice,
cSC4RewardAdvice, cSC4CitySituationAdvice) plus the neighbour-deal advice and the subject, behind four interfaces
(section 12.1-12.4: 29 + 21 + 31 + 14 slots) and a host of 64 callbacks. Those callbacks cover the Lua side (advisors/advices
tables, compile/call trigger, conditions, tags + gc, register_api, detokenizer) on the app's `cISCLua` (78 slots,
section 12.5), the readers of the subsystems in section 13's table (`value`, `list`, history, ordinances, civic conditional status, occupants),
the message server, cheats, 3D view effects and save refs. That cannot be built and run clean in the game through 2032
inside one 150-call round; a partly built class would not be enabled and would prove nothing. Three spec gaps also
block an exact replacement: AD1 (slots 23-28, the advisor window: names only, no bodies), AD2 (whether code that stays
original reads the private global `0x00B207C0`), AD3 (who reads and changes the original `std::map`s that slots 8 / 10-11
return).

**Coverage (measured, not estimated).** advisors: specified yes, implemented yes (core + capi), shadow -, verified -,
replaced **absent**. Owner classes covered by plugin glue: none. No map path proposal: no class is replaced.

**Gate / game.** Docs-only branch (this file + `docs/specs/questions/advisors.md`): no build or game run needed, none
made. REA preflight: n/a (clean-room engine lane, no RE tools used).

**Planned stages (for the next rounds, each one game-tested):**
1. `FC4AdvisorsHost` C++: the 64 callbacks on the game interfaces, starting with Lua on the app's `cISCLua`
   (tutorial glue `cFC4Tutorial.cpp` already takes it) and `value`/`list` readers; ctest seam against the 25 vectors.
2. GZCOM classes for system / advisor / advice / subject (four vtables, game-layout maps via FC4GameMap.h),
   `enable = advisors` (default off), Read before Init kept free of city state.
3. Game run `sim_turbo = 1` through 2032 with a trace of PostCityInit, ticker, Simulate per advisor, triggers fired,
   popups posted; then the normal-speed check.
