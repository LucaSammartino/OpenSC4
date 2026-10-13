# Open spec questions

Questions from engine authors that the spec writers need to answer from the original. Answered questions move into
the relevant spec, and their entry here is deleted.

## From the zones core (filed by the coordinator from the lane's report, 2026-10-13; zones merged as `f56e52a`)

All were about `docs/specs/zones.md`. The lane implemented each from the prose and flagged where no conformance
vector pins the choice — so these are places where the core could be wrong and nothing would catch it.

~~**ZQ1 — §8.3 slot 10, `GetLotsSurroundingRect` walk order.**~~ **ANSWERED 2026-10-13** from the exe and written
into §8.3: row by row, ascending z, with interior rows visiting only their two side cells. The lane's row-by-row
inference was correct. Kept here struck through rather than deleted so the numbering of ZQ2–ZQ6 still matches the
lane's report.

**ZQ2 — §7.4 `CreateParcellizableRegion`, the parenthetical "(real mode or the cell is occupied per §8.4)".**
Code 7 of §8.4 is only reachable if codes 0/1/2/3 all pass first, so a cell holding a lot that is *also* unzoned, or under
a network, returns 2 or 3 — not 7. "Occupied per §8.4" is therefore not a clean predicate. The lane implemented it
literally as `can_create_lot(x, z, 1, 1, false, false) == 7`. No test exercises the highlight-mode branch of §7.4 at all
(see ZQ4).

**ZQ3 — §7.2.1 `DetermineLotSize` has no conformance test in §15.**
Tests 19–29 all state "the harness bypasses `DetermineLotSize`", and no other test covers it. It is the most
arithmetic-heavy function in the spec — the x87 chain
`ftol(r24(r24(float(k)/float(sx·sz)) × float((min+max)/2)))`, the `hash_set` iteration order, the developer tie-break on
`EstimateDesirability` then demand, and the stage walk. The lane implemented it from the prose but **cannot verify any of
it**. A test 30 (a scripted BDS + configurations + developer, expected width/depth/minDepth/maxDepth/block) would close
what the lane calls the largest correctness risk in its work.

**ZQ4 — no vector covers highlight mode.**
`HighlightParcels` (§7.1), virtual lots, virtually deleted cells, and the §7.13 draft fallbacks are all untested. The
draft fallbacks need the network tools' `OccupantExists(x, z, flag, &mask)`, which §8.8/§7.13 describe only for highlight
mode. Partly a headless limit (the highlight is a UI preview) and partly a missing spec test.

**ZQ5 — §7.3 `ExamineBoundaries` city-border behaviour is untested.**
Quirk 5 ("examines the region's own edge cells at the city border") and the rule "at the edge the 'outside' coordinate
stays on R's own edge" have no vector: test 13's region is at x 10..12, z 10..12, well inside a 64×64 city. The
implementation follows the prose but nothing pins it.

**ZQ6 — §7.5 step 1 table, three of six cells unpinned.**
`zn15_30`/`zn15_31` pin the x-line `_` arm (Q=3) and the `0|2` arm in its `D = 3` branch, and the z-line `_` and `2`
arms. Not pinned: the x-line `1` arm, the `D != 3` branch of the x-line `0|2` arm, and the z-line `0` and `1` arms.
Documented in `engine/conformance/README.md` rather than left implicit.

## From the residential core (filed by the coordinator from the lane's report, 2026-10-13; branch `residential-core`, tip `cc6e25d`, NOT merged)

All were about `docs/specs/residential.md`. The lane's code compiles clean and its own geometry/tuning tests pass, but
`simulator.rs` has zero tests of its own and no conformance vectors exist, so these questions are unpinned choices in
unexercised code. Preserved here from the lane's branch by hand — the branch's own edit to this file was written
against the older `(No open questions.)` version and cannot be merged as-is.

~~**RS1 — is the `0.01` in the percentage property getters the float or the double constant? (load-bearing)**~~
**ANSWERED 2026-10-13** from the exe: **float32 `0.01f`** (`0x00A823A4`) at every site, in the getter `0x005FDAC0`,
§5.6 step 4, slots 30/31, the §7.2 tuning and the save reader. Protection was right. Residential §15 test 8 was
wrong (EQ 50 → 39.999996, HQ 60 → 47.999996) and is corrected; test 10 holds, but only because each product is
rounded to float32 before the truncation. Written into residential.md §5.6, §7.1, §15, REH §4 and foundations §18.3.
Kept struck through so RS2–RS6 keep their numbers.

**RS2 — §5.5 step 5: is the `GetSimDate` month read per tract, or latched at pass start?**
The tract is aged only when `month = 12`. The core reads it per tract (the literal reading). The two differ only if a
month boundary falls inside one state-4 pass — impossible in the original (one state per day) but reachable if a host
runs a whole pass in one call across a date change.

