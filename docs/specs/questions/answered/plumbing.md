# plumbing: answered questions (archive)
<!-- Archive for [plumbing.md](../../plumbing.md). Lanes do not read this file. Open questions are in ../plumbing.md. -->

<!-- from: From the plumbing grid shadow (2026-10-14) -->
- ~~**PS1:** the order in which unvisited water connections are listed is inferred; the read-only host cannot find
  them, so the shadow reproduces it over its own visited flags. The original's order? [inferred]~~
  **ANSWERED 2026-10-14** from the exe (call `0x006DD7F8` with type 5 into `FindUnvisitedConnection` `0x006CF720`,
  PW1): **append order.** The list is walked from its head and the first type-5 connection to the deal's city with
  the visited bit clear is returned. This is the order of `GetConnectionsOfTypeToCity(5, city)` without the visited
  ones, so a shadow that walks that list over its own visited flags matches. Written into plumbing.md §6.2.

<!-- from: From the plumbing grid shadow (2026-10-14) -->
- ~~**PS2:** pipe condition is a distress fraction that drops to 0 at funding of 50% or more. Intended? [as stated]~~
  **ANSWERED 2026-10-14** from the exe (`ApplyPipeFunding` `0x006DD110`): **the spec's formula is right, the summary
  is not.** The curve is a monthly change: below 50 % cond grows (0.01 a month at 0 %), at 50 % it holds, and above
  50 % it falls gradually (0.02 a month at 100 %). It is 0 at 50 % or more only when it was 0 already, as after a
  load. New quirk: a step above 1 sets 1 and skips the "no pipes → 0" test. Written into plumbing.md §7.1 with a
  400-pipe example.

<!-- from: From the plumbing grid shadow (2026-10-14) -->
- ~~**PS3:** the original's pipe condition is not saved and not observable; the shadow starts its own at 0, so the
  first month after a load assumes the original also restarts at 0. Right?~~ **ANSWERED 2026-10-14** from the exe
  (cond `+0x180`: written by the constructor `0x006DC61C` and by `ApplyPipeFunding` only): **right, the bracket
  matches.** `Read`, `Init` and `Shutdown` never touch it, so the original restarts at 0 too. The first month
  then repairs the saved distressed pipes down to the new target. The ageing flag is not saved either, so a save
  taken between a month start and its state 8 loses that month's ageing. Written into plumbing.md §7.1 with an
  example.

<!-- from: From the plumbing grid shadow (2026-10-14) -->
- ~~**PS4 — §5.3:** demand is a running sum the original builds as things are added; the shadow takes it from the
  original's record instead of recomputing it. Acceptable? [taken from the record]~~ **ANSWERED 2026-10-14** from the
  exe (demand is written only by `DoBuildingChanged` and `Read`) and the shipped saves: **yes, the bracket matches.**
  The original never recomputes it either. Take it at the comparison, not at the pass start (PS8). In the eight
  base-game saves with buildings the saved demand equals a recount of the saved buildings exactly (Konradshohe 23845),
  so here, unlike power (PW8), nothing is inherited. Written into plumbing.md §5.3.

<!-- from: From the plumbing grid shadow (2026-10-14) -->
- ~~**PS5:** the monthly funding comes from department `0xAA4014B4`, a choice the spec does not state. Which
  department? [`0xAA4014B4`]~~ **ANSWERED 2026-10-14** from the exe (`DoMonthlySimulation` `0x006DF543` reads the
  member `+0xA4`; `Init` `0x006DF238`–`0x006DF258` creates it as `0xAA4014B4`): **the bracket matches.** The funding is
  the simulator's own held department. That is `0xAA4014B4` on a new city and the saved ref after a load, and the
  saved ref is `0xAA4014B4` in every save checked. Production (§7.2) and ageing (§7.3) read the same member.
  Written into plumbing.md §7.1.

<!-- from: From the plumbing grid shadow (2026-10-14) -->
- ~~**PS6:** a pass's start is seen only when state 2 changes a probe cell; if the next working grid already matches
  what state 2 would write, the start is missed and the pass is marked not exact. [as stated]~~ **ANSWERED
  2026-10-14** from the exe (`DoIdle` state 0 `0x006DD3D9`): the probe is shadow glue, so the implementer's choice
  stands, but **the original has a better marker.** passCounter (record field 7, object `+0x9C`) is incremented once
  by state 0 and written otherwise only by the constructor and `Read`. A record read whose passCounter is one higher
  than the previous read means that call ran state 0, with no probe cell needed. *[choice]*: detect starts by
  passCounter. Written into plumbing.md §6.1.

<!-- from: From the plumbing grid shadow (2026-10-14) -->
- ~~**PS7:** when a state 8 may have run unobserved, ageing is logged as `unknown` and producer condition is not
  compared. [as stated]~~ **ANSWERED 2026-10-14** from the exe (ageing flag: set `0x006DF551`, tested `0x006DEDBD`,
  cleared `0x006DEDD9`): shadow glue, **no original behaviour; the implementer's choice stands.** Facts that can narrow
  "unknown": exactly the first state 8 after a month start ages, and an ageing changes each producer's
  `0xC910B4CF` whenever its increment is not 0, which the shadow can read. Written into plumbing.md §6.7.
- Not compared: the pipes' watered states (only reachable through a non-virtual call), redisplay, posts, history,
  line-item expenses, lot conditions and deal flags; the original's pipe condition and pass state (not saved, not
  exposed). New bursts are range-checked only (random draw).

<!-- from: From the plumbing grid shadow (2026-10-14) -->
- ~~**PS8 — first in-game shadow (run 81, coordinator):** of 95 replayed passes, 4 differ only in `demand`, ours
  lower by 8 to 51 (e.g. 25315/25287 in 2026-03, 35967/35916 in 2030-11); ours equals the pass's consumed total.
  The original's demand is a running sum kept as things are added and removed. Which additions does it count that
  the pass's consumption does not (lots whose water use changed since insertion, removals not subtracted)? Compare PW8.~~
  **ANSWERED 2026-10-14** from the exe (demand `+0x130` is written only by `DoBuildingChanged` `0x006DBC64`–`0x006DBCA0`
  and `Read`) and the run-81 log: **the occupant messages delivered during the pass.** They are posted, and a pass
  runs one state per call, so the original's demand after state 8 already holds them. The shadow's carries the start
  value. The four passes fit same-cell replacements, which leave the flag map unchanged. Unlike PW8, nothing is
  inherited: the next pass agrees each time. *[choice]*: compare demand with the record read at the comparison.
  Written into plumbing.md §5.3 with a table.
