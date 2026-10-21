| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-citysit4: GZCOM replacement enable=citysit over the city-situation core | glue-citysit4 / .claude/worktrees/glue-citysit4 | ready: replacement game-clean through 2032 with a trace; gate PASS; CS12, CS13 filed | coordinator: add-replace citysit to the regress union; answers to CS12/CS13 then wire indicator view and sitmgr |

## Report (run4, one round, 2026-10-21)

**Starting point.** `enable = citysit` already existed (glue17: `engine/plugin/cFC4CitySituation.*`, the spec §8
"minimal replacement" over `fc4_citysit`, registered in `kCitySitClasses` through the automata director's CLSID
`0x0BB14381`). Several host callbacks the comment called wired were stubs, and one leaked references.

**Changed (engine/plugin/cFC4CitySituation.{h,cpp}, d5f9fc83):**
- Animation tick (§3, §4.1 step 2): `AddTickableObject(+4, 0xFF)` / `RemoveTickableObject` on the framework service
  `0x86AD10EC` (IID `0x86AD10ED`, slots 5/6 as lot-developer.md §6.7.8 / FC4LotDevInterfaces.h).
- Preferences reward list (§4.1 step 8, §4.5 step 2, slot 33): the 64 u32 at preferences `+0x1334`
  (`cISC4App` slot 38, SDK `tutorialPreferences2.field1_0x400`, static_asserted), written and saved
  (`cISC4App` slot 17) only on a mission success or slot 33.
- Bug fix: `IsAutomaton` leaked the `QueryInterface` reference of every automaton inserted (occupant-inserted path).
- Thread safety: every core call runs under a recursive mutex (the core is RefCell; slots 19/27/28 come from the
  simulation thread, the tick from the main thread).
- Trace: first call of each §8 slot, first tick, first 16 messages, and at Shutdown
  `citysit trace: ticks=N messages=N calls: sK=N`.

**Game (Konradshohe, turbo to 2032-01-01):**
- `citysit4-replace-1` (`enable = citysit`, three scripted saves): **clean**, ref_warnings=0. Read before Init
  (record 0 successes, indicators 1), Init tick=1, 3 Writes, Shutdown, destroy; trace `ticks=139 messages=404 calls:
  s3=1 s4=1 s6=10`. The shadow reports itself inactive under the replacement (by design, unchanged).
- `citysit4-shadow-1` (`citysit_shadow = 1` only): **clean**, ref_warnings=0, shadow 73/73 lines exact, 1095 values,
  0 mismatched (old shadow behaviour preserved).

**Gate:** baseline and final `qbuild all` PASS: cargo 3638/0, clippy 0, ctest 62/62. No C ABI change, no FC4_ABI_VERSION bump.

**Questions filed** (docs/specs/questions/city-situation.md; QUESTIONS.md index not regenerated: tools/ is off limits):
- CS12: the dispatch vehicle view's `ShowAllIndicators` / `TrackAutomaton` slots (indicator view still a stub).
- CS13: how `sitmgr` is published (registry entries vs Lua table) and the `cIGZCommandParameterSet` slot table;
  `sc4game.sitmgr` is absent while the replacement is on.

**Still stubs (by spec §0/§8 scope or the questions):** cheat `csitrigger`, indicator view, sitmgr table, advisor 29
advice list, mission gameplay (targets, pursuit, popup, player drive).

**Next:** coordinator union-regresses add-replace citysit; after CS12/CS13 answers wire the view and `sitmgr`.
