# flammability: answered questions (archive)
<!-- Archive for [flammability.md](../../flammability.md). Lanes do not read this file. Open questions are in ../flammability.md. -->

## From the land value and flammability shadows (filed by the coordinator from the lane's report, 2026-10-14)

About `docs/specs/landvalue.md` and `docs/specs/flammability.md`. Ini: `landvalue_shadow = 1`,
`flammability_shadow = 1` (each needs the ORIGINAL simulator: do not enable `landvalue` / `flammability`). The land
value shadow compares on `LandValueMapChanged` (`0x87CC75B3`, end of state 8); the flammability shadow on
`SimCycleComplete` (`0x08F415FE`), with a mirror fed the original's six messages. The shadow's current answer is in
brackets.

<!-- from: From the land value and flammability shadows (2026-10-14) -->
- ~~**FL1:** is 2 × tractsPerDay the right bound for tracts left unvisited at `SimCycleComplete`? It assumes the
  message is delivered queued, after the day agent. [2 × tractsPerDay]~~ **ANSWERED 2026-10-14** from the exe
  (framework `AddToTick` `0x00879BB9` / `OnTick` `0x0087A74B`; tick priorities 5,000,000 for the message server and
  −2,000,000 for the simulator; message server tick `0x0092E040`) and written into flammability.md §9. **Right for
  shipped tuning.** The message is delivered at the next frame's message-server tick, which comes before the
  simulator's. Up to then the day agent visits the rest of the wrap day (≤ `tractsPerDay`) plus one more day only at
  speed 2, where 50 ms per day fits in a 66 ms tick. A mod with a shorter day at speed 2 raises the bound.

<!-- from: From the land value and flammability shadows (2026-10-14) -->
- ~~**FL2:** the multiplier g is 0 until the first season or ordinance message; the mirror takes the original's g at
  the baseline. Right? [taken at the baseline]~~ **ANSWERED 2026-10-14** from the exe and written into
  flammability.md §6.4. **Right.** Only the constructor, `Read` and the SeasonChange/OrdinanceUpdated branch of
  `DoMessage` (`0x006AFB87`) write g.

<!-- from: From the land value and flammability shadows (2026-10-14) -->
- ~~**FL3:** a tract-size difference between the original and ours is reported as `INCOMPLETE`. [INCOMPLETE]~~
  **ANSWERED 2026-10-14**: no original behaviour; the implementer's choice stands. The original creates its grids
  with tract size 1 and never changes it, so a difference needs a save the original did not write (flammability.md §2).

<!-- from: From the land value and flammability shadows (2026-10-14) -->
- ~~**FL4:** the message server's delivery order to the original's target and to the mirror is unspecified; both read
  the same game state. Is that enough? [assumed so]~~ **ANSWERED 2026-10-14** and written into flammability.md §5.
  **Yes, and the order is not unspecified**: one send delivers to the subscribers in subscription order (foundations
  §5.1), so the original, which subscribes in `Init`, gets each message just before a later-subscribed mirror. The
  handlers read nothing that the subscribers in between write *[inferred]*.

<!-- from: From the land value and flammability shadows (2026-10-14) -->
- ~~**FL5 — first in-game shadow (run 82, coordinator):** 64 of 65 compared cycles exact; in cycle 66 (2033-03-15)
  a 2×2 block of adjusted tracts, (7..8, 53..54), is 81 in the original and 65 in ours (max |d| 21; raw grid
  identical). One lot's flammability contribution changed in a way the mirror missed or counted differently: which
  event sets an adjusted tract outside the day visit (a lot condition or occupant change mid-cycle)?~~ **ANSWERED
  2026-10-14** with FL6, from the exe (`DoDailySimulation` `0x006AF8D0`), and written into flammability.md §6.5, with a
  worked example. **No event outside the day visit: it is the visit's no-water factor.** 81 = roundHE(65 × 1.25),
  1.25 being the shipped no-water multiplier. The original visited those tracts while plumbing slot 11 said "not
  watered". The plumbing pass published a watered state for them later, so a mirror that evaluates the watered state
  at comparison time misses the factor.

<!-- from: From the land value and flammability shadows (2026-10-14) -->
- ~~**FL6 — second occurrence (run 83, coordinator):** 69 of 70 compared cycles exact; in cycle 15 (2027-08-11)
  595 adjusted tracts differ (raw grid identical; max |d| 21), starting with the column (0, 42..46): 100/80,
  106/85 original/ours, the city average 43 against 40..41. The original is about 1.25 times ours, the same ratio
  as FL5's 81/65, so a ×1.25 factor reached a whole region mid-cycle. Which event applies it (a fire, a disaster or
  an ordinance), and does it go through the day visit?~~ **ANSWERED 2026-10-14** with FL5 (flammability.md §6.5).
  **None of those: a plumbing pass that changed the watered state of a region mid-cycle, and yes, through the day
  visit** (100 = 80 × 1.25, 106 = roundHE(85 × 1.25)). Fires and disasters only read the grid, and g is never 1.25 with
  shipped tuning. The mirror must accept either watered state for tracts whose state changed during the cycle. A
  change of the plumbing simulator's `GetWateredGrid()` pointer marks each pass.