**RS3 — §5.5 step 1: the pollution average's accumulator width.**
REH §4.7 says "u32 division". The core sums the `S × S` `GetPollutionValue` results (each 16 bits) in a wrapping
`u32`. With `S = 4` the maximum is 1,048,560, so no wrap; a large `S` would. Confirm the original's accumulator is
32-bit and wraps rather than being wider.

**RS4 — §5.6 step 4 / REH §4.8 `GetGlobalEQ`/`GetGlobalHQ`: what is the per-tract traversal order?**
Float addition is not associative, so the accumulated `sum` depends on it. The core walks the grid's tracts x outer,
z inner (the same order as `ProcessTwoGridsWithCallback`, simgrid §7.3). Confirm, or state that the original's order
is the raw-data order of the grid (the same thing for a contiguous row-major grid).

**RS5 — §5.3 step 2: does the distance sweep's running `d` widen past a byte?**
`d` starts at 255 and computes `d := min(d + 1, v)`. If `d` is a byte accumulator, `255 + 1` wraps to 0 and the whole
field collapses to 0 on the first sweep. The core widens `d` to `i32` and clamps the stored value to `0..=255`, which
reproduces the Manhattan distance capped at 255 that §5.3 states and that §15 tests 1–3 require. Confirm the original
widens too (or that its `min` is evaluated in a wider type) — the two differ for every grid with no seeded tract.

**RS6 — §6.2 `GetAverageEQGrid`/`GetAverageHQGrid`: are `min`/`max` updated for unpopulated tracts?**
REH §4.8 says yes ("Each is updated with every tract, populated or not") and the core follows it. With an empty city
that gives `min = max = 0.0` rather than the initial `255.0` / `0.0`, which a caller may observe. residential.md §6.2
only says "track the stored values", so confirm REH is normative here.

## From the budget shadow differential (filed by the coordinator, 2026-10-13; merged as `6ca4b65`)

About `docs/specs/budget.md`. Run 39 matched on every value for five months, so these are about the edges of what the
shadow proves, not about a known mismatch. The lane's third question (does the original budget's QI(0x452294AA)
pointer equal its month agent?) was answered by run 39 itself: the agent `139BE6F0` was matched through its
`cIGZMessageTarget2` interface, 4 bytes into the budget object `139BE6EC`, not by the name fallback.

**BQ1: can the original's own lot list differ from `GetLotsInCellRect(…, false)` over the whole city?**
The shadow scans lots through `GetLotsInCellRect`, and the original settlement walks its own lot list. Lots that are
mid-transition, preview/placement lots, or lots outside the city bounds could be in one set and not the other. Run 39
counted 413–417 lots and matched exactly, so no difference showed up on Spandau. Answer from the settlement's lot
iteration: which container it walks and which lots it filters out.

**BQ2: at the January 1 settlement, what do GetYTDIncome/GetYTDExpenses return?**
The shadow reads the month's income and expense as the change in the year-to-date values across the settlement. On
January 1 the year-to-date is reset, so that month's income/expense is reported as `year end: … not observable`
(run 39, 2002-01-01). Does the reset happen before or after that month's amounts are added? If after, the January
values are readable directly as the new YTD, and the shadow can compare that month too.

## From the demand shadow differential (filed by the coordinator, 2026-10-13; runs 42/43)

About `docs/specs/demand.md`. Runs 42/43 matched demand bit-for-bit except for one month (2001-10-01). In that month
the tax modifier of 11 of the 12 types was 4 ulp off.

~~**DQ1 — §8.1: operation order of the tax ratio `maxRate · 0.1 / neutral`.**~~ **ANSWERED 2026-10-13** from the exe
and written into §8.1, with a bit-level worked example. `UpdateTaxAndEconomyModifiers` (`0x006A6070`, unrolled into
12 copies) computes `recip := r24(1.0f / neutral)` once, in the first copy, and stores it as float32. Every type then
computes `r24(r24(maxRate · 0.1f) · recip)`. `0.1f` is a float32 literal (`0x00A8C950`). This is the order the
implementer found empirically. It reproduces `0x3DAA8124` (2001-10-01), `0xB9794400` (I-R) and `0x3DAA8328`
(2001-09-09). Also answered on the way: the link factor is `r24(float(percent) · 0.01f)` with a float32 `0.01f`
(§4.1).

**DQ2 — §15 Q5: can the CO/I economy bonuses (`this+0xA0`, `this+0xA4`) be read or written through any interface?**
In the original these are float32 fields, added with `FADD dword` to the economy modifier of CO$$/CO$$$
(`0x006A654F`, `0x006A65FD`) and of the four I types (`+0xA4`, for example `0x006A68AF`). In the exported decomp of
`cSC4DemandSimulator`, the only other code that touches them is the save write, the save read, and the
Init/Shutdown zeroing. That was not checked against the vtables. Still open: does any `cISC4DemandSimulator` slot,
or any other class, read or write `+0xA0`/`+0xA4`, for example through a getter that returns a pointer? A vtable
scan of the demand simulator's interfaces would settle whether these fields are dead.