<!-- from: From applying the residential, budget, demand, land value and flammability answers (2026-10-14) -->
- ~~**FL7:** FL5/FL6: exactness assumes the plumbing pass-end message (posted in state 8) is delivered before the next
  pass swaps its grid (end of state 6). Two passes between polls would bring the pointer back unseen. [assumed; polled
  at every forwarded message and at the cycle end]~~
  **ANSWERED 2026-10-14** from the original's pass structure (plumbing §5.1 abort, §6 restart rule and pass length,
  state 6 swap and state 8 post; simulator §5.1 idle rounds; flammability §9 delivery) and written into flammability
  §6.5. Not always. A pass aborted in state 7 by a water deal status message has swapped but posts nothing, and a pass
  that ends with the recompute flag set restarts at once and can reach its next swap in the same frame, before its
  pass-end message is delivered. Either way the pointer can come back unseen. *[choice]* (recommended): compare the
  published grid's watered bits at every poll instead of the pointer.

<!-- from: From applying the residential, budget, demand, land value and flammability answers (2026-10-14) -->
- ~~**FL8:** FL5/FL6: a pass first seen by the comparison's own poll, after the wrap day's early visits. [those tracts
  are accepted by the existing previous-value rule, at most 2 × tractsPerDay "unvisited"; marks are not carried into
  the next cycle]~~
  **ANSWERED 2026-10-14** from the original's timing (flammability §6.5 and §9) and written into flammability §6.5. The
  original visits the new cycle's first tracts (at most 2 × `tractsPerDay`) under the same pre-pass state as the old
  cycle's last ones, and they keep those values for a whole cycle. The bracket is right for the comparison in hand, but
  with marks dropped at the boundary those tracts mismatch at the next comparison. *[choice]* Carry the pass's marks for
  the tracts that the new cycle has already visited.

<!-- from: From applying the residential, budget, demand, land value and flammability answers (2026-10-14) -->
- ~~**FL9:** FL5/FL6 with FreeCity4's own plumbing: detection relies on its published grid alternating too, which it
  does today. [pointer detection only]~~
  **ANSWERED 2026-10-14**: no original behaviour; the implementer's choice stands while FreeCity4's plumbing keeps
  alternating. Note that plumbing §12.2 allows a replacement to keep one fixed published grid, which would defeat
  pointer detection. The content comparison recommended for FL7 (flammability §6.5) does not depend on alternation.

## From applying the REH, flora, residential and flammability answers (filed by the coordinator from the `apply9d` lane's report, 2026-10-14)

apply9d applied AQ1–AQ10, RG24 and FL7–FL9 (merged `ccfe2b4`). Every in-game run stayed clean and exact:
flammability 18/18 under turbo and 117/117 at normal speed, residential 384/384 and flora 25/25. Under turbo the
flammability cycle-end message arrived up to 17 days after the wrap day.

<!-- from: From applying the REH, flora, residential and flammability answers (2026-10-14) -->
- ~~**FL10:** FL8: the tracts a new cycle visits come from a `rand()` draw. Which marks carry into the next cycle?
  [every tract changed after the wrap day, a superset]~~ **ANSWERED 2026-10-15 (re-answers10):** glue; the bracket (a superset) is safe and accepted. The exact set can be read from the original: the iterator at flammability `+0x30` keeps index `+0x24`, current `+0x28` and step `+0x2C` (`0x0049CB10`), so the new cycle has visited the `index` positions `(current − j × step) mod count`. The marks to carry are the tracts in that set that changed at a poll from the wrap day's up to the comparison's. flammability §6.5.

<!-- from: From applying the REH, flora, residential and flammability answers (2026-10-14) -->
- ~~**FL11:** FL8: the day count starts when the shadow is created (the first event, before the original's first
  day) and uses our own tractsPerDay. It is checked only against the number of cycle ends delivered, so an offset of
  whole cycles is not detected. [once it is off, it stays off for that city and only the comparison's own poll
  carries]~~ **ANSWERED 2026-10-15 (re-answers10):** glue, accepted as a fallback. The day count drifts (it starts before the original's `Init` wrap and uses our `tractsPerDay`), while the original's iterator index (`+0x24`) is the new cycle's exact position at the comparison. Prefer reading it. flammability §6.5.

<!-- from: From applying the REH, flora, residential and flammability answers (2026-10-14) -->
- ~~**FL12:** FL1/FL8: what is the bound on unvisited tracts under the turbo clock? [max(2 × tractsPerDay, the
  new-cycle tracts the count gave at the previous comparison); in game the bound reached 7098 and the unvisited
  count stayed at most 155]~~ **ANSWERED 2026-10-15 (re-answers10):** **DIFFERS** in kind: the original has no fixed bound. The number of new-cycle tracts at the comparison is the remainder of the wrap day plus `tractsPerDay` per further SimNewDay in the same simulator tick, at most one day per round of the tick's loop, so it grows with the frame budget under a short day. It is exactly the iterator's index (`+0x24`). The bracket's heuristic is acceptable where the iterator cannot be read. flammability §6.5, §9.

<!-- from: From applying the REH, flora, residential and flammability answers (2026-10-14) -->
- ~~**FL13:** FL7: what polls when there are no day polls, for example when the game's own simulator delivers the days?
  [every forwarded message plus the cycle end. Polling still needs the host's `watered_grid` to be non-NULL, although
  content polls read only slot 11]~~ **ANSWERED 2026-10-15 (re-answers10):** glue, accepted. The original reads the watered state only through plumbing slot 11, never `GetWateredGrid`. Every message the shadow sees, including posted SimNewDays, arrives at the message-server tick, after all of the previous simulator tick's visits, and intermediate plumbing states between polls cannot be seen from outside the simulator. Polling at every forwarded message plus the cycle end is the finest grain without an agent. The non-null `watered_grid` requirement is a glue limitation: fall back to slot 11. flammability §6.5.
