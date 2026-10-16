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

~~**ZQ2 — §7.4 `CreateParcellizableRegion`, the parenthetical "(real mode or the cell is occupied per §8.4)".**~~
**ANSWERED 2026-10-13** from the exe and written into zones.md §7.4, with a highlight-mode table. The predicate is the
zone developer's per-cell `IsCellOccupied(x, z)` (`0x0072C050`), called only in highlight mode:
- real mode: a real lot is on the cell;
- highlight mode: a real lot that is not virtually deleted, or a virtual lot.

That is §8.4's code-7 cell condition without codes 0–3, so `can_create_lot(…) == 7` is not equivalent. The lot then
examined is always the real one (`GetLot(x, z, false)`).

~~**ZQ3 — §7.2.1 `DetermineLotSize` has no conformance test in §15.**~~ **ANSWERED 2026-10-13** from the exe:
§7.2.1 was re-checked against the bytes and zones.md §15 now has **test 30 (a–g)**, with a scripted BDS,
configurations and developers and every expected width, depth, minDepth, maxDepth and block. Three corrections were
written into §7.2.1:
- the configuration filter also requires growth stage ≤ stage;
- the configuration `hash_set` is keyed by **pointer**, so the original's order is heap-address order. It is
  observable only through a score tie. *[choice]* FreeCity4 uses ascending configuration ID, pinned by test 30e;
- the x87 chain equals the exact floor for k ≤ 64 (shipped k = 16) but not above (test 30g).

~~**ZQ4 — no vector covers highlight mode.**
`HighlightParcels` (§7.1), virtual lots, virtually deleted cells, and the §7.13 draft fallbacks are all untested. The
draft fallbacks need the network tools' `OccupantExists(x, z, flag, &mask)`, which §8.8/§7.13 describe only for highlight
mode. Partly a headless limit (the highlight is a UI preview) and partly a missing spec test.~~ **ANSWERED 2026-10-14**
from the exe. zones.md §7.1 now covers HighlightParcels, ClearHighlight and a table of everything highlight mode
changes. §7.2 step 6 gives the `*out` lifecycle, including the unchecked point read (a *[choice]*). §8.8 gives the
`OccupantExists` contract. Two corrections: §7.12's parallel test does have a draft branch, and `ExistsNetworkOfType`
skips the dirt-road tool when the street bit is set. §15 **test 33 (a–e)** pins the preview of tests 19 and 23, a
virtually deleted lot, the point filter and the draft queries.

~~**ZQ5 — §7.3 `ExamineBoundaries` city-border behaviour is untested.**~~ **ANSWERED 2026-10-13** from the exe:
the clamp was checked on the bytes (`0x00731C8D`) and zones.md §15 now has **test 31**, with both low edges, both
high edges and the placeStreets-off case. §7.3 now states the consequence: at the border the examined cell is R's
own zoned, empty cell. It scores like an empty outside cell (+1 per cell with `placeStreets`), so a border side is
neither skipped nor scored as an obstacle.

~~**ZQ6 — §7.5 step 1 table, three of six cells unpinned.**~~ **ANSWERED 2026-10-13** from the exe. The table was
checked on the bytes and is correct as written: Q is `+0xCC`, D is `+0xD0`, and the jump table is at `0x0072EC44`.
zones.md §15 now has **test 32**, which gives the street positions for all eight perpendicular (Q, D) pairs on a
16 × 16 rect (n = 6), plus the edge rule. That pins every arm, including the four listed here.

From the ZQ3/ZQ5/ZQ6 follow-up lane (filed by the coordinator, 2026-10-14; merged `8ee3253`; vectors
`zn15_32`–`zn15_35`). About zones.md §7.2.1 `DetermineLotSize`; the core's current answer is in brackets.

~~**ZQ7 — step 2: a missing candidate developer.** The original does not null-check the candidate developers. What
happens when the BDS lacks one? [skipped: the first existing candidate becomes best]~~ **ANSWERED 2026-10-14** from
the exe: the original reads each result's vtable at once (`0x00732C72`), so a missing developer crashes it. The
bracket is kept as a *[choice]*, with no candidates at all taking the no-best path (zones.md §7.2.1 step 2).

~~**ZQ8 — step 5: signedness of `GetGrowthStage() ≤ stage`.** The growth stage is a u8; the stage is an int32. Signed
or unsigned compare? [zero-extended, compared signed; the two differ only for a negative `maxStage`]~~ **ANSWERED
2026-10-14** from the exe: the bracket matches (`movzx`, then `setle` at `0x0072A0B7`–`0x0072A0BD`). zones.md §7.2.1
step 5 now says so, with the negative-stage example.

~~**ZQ9 — step 6: the initial best score, and wrap of `|v − bestDemand|`.** [the first scored configuration is always
taken; the difference is computed without a 32-bit wrap]~~ **ANSWERED 2026-10-14** from the exe: the bracket differs
at the extremes. The best score starts at `0x7FFFFFFF` (`0x00732F77`), and the take test is signed `<`
(`0x007331B1`) on an int32 difference that wraps, with an abs that leaves −2³¹ alone. So a score of `0x7FFFFFFF`
is never taken, and a wrapped −2³¹ always wins. The two agree for every |difference| < 2³¹ − 1. The int32 rule is
recommended (zones.md §7.2.1 step 6, with an example).

~~**ZQ10 — step 7: signedness of `minDepth > 2`.** [unsigned, like the widening compares; both agree for real depths]~~
**ANSWERED 2026-10-14** from the exe: the bracket matches (`jbe` at `0x00733342`). zones.md §7.2.1 step 7 also notes
that `width mod sx` is an unsigned `div` (`0x007332AA`), so `sx = 0` crashes the original. That case is now a
*[choice]*: such a configuration fails the size filter.

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

~~**RS2 — §5.5 step 5: is the `GetSimDate` month read per tract, or latched at pass start?**~~ **ANSWERED
2026-10-13** from the exe and written into residential.md §5.5 step 5: **per tract.** `DoPopulationChange` calls
`GetSimDate` itself (`0x006FAA95`) for every tract with total ≠ 0. Nothing is latched, so the core's literal reading
is right.

~~**RS3 — §5.5 step 1: the pollution average's accumulator width.**~~ **ANSWERED 2026-10-13** from the exe and
written into residential.md §5.5 step 1 and REH §4.7. The accumulator is a **32-bit unsigned dword that wraps**:
zero-extended 16-bit adds, then an unsigned `div` by the 32-bit product S·S (helper `0x006F7FF0`). The core's
wrapping u32 is right.

~~**RS4 — §5.6 step 4 / REH §4.8 `GetGlobalEQ`/`GetGlobalHQ`: what is the per-tract traversal order?**~~
**ANSWERED 2026-10-13** from the exe and written into REH §4.8, with a pointer from residential.md §5.7. The order is
**x outer, z inner**, both ascending from 0, over the R$ EQ (HQ) grid's tract counts. Every value is read through
`GetTractValue` (`0x006F7720`/`0x006F7870`), and no raw memory is read. This confirms the core.

~~**RS5 — §5.3 step 2: does the distance sweep's running `d` widen past a byte?**~~ **ANSWERED 2026-10-13** from the
exe and written into residential.md §5.3 step 2, with a worked example. **Yes.** `d` is a 32-bit register: it starts
at 255, `inc` takes it to 256 without wrapping, and the min uses a signed 32-bit compare (`0x005E4E80`). After the min
it is ≤ 255, so storing its low byte never truncates. The core's i32 plus clamp is equivalent; the clamp never acts.

~~**RS6 — §6.2 `GetAverageEQGrid`/`GetAverageHQGrid`: are `min`/`max` updated for unpopulated tracts?**~~
**ANSWERED 2026-10-13** from the exe and written into residential.md §6.2, with a worked example. **Yes, and REH §4.8 is
normative.** In `AverageGrids` (`0x006F70E0`) an unpopulated tract stores 0.0 and goes through the same strict min and
max compares, so an empty city returns `min = max = 0.0`.

## From the budget shadow differential (filed by the coordinator, 2026-10-13; merged as `6ca4b65`)

About `docs/specs/budget.md`. Run 39 matched on every value for five months, so these are about the edges of what the
shadow proves, not about a known mismatch. The lane's third question (does the original budget's QI(0x452294AA)
pointer equal its month agent?) was answered by run 39 itself: the agent `139BE6F0` was matched through its
`cIGZMessageTarget2` interface, 4 bytes into the budget object `139BE6EC`, not by the name fallback.

~~**BQ1: can the original's own lot list differ from `GetLotsInCellRect(…, false)` over the whole city?**~~
**ANSWERED 2026-10-13** from the exe and written into budget.md §5.2. **No, on every known path.**
- `ComputeTax` walks the lot manager's master list through a safe iterator. Its only filters are: no building, not
  livable, and an untaxed cell.
- The cell map and the list change together in `CreateLot`/`DeleteLot`, and a load rebuilds one from the other.
  Highlight-mode lots are in neither set.
- Right after a load the two **orders** are identical too (first cell, x outer).

The only divergences are a direct `CreateLot` over an existing lot, or a null-list insertion point. Neither is
known to occur.

~~**BQ2: at the January 1 settlement, what do GetYTDIncome/GetYTDExpenses return?**~~ **ANSWERED 2026-10-13** from
the exe and written into budget.md §5.1 and §15 test 4. **Both return 0.** The settlement adds the month first, then
records history, and only then resets the year (`0x00692A4E`), so December's amounts are added and wiped in the same
call. The January 1 amounts are readable from history series `0x6A51178B`/`0xCA5BA3FB` (written before the reset),
and their net from the funds change.

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

~~**DQ2 — §15 Q5: can the CO/I economy bonuses (`this+0xA0`, `this+0xA4`) be read or written through any interface?**~~
**ANSWERED 2026-10-13** from the exe and written into demand.md §15 Q5 and §8.1. **No.** None of the 24 slots of the
three vtables touches them, nor does any other class. The only code that does is the constructor, the zeroing in
`Init`, `Write`, `Read` and the six `FADD`s. The save record is the only way in or out, so the fields are dead in
play and 0 in every new city.

## From the traffic network core (filed by the coordinator from the lane's report, 2026-10-13; merged with the core)

About `docs/specs/traffic-network.md`. The core implements every section except §4.14 (needs the simulator's safe lot
iterator), §6 persistence (the record is in traffic-simulator.md §6) and the GZCOM glue. Where the spec left a choice
open, the core's current answer is in brackets; "[none]" means the spec must decide.

- ~~**TN1 — §4.7.1:** does the step-1 "stop" also skip the ferry step 6? [yes]~~ **ANSWERED 2026-10-14** from the exe
  and written into traffic-network.md §4.7.1 step 1 and §12, and traffic-simulator.md §4.1.3. **Yes, and more:** the
  stop is a return from DoLotActivated (`0x00723BAC` → `0x00724188`). It also skips the `DoConnectionsChanged`, the
  bucket registration, the `RandomUint32Uniform(100)` draw (`0x00723FBE`), the budget items and the destination
  bits. Also corrected while tracing: the destination bits are **stored** over the lot rectangle, not ORed. The
  "byte size" is a raw byte count: the property is a u8 array in every shipped switch (§4.7.2), so no shipped
  building stops.
- ~~**TN2 — §4.7.1:** does step 6 run for a building without property `0xE90E25A1`? [yes]~~ **ANSWERED 2026-10-14**
  from the exe and written into the §4.7.1 intro and step 6. **Yes:** a missing property jumps straight to step 6
  (`0x00723B7B` → `0x00723ED0`). Ferry activation and its `DoConnectionsChanged` run if the building has
  `0x0BD86FD3`. Otherwise there is no rebuild, because no switch was created. Step 7 (traffic-simulator.md
  §4.1.3) follows in both cases.
- ~~**TN3 — §6 vs §4.7.2 (contradiction):** §6 says the switch record saves the rules; §4.7.2 says the save does not,
  so DoPostCityInit re-reads them. Which is right? [none]~~ **ANSWERED 2026-10-13** from the exe and written into
  §4.7.2 and §6. **§6 is right:** `Write` saves the rules and `Read` reads them for versions above `0x1C`.
  DoPostCityInit still re-reads every switch's rules (`0x00720466`) and **appends** them, so a loaded switch holds
  its rules twice *[quirk]*. This has no effect on the rebuild and shows only in the next save. *[choice]* FreeCity4
  replaces the rules instead.
- ~~**TN4 — §4.7.2:** when DoPostCityInit re-reads the rules, are the flags and the "to" mask recomputed, and which
  rectangle is used to refresh the access point? [recomputed; the building's bounding cells]~~ **ANSWERED
  2026-10-14** from the exe and written into §4.7.2 (the TN3 re-read steps, plus "Access points after the re-read").
  The bracket is **partly right.**
  - The mask and flags are recomputed only when the lot at the NW corner exists: the mask from the re-read entries,
    the flags from the first n rules. Otherwise the mask is 0 and the flags are as `Read` left them.
  - The access point is refreshed only for a switch with flag 4 and no saved access point. It is then the **NW
    cell** of the building's bounding cells, with no occupant search. A saved point is kept, and with flag 4 clear
    "has access point" := 0.
- ~~**TN5 — §4.6:** are tunnel cells and cells without car info reached but not extended, or skipped entirely?
  [reached, not extended]~~ **ANSWERED 2026-10-14** from the exe (`UpdateIntersectionData` `0x0071BB1B`–
  `0x0071BC3A`) and written into §4.6. **The bracket is right: reached, not extended.** The intersection test runs
  first, so such a cell can set `best`, and then the search stops there. The order of the stop tests and the absence
  of a visited set are now stated.
- ~~**TN6 — §4.12:** is `GetNeighborPoolPropertyHolder` asked with the off-map cell or the mapped in-city cell?
  [off-map]~~ **ANSWERED 2026-10-14** from the exe (`UpdateSubnetworkInfo` `0x0071674F`–`0x00716755`) and written into
  §4.12. **The bracket is right: off-map.** The mapped cell is used only to find the car record and its `subnet[]`.
  The holder is asked with the connection's own (off-map) cell.
- ~~**TN7 — §11 test 3:** the test lists L1's ring in a different order from BoundaryIterator. Only the order in which
  IDs are found depends on it. Which order is right? [BoundaryIterator]~~ **ANSWERED 2026-10-14** from the exe
  (`BoundaryIterator::GetNext` `0x0060CEE0`, set up with x as the fast coordinate at `0x0071633C`) and written into
  §4.12 step 1 and test 3. **The bracket is right: BoundaryIterator.** The order is first row, then the middle rows
  at x0 and x1, then the last row. So L1's ring is (1, 1), (0, 2), (2, 2), (1, 3), and the test now says so.
- ~~**TN8 — §4.9:** does the DoPostCityInit ferry pass rebuild the neighbour-cell vector and rotate the counters? [no]~~
  **ANSWERED 2026-10-14** from the exe (`0x00720C8A`–`0x00720CAE`, compared with the state-19 entry
  `0x0072134C`–`0x007214CA`) and written into §4.9.3. **The bracket is right: no.** DoPostCityInit only sets the
  re-route flags, points the pass at the first terminal and zeroes its sub-step. The neighbour step therefore walks
  the vector as it stands, which is empty after construction.
- ~~**TN9 — §4.9.5:** do the routes-in-use records report the last or the current counters? [last]~~ **ANSWERED
  2026-10-14** from the exe (`GetFerryRoutesInUse` `0x0071C250`) and written into §4.9.5. **The bracket is right:
  last.** Both the non-zero test and the copied counters read the last fields: `+0x20`/`+0x24` of a terminal link,
  `+0x2C`/`+0x30` of a neighbour link. The field offsets are now listed.
- ~~**TN10 — §4.9.1:** does a water marker with no qualifying side overwrite an earlier one? [no]~~ **ANSWERED
  2026-10-14** from the exe (`DoFerryTerminalActivated` `0x0072384A`–`0x007238AB`) and written into §4.9.1. **The
  bracket is right: no.** With no qualifying side, the loop ends without storing, so an earlier water cell and its
  flag stay. Also clarified: the stored water cell is the under-water **neighbour** N, not the marker's own cell.
- ~~**TN11 — §4.8:** does a re-insert overwrite the tunnel record, and does a portal with no pair get no record?
  [overwrite; no record]~~ **ANSWERED 2026-10-14** from the exe (`DoTunnelChanged` `0x00714154`,
  `0x00714386`–`0x00714396`) and written into §4.8. **The bracket is right on both counts.** A re-insert does
  find-or-insert and copies the whole record over it. A null paired portal (or no matching type) returns before
  that, so no record is created and an existing one is left as it was.
- ~~**TN12 — §4.2:** are edge-density entries created for both evening values? [both]~~ **ANSWERED 2026-10-14**
  from the exe (`DoConnectionsChanged` `0x0071AD20`–`0x0071AE1E`) and written into §4.2 step 3. **The bracket is
  right: both.** For every t on n, the evening-0 key and then the evening-1 key (`| 1 << 24`) are each looked up.
  A missing one is inserted as a copy of the zero record; an existing one is not touched.
- ~~**TN13 — IDs:** are `0x69D5C424` and `0x69D5C42D` grid instance IDs or classes? traffic-simulator.md calls
  `0x69D5C424` the "wormhole byte map". [none]~~ **ANSWERED 2026-10-13** from the exe and written into
  traffic-network.md §1 (Init step 5) and §2.3. Both are **instance IDs**, set by `SetInstanceID` (grid slot 6) at
  `0x0071EF56`/`0x0071EFD2`. The classes are the framework's u8 (`0x49B9E602`) and u32 (`0x49B9E606`) grids.
  `0x69D5C424` is the switch **mode-mask** grid that `DoLotActivated` writes (`0x00723E2B`), not a wormhole map:
  the wormhole bits live in the cell connection record. The name is corrected in traffic-simulator.md (grid table,
  §6.1 row 9) and save-records.md.
- ~~**TN14 — §4.10.2:** is the 0.2 a double or a float32 constant? [double, then 24-bit rounding]~~ **ANSWERED
  2026-10-14** from the exe (`0x00725370` `fmul dword [0x00AB34F4]`) and written into §4.10.2. **The bracket is
  wrong: float32** `0x3E4CCCCD`, as are 40.0, 0.1 and 1.0. The double would change the step cost for 25 depth
  bytes, for example 47 (`0x4081EB85` vs `0x4081EB86`). A worked example is in the spec.
- ~~**TN15 — §4.10.1:** is a water occupant's "cell rectangle" its single cell or its bounding cells? [single cell]~~
  **ANSWERED 2026-10-14** from the exe (`cSC4WaterPathFinder::DoMessage` `0x00724B19`–`0x00724BC8`) and written
  into the §4.10.1 table. **The bracket is wrong: bounding cells.** The rectangle is occupant slot 11
  `GetBoundingCityCells`, and only its NW corner `(x1, z1)` is tested under water. It is added to the dirty
  rectangle as is, not grown or clamped. For an ordinary one-cell network piece the two answers coincide.
- ~~**TN16 — §4.11 Fill step 3:** does a popped state resume after step 3 or redo it? [after; redoing it can loop
  forever]~~ **ANSWERED 2026-10-14** from the exe (`Fill` `0x00717EC0`: `0x00718186`, `0x007181A5`) and written
  into §4.11. **The bracket is right: after.** The state's k runs 0…5. Step 3 runs only at k = 4 and sets k := 5
  before pushing, so a popped tunnel state goes straight to the next pop.
- ~~**TN17:** what does `GetConnectedDestinationCount` return for a negative k? [0]~~ **ANSWERED 2026-10-14** from
  the exe (`0x007106C0`: `cmp k, 3` / `jle`, then index `7·id + 3 + k`) and written into §4.13 under the table. **The
  bracket is wrong:** a negative k passes the signed `k > 3` test and reads before `capacity`. k = −1…−3 read the
  record's population map object (k = −3 is its header pointer, a heap address); lower k read earlier records.
  The spec recommends returning 0, since no known caller passes k < 0.
- ~~**TN18:** what does `GetFerryTerminalCount` return for a type above 2? [0]~~ **ANSWERED 2026-10-14** from the
  exe (`0x0071FFF0`; constructor `0x0071FF7C`/`0x0071FF83`; activation `0x0072368A`) and written into §2.3's
  table, §4.9.1 step 1 and §4.9.5. **The bracket is wrong.** There is no bounds check: it returns the u32 at
  `+0x128 + 4·type`. Only types 0 and 1 have count words. Type 2 reads the potential-accident vector's begin
  pointer, and types 3 and above read its end, its capacity and later members.

  A side effect is that a type-2 ferry terminal can never activate, because its "count" is a heap pointer and so
  always ≥ 16. The spec recommends returning 0 for type ≥ 2.
- ~~**TN19 — §4.7:** are the switch flags taken from all rules or only from kind-0 rules? [all]~~ **ANSWERED
  2026-10-14** from the exe and written into §2.4 and §4.7.2. **All:** the loop at `0x00715EF8`–`0x00715F6D` has no
  kind test and covers the first n rules of the vector. A worked example (shipped station `0x0ADE0000`, flags
  `0x4C`) shows that the subway access point comes from a kind-1 rule.
- ~~**TN20 — §11 test 7:** the Walk-on-Road speed is not stated (3.5 was worked back from the stated Walk cost). Please
  list the shipped speeds of every travel mode.~~ **ANSWERED 2026-10-13** from the shipped data (Traffic Simulator
  exemplar `0xC9133286`, `SimCity_1.dat`). The full 9 × 13 table and the resulting `invMaxSpeed` bits for
  CellWidthX 16 are now in traffic-simulator.md §4.14, and test 7 states its speeds. Walk on Road is **3.5**, as the
  core inferred. Pipe, PowerPole and DirtRoad are 0 for every travel type.

## From the coordinator (run 61, 2026-10-14)

- ~~**CR1 — power.md §13 Q2: who posts ConnectionsReady `0x6AC284F3`, and what triggers it?** In run 61 it never
  arrived after loading Konradshohe.~~ **ANSWERED 2026-10-14** from the exe and written into foundations.md §11.6,
  with pointers in power.md (§4 table and §13 Q2), plumbing.md, neighbors.md and advisors.md §6.8.1.
  - The only poster is `cSC4RegionalCity::CreateSymmetricConnections` (`0x004AB778`): queued, no data, on
    TerrainReconciliation for the current city.
  - TerrainReconciliation comes from the terrain's second animation tick after its PostCityInit, so once per load.
    The player's reconcile-edges command also posts it.
  - With the default preference the tick defers to `ReconcileEdgesWithNeighboringCities`. That function waits while
    a modal window is open and **posts nothing while popup dialogs are disabled** (`SetPopupDialogsEnabled(false)`).
  - None of land value, weather, ordinance or demand appears in this chain.
  - **Confirmed in game** (coordinator): plumbing alone stays at `enabled=0` with `popup_dialogs = 0` set at
    PostAppInit (run 64) and is enabled every month without it (run 65). The plugin now sets the flag only at
    ConnectionsReady (`d161d20`, the §11.6 [choice]); run 66 has both: no popups, plumbing enabled all 38 months.

## From the residential tests and record (filed by the coordinator from the lane's report, 2026-10-14; merged with the core)

About `docs/specs/residential.md` and `residential-education-health.md` (REH). All 24 §15 tests pass. REH tests
1–7, 10 and 18–21 belong to the host's education/health hooks. The core's current answer is in brackets.

- ~~**RS7 — §15 test 8 vs the shipped LE curve (inconsistency):** the shipped Health Quotient → Life Expectancy curve
  gives 65 at HQ 60, not the 61 the test states. The vector uses a flat custom curve to get round it. Which is
  right: the test's number, or the curve the test should use? [custom curve in the vector]~~ **ANSWERED
  2026-10-14** from the shipped exemplar ("Residential simulator", `0x6941A67F` = (0,50) (20,55) (40,60) (60,65)
  (200,100)) and foundations §4.3. Test 8 is rewritten in residential.md §15. **The curve is right and the test's
  61 was wrong:** at GetCellHQ = 60 the curve is exact at its point, so LE = 65. Since 70 > 65 the reset still
  happens, with the same EQ/HQ results; only leSum changes (65 × 200). The test now uses the shipped curve, with
  EQ/HQ stored as 50.2 / 60.2, which decay to exactly 50 / 60. The custom curve can go. For LE 61, use HQ 44
  (exact).
- ~~**RS8:** does the "other" list move into the health-affecting list? The spec does not say so. [moved; inferred]~~
  **ANSWERED 2026-10-14** from the exe (state 0 `0x006FF75F`–`0x006FF7E8`; `DoBuildingChanged` `0x006FB5C1`–
  `0x006FB9C1`) and written into the new residential.md §5.8. **The bracket is right: moved.** State 0 appends
  every pending entry, in order and unfiltered, to the end of the health-affecting list, moving the reference,
  and then clears the pending list. Also specified: the add test (both properties > 0), and a removal that tries
  the health-affecting list first and then the pending one.
- ~~**RS9 — record Read:** what should Read repair after a stream error, and what happens with a version it cannot
  read? [nothing repaired; Read fails]~~ **ANSWERED 2026-10-14** from the exe (`Read` `0x007009A0`, helpers
  `0x005D9730`, `0x005DB3C0`, `0x006F7BA0`) and written into residential.md §9.2 and test 20. The original rejects
  no version (≥ 7 is read as 7, 0–2 as 2), never checks the stream and returns true. After an error, later refs
  are null and the state, list counts and department scalars are stale stack values. Init repairs only the grids,
  the departments and the department scalars.

  **Choice:** return false and leave the simulator **as constructed**, rather than partly read, so that Init
  starts a new city. This is the crime.md §12 rule. Accept every version.
- ~~**RS10 — record Read:** are list and department entries that a Read replaces released? [released, departments
  with `release_department`]~~ **ANSWERED 2026-10-14** from the exe and written into residential.md §9.2. The
  original releases **nothing**: list clears free nodes only (`0x00413E60`), and ref reads null the destination
  without `Release` (`0x005D9742`). This cannot be observed, because Read only ever runs on a freshly constructed
  object, where every ref is null and every list empty. The core's releasing is fine.
- ~~**RS11:** how do coverage and list vectors with counts other than 7 / 3 behave? [none]~~ **ANSWERED 2026-10-14**
  from the exe and written into residential.md §9.2. They arise only from a hand-made or damaged record, since
  the game writes 7 and 3. The original accepts them and never checks a bound: the monthly states loop over the
  building-list vector's size, and Init loops over the coverage vector's size. Both index fixed 7- and 3-entry
  tables (`0x00AB2B60`/`7C`/`98`/`A4`) and each other's vectors. A larger count reads the next table, and a
  smaller one drops and leaks pending buildings. **Choice:** treat such a count as a damaged record (RS9 rule).
- ~~**RS12:** can the lists hold null entries, and what does each pass do with one? [skipped]~~ **ANSWERED
  2026-10-14** from the exe and written into residential.md §9.2. Only `Read` can produce them, through an
  unresolvable reference or a stream error; the game never writes them. The original does **not** skip them:
  distribution (`0x006FF622`), coverage (`0x006F8710`), grading (`0x006FFE61`) and `GetBuildings` (`0x006FAD2C`)
  all dereference the entry and crash. **Choice:** drop nulls in Read, so passes never see one; skipping them in
  each pass is equivalent.
- ~~**RS13:** do the state-0 grade totals live in the residential simulator or in the host (REH)? [host]~~
  **ANSWERED 2026-10-14** from the exe and written into REH §3 (table), REH open question 3 and residential.md
  §5.1. **The bracket is right: host (REH).** In the original they are two floats of the one object (`+0x314`,
  `+0x318`). They are zeroed at the start of state 0 (`0x006FF567`) and accumulated by grading in states 6 and 8
  (`0x006FFED3`, `0x00700053`). Nothing else in the class touches them, no slot returns them and they are not
  saved. They are therefore write-only, and the host may keep them or drop them without any observable change.
- ~~**RS14:** what does the state machine do in an unknown state? [waits]~~ **ANSWERED 2026-10-14** from the exe
  (`DoDailySimulation` `0x006FF546` and its jump table; `DoMessage` `0x00700397`) and written into residential.md
  §5.1. **The bracket is right, with a caveat: it waits forever.** A state above 15 does what state 10 does
  (nothing; returns true), but the month message resets only state 10, so the simulator never resumes.
  **Choice:** keep it, but let Read reject a saved state above 15 as damaged (RS9 rule).
- ~~**RS15:** is the pollution loop's S capped (a hostile S is unbounded)? [no cap]~~ **ANSWERED 2026-10-14** from
  the exe (`0x006F7FF0`) and written into residential.md §5.5 step 1. **The bracket is right: no cap.** S² calls
  are made per tract and type. A wrapped `S·x + S` empties the loop. For S = 0 or S ≡ 0 (mod 65536), `S·S` is 0
  and the `div` at `0x006F8062` faults. **Choice:** Read rejects P = 0 or P > 256 as a damaged record.
- ~~**RS16:** what does a zero divisor give in the averages? [0]~~ **ANSWERED 2026-10-14** from the exe
  (`PostPopulationChange` `0x006F8070`, `DoDailySimulation` state 9 `0x007000B5`, `DoPopulationChange`
  `0x006FAAC7`, `GetTractQuotient` `0x006F6FC0`, `GetGlobalEQ`/`HQ` `0x006F7720`/`0x006F7870`) and written into
  residential.md §5.6 (new "Zero divisors" table) and §5.5 step 6. **The bracket is right: 0 everywhere, and none
  of them faults.** One correction: the life-expectancy guard is `population ≠ 0`, not `> 0`. A negative population
  (damaged record only) divides as unsigned 64-bit by 2^64 − |population|, which also gives 0.
- ~~**RS17:** does a version-3 record keep the running population the constructor set? [yes]~~ **ANSWERED
  2026-10-14** from the exe (`Read` `0x00700D5D`–`0x00700D63`; constructor `0x00700510`, store at `0x007005BF`)
  and written into residential.md §9.2, §9.3 and open question 4. **The bracket is right: yes, and the value is
  0.** A version-3 record skips the running population and changes nothing else (versions 0–2 zero it and move
  state 4 to 11). Because `Init` then restarts the pass with leSum 0, a version-3 save made in state 4 does **not**
  get the double count that §9.3 describes for version ≥ 4 (worked example there).

## From the pollution glue and grid shadow (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/pollution.md` and its `docs/abi` rows. The glue's current answer is in brackets.

- ~~**PG1 — slot 55 `GetRecyclingCenterCurrentCapacity`:** the spec says a bool in AL; `docs/abi` says ST0 (a float).
  Which? [bool; the slot is waived in abi_conformance_test]~~ **ANSWERED 2026-10-14** from the exe (`0x006E3720`)
  and written into the pollution.md slot table. **The bracket is right: bool in AL** (slot 53's result, untouched).
  The percentage is stored through the reference, and the x87 stack is empty on return. `docs/abi`'s ST0 is wrong;
  the abi rows are the coordinator's to fix.
- ~~**PG2 — slot 51 `GarbagePutInLandfill`:** the spec says it returns the new int64 total; `docs/abi` says eax or
  void. [int64; waived]~~ **ANSWERED 2026-10-14** from the exe (`0x006E3250`–`0x006E32CE`) and written into the
  pollution.md slot table. Both descriptions are loose. The original leaves EAX = the new high dword of
  `landfilled` and EDX = the sign of n, which is meaningless as an int64. Its known caller (landfill §4.7.2) ignores
  it. **The bracket is fine:** declare it int64 in EDX:EAX with pop 4 (SDK), since any value conforms. FreeCity4
  returns the new total.
- ~~**PG3 — slot 54:** the spec says AL; the table says x87 or eax. Is it a float? [as the spec]~~ **ANSWERED
  2026-10-14** from the exe (`0x006E2D70`; getter `0x005FDAC0`) and written into the pollution.md slot table.
  **The bracket is right: AL, a bool.** The fraction goes through the reference and the x87 stack is empty on
  return. Also added to the row: the property is read through the shared percentage getter, so it is × 0.01f
  (float32), like slot 55.
- ~~**PG4 — §4.10 visual effects:** the effects manager's `CreateVisualEffect` and the `cISC4VisualEffect` /
  `cS3DTransform` layouts are not in `docs/abi`. [visual effects not implemented]~~ **ANSWERED 2026-10-14** from the
  exe (calls `0x006E6836`–`0x006E693D`; `CreateVisualEffect` `0x00593AE0`; effect vtable `0x00AA1170`; transform
  copy `0x005BEC20`; RNG `0x00B49BD0`) and written into pollution.md §4.10. **Not drawing them is the implementer's
  choice, but part of the pass must stay.**
  - **The interfaces.** `CreateVisualEffect` is effects manager slot 6 (pop 12, AL; `*out` is AddRef'd). The
    effect's slots are 3 `Start`, 4 `Stop`, 5 `IsStarted`, 8 `SetTransform`, 12 `SetParameter(index, float*,
    count)` and 2 `Release`.
  - **`cS3DTransform`** is 0x38 bytes: a counter byte, a flags byte, a 3×3 matrix at `+0x04`, the translation at
    `+0x28` and the scale at `+0x34`.
  - **Must still run without effects.** The pass's one `rand()` call (the effect iterator's reset) shares the
    main-thread stream, and the worst tracts feed slots 17–19.
  - **Safe to drop.** The water-position draws use a `cRZRandomSuper` that nothing else uses.
- ~~**PG5 — §10.2 Q1:** is the prop requester re-added after a load? [open in the spec]~~ **ANSWERED 2026-10-14**
  from the exe (prop developer `Write` `0x006F1970`–`0x006F19B6`, `Read` `0x006F3680`–`0x006F36C8`;
  `AddPropRequester` `0x006F21C0`) and written into pollution.md §4.1 and §10.2 Q1. **No; the prop developer's own
  save restores it.**
  - **How it is restored.** The prop developer writes its requester vector by ref and reads it back with IID
    `0x89C498BE`. A loaded city therefore still gets garbage props.
  - **Do not re-add it.** `AddPropRequester` has no duplicate check, so adding it again would register it twice.
  - **For a replacement.** It must answer `0x89C498BE` and be saved by ref.
- ~~**PG6 — shadow method:** the shadow captures the original's state by calling the original's own `Write`. Is that
  acceptable as read-only? [used]~~ **ANSWERED 2026-10-14** from the exe (`Write` `0x006E3380`–`0x006E371A`; ref
  helper `0x005D99A0`; list writer `0x005DAFF0`; iterator writer `0x005DAB70`) and written into pollution.md §6.
  **Yes; the bracket stands.**
  - **No stores.** `Write` stores to no member of the simulator, clears no dirty flag and allocates nothing.
  - **Refs depend on the stream.** If the stream refuses `cISC4DBSegmentOStream` (`0x6999A8FB`), every ref writes
    nothing and the referenced objects are not touched. The capture then has no grid refs, no list entries (the
    counts remain) and no department.
  - **If the stream answers,** each object is QueryInterface'd and released again, so its reference count does not
    change. A save's own stream would also register the object with the class-object store, so never capture
    through it.
- ~~**PG7:** `radiationSum` is not saved, so the radiation history at the end of a cycle after a load diverges from a
  run without the load. Intended? [as the spec]~~ **ANSWERED 2026-10-14** from the exe (`Write` skips the field
  between `developedTracts` and `avgGarbage`; `Read` never touches it; ctor `0x006E206A` sets it to 0; new-city
  zeroing `0x006E7288`; end-of-cycle zeroing `0x006E7A34`) and written into pollution.md §6 with a worked example.
  **The bracket matches; reproduce it as a *[quirk]*.** After a load the sum restarts at 0. Only the first end of
  cycle after the load differs, by one entry each in `0xEA110137` and `0xEA110138`. Whether it was intended
  cannot be read from the exe.
- ~~**PG8 — §4.6.1 step 3:** is the unsigned `wteRemaining` converted to float as signed or unsigned? [as written]~~
  **ANSWERED 2026-10-14** from the exe (waste-to-energy `0x006E38EA`–`0x006E3903`, incinerators
  `0x006E39A5`–`0x006E39BE`; constant `4294967296.0f` at `0x00A80AA8`) and written into pollution.md §4.6.1 with a
  table. **Unsigned, so the bracket matches, but it is not rounded to `float32`.**
  - **How it converts.** The value is loaded as signed, and 2³² is added when its top bit is set. It is then
    compared on the x87 stack with the `float32` `g`.
  - **Below 2³¹** the comparison is exact. For example, 2²⁴ + 1 > 16,777,216.0 is true.
  - **From 2³¹ up** the add rounds the value to `float32` under the 24-bit precision.
  - **In practice.** Both differ from a plain `f32` conversion only above 2²⁴. Steps 3 and 4 behave the same.
- ~~**PG9:** can landfill usage change within a day? [the shadow keeps the first value it reads]~~ **ANSWERED
  2026-10-14** from the exe (`GetLandfillUsage` `0x006B9820` reads cached totals; they are written only by the ctor
  `0x006B9780` and `UpdateCapacityValues` `0x006B8F20`; its callers are `0x006B9302` month, `0x006B9CE2` zoning,
  `0x006B9F2B` PostCityInit) and written into pollution.md §4.6.1. **Not within one day pass, so the bracket
  matches if the value is read again each day.** None of those callers runs inside pollution's day pass, which
  finishes in one delivery. The values can change between day passes: after zoning, after the landfill month
  agent, or after a load.
- ~~**PG10:** without an ordinance simulator, `cumulative_factor` returns 1.0. [choice]~~ **ANSWERED 2026-10-14**
  from the exe (52 unchecked calls through the held pointer, `0x006E83DD`–`0x006E8479`) and written into
  pollution.md §4.4.3. **The original has no such path: a null pointer crashes on the first read, and the city
  always creates the simulator.** The *[choice]* of 1.0 stands. It equals `GetCumulativeFactor` with no enacted
  ordinance carrying the property (ordinance.md §6.1).
- ~~**PG11 — slots 29/30:** they return int32 (the applied delta and the argument respectively) where the SDK has
  bool. Confirm. [int32]~~ **ANSWERED 2026-10-14** from the exe (slot 29 `0x006E32E0`, slot 30 `0x006E5050`; every
  call site checked) and written into pollution.md §8.1 with a table. **Confirmed; the bracket matches.**
  - **Neither is a `bool`.** EAX holds the applied delta (taken before the final clamp at 0) and the argument
    respectively. AL is just the low byte, so `AddToGarbageSupply(256)` gives AL 0.
  - **No caller reads either value.** That covers the landfill developer, the landfill zones and the simulator
    itself.
- ~~**PG12 — §5.2 posts:** no void or IGZUnknown is set on them, since the spec names none. Confirm. [none set]~~
  **ANSWERED 2026-10-14** from the exe (post sites `0x006E70EA`, `0x006E7A64`–`0x006E7AB2`, `0x006E7B3D`; helper
  `0x005F98C0` → `0x005F9800`; message ctor `0x009134D6`) and written into pollution.md §5.2. **Confirmed; the
  bracket matches.** The helper sets the IGZUnknown explicitly to the argument pollution passes, which is always
  null. void1 is never set and stays null from the constructor, which zeroes every field. The messages are queued
  with `MessagePost(msg, false)`.
- ~~**PG13 — slot 5:** what does it return for a type above 3? [null; choice]~~ **ANSWERED 2026-10-14** from the
  exe (`GetPollutionGrid` `0x006E3360`; every caller found checked) and written into pollution.md §8.1 slot 5.
  **Never a grid; the null *[choice]* stands.**
  - **What the original returns.** Type 3 is special-cased. Every other type reads the member at
    `this + 0x50 + 4 × type` with no bounds check:
    - 4 and 5 return `cellX` / `cellZ` as a pointer;
    - 6–9 return the four occupant lists' head pointers;
    - larger types read past the object and can fault.
  - **No caller passes a type above 3.** MySim, the query tool, the growth developer, the map view and aura all
    pass a constant or loop over 0..3.
- ~~**PG14 — in-game differential (run 68, coordinator): `dirtyWater` after a day.** With the original pollution
  simulator and `pollution_shadow = 1`, our core matched all eight grids cell by cell for 94 months (2026-02 →
  2033-11; 1.4 M cells a month, 0 mismatches) and every saved field except one: on 104 of about 2,800 days the
  original's `dirtyWater` is **true** after its day while ours stays false, with identical grids. The likely place
  is §4.4.4 step 10, the water neighbour special case. When a tract at 1 would decay to 0 and the case keeps it at
  1, does the original still call `SetWaterValue` (marking `dirtyWater`) although the value did not change? Or does
  some other day path set the flag without changing a cell? It decides how often "water changed" `0x5A6DD1AA` is
  posted. [ours: written only when the value differs, as step 9 says for air]~~ **ANSWERED 2026-10-14** from the
  exe (`DoDailySimulation` `0x006E8B87`–`0x006E8C4B`; `SetWaterValue` `0x006E30DC`) and written into pollution.md
  §4.4.4 step 10. **The hypothesis is right.** Once the decrease gives 0, the original always calls
  `SetWaterValue`, with 1 or 0, without comparing. So v = 1, W = 0 and a neighbour > 1 rewrites the same 1 and
  sets `dirtyWater`. No other day path sets the flag: this is the day's only `SetWaterValue` call, and the in-day
  end of cycle clears it. Fix: mark `dirtyWater` whenever the special case runs.

## From the landfill tests and the neighbours core (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/landfill.md` (tests §11 1–23, vectors `lf11_*`) and `docs/specs/neighbors.md` (tests §18 1–12,
vectors `nb18_*`). The core's current answer is in brackets.

- ~~**LF1:** which of `AttemptAreaLevelling`'s two bools is the landfill's second `true`? Test 22 cannot tell them
  apart. [passed as `waterFlag`]~~ **ANSWERED 2026-10-14** from the exe: the bracket matches. The third argument
  (`withWater`) feeds the vertex average and the slope test; the fourth (`waterFlag`) is `ModifyTerrainHeight`'s
  second bool (`0x006C44F0`). Both are literal 1 (`0x006BA785`/`0x006BA787`), so the order is unobservable. Also:
  this path passes the **unclamped** rect (landfill.md §4.5.3).
- ~~**LF2 — zone `Init`:** is "levelAltitude is exactly 0.0" an exact float compare? [−0.0 counts as zero; NaN does
  not]~~ **ANSWERED 2026-10-14** from the exe: the bracket matches. It is `FUCOMPP` against +0.0 with `TEST AH,
  0x44` / `JP` (`0x006BB6E2`–`0x006BB6F2`): only an ordered equal compare recomputes. The centre cell is the C
  integer mean of the rect corners (landfill.md §4.4.9).
- ~~**LF3:** in what order does Shutdown release the message server and the city? [one `release_services` call after
  the zones shut down]~~ **ANSWERED 2026-10-14** from the exe (`0x006B94B0`). The original order is: unsubscribe
  and release the message server **first**; then shut down and release the zones; then release the department,
  budget, pollution, terrain and city; then `RemoveAgent` through the global city. The bracket differs, but zone
  `Shutdown` uses only its own references, so the difference is unobservable. Matching the original is recommended
  (landfill.md §1.2, §4.4.9).
- ~~**LF4 — §4.7.5:** in what ring order does the zot spiral run? [clockwise from each ring's top-left corner; the
  second half of `lf11_11` depends on it]~~ **ANSWERED 2026-10-14** from the exe (`0x006BB3B0`–`0x006BB4DA`): **the
  bracket does not match.** The zot uses the plumbing §7.2 `SpiralIterator` mode over the zone rect. Ring r starts
  at `(cx + r, cz − r + 1)`, runs +z, −x, −z, +x, and ends at `(cx + r, cz − r)`, so ring 1 starts at `(cx+1, cz)`,
  not `(cx−1, cz−1)`. Out-of-rect cells are skipped one by one. landfill.md §4.7.5 has two worked examples, and
  test 11 now cites them. `lf11_11` must be regenerated.
- ~~**LF5 — §4.4.4 step 3:** `LevelRect((x, z, x+1, z+1))` per cell is a 2×2 block if the rectangle is inclusive. Is
  that intended? [passed as written]~~ **ANSWERED 2026-10-14** from the exe (`0x006BC150`): the bracket matches. The
  rect is inclusive, so each region cell averages and levels the 2×2-cell block (vertices `x..x+2 × z..z+2`):
  `ModifyTerrainHeight(1, 1, x, z, x+2, z+2, avg, null, 5, null)`. The levelling is chained cell by cell (x outer,
  z inner) and reaches one cell past the region's +x and +z edges. Whether that was the author's intent cannot be
  known; it must be reproduced. Worked example in landfill.md §4.4.4 step 3.
- ~~**NB1:** the Julian day of `DEFAULT_DATE`. [0]~~ **ANSWERED 2026-10-14** from the exe: there is no fixed default.
  The deal's date objects are default-constructed from the **wall clock** (`0x0091307C`: `time`, `localtime`, then
  `Jday`), so a deal built on 2026-10-14 starts at 2461328. The bracket's 0 is kept as a *[choice]*, because the
  original is clock-dependent. The date survives only in fields nothing reads before overwriting them (neighbors.md
  §5.1).
- ~~**NB2:** does `Activate` change the flags directly, or through `SetFlags` with its status-0 reports? [directly]~~
  **ANSWERED 2026-10-14** from the exe (`0x006CE640`): **the bracket does not match.** `Activate` calls its own
  virtual `SetFlags` three times: (Active, true), (Proposed, false), (Cancelled, false). Each call that changes a
  non-Silent deal posts status 0, so activating a proposal posts 0, 0, then `simulate_deals`'s 1 (neighbors.md
  §5.4, §7.2).
- ~~**NB3 — §6.3:** does the marker altitude `GetAltitude(x, z)` use the connection cell as given or the inner cell?
  [as given]~~ **ANSWERED 2026-10-14** from the exe (`0x006D494A`–`0x006D4A4F`): **the bracket does not match.** The
  height is `GetAltitude(px, pz)` at the **inner** cell's centre, the same point the prop is placed at. The
  connection cell is not used (neighbors.md §6.3, with an example).
- ~~**NB4 — §12 vs §7.1:** ProposeDeals reads `0xCA469346` (f32) in §12 but `0xCA46931F` in §7.1. [`0xCA46931F` for
  proposals; `0xCA469346` as GetDealCost's f64 maximum markup]~~ **ANSWERED 2026-10-14** from the exe: the bracket
  matches. `ProposeDeals` reads both. The `0xCA469346` read (`0x006D22B4`) becomes `1 + v` in a local that is never
  read; the budget test uses `0xCA46931F` (`0x006D24EE`). Both sections were right, and neighbors.md §12 now marks
  the read as dead.
- ~~**NB5:** what does `UpdateCost` do with no current simulator? [nothing]~~ **ANSWERED 2026-10-14** from the exe:
  the original **crashes**. `UpdateCost` (`0x006CF9C0`) checks neither the global city nor `GetNeighborsSimulator()`
  (`0x006CF9E8`); only `SetAmount` guards both. The bracket is kept as a *[choice]*: no cost change and no report
  (neighbors.md §5.4).
- ~~**NB6 — §2.2:** `neighbour_connection_cell` for an interior cell has no row. [`None`]~~ **ANSWERED 2026-10-14**
  from the exe (`0x006CE920`): **the bracket does not match.** The outputs start as null, 0, 0. An interior cell
  keeps our own region position, finds our own regional city there, and returns **true** with `(0, 0)`.
  `FindConnectionOnOtherSide` still ends with null, so only the helper's result differs. neighbors.md §2.2 has the
  row and an example.
- ~~**NB7 — §13:** null or host references in the connection list are skipped on read. Correct? [skipped]~~
  **ANSWERED 2026-10-14** from the exe (`0x006CFE10`–`0x006CFEDD`). Failing references (wrong interface, unknown
  class) are skipped: the bracket matches. A **null** reference succeeds: the original appends the null and then
  releases it unchecked (`0x006CFEC5`), so it crashes inside `Read`. Skipping it is kept as a *[choice]*
  (neighbors.md §13 table).
- ~~**NB8 — §8.1:** does `existing` include the deal being priced? [yes: its own `UpdateCost` counts it; 262 in
  `nb18_03`]~~ **ANSWERED 2026-10-14** from the exe (`0x006D33A5`–`0x006D33E1`): the bracket matches when the deal
  is in this simulator's list and is the first of its type with that city. `existing` is the first `FindDeals(type,
  C)` match, with no flag filter. An earlier deal of the same type and city takes its place, and an unlisted deal
  never counts itself. The 262 is reproduced in neighbors.md §8.1.
- ~~**NB9:** §5.2 says pricing ignores `0x3D` deals, but §8.1's `FindDeals` has no filter. [as §8.1]~~ **ANSWERED
  2026-10-14** from the exe: the bracket matches. Both `FindDeals` overloads (`0x006D0760`, `0x006D0840`) match on
  type and city only, and `GetDealCost`/`GetDealMaxAmount` check no flags. §5.2 was wrong and now limits the `0x3D`
  rule to the totals (neighbors.md §5.2).
- ~~**NB10:** SetAmount's synchronous report re-enters `simulate_deals` during `ProposeDeals(add = true)` and
  activates the proposal. Intended? [as written]~~ **ANSWERED 2026-10-14** from the exe: the bracket matches, and
  it must be kept. In `0x006D0520` the order is: un-Silence, append, then `SetAmount(0)`, which sends status 0
  synchronously. The handler runs a full `simulate_deals`, since `ProposeDeals` sets no busy flag, and that run
  activates the proposal. `Propose` writes the dates afterwards (neighbors.md §7.1, step by step).
- ~~**NB11:** the store ignores a false `Read` (foundations §3.5), so a version-3 simulator record loads "clean" with 0
  connections. Confirm. [yes]~~ **ANSWERED 2026-10-14** from the exe: confirmed. `Read` returns false right after
  the `u16` (`0x006CFDDB`), with the stream still good. The store ignores the result, and the freshly created
  simulator keeps its empty list (neighbors.md §13).
- ~~**NB12:** `UpdateCommodityStats` takes a regional-city token. Is that the right subject? [yes]~~ **ANSWERED
  2026-10-14** from the exe (`0x006D2A2C`–`0x006D2A46`): **no.** The argument is the `cISC4City*`, compared by
  identity with the current city. Null or any other pointer, including our own regional-city object, does nothing.
  The core should take a city handle. Also, `R` is not null-checked (`0x006D2A6C`); returning early there is a
  *[choice]* (neighbors.md §8.2).
- ~~**NB13:** a present `0xEA46934F` of 0 is divided by literally (inf or NaN); a missing one gives 0. [as stated]~~
  **ANSWERED 2026-10-14** from the exe. Present 0: the bracket matches; the x87 divide at `0x006D309C` gives +∞, or
  NaN when free = 0. Missing: the original divides by an **uninitialized** stack float (no default is written before
  the read, `0x006D2AE3`). Its result is undefined, so "missing gives 0" is kept as a *[choice]* (neighbors.md §8.2,
  §12).
- ~~**NB14:** the order of `GetAdjacentCities`. [choice: by region position, x then z]~~ **ANSWERED 2026-10-14** from
  the exe (`0x006D01E0`): the original collects into a `std::set<cISC4RegionalCity*>` with `std::less`, so the order
  is ascending heap address, which cannot be reproduced. The *[choice]* stands. The order mainly affects list and
  message order; the one effect on values is in `ProposeDeals(…, true)` (neighbors.md §9.1).
- ~~**NB15:** without a current regional city, the UpdateState budget test is skipped. [skipped]~~ **ANSWERED
  2026-10-14** from the exe (`0x006CF360`): for a buy deal the original dereferences the missing regional city and
  **crashes** (`0x006CF386`); sell deals never use it. Skipping is kept as a *[choice]*. The test is `cost > budget`,
  and an affordable buy deal then goes on to the connection test (neighbors.md §7.3).
- ~~**NB16:** a missing payer at renewal counts as "cannot afford". [yes]~~ **ANSWERED 2026-10-14** from the exe
  (`0x006D13A5`): the original does not null-check the payer, but cannot reach the case. A sell deal with no
  neighbour gets NoConnection earlier in the same pass; a buy deal with no `R` crashes in `UpdateState` first
  (NB15). The bracket is kept as a *[choice]*, needed only because of the NB15 choice (neighbors.md §7.3).
- ~~**NB17:** the return types of `RequestCreateConnection` and `CreateConnection`. [choice: the request returns
  `ConnectionRequest`, `CreateConnection` the new connection or null]~~ **ANSWERED 2026-10-14** from the exe. The
  internal types are glue: there is no original behaviour, so the implementer's choice stands. At the interface,
  `RequestCreateConnection` returns a real `bool` (confirmed: true; exists or declined: false), and the shim must
  match it. `CreateConnection` leaves a leftover `EAX` that callers ignore (neighbors.md §6.2).
- Lane choices worth knowing: the proposal generator is per simulator, seeded `derive_seed(session, 0xC990C0A4)`
  (the original's is process-wide and clock-seeded); save reads are capped at 65,536 zones.

## From the exemplar reader (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/data-formats.md` §4, §5 and the §11 exemplar rows (`engine/formats/src/exemplar.rs`). The install
sweep loads 9,347 of 9,347 exemplars. The reader's current answer is in brackets.

- ~~**EX1:** how are Empty (tag 0) properties stored? [`Values::Empty`; getters return None]~~ **ANSWERED
  2026-10-14** from the exe: **the storage half matches.** Tag 0 has no value bytes; the property is added and holds
  an Empty variant (memory type 0, `GetCount()` 0), so `HasProperty` is true. The getter half does not match: see
  EX5. No shipped record uses tag 0 (data-formats.md §4.2, with an example).
- ~~**EX2:** Uint64 to float, signed or unsigned? [unsigned]~~ **ANSWERED 2026-10-14** from the exe (`0x00915B85`):
  **the bracket matches**, with one addition. The value is unsigned (low 63 bits + 2⁶³), but the add runs at the
  main thread's 24-bit precision, so a Uint64 keeps 24 significant bits even when read as Float64: 16,777,217
  gives 16,777,216.0. Uint32 ≥ 2³¹ goes through the same kind of add (data-formats.md §5.1).
- ~~**EX3:** float to int when out of range or NaN? [saturates; NaN gives 0]~~ **ANSWERED 2026-10-14** from the exe
  (`0x00915AA5` → `_ftol` `0x009EEF04`): **the bracket does not match.** The float is truncated to an int64 and the
  target keeps the low bits, so out-of-range values wrap (300.7 as Uint8 gives 44). NaN, ±∞ and |x| ≥ 2⁶³ give
  `0x8000000000000000`: 0 for targets of 32 bits or fewer, `0x8000000000000000` for 64-bit targets, true for Bool
  (data-formats.md §5.1, with examples).
- ~~**EX4:** is RZChar signed when widened? [zero-extended]~~ **ANSWERED 2026-10-14** from the exe (`0x00915AA5`,
  `0x00915B85`): **the bracket does not match.** RZChar and Char are **sign-extended** for integer and float
  targets: RZChar `0xE9` read as Uint32 gives `0xFFFFFFE9`, as Float32 −23.0. Only RZUnicodeChar is zero-extended
  (data-formats.md §5.1).
- ~~**EX5:** what do the getters return for Empty or object values? [None]~~ **ANSWERED 2026-10-14** from the exe
  (`0x00917350`, `0x00916EAA`, `0x009167C5`): **the bracket does not match.** Both take the text round trip. Empty
  becomes empty text, which parses as 0, so the getters succeed with 0, 0.0 or false. An object becomes its
  `cIGZString` text or its address as `0x%08x`, so `GetProperty(id, uint32&)` returns its address. The string
  getter writes the text but returns false for every non-string type. The object case is a *[choice]*: missing
  (data-formats.md §5.1).
- ~~**EX6:** Char, RZUnicodeChar and void arrays in the text round trip. [Char and RZUnicodeChar arrays as text; a void
  array gives its first byte as `%u`]~~ **ANSWERED 2026-10-14** from the exe (`0x00916EAA`): **the first half
  matches**: Char and RZUnicodeChar arrays give all their characters, RZUnicodeChar units of `0x80` and above as
  `?`. **The void-array half does not**: a void array (and a VoidPtr array) gives the empty text, which parses as
  0 (data-formats.md §5.1, with examples).
- ~~**EX7:** an RZChar scalar as a string. [that one character]~~ **ANSWERED 2026-10-14** from the exe
  (`0x00916EAA`): **the bracket matches.** Char and RZChar scalars give their one character; an RZUnicodeChar
  scalar gives one byte when it is `0xFF` or less, otherwise `?` (data-formats.md §5.1).
- ~~**EX8:** what `%.24g` prints for the exponent and for nan/inf. [2-digit exponent; "nan"/"inf"]~~ **ANSWERED
  2026-10-14** from the exe (`0x0091052A`): **the bracket does not match.** There is no `%.24g`: floats are printed
  fixed-point, never with an exponent, with at most 24 digits after the fraction's leading zeros, cut, not rounded
  (Float32 1e30 gives 31 digits). NaN and infinities print as `1.#INF`, `-1.#IND` and the like *[inferred from the
  MSVC runtime]*. Values do not always survive the round trip: Float32 3.14159 comes back 2 ulps lower
  (data-formats.md §5.1).
- ~~**EX9:** is the round-trip parse's `0x` case-sensitive, and what does "after a digit" mean? [lowercase only;
  anywhere after the first digit]~~ **ANSWERED 2026-10-14** from the exe (`0x009167C5`): **the bracket
  matches**, with additions. `0x` is lowercase only, must start the text and be followed by a hex digit (16
  digits at most). "After a digit" applies only to `e`/`E` and means any earlier digit; `.` makes the text
  floating point wherever it is. `t`/`true`/`f`/`false` are case-sensitive whole-text matches. Char targets
  always get 0 (data-formats.md §5.1, with examples).
- ~~**EX10:** the ParentCohort line: field count and spacing. [exactly 3 fields, leading spaces trimmed]~~
  **ANSWERED 2026-10-14** from the exe (`0x0067DFD0`): **the bracket does not match.** Fields are not counted:
  a fourth and later field is ignored, and a missing field reads 0. Nothing inside the braces is trimmed: a
  space after `{` or `,` makes a `0x` field read 0, while a decimal field survives (data-formats.md §4.3).
- ~~**EX11:** are PropCount, reps and the type name trimmed? [PropCount and reps trimmed; the type name must match
  exactly]~~ **ANSWERED 2026-10-14** from the exe (`0x0067E250`, `0x0067E780`): **the bracket matches for
  PropCount and the type name, not for reps.** PropCount is trimmed of spaces and tabs like every `name=value`
  line. The type name is compared exactly, with no trimming at the `:`. reps is not trimmed: rule N reads it, so
  a leading space is harmless for decimal but turns `0x3` into 0. Rule N saturates at `0xFFFFFFFF`; too few
  property lines fail the read (data-formats.md §4.3).
- ~~**EX12:** is there a cap on reps? [more reps than the record length fails]~~ **ANSWERED 2026-10-14** from the
  exe (`0x0067E780`): **the bracket does not match.** There is no cap and no failure: missing values read 0
  and extra values are ignored. A huge reps crashes the original, because the buffer size (reps × element size,
  32-bit) wraps or cannot be allocated, and the result is not checked. Recommended *[choice]*: fill with 0 and
  fail the read above 16 MiB of values (data-formats.md §4.3).
- ~~**EX13:** are numeric values cut at `}` too? [no, only strings]~~ **ANSWERED 2026-10-14** from the exe
  (`0x0067E780`): **the bracket matches in effect.** Only strings are cut at `}`; a number stops at the first
  character that cannot continue it, so `}` ends it anyway (the last Float32 array value is split at `}`, with the
  same result). Leading spaces are trimmed for array values only, which matters for `0x` in rule N. Uint64 and
  Sint64 are hexadecimal with no sign (data-formats.md §4.3).
- ~~**EX14:** how is Float32 text parsed? [as f64, then rounded to f32; no hex/inf/nan]~~ **ANSWERED 2026-10-14**
  from the exe (`0x00910426`): **the first half of the bracket does not match; "no hex/inf/nan" does.** The game's
  own parser adds the digits one by one in x87 arithmetic, rounded at every step to the main thread's 24-bit
  precision; the exponent's power of ten is a wrapping 32-bit integer. So `0.05` gives `0x3D4CCCCC`, not
  `0x3D4CCCCD`, `1e10` gives 1,410,065,408, and `1e32` gives 0 (data-formats.md §4.3, with examples).
- ~~**EX15:** what counts as a resolvable kind-1 resource key? [an exemplar or cohort that loads, or a key registered
  from the package index]~~ **ANSWERED 2026-10-14** from the exe (`0x009732EB`, `0x00972EAE`): **the first half
  matches, the second does not.** A key resolves if a resource cache already holds it, or if a factory is
  registered for its type, a loaded package holds the record, and the factory's read succeeds. Index presence
  alone is not enough. Any type with a factory counts, not only exemplars and cohorts (data-formats.md §4.6).
- ~~**EX16:** shipped data: `Plugin_002_TheLivingMall.dat` writes `Uint8:0:{}` for two LotConfig properties, now read
  as a scalar 0 (§4.3). Intended? [scalar 0]~~ **ANSWERED 2026-10-14** from the exe (`0x0067E780`, `0x005FFB40`):
  **the bracket matches.** The original stores a scalar Uint8 0. Both readers of these properties use a byte-list
  getter that copies `GetCount()` bytes, 0 for a scalar, so the lot gets no wealth or purpose types. That is
  the same as an empty array, so the difference is not observable (data-formats.md §4.3).
- ~~**EX17:** slot 5 `GetProperty(id, void*, uint32&)`: what type does the caller request? [not implemented]~~
  **ANSWERED 2026-10-14** from the exe (`0x005FD290`, `0x00915714`): **the bracket does not match.** The original
  implements it as a raw copy from a **void array** only (memory type `0x8000`): at most `length` bytes, with
  `length` lowered to the array's size. Any other type returns false, with no conversion. No shipped record has a
  void array, so on shipped data it always returns false, and "not implemented" (always false) is not
  observable there (data-formats.md §5.2).
- ~~**EX18:** are §10.3 tests 18–22 (resource-manager load order) in scope for `engine/formats`? [out of scope]~~
  **ANSWERED 2026-10-14**: no original behaviour; the implementer's choice stands. The tests check the resource
  manager and startup registration (§3, §3.1), not the package reader, so they belong to whichever module
  implements §3.1 (data-formats.md §10.3).
- ~~**EX19:** how is depth counted for `MAX_PARENT_DEPTH`? [at most 32 nested loads below the record; a record that
  fails only on depth is retried when reached shallower]~~ **ANSWERED 2026-10-14** from the exe (`0x009732EB`):
  no original behaviour for the limit, because the original has none (§4.4); the implementer's choice stands.
  The retry half is consistent with the original, which caches only successful loads, so a failed key is loaded
  again on the next request (data-formats.md §4.4).
- ~~**EX20 — §5.2:** a raw accessor on a type mismatch. [choice: a missing value]~~ **ANSWERED 2026-10-14** from
  the exe (`0x0060CE80`, `0x005FD610`–`0x005FD640`): the original does not check. It returns the stored bytes reinterpreted,
  such as `0x40300000` for a Float32 2.75 read as Uint32, or a heap address for an array. That is garbage
  for the caller, so the *[choice]* of a missing value stands (data-formats.md §5.2).

## From the protection glue and grid shadow (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/protection.md` (fire and police) and its `docs/abi` rows. The glue's current answer is in brackets.

- ~~**PR1 — §5 dispatch record:** "callID" is taken as the record's `+0x08` field (vehicle slot) and "active" as the
  `+0x10` flag (state 5). [inferred]~~ **ANSWERED 2026-10-14** from the exe (`GetDispatchInfo` `0x00469D40`,
  `InitDispatch` `0x00469E50`, the active setter `0x00469BE0`, `EndDispatch` `0x0046ACE0`; `UpdateUnadjusted`
  `0x006F5D94`; end handler `0x006F5A40`) and written into protection.md §5. **Confirmed; the bracket matches.**
  - **`+0x08`** is the unit's call ID (`base + i`), from the source's dispatch call or the pylon's
    `GetCallNumber`.
  - **`+0x10`** is a byte set to (dispatch state == 5) by `0x00469BE0`. It posts active or inactive on a change,
    and `EndDispatch` clears it.
  - **Also.** The record is 44 bytes, copied with no AddRef. `+0x14`–`+0x1C` is the position and `+0x20` the
    source station. `EndDispatch` already revokes the unit, so the simulator's own revoke on the end message fails
    harmlessly.
- ~~**PR2:** the dispatch automaton is handed to the core by a `cISC4Occupant` (`0x26BB767E`) QueryInterface, treated
  as borrowed. Right identity? [as stated]~~ **ANSWERED 2026-10-14** from the exe (`UpdateUnadjusted` `0x006F5DB1`;
  `InitDispatch` `0x00469E50`) and the shipped exemplars, and written into protection.md §7.2 and §5.1.
  **Borrowed is right; "automaton" is the wrong identity.** The occupant passed to `GetDispatchEffect` is the
  record's `+0x20`, the **source station** the unit left from, already a `cISC4Occupant*`. The original does no QI
  and no AddRef. The dispatch effect properties (`0x29233E9A–C`, `0x48D71ED9–DB`) exist only on the shipped station
  exemplars. A QI for `0x26BB767E` returns the same object plus a reference, which the glue must release.
- ~~**PR3:** `AddControllersFromOccupantGroup(name, occ, 0)` and its Remove: the name as an `SC4String` gives slots 10
  and 12. Right overload? [yes]~~ **ANSWERED 2026-10-14** from the exe (call sites `0x006ADD5E`/`0x006ADD65`,
  `0x006E19BD`/`0x006E19C4`, `0x006E1C46`; slots 10/12 `0x00517150`/`0x0051AB00`; group lookup `0x0051CFC0` →
  `0x0051C9F0`) and written into protection.md §6.3 and §14.
  - **The overload: yes, the bracket matches.** Slots 10 and 12 are the by-name overloads. Slots 9 and 11 take a
    `u32` group ID.
  - **Caveat: the string layout.** The lookup reads the argument's string directly as the game's `cRZString`
    layout: begin pointer at `+0x04`, end at `+0x08`. It makes no virtual call. An `SC4String` works only if it
    has exactly that layout. A class with an inline small-string buffer at `+0x04` makes the game dereference
    text as an address and crash.
  - **How to call it.** Build the name with the game's own `cRZString(const char*)` (`0x00408480`) and its
    destructor, or with a layout-identical struct, as the original does.
  - **Testing.** The path runs only when a station starts or ends a strike, or on a jailbreak start, so a quiet
    test city may never reach it.
  - **The glue is safe.** The coordinator checked the glue: its `SC4String` has exactly this layout (begin, end,
    capacity, refcount; 0x14 bytes; always a heap buffer). So there is no crash risk. The layout goes into
    `docs/abi/layouts.md`.
- ~~**PR4:** the fire simulator getter is city slot 57, inferred from the SDK order; the shadow depends on it. §14 lists
  only police slot 65. [57]~~ **ANSWERED 2026-10-14** from the exe (slot 57 body `0x006DC690`; city new-city path
  `0x0045ADF7`; city `Read` `0x0045CEDB`; cache fill `0x00601E8A` → `0x00B43D78`) and written into protection.md §1.
  **Confirmed; the bracket matches.** Slot 57 (`+0xE4`) returns city `+0x174`. The city creates that member as the
  fire class `0x2990C05A` with IID `0x88F54C7B`, and loads it from component ref #25. The neighbouring getters follow
  the component order: slot 55 crime `+0x16C`, 56 demand, 57 fire, 58 flammability, 59 flora, 60 industrial, 61
  land value.
- ~~**PR5:** is there a read-only way to get the original's unadjusted int16 grid? [no; only the adjusted grid is
  compared]~~ **ANSWERED 2026-10-14** from the exe (slot 5 `0x006EFA50`; fire `Init` `0x006AE304`, police `Init`
  `0x006E28A0`; `ProtectionInit` `0x006F6C18`–`0x006F6CDE`) and written into protection.md §2. **No interface returns
  it, but there is a read-only way.**
  - **Where it lives.** Both original classes keep the unadjusted `int16` grid pointer at primary-interface
    offset `+0x84`, next to the adjusted one at `+0x80`.
  - **How to read it.** A shadow that knows it is facing the original class can read that pointer, borrowed, and
    use the grid's read-only slots. Each save also holds the grid as its own record.
  - **Spec correction.** The original builds both grids for a new city directly (global heap, inlined
    constructors), not through GZCOM.
- ~~**PR6:** city slot 97: interfaces.tsv says ST0, §14 lists only its outputs. [outputs used, ST0 discarded]~~
  **ANSWERED 2026-10-14** from the exe (body `0x00459440`; constant `0x00A84D2C` = 0.5; cell size at city
  `+0x108`/`+0x10C` = 16.0, set at `0x0045A13C`; all 83 indirect `+0x184` call sites) and written into protection.md
  §14. **The bracket matches in effect, but there is no ST0 to discard.**
  - **No return value.** The function returns nothing: both `FSTP`s pop, so the x87 stack is empty at `RET 0x10`.
    The `x87` return in interfaces.tsv is wrong, and no caller reads ST0.
  - **Arithmetic.** `px = (x + 0.5) × 16`, `pz = (z + 0.5) × 16`. Both steps are exact; for example, cell (10, 20)
    gives (168.0, 328.0).
  - *[choice]* Declare it `void`. A float return would pop an empty register, giving NaN and setting the
    invalid-operation flag.
  - **Slot 96** (`0x00459410`, the corner) is the same without the `+ 0.5`.
- ~~**PR7:** which slots does the game call from its second thread, and are these fallbacks acceptable: lists return 0
  untouched; chance of strike, efficiency and radius return 0; strike length 0xFFFFFFFF with no date, else 0; scalars
  from the month-end snapshot? [as stated]~~ **ANSWERED 2026-10-14** from the exe (all 79 reads of the fire and police
  globals `0x00B43D78`/`0x00B43D98`, by containing function; the ambience condition evaluators
  `0x004E2AA0`–`0x004E2CB0`) and written into protection.md §13.1. **None is known; no original behaviour; the
  implementer's choice stands.** Every reader of the protection globals is main-thread code: simulator agents, view
  input controls, UI windows, automata, the disaster layer, the advisors. The measured off-main caller (`IsPaused`,
  foundations §19.2) fits the ambience sound system, whose conditions read only the simulator and view globals. The
  original returns live values with no locking. If the fallbacks ever fire, they differ from it. A `Mutex` that
  answers exactly from any thread would be closer than fallbacks. *[inferred: no thread trace; static reachability
  through the globals only]*
- ~~**PR8:** slots 21/22/32, dispatch-source slot 6 and slot 42 (non-empty) clear the caller's list without releasing
  its elements, which leaks the caller's references as the original does. Intended? [reproduced]~~ **ANSWERED
  2026-10-14** from the exe (shared helper `0x006DFC70`; list clear `0x00413E60`; every original caller) and written
  into protection.md §15.4 and §16 quirk 7. **Confirmed; the bracket matches.**
  - **What the helper does.** It clears the list with the plain list clear, which frees the nodes and never calls
    `Release`. Then it appends the simulator's list, calling `AddRef` on each element.
  - **Original callers are not affected.** Every one passes a fresh empty list and calls `ReleaseOccupantList`
    afterwards: `0x00416189`, `0x004164B1`, `0x007A1A8E`, `0x007A1AF0`, and the dispatch manager at `0x0046AFA2` and
    `0x0046B116`. So the leak happens only for a plug-in caller that passes a non-empty list.
  - *[choice]* Reproduce it. Releasing instead would over-release a list of borrowed pointers.
- ~~**PR9:** the property setters `0x005FE3C0` / `0x005FE6B0`: scalar u32 or a one-element vector? [open]~~
  **ANSWERED 2026-10-14** from the exe (setters `0x005FE3C0`/`0x005FE6B0`; variant slot 26 `0x0091587D` and slot 102
  `0x00915014`; every protection call site) and written into protection.md §14.
  - **`0x005FE3C0`** writes a **scalar** `Uint32` (variant type 6), not a one-element vector. It is used for the
    strike start, the availability masks, the base call numbers, the jail population and the air sources'
    properties.
  - **`0x005FE6B0`** writes a `Uint32` **array** of the vector's length (type `0x8006`, elements copied). It is
    used only for the station statistics `0x8A70A0B9`, always 4 elements.
  - **Both** add the property when it is missing and replace the variant's type.
  - **Quirk.** The lookup falls back to the occupant's exemplar, so an ID defined in a plug-in exemplar would be
    changed there. No shipped exemplar has any of these IDs. *[choice]* Write onto the occupant.
- ~~**PR10 — §5.3:** the zone manager's slot-5 grid class is not in docs/abi; `GetZoneType(x, z)` (slot 12) is used
  instead, 0 outside the city. Same values? [slot 12]~~ **ANSWERED 2026-10-14** from the exe (slot 5 `0x00698CF0`
  returns `+0x48`; created in zone manager `Init` at `0x004DE26F`–`0x004DE2D0`; `GetZoneType` `0x004DC740`; tract
  test `0x006F6180`) and written into protection.md §5.3. **Same values; the bracket matches inside the city.**
  - **The grid.** It is a `cSC4SimGrid<int8>` (vtable `0x00A87950`, CLSID `0x49B9E603`, tract size 1).
  - **Same byte.** `GetZoneType` reads the same byte of the same grid, sign-extended. So "≠ 0" agrees for every
    in-grid cell.
  - **Outside the grid.** Neither the original's raw read nor `GetZoneType` checks bounds, so there is no original
    value. "0 outside" is the implementer's choice, and it never triggers for grids the game creates.
- ~~**PR11 — §12:** "void1 = the simulator" posts the primary interface pointer. Right pointer? [primary]~~
  **ANSWERED 2026-10-14** from the exe (cycle-end posts fire `0x006AD3A3`, police `0x006E0616`; QIs fire
  `0x006ACEFE`, police `0x006E008E`; police posts `0x006E0B92`, `0x006E166F`, `0x006E1DD8`, `0x006E1BFA`,
  `0x006E1D55`; message slots 12 `0x00913322`, 33 `0x0091346B`) and written into protection.md §12. **Confirmed;
  the bracket matches.**
  - **The pointer.** Every "the simulator" field is the primary interface pointer (vtables `0x00AB1160` /
    `0x00AB2490`).
  - **How it is obtained.** At a cycle end it comes from `QueryInterface(1)`, which returns the primary itself and
    is released after the post. The station and jailbreak posts use `this` or `this − 8` directly.
  - **How the message holds it.** void1 is stored raw. IGZUnknown is a counted reference.
- ~~**PR12 — §4.1:** the simulator, flammability and crime simulators are not held (never called); only reference
  counts differ. Acceptable? [not held]~~ **ANSWERED 2026-10-14** from the exe (fire `Init` `0x006AE2D4`–`0x006AE2ED`,
  police `Init` `0x006E285E`–`0x006E286F`; `Shutdown` fire `0x006AD222`, police `0x006E03AF`; `ProtectionInit`
  `0x006F6BC7`–`0x006F6E09`) and written into protection.md §4.1. **Acceptable; the bracket matches.**
  - **Flammability and crime.** The original holds each at `+0xCC` from `Init` to `Shutdown`. No other code in
    either class reads it, at any sub-object offset.
  - **The simulator.** The original does not hold it at all: it reads the cache global `0x00B43D5C` for
    `AddAgent`.
  - **Spec correction.** Base init holds the city, the **budget simulator** and the **zone manager**. It holds the
    message server and subscribes only when the server is not held yet.
  - **Effect of not holding.** Only the reference counts, and so the objects' lifetimes, differ. Neither class can
    observe that.
- ~~**PR13 — §13.2:** the air sources are created through GZCOM and their Init/Shutdown called by address while our
  replacement classes are active. Intended reading? [yes; no air dispatch on an unidentified executable]~~
  **ANSWERED 2026-10-14** from the exe (fire `Init` `0x006AE323`–`0x006AE373`; police `0x006E28BE`–`0x006E290D`;
  factories `0x006ACEA0` / `0x006E0030`, registered at `0x00460B33`–`0x00460B52`; source QI `0x006AC550`) and
  written into protection.md §4.1, §8.4 and §13.2. **Yes; the GZCOM path gives the original's object.**
  - **Same object both ways.** The registered factories do exactly what `Init` does: `GlobalHeapAlloc(0x28)` plus
    the same constructor. The original itself creates the sources through the registry when it loads a save with
    a source ref. So the spec's "operator new" is corrected.
  - **Which IID to request.** Ask GZCOM for IID `0xC9C00435`, which gives the object start that `Init` and
    `Shutdown` need in ECX. The sources refuse IID 1 (`cIGZUnknown`).
  - **Unidentified executables.** Skipping air dispatch there has no original behaviour; the implementer's choice
    stands.
- Not implemented: the 14 extra message-target slots 4–17 (§15.3; no outside caller found).

## From the pathfinder core (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/pathfinder.md`. All fourteen §11 tests pass as unit tests and vectors `pf11_01`–`pf11_14`; extra
vectors `pf11_15`–`pf11_28`. The core's current answer is in brackets.

- ~~**PF1 — §4.9 step 5:** "not g < node.g → ignore" contradicts its note that a NaN g passes both tests.
  [`node.g <= g → ignore`, so a NaN g passes]~~ **ANSWERED 2026-10-14** from the exe (`AddTripNode`
  `0x006D9070`, `0x006D9085`) and written into §4.9 step 5, with a table. **The bracket is right.** Both tests are an
  ordered `node.g ≤ g`: `fcomp`, then only C0 is tested, and an unordered result sets C0. So the node is taken when
  `g < node.g` or when **either** value is NaN; a stored NaN `g` is replaced by any `g`. The old wording was wrong.
- ~~**PF2 — §4.17:** which of the record's two arrays do the "16 counters" go to? [`current`]~~ **ANSWERED
  2026-10-14** from the exe (`ApplyTraffic` `0x006DA2D3`, `0x006DA324`–`0x006DA330`; edge-density `operator[]`
  `0x006D7820` returns the value at node `+8`) and written into §4.17 step 3, test 4 and §10 Q3. **The bracket is
  right: `current`**, the record's second array at `+0x22`; `last` and `usage` are never written here. Also
  corrected: for style 2 the key-bit-1 record gets the **mirrored** index `4·exit + entry`, like `ApplyRoute`. The
  spec said "not mirrored" (also fixed in traffic-simulator.md §4.5).
- ~~**PF3 — §8:** which value does the pathfinder pass for `GetLot`'s bool? [left to the host]~~ **ANSWERED
  2026-10-14** from the exe and written into §8 and §4.6. **Always false** (`push 0` before slot 6 at `0x006D8B1D`
  lot start, `0x006D7AC7` filter-0 goal, `0x006D8688` §4.5 step 5). The bool is `originOnly` (zones.md §8), so any
  cell of a lot finds it. The host should pass false.
- ~~**PF4 — §4.8.3 step 3:** `conn[t]` is shifted by the node's entry side `s`, not the loop's `s2`. [literally
  `s`]~~ **ANSWERED 2026-10-14** from the exe (`FindPath` `0x006D99D1`–`0x006D99E2`) and written into §4.8.3 step 3.
  **The bracket is right: `s`**, the popped node's own side, re-read in each pass of the `s2` loop. The test does
  not depend on `s2`, so all four nodes are added or none. The branch stays unreachable (no code sets wormhole
  bit 1).
- ~~**PF5 — §4.6 fallback:** several links of one connection match the start cell: which link cost? [the
  first]~~ **ANSWERED 2026-10-14** from the exe (`CreateStartNodes` `0x006D8B54`–`0x006D8F8E`; terminal layout
  from `DoFerryTerminalActivated` `0x0072392C`–`0x0072398B`) and written into §4.6. **The bracket is wrong: the
  last.** Nothing ends the loops and nothing checks for an existing node, so each matching link overwrites `g`/`f`
  of the same start node and pushes it again (the heap holds it once per match). Also corrected: the loops run over
  the **ferry terminals** and their neighbour links (traffic-network.md §2.6), not over neighbour connections, and
  the start node is at the terminal's **arrival cell**.
- ~~**PF6:** neighbour-connection link lists and `GetTrafficNeighborConnection` costs are not in the traffic network
  core. [host methods]~~ **ANSWERED 2026-10-14** from the exe (`0x0070F970`, `0x007104A0`, `0x007114E0`, Add
  `0x007153C0`, Remove `0x00713AB0`, sweep `0x007138B0`, capacities `0x00714E60`, roll `0x00714BE0`) and written
  into traffic-network.md §2.7.1–§2.7.5. **The bracket is right: host methods** of the traffic simulator, now
  specified. The "link lists" are the ferry terminals' neighbour links (PF5, §2.6). The cost is
  `max(cost, f)` per Add (f = the other side's `0xCA947077` average trip length, else commuteMaxTime), never
  lowered. The lookup never fails, so pathfinder.md §4.13 step 3 always adds it. Also corrected: the per-travel-type
  arrival counts go on the connection's own holder, not the pool (traffic-simulator.md §4.2).
- ~~**PF7:** do exit nodes, closed without expansion, advance the work counter? [yes; test 13 holds either way]~~
  **ANSWERED 2026-10-14** from the exe (`FindPath` `0x006D9540` → close `0x006D9C81`, counter `0x006D9C97`) and
  written into §4.11. **The bracket is right: yes.** Every popped node that is not a goal reaches the one close,
  which advances the counter when a max work time is set; an off-map node jumps there without expansion. A goal
  node does not count, and with max work time 0 the counter stays 0.
- ~~**PF8 — §4.18:** which cell gives an exit node's H when its score is recomputed? [its own off-map cell]~~
  **ANSWERED 2026-10-14** from the exe (`RecalculateNodeScores` `0x006D67FF`/`0x006D6825`; key decode `0x006D5DF0`)
  and written into §4.18 (with an example) and §4.7. **The bracket is right: its own off-map cell**, the node's
  stored cell, decoded from the key (low bytes zero-extended, plus `dir(side)` for bit 31). It differs from the
  in-city cell AddTripNode used, so the recomputed `H` is usually off by one. Also noted in §4.8.4: the park-and-ride
  target is never off-map (the old "lands on P" remark was unreachable, and wrong at the −x/−z edges).
- ~~**PF9 — §4.9:** Update of an open node whose heap position is stale. [no-op unless the heap slot holds it]~~
  **ANSWERED 2026-10-14** from the exe (`UpdateNode` `0x006D6460`, `Push` `0x006D88B0`, `Pop` `0x006D6380`) and
  written into §4.7 ("Position"). The original does **not** check: it sifts up from the slot the node was last
  written to and writes the node at the end, overwriting whatever the slot held. But that case never happens in
  the game: the only open node with a stale slot is the one being expanded, and no move produces its own key.
  **The bracket stands as a [choice]**; it is equivalent in every reachable flow.
- ~~**PF10 — §4.15:** `GetNeighborConnectionInnerCell` fails for an off-map node. [the node's own cell, low
  bytes]~~ **ANSWERED 2026-10-14** from the exe (`0x0070DE01`/`0x0070DE10`; `GetCompressedRoute` `0x006D71E2`,
  `0x006D7550`) and written into §4.15. On failure (a corner cell) the function writes nothing, and the original
  emits the low bytes of two uninitialised or stale stack variables. **It cannot happen in the game:** the root is
  always in city and jumped-to off-map nodes are ferry exits with a valid inner cell. **The bracket stands as a
  [choice].**
- ~~**PF11:** FindPath on an object that is not initialised. [returns 0, does nothing]~~ **ANSWERED 2026-10-14**
  from the exe (`FindPath` has no initialised test; constructor `0x006D89C0` leaves W, H and the table bases unset;
  `Shutdown` `0x006D5D63` nulls the bases; first write `0x006D6060`) and written into §4.3. The original faults or
  writes through garbage in `SetupDistanceTables`. It is unreachable (`CreatePathFinder` always runs `Init`).
  **The bracket stands as a [choice].**
- ~~**PF12:** `LengthOfLine` precision. [the exact square root, rounded once to float]~~ **ANSWERED 2026-10-14**
  from the exe (`0x005D7660`: three `fsub`, three `fmul`, two adds, `fsqrt`, all x87 at 24-bit precision; called
  from `DistanceToRectangle` `0x006D5F01` etc. with `y = 0`) and written into §4.12 with examples. **The bracket is
  right.** Every step before the root is exact for cell coordinates, and `fsqrt` under 24-bit precision control is
  correctly rounded.
- ~~**PF13:** a switch nibble naming a travel type above 8. [ignored]~~ **ANSWERED 2026-10-14** from the exe
  (`FindPath` `0x006D97D1`–`0x006D97DD`) and written into §4.8.2 (b). The only gate is bit `t2` of the u16 allowed
  mask, and no game caller sets a bit above 8, so the original ignores such a nibble in practice; with such a bit
  set it would index its 9-entry tables past their ends. **The bracket is equivalent and stands as a [choice].**
- ~~**PF14:** distance-table indices outside −1…W / −1…H (unreachable). [the formula]~~ **ANSWERED 2026-10-14**
  from the exe (`H` `0x006D6160`, `SetupDistanceTables` `0x006D6040`, `RecalculateNodeScores` `0x006D6825`) and
  written into §4.4 ("Index range"). **Confirmed unreachable:** every cell whose `H` is read is in city or one step
  off it. The original has no check and would read the neighbouring heap. **The bracket stands as a [choice].**
- ~~**PF15:** the entry side of a ferry neighbour-link exit node. [`EdgeTo(off → edge)`; assumes adjacent
  cells]~~ **ANSWERED 2026-10-14** from the exe (`FindPath` `0x006D9B55`–`0x006D9BA5`) and written into §4.8.3
  step 4, with an example. **The bracket is right:** the call passes the in-city edge cell, side `EdgeTo(off →
  edge)` and the exit flag, so the node lands one step from the edge cell towards the off-map cell. Every link the
  game makes has adjacent cells (the edge cell is `GetNeighborConnectionInnerCell(off)`), so that is the off-map
  cell; only a damaged save could break it.
- Deviations from the compiled interface (choices): `SetStartingCostArray` copies the 9 floats; `SetRouteInfo` keeps
  its own route list; the process-wide work-check values are one shared cell.

## From the crime glue and grid shadow (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/crime.md` and its `docs/abi` rows. Ini: `enable = crime` (optional `crime_seed`); shadow
`crime_shadow = 1`. The glue's current answer is in brackets.

- ~~**CQ1:** `GetCrimeIncidentArray`: allocators.md does not list its container. [the game's allocator]~~
  **ANSWERED 2026-10-14** from the exe (`0x006A1960`, resize `0x006A1380`, growth `0x0069FE90`; caller
  `0x00553370`) and written into crime.md §11.4 and §14.2. **The bracket is right, and the allocator is B** (the
  STL node allocator, `0x0090CF54`/`0x0090CF63`): `out` is a default-allocator `std::vector`. The only caller
  passes an empty vector and frees the buffer with B. New elements of lot-less incidents are a fill value with a
  zero icon key and 16 uninitialised bytes; *[choice]* zero-fill. allocators.md needs a row (coordinator).
- ~~**CQ2:** `PropProfile.crimeIncidentID` is read at offset 24, from the SDK layout; layouts.md does not list
  `PropProfile`. [24]~~ **ANSWERED 2026-10-14** from the exe (`SelectProp` `0x006A016B`–`0x006A016E`: prop
  manager slot 13, then `mov eax, [eax+0x18]`) and written into crime.md §11.6 and §13. **The bracket is right:
  24** (`u32`). Nothing else of the profile is read by crime.
- ~~**CQ3:** lot `GetCenter` (slot 15): docs/abi says ST0, §13 says the out parameter with nothing left on the x87
  stack. [both handled; the month line counts which one happens as `center st0=a/b`]~~ **ANSWERED 2026-10-14**
  from the exe (`cSC4Lot::GetCenter` `0x006BF560`, the only implementation) and written into crime.md §13. **§13
  is right; docs/abi is wrong.** The result is only in the out parameter, and EAX = `&out`. Four `fild`s feed two
  `fstp` stores, and the last value is popped (`fstp st(0)`, `0x006BF5B6`), so the **x87 stack is empty** on
  return. Declare it returning the pointer (or void), never `float`: popping ST0 after the call would underflow
  the x87 stack. The glue's ST0 branch should never fire. The abi row is the coordinator's to fix.
- ~~**CQ4:** `GetOccupantsByBBox`: §13 names the filter overload (slot 36). [the typed overload, slot 37, last
  argument 0xFFFFFFFF]~~ **ANSWERED 2026-10-14** from the exe (`GetNearestCrimeSim` `0x006A121D`,
  `GetNearestCrimeProp` `0x006A0F7D`; filters `0x0069F4D0`/`0x0069F490`; slot 37 `0x0049DD40`; the manager's
  typed filter `0x004BDF90` → `0x005E9740`) and written into crime.md §11.3 and §13, closing §16 Q5. The original
  calls **slot 36** with a per-call filter object of its own (type plus group, or type plus property) and flags
  `0xFFFFFFFF`. **The bracket is equivalent**, as long as the group or property test is applied to each listed
  occupant before it can become a candidate. Slot 37's filter is `GetType() == type`, and the walk and order are
  the same.
- ~~**CQ5:** the session seed "local wall-clock seconds". [`_mkgmtime(localtime(now))`]~~ **ANSWERED 2026-10-14**
  from the exe (static initializer `0x00A74B70`; `cRZTime::cRZTime` `0x00470B50`) and written into crime.md §9.
  **The bracket is not the original value.** The seed is `cRZTime()` XOR `0x6C09C3F6`, and `cRZTime()` is u32
  seconds since **1901-01-01 00:00 UTC**, rebuilt from the `localtime` fields with `_timezone` and the library's
  fixed old-US daylight rule. That is `time() + 2177452800` when the host's daylight rule agrees, one hour off when
  it does not, and 0 in the library's spring-forward hour. The bracket counts local seconds from 1970 instead. No run
  can observe the seed, so this is a *[choice]*: crime.md §9 recommends `(uint32)(time(NULL) + 2177452800) XOR
  0x6C09C3F6`.
- ~~**CQ6 — §16.1:** the ordinance crime factor after a load with no ordinance simulator. [left unchanged]~~
  **ANSWERED 2026-10-14** from the exe (crime `Init` step 6; OrdinanceUpdated handler `0x006A22D5`; the only poster
  of `0xC2C09554` is `SetOrdinanceOn`, `0x0080D714`) and written into crime.md §4.1 and §16 Q1. **No original
  behaviour; the implementer's choice stands.** With no ordinance simulator the original crashes in `Init` step 6
  (the AddRef has no null check), before any month. "Left unchanged" means 1.0, the value `Init` step 9 sets. §16 Q1
  is closed: nothing posts OrdinanceUpdated after a load. Even with an ordinance simulator, a loaded city runs at
  1.0 until some ordinance's on flag changes. Then the handler re-reads the factor over all on ordinances.
- ~~**CQ7:** the prop requester is never unregistered at Shutdown; the spec is silent. [not unregistered]~~
  **ANSWERED 2026-10-14** from the exe (crime `Shutdown` `0x006A1860`; prop developer `AddPropRequester`
  `0x006F21C0` and its `Shutdown` `0x006F2F70`–`0x006F2FCE`) and written into crime.md §4.2. **The bracket
  matches.** Crime's `Shutdown` makes no prop-developer call. The prop developer AddRefs each requester on
  registration (no duplicate check) and releases each one once at its own `Shutdown`, which empties the list. So
  the crime object lives until then.
- ~~**CQ8 — §16.8:** crime type names: which LTEXT key, and the fallback when it is missing. [open]~~
  **ANSWERED 2026-10-14** from the exe (`LoadCrimeTypeFromExemplar` `0x006A0D12`, `SC4LoadExemplarString`
  `0x00603270`/`0x00603220`, `SC4LoadString` `0x00603040`) and the 13 shipped language packages, and written into
  crime.md §7, closing §16 Q8.
  - **The key.** Values [1] and [2] of `0x8A416B2E` are the LTEXT group G and instance I; [0] is ignored. The
    lookup is (`0x2026960B`, G + language ID, I), then (`0x2026960B`, G, I).
  - **The text.** It is detokenized in place, then U+2022 becomes U+200B.
  - **The fallback.** On failure the inline string `0xAA12FCB0` is used; without it the name stays empty.
  - **Shipped data.** Only `0xEA5EA698` exists, in every language, so Generic gets an unrelated building text and
    the other 13 types are empty.
  - **Choice.** Reproduce the steps, or call the game's `SC4LoadString`.
- ~~**CQ9:** `Read` on a stream error. [returns false, no state changed]~~ **ANSWERED 2026-10-14** from the exe
  (`cSC4CrimeSimulator::Read` `0x006A05E0`) and written into crime.md §12. **The bracket is not the original but
  matches the §12 *[choice]*, which stands.** The only early exit is the version test. On a stream error the
  original reads on and sets `loadedFromSave`, then returns false. It nulls the grid refs, resizes the iterator
  table to the stream pointer's value, and loops over an uninitialised incident count, so it reads garbage. The
  following `Init` then leaves the simulator unusable.
- ~~**CQ10:** crime type list order. [ascending instance]~~ **ANSWERED 2026-10-14** from the exe
  (`LoadAllCrimeTypesFromExemplars` `0x006A1450`, `GetAvailableResourceList` `0x009739CB`, the package hash-table walk
  `0x0096F9AB`) and already written into crime.md §7 ("Order of the list"). **The bracket matches the original for
  the shipped types.** The original uses the resource manager's key-list order. That order is segment order, and
  within each package the index hash-table buckets (`instance mod B`, B ≥ 53). All 14 shipped types are in
  `SimCity_1.dat` with instances below 53, so the order is ascending instance. Plug-in types can land elsewhere;
  ascending instance stays the §7 *[choice]*.
- ~~**CQ11:** police slot 5 returns a uint8 grid (the SDK says short) and slot 27 an int32 count, as protection
  declares them. Confirm. [as protection]~~ **ANSWERED 2026-10-14** from the exe (slot 5 `0x006EFA50`, slot 27
  `0x006E0A00`) and written into crime.md §13. **Confirmed; the bracket matches.** Slot 5 returns the adjusted grid
  pointer at `+0x80`, the `uint8` grid, without an AddRef. Slot 27 returns its int32 count in EAX, after a float→int
  conversion.
- ~~**CQ12:** arson calls `SetCause` whatever `IgniteOccupant` returns. Right? [yes]~~ **ANSWERED 2026-10-14** from
  the exe (`0x006A1D07`–`0x006A1D10`) and written into crime.md §6.8. **Yes; the bracket matches.** AL from
  `IgniteOccupant` (slot 4) is not tested. `SetCause(2)` (slot 9) follows unconditionally on the same manager.
- ~~**CQ13:** property 0xCA5B9306 is read with numeric conversion (`GetU32s`, low bytes). [as stated]~~
  **ANSWERED 2026-10-14** from the exe (`DoOccupantChanged` `0x0069F27B`–`0x0069F2B1`; variant slot 95 `0x0060CE80`)
  and written into crime.md §6.10. **The bracket differs from the original but is equivalent for shipped data.** The
  original takes the variant's raw data pointer whatever its type, and reads bytes 0 and 1 with no conversion and no
  checks. That equals `GetU32s` + low bytes only for byte arrays, and all 6 shipped buildings are `Uint8` ×2. A
  `Uint32` array {100, 8} gives m = 100, r = 0. A scalar is dereferenced as a pointer, which crashes or reads
  garbage. *[choice]*: read raw bytes 0 and 1, and skip scalars and arrays shorter than 2 bytes.
- ~~**CQ14:** a failed QueryInterface nulls the out pointer per crime.md §1; flammability, plumbing and pollution leave
  it untouched. Which is the original's rule per class? [crime nulls]~~ **ANSWERED 2026-10-14** from the exe (crime
  `0x0069F1FC`, flammability `0x006AE3D5`, plumbing `0x006DA405`, pollution `0x006E2D1C`; 64 implementations
  checked) and written into crime.md §1, which was wrong. **The bracket is wrong.** Crime returns AL = 0 and leaves
  the out pointer **untouched**, like flammability, plumbing, pollution and every other simulator class. The only
  class seen nulling it is `cSCResExemplarCohort`, on entry (`0x0067C15D`). One rule for all simulators: false with
  the out pointer untouched.
- ~~**CQ15:** the return value of the residential `GetPopulationGrids` / `GetEQGrids` is ignored; only the three out
  pointers are used. [ignored]~~ **ANSWERED 2026-10-14** from the exe (`DoCensus` `0x0069FB24`/`0x0069FB3E`) and
  written into crime.md §13. **Confirmed; the bracket matches.** EAX is never read after either call (the next
  instruction clears EBX and the loop reads the out pointers). The order is `GetPopulationGrids` (slot 25) first,
  then `GetEQGrids` (slot 23).
- Not compared by the shadow (stated on every line): the number of crimes to commit (needs
  `ResetUnjailedCriminals`, which has side effects), which attempts the police stopped and each crime's type (the
  original's draws), which lots were tried (the lot manager's generator), the crime-effect grid (no interface).

## From the demand glue: capacity maps and population summary (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/demand.md` (§6.3 capacity, `GetLocalPopulationSummary`) and the STLport map layout in
`docs/abi/allocators.md`. Switches `demand_capacity` and `demand_direct_calls` (both on by default). The glue's
current answer is in brackets.

- ~~**DQ3:** a NaN or out-of-range summary value: Rust `as i32` saturates and gives 0 for NaN; the game's CRT
  conversion gives 0x80000000. Which does the original produce? [saturating]~~ **ANSWERED 2026-10-14** from the exe
  (`GetLocalPopulationSummary` `0x006A6013`/`0x006A6018`; `_ftol` `0x009EEF04`) and written into demand.md §5.
  **Neither: the original wraps, and the bracket matches only for NaN and in-range values.**
  - **How the CRT converts.** It truncates to a **64-bit** integer, and the summary keeps the low 32 bits.
  - **NaN, ±∞ and |v| ≥ 2^63** give 0. The integer indefinite is `0x8000000000000000`, whose low half is 0, so the
    result is not `0x80000000`.
  - **2^31 ≤ |v| < 2^63** wraps modulo 2^32. For example, 3e9 → −1294967296 and 2^31 → −2147483648, where
    saturation gives 2147483647. +∞ gives 0, not `i32::MAX`.
  - *[choice]* Reproduce it: 0 when `v` is NaN or |v| ≥ 2^63, else `(v as i64) as i32`.
- ~~**DQ4:** a map with count 0 but a non-empty tree shape. The spec only says "with a count of 0, the map is left
  untouched". [refused]~~ **ANSWERED 2026-10-14** from the exe (clear `0x006A5F95`–`0x006A5FB7`; the writers of
  regional city `+0x3C`: constructor `0x004AC09E` and the reader's `operator[]` at `0x004AC841`/`0x004AC8A1`) and
  written into demand.md §5. **No original behaviour; the implementer's choice stands.**
  - **Unreachable in the game.** Every writer keeps the count equal to the number of nodes.
  - **What the original would do.** It skips the clear, keeps the stale nodes and inserts into the stale tree,
    overwriting matching keys in place. The count ends as the number of new nodes only.
  - **Verdict.** That is a corrupt map, so refusing it is fine.
- ~~**DQ5:** the census asks for the profile at every cell, so the capacity walk runs there too, although §6.3 needs
  only the purpose. Intended? [walks at every cell]~~ **ANSWERED 2026-10-14** from the exe (`PerformCensus`: BDS
  slot 39 at `0x006A5A53`, `purpose` read at `0x006A5A5D`) and written into demand.md §6.3. **Not intended: the
  original does no walk.**
  - **What the original does.** At each cell that passes the lot tests, slot 39 returns a reference into the
    profile table, and only `purpose` (+0x00) is read. Nothing is copied or walked.
  - **The glue's walk.** Walking the capacity maps per cell has no original counterpart. It is harmless while it is
    read-only.
  - *[choice]* Read `purpose` only, and walk the maps only in `UpdateCapacity` (§7.2).
- ~~**DQ6:** the header colour of a copied capacity map is assumed to be 0. [colour 1 refused]~~ **ANSWERED
  2026-10-14** from the exe (map copy constructor `0x004C9470`, colour write `0x004C9499`; called from the vector
  copy `0x004CDF5F` under the profile copy `0x004D1230`) and written into demand.md §7.2. **Confirmed: 0.** The copy
  constructor writes colour 0 into its new header before copying the nodes, as every other map constructor does,
  and nothing changes it later. Colour 1 never occurs in the game, so refusing it is the implementer's choice, and
  that choice stands.
- ~~**DQ7:** limits of 16 maps, 128 capacity entries and 4,096 summary nodes. Are larger real maps possible? [refused
  above them]~~ **ANSWERED 2026-10-14** from the exe (`LoadBuildingProperties` pushes `0x0069BC59`/`0x0069BC7E`) and
  every shipped package (property `0x27812834`; demand exemplars) and written into demand.md §7.2. **Not in the
  shipped game; the implementer's choice stands.**
  - **Capacity maps.** A profile holds at most 1 map, by construction.
  - **Capacity entries.** All 558 shipped capacity properties carry exactly one (key, amount) pair, so each map
    has 1 entry.
  - **Summary nodes.** The shipped summary has 40 nodes.
  - **What could exceed the limits.** The original sets no limit, so only plug-in data could: an exemplar with more
    than 128 distinct capacity keys, or more than 4,096 demand nodes with region use.
- ~~**DQ8:** which thread calls `GetLocalPopulationSummary`? [unspecified; the first success is logged with its thread]~~
  **ANSWERED 2026-10-14** from the exe (regional city `DoMessage` `0x004ABD40`, calls `0x004ABE76`/`0x004ABE9B`;
  senders `cSC4App::NewCity` `0x00454963`, `LoadCity` `0x004558C5`, `SaveCity` `0x00456102`) and written into
  demand.md §5. **The main thread** *[inferred: static; no thread trace]*.
  - **When it runs.** Its only caller, the regional city's `UpdateCityCache`, runs on `PostCityInit`
    (`0x26D31EC1`) and `PreSave` (`0x26C63343`) for its own city.
  - **Where those messages come from.** Only the app's new-city, load-city and save-city paths send them, and the
    game has no message dispatch thread.
  - **Settling it at run time.** The glue's log of its first call would confirm the thread.
- ~~**DQ9 — §5 `GetNeutralTaxRate` precision (in-game differential, run 73, coordinator):** with census supply
  R/C/I = 27207 / 2569.47998 / 10583.1006 (C and I fractional, now that the demand glue reads capacity), the
  original returns **8.97309399** and our core **8.97309303** (1 ulp; the demand shadow's only mismatch in 2,955
  lines, on all 27 days of that census month). Every other demand value is bit-identical. Which precision and order
  does the original use for `node[0x1001].supply + node[0x3001].supply + node[0x4001].supply` (float32 or x87
  extended, left to right?), and for the curve interpolation (foundations §4.3) on the way to the float32 return?
  [ours: as the spec reads today]~~ **ANSWERED 2026-10-14** from the exe (`GetNeutralTaxRate` `0x006A3DD0`,
  `FADD dword` at `0x006A3E39`/`0x006A3E3D`; curve `0x005F4250`) and written into demand.md §5, with a worked example
  on these inputs. **The bracket (the spec's left-to-right reading) is wrong.**
  - **The sum.** It is `float32`, in the order **(I + C) + R**: node `0x4001`'s supply stays in ST0, the `0x3001`
    local is added to it, then the `0x1001` local. Each `FADD` rounds to 24 bits at the main thread's precision.
    Both roundings here are exact ties, resolved to even: 13152.580078125, then 40359.578125 (`0x471DA794`).
  - **The curve.** It is unchanged: three `float32` operations (foundations §4.3). The result is 8.97309399
    (`0x410F91CB`), the original's value.
  - **What ours does.** Both R + C + I and one rounding of the exact sum give `0x471DA795` and 8.97309303.

## From the protection fix: saved grid refs and their geometry (filed by the coordinator from the lane's report, 2026-10-14; merged)

Run 70 found the fire replacement's grid at 0×0 after a load. The fire record's grid ref points to a grid whose own
record is read later, so the grid is still empty during the simulator's `Read`. The host grid now keeps a geometry
only once it is non-empty. The current answer is in brackets.

- ~~**HG1 — foundations (load order):** are all grid records read before any simulator's `Init`, so that the first
  non-empty geometry is the final one? Which order does the class store read records and resolve refs in, relative
  to `Read` and `Init`? [assumed yes]~~ **ANSWERED 2026-10-14** from the exe (`LoadCity` runs the store load at
  `0x0045574C` → `0x005D2080` → `0x005D38D0`, classes from a sorted map, `Read` at `0x005D3C63`; `cSC4City::Init`
  only at `0x004558A2`, after PreCityInit at `0x00455898`) and written into foundations §3.7 ("When objects, grids
  included, are read"). **The bracket matches: yes.** Every saved object is read in the store load, in ascending
  class ID and record order, before the city's `Init` runs. A reference only creates an empty object; its `Read`
  comes when the walk reaches its entry. Subsystems with class IDs below `0x49B9E602` (fire protection among them)
  see their grids empty during `Read`; all see them final by `Init`. Caveat: a grid saved as 0 × 0 stays empty.

## From applying TN1–TN20 to the traffic network core (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/traffic-network.md`. The core's current answer is in brackets.

- ~~**TN21 — §4.6 / TN5:** if the intersection search's start cell is itself a tunnel cell, is it still extended?
  [yes, the start cell is always extended]~~ **ANSWERED 2026-10-14** from the exe (UpdateIntersectionData
  `0x0071B9A0`: the depth-0 entry runs the same tests, tunnel bit test `0x0071BBB8`, empty stack → `0x0071BD98`) and
  written into traffic-network §4.6 (the start-cell bullet and a worked example). **The bracket does not match: no.**
  The start cell goes through the same checks as every other cell. A tunnel start cell is not extended, so its
  search ends at once with `best = N + 1` and it keeps its table value (a Street tunnel next to an intersection
  stays at capacity 100, not 80).
- ~~**TN22 — §4.7.2:** what count does an object-reference or empty variant give for `0xE90E25A1`? [0: a switch
  with no rules]~~ **ANSWERED 2026-10-14** from the exe (`GetCount` `0x0040CE60` reads field `+0x10`; constructors
  and `Destroy` `0x00914EA3` zero it; the object-reference setter `0x00915A5F` leaves it; the reader `0x005FFB40`
  copies `count` bytes) and written into traffic-network §4.7.2 ("Empty and object-reference values"). **The bracket
  matches.** Both give count 0, so 0 bytes are copied and DoLotActivated builds a switch with no rules (mode mask 0,
  flags 0, no access point). Only an absent property skips the switch.
- ~~**TN23 — §4.7.2 TN3/TN4:** how do the DoPostCityInit walks treat a switch with building 0 (a default inserted by
  a §4.7.3/§4.7.4 lookup), where the original would use a null building? [not re-read; flag 4 clear still clears the
  access point; flag 4 set gets no NW cell]~~ **ANSWERED 2026-10-14** from the exe (re-read dereferences the
  building at `0x00715CCF`; access walk calls its slot 11 at `0x00720504`; neither checks it; the map lookup
  `0x006D90F0` copies an uninitialised temporary, zeroing only `+0x18` and the rules at `0x006D9145`–`0x006D9155`)
  and written into traffic-network §4.7.2 ("A switch without a usable building"), §4.7.3 step 1 and §11 test 10.
  **The premise does not hold, and the bracket stands as a *[choice]*.** A null building crashes the original. An
  inserted switch's building is not null but a stale stack value, and so are its load, capacity, flags and other
  fields. FreeCity4's all-zero switch is not re-read; flag 4 clear clears the access point; flag 4 set keeps it as it
  is. The §11 test "unknown ID → `0x7EFFFFFF`" is now marked *[choice]*.
- ~~**TN24 — §4.10.1 TN15:** when the unclamped dirty rectangle reaches past the map, are off-map cells asked or
  written during the update? [only in-map cells]~~ **ANSWERED 2026-10-14** from the exe (UpdateDepthMap `0x00724840`:
  no clamp and no bounds check; `IsNavigable` `0x0073EBF0` returns false through `CellIsInBounds` `0x004594D0`; the 0
  is stored at `0x007249A6`–`0x007249B2`) and written into traffic-network §4.10.1 ("The update"). **The bracket
  stands as a *[choice]*.** The original asks about every cell of the rectangle and writes 0 for each off-map cell
  through the raw rows. That write lands outside the grid, so it is an out-of-bounds heap write. In-map cells get
  the same values either way.
- ~~**TN25 — §4.7.2 TN3:** FreeCity4 replaces the rules on the re-read, so the flags come from the property's rules;
  the original takes them from the first n rules of the vector (the saved copy). They differ only when the saved
  rules differ from the exemplar. Acceptable? [flags from the re-read rules]~~ **ANSWERED 2026-10-14** from the exe
  (flags from the vector's first n entries at `0x00715EF8`–`0x00715F6D`; the append-only re-read and the §4.7.4
  rule order as already established for TN3/TN4) and written into traffic-network §4.7.2 ("When the saved rules
  differ from the property", with a worked example). **The bracket stands as a *[choice]*, but the difference is
  wider than the flags.** It arises only when the station's `0xE90E25A1` changed between the save and the load.
  Then the original also keeps applying the old rules before the new ones, so their connections survive, and it
  writes them back on every save. Its flags come from the old rules (plus new ones when m < n), so the access-point
  walk can differ too. FreeCity4 follows the current property throughout.

## From the aura glue and grid shadow (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/aura.md` and its `docs/abi` rows. Ini: `enable = aura`; shadow `aura_shadow = 1` (compared at
each cycle-completed message `0x2A09B21A`). The glue's current answer is in brackets.

- ~~**AU1 — §11.3:** does Read replace the departments and hotspots, or merge/append? [replace]~~ **ANSWERED
  2026-10-14** from the exe (`Read` `0x0068E340`: map `operator[]` `0x00497A00` at `0x0068E45F`; list insert before
  the end `0x0068E548`–`0x0068E552`) and written into aura.md §11.3 (addresses added; §16 Q12 and test 16 already
  had it). **The bracket differs: the original merges and appends.** An existing department ID gets the new grid
  pointer (the old grid is not released), other IDs stay; hotspots are added after the ones already held. In play
  both give the same result, because the game reads into a fresh object; keep test 16's used-object case.
- ~~**AU2 — §10:** the transient tuning is read through properties, not the raw f32 copy. Same values? [properties]~~
  **ANSWERED 2026-10-14** from the exe (copy helper `0x005FFCC0`, called at `0x0068D94E`; reads
  `0x0068D958`–`0x0068D966`) and written into aura.md §10 ("Transient tuning in detail"). **Same values for the
  shipped data** (Float32 arrays of 2). The original copies raw words and returns true even for count 0, then reads
  `v[0]`, `v[1]` unchecked: a scalar or empty array crashes (null read), one value gives a garbage radius, other
  element types are reinterpreted bit for bit. *[choice]* only a Float32 array of at least 2 values is used; anything
  else keeps {0, 0}.
- ~~**AU3:** `GetPopulations` (lot slot 54) returns a container not in `docs/abi`; the glue calls `GetPopulation(type)`
  (slot 53) for the 12 developer types and keeps non-zero counts, so a mixed lot whose residential count is 0 reads
  −128 instead of 0. What is the container's layout (allocators.md)? [slot 53 loop]~~ **ANSWERED 2026-10-14** from
  the exe (slot 54 `0x006C21E0`; slot 53 `0x006C04A0`; walk `0x0068CE60`–`0x0068CF23`) and written into aura.md
  §6.5.1 ("The population container"), §13 and test 5. **The bracket differs in play.** Slot 54 returns a pointer to
  the lot's `std::vector<std::map<uint32_t, uint16_t>>` at lot + 0x54 (12-byte STLport maps, 0x18-byte nodes, key
  +0x10, uint16 count +0x14, allocator B; 0 or 1 maps in saves). The original walks every map and entry; a
  residential key with count 0 counts as seen, and keys outside the 12 types count in the total. Saves hold 0
  counts, so read the container through slot 54. docs/abi row in the lane report.
- ~~**AU4:** `RefSint32` on a non-Sint32 property: a Uint32 is accepted bitwise, anything else counts as absent.
  Right? [as stated]~~ **ANSWERED 2026-10-14** from the exe (`DoOccupantChanged` `0x0068ECF0`; `RefSint32` slot
  105 body `0x0060CE80`, shared by every `RefX` slot; `GetValSint32` `0x00917721`) and written into aura.md §7 ("How
  the values are read") and test 15. **The bracket is a sound choice, with one addition.** The original checks
  neither type nor count: it uses the word at variant + 4 as an int32 array. Uint32 arrays read bit for bit (as the
  bracket); other element types give meaningless values; a scalar's value is used as a pointer (crash); a
  one-value array reads the radius past the end. *[choice]* also treat fewer than 2 values as absent.
- ~~**AU5:** the safe lot iterator's layout is not in `docs/abi`; the glue walks cells with `GetLot(x, z, false)`,
  takes each lot once and re-checks the pointer at the next step. Layout of the iterator? [cell walk]~~ **ANSWERED
  2026-10-14** from the exe (slot 15 `0x00734850` → `0x00734860`; next `0x0045D3F0`; release `0x0045D5B0`) and
  written into aura.md §6.5 ("The safe lot iterator"). **The cell walk stands** *[choice]*: stamping is a maximum,
  so the order does not matter. Layout: an 8-byte global-heap (C) block {current node, lot list object = lot
  manager + 0x5C}, registered in the list object's iterator list (+4) so lot removal moves it; nodes {next, prev,
  `cISC4Lot*`}; the walk ends at the head node; released by a non-virtual function (unregister, free with C).
  docs/abi/allocators.md table 1 already had the block and its allocator.
- ~~**AU6:** the game-data registry node "aura" is a stub. What does the original publish there? [stub]~~ **ANSWERED
  2026-10-14** from the exe and the shipped data, written into aura.md §4.3. **The stub is acceptable.** The
  original publishes what §4.3 says: the node `"aura"` and the commands `add_transient_effect(x, z, radius,
  magnitude)` and `end_strike(type)`. No shipped file calls them (every entry of `SimCity_1.dat`–`SimCity_5.dat` and
  `EP1.dat` searched): there are only the empty Lua placeholders and the `END_STRIKE_*` constants. Only mods would
  notice.
- ~~**AU7 — shadow timing:** the other inputs are read at the end of the original's cycle, so a mid-cycle change shows
  as a difference; the tax baseline is taken at the first compared cycle. [as stated]~~ **ANSWERED 2026-10-14**
  (with AU13) and written into aura.md §5.3 and §14.3. **Right, and it explains AU13–AU15.** The original reads each
  input in its own state's call (§5.3). Its tax baseline is the rate read in the first state 4 after the object was
  created, so the first cycle after a load; a shadow that starts with that cycle and reads the rates in its state 4
  matches. To remove the timing differences, follow the original's state field step by step (§14.3).
- ~~**AU8:** `GetCumulativeOffset` (ordinance slot 24): the SDK says int64, the spec and table int32 in EAX. [int32,
  declared as `GetCumulativeOffset32` in `cFC4ISC4OrdinanceSimulator`]~~ **ANSWERED 2026-10-14** from the exe
  (`0x0080DD70`, epilogue `0x0080DEA1`–`0x0080DEAC`) and written into aura.md §13 (SDK errors). **The bracket is
  right.** The sum is an int32 in EAX, the callee pops 8 bytes, and EDX is not set for the caller (a leftover), so an
  int64 declaration reads a garbage high half. docs/abi/interfaces.tsv already has pops 8 and EAX.
- ~~**AU9:** a cycle in an empty city keeps the lot iterator open until the next state 4. Intended? [as the spec]~~
  **ANSWERED 2026-10-14** from the exe (state 4 `0x0068F325`–`0x0068F379`, iterator `0x0068F32F`–`0x0068F353`) and
  written into aura.md §6.4 (corrected). **The spec was wrong about "fresh", and nothing observable depends on it.**
  The original gets one iterator at the first state 4 of the object's life, rewinds it at every later state 4, and
  releases it only in `Shutdown`. So it stays open between all cycles, not only after an empty-city cycle. The
  glue's choice stands (with the cell walk of AU5 it needs no iterator at all).
- ~~**AU10:** with no land value simulator the land value type is 0; the spec is silent. [0]~~ **ANSWERED 2026-10-14**
  from the exe (`LotBaseAura` `0x0068CE00`, unchecked global call at `0x0068CFB7`) and written into aura.md §6.5.1.
  **The original crashes**: after the curve test it calls the land value simulator with no null check. The shipped
  game always creates one. *[choice]* skip the value/wealth term (result = the lot average), as if the curve were
  absent. The bracket's type 0 gives `d = −wealth` and adds +3 to +5 to every residential lot with the shipped
  curve.
- ~~**AU11 — §16 Q7:** the base-aura process table is not written. [not written]~~ **ANSWERED 2026-10-14** from the
  exe (table `0x00B0D150`; every instruction naming it: tuning `0x0068D8E3`/`0x0068D8F1`, state 4 `0x0068C002`–
  `0x0068C098`, `LotBaseAura` `0x0068CE7E`–`0x0068CEB2`) and written into aura.md §10 ("The base-aura table"), §3
  and §16 Q7 (resolved). **Not writing it is fine.** It is the aura's own storage: three entries {developer type,
  tax index, wealth, tuned base, effective base} for R$, R$$, R$$$, and no other code refers to it. State 4 rewrites
  the effective values before state 5 reads them, so keeping them in the object changes nothing.
- ~~**AU12:** the CPU-cycle limit timer runs on `QueryPerformanceCounter` instead of the TSC. Acceptable? [QPC]~~
  **ANSWERED 2026-10-14**, written into aura.md §6.5 (time slicing). **Acceptable; no original behaviour depends on
  it.** The original reads the time-stamp counter, but the clock only decides how many lots one state-5 call stamps.
  Stamping takes the maximum, so the work grid is the same for any clock. The implementer's choice stands.

First in-game aura shadow (coordinator, `regress -Tag r7c3f`, Spandau, 98 cycles 2026-01 to 2034-02; the replace run
with aura was clean, 0 problems). The park, landmark and riot fields aside, **6 of 98 cycles match exactly**:

- ~~**AU13 — small base differences every cycle.** 0 to 43 tracts of 1024 differ by 1 to 3 (most cycles 2 to 12
  tracts, max|d| = 2), in both directions from one cycle to the next (cycle 1 `(6,21)` 59/58, cycle 2 58/59), and
  the aura grid carries the same differences. The mayor rating is off by 1 in 11 cycles. The first cycles' tracts
  sit in one area (x 4 to 13, z 21 to 28). Is this the AU7 timing (an input that changes during the original's
  cycle: which inputs does the original read in which state, and on which day), or a rounding step the spec does
  not give (the order of the sums, a truncation instead of a rounding)? Name the inputs read per state.~~
  **ANSWERED 2026-10-14** from the exe (`DoIdle` `0x0068F190`; lot value `0x0068CE00`; rule A callbacks
  `0x0068BBB0`–`0x0068BEE0`; state 14 `0x0068F6FE`–`0x0068F94E`; state 15 `0x0068DA80`) and written into aura.md
  §5.3 (inputs per state, worked examples) and §14.3 (step-by-step shadow, the original's field offsets). **Timing,
  not rounding.** The arithmetic has no step beyond §6. Each state reads its inputs in its own `DoIdle` call. Every
  other idle agent (growth developers, land value, residential, crime, …) runs between two steps, and the day
  agents (traffic trip lengths and congestion, pollution) run when a day passes. Occupant cones arrive by posted
  messages. A base near equilibrium moves by at least 1 for any non-zero `d`, so one work cell off changes it by 1
  or 2. Track the original's state field (+0x70) and run our steps when it moves.
- ~~**AU14 — park and landmark maps in cycles 2 and 3 only.** 193 tracts (max|d| 200) in cycle 2 and 208 (max 188)
  in cycle 3 differ in both maps, then 0 for the other 95 cycles. When does the original rebuild these maps (on
  load, on the first cycle, when a park lot is added), and does it clear them first?~~ **ANSWERED 2026-10-14** from
  the exe (`DoIdle` states 2 and 3, `0x0068F1D7` and `0x0068F222`; occupant handler `0x0068ECF0`) and written into
  aura.md §6.3 ("When the park and landmark maps change") and §5.3 (worked example). **Every cycle, and no clear is
  needed.** State 2 rewrites every tract of the park map. State 3 copies landmark work over the landmark map after
  the last department (the first department overwrites landmark work); with no departments it never writes it. Load
  restores both maps; an occupant change touches only the raw park, department and building-effect grids, and
  only when its posted message is delivered. The differences are landfill tiles (group `0x07BDDF1C`, park and
  landmark {−100, 32}, radius 8 tracts, 193 tracts per cone) created on the 1st: their messages arrive after the
  original's states 2 and 3, so its maps catch up one cycle later. Two tiles in one tract give 193 tracts with −200;
  two tiles one tract apart give 208 tracts with −188.
- ~~**AU15 — rare large single-tract differences.** Three cycles have a base difference near 180 (cycle 3: 182,
  cycle 7: 188, cycle 88: 175) while the aura grid differs by at most 54/60/47 there. A lot placed or removed during
  the cycle? Which state reads the lots, and is a lot's effect clamped before or after it is added to the base?~~
  **ANSWERED 2026-10-14** from the exe (state 5 `0x0068F37C`; `LotBaseAura` `0x0068CE00`; state 14
  `0x0068F6FE`–`0x0068F94E`) and written into aura.md §6.5 ("Which lots, and where the clamps are") and §5.3
  (worked example). **Yes, a residential lot seen by one side only.** 182 = 54 − (−128): one side's base is −128
  (no lot), and the aura of a −128 tract is 0, so the aura differs by 54. Only state 5 reads lots for their value.
  A lot's value is clamped before it is stamped (after the average and after the value/wealth term); every rule A
  stage clamps. The base averages the work cells and clamps again, and a base that was −128 takes the new value at
  once.

## From the residential education and health core (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/residential-education-health.md` (REH) §2 to §7. RS7 to RS17 are applied. The lane's answer is in
brackets; the module doc of `core/src/residential/education_health.rs` lists 1 to 13.

- ~~**EH1:** do the efficiency ratings and the base staff efficiency start (a, b) at (0, 0)? [yes]~~ **ANSWERED
  2026-10-14** from the exe (`GetOccupantEfficiencyRatings` `0x006F8749`–`0x006F8759`, `GetBaseStaffEfficiency`
  `0x006F8611`–`0x006F8621`) and written into REH §4.2. **Yes.** Both set a = b = 0.0 and always call slot 20 before
  the staff-item test. With a staff item whose department is missing, the curves are read at 0: shipped school e1 =
  0, e2 = 0.5, base = 0.
- ~~**EH2:** slot 19 sets both percentages: does it end with the dirty flag set? [yes]~~ **ANSWERED 2026-10-14** from
  the exe (`SetOccupantFundingPercentages` `0x006F8290`; flag store `0x006F834A`) and written into REH §4.2. **Yes.**
  The flag store is shared by both "no item" exits and the normal end; only a null department returns without it.
  The flag is never read.
- ~~**EH3:** after `ChangeBudgetItemLine`, is the line item's line the building type? [yes]~~ **ANSWERED 2026-10-14**
  from the exe (`DoBuildingChanged` `0x006FB120`: hospital `0x006FB17F`, school `0x006FB39A`) and written into REH
  §4.1. **Yes.** The call stores the building type into its own copy of the item right after `ChangeBudgetItemLine`,
  and every later step of the call uses that line; the item is not read again.
- ~~**EH4:** on removal with no line item for the line, is the count step skipped? [skipped]~~ **ANSWERED
  2026-10-14** from the exe (`DoBuildingChanged`: removal path `0x006FB677` → count step `0x006FB1F4`; line item
  used unchecked at `0x006FB200`; schools `0x006FB41B`) and written into REH §4.1. **No: the original crashes.** It
  fetches the line item and calls it with no null check. With consistent budget data the line item exists
  whenever a building of that line is removed. *[choice]* skip the count step and `RemoveLineItem` (the bracket).
- ~~**EH5:** a building added without a capacity: is its effective capacity 0? [yes]~~ **ANSWERED 2026-10-14** from
  the exe (`DoBuildingChanged` capacity reads `0x006FB741` / `0x006FB8A2`, getter `0x005FD480`; grading
  `0x006FDBB7`) and written into REH §4.1 and §4.5 (corrected). **No: the original writes a leftover.** The getter
  leaves its output alone, so the add writes a stack leftover (the health department pointer, the school line
  item pointer, or older garbage), and grading uses the coverage-radius property ID as the capacity (not 0, as
  §4.5 had inferred). No shipped building lacks a capacity (all 18 school and hospital buildings checked).
  *[choice]* 0 in both places (the bracket).
- ~~**EH6:** do "served", the tract list and the boost all use the lot centre, else `GetPosition`? [yes, all three]~~
  **ANSWERED 2026-10-14** from the exe (`GetCoveredTractList` `0x006F7E00`; `GetPopulationServed` `0x006F8F60`,
  position at `0x006F8FB7`; boost centre in `ComputeOccupantGrade` `0x006FDE22`–`0x006FDE4E`) and written into REH
  §4.5. **No, two of three.** The tract list and the boost use the lot centre, else `GetPosition`. "Served" takes
  the building's own tract from the occupant's `GetPosition` only, with no lot lookup, so the two tracts can
  differ (worked example in §4.5).
- ~~**EH7:** how do slots 28 and 58 pick the service? [like slot 45: hospital capacity first, then school]~~
  **ANSWERED 2026-10-14** from the exe (slot 28 `0x006FD510`, slot 58 `0x006FD680`, slot 45 `0x006FD260`) and
  written into REH §4.10 and §4.11. **The bracket differs (school first in both).** Slot 28: school if the building
  has the school capacity `0x8922041B`, else hospital (unchecked). Slot 58: school if the school query succeeds,
  else hospital if the hospital query succeeds, else AL = 0. Also slot 45's "hospital wins" covers only the radius,
  kind and ordinance list; its school flag stays set. No shipped building has both capacities, so only mods see it.
- ~~**EH8:** slot 45 with no matching point or no radius? [AL = 1 whenever the building's kind is found, distance 0,
  radius 0]~~ **ANSWERED 2026-10-14** from the exe (`GetOccupantCoverage` `0x006FD260`: radius local zeroed
  `0x006FD308`, dist zeroed `0x006FD435`) and written into REH §4.11 ("Edge cases"). **The bracket matches**, with
  one more case: a distance curve with no points (a property with fewer than two values) skips the step and leaves
  dist unwritten (the caller's value). *[choice]* write 0 there too.
- ~~**EH9:** does the ideal funding test the served property itself for `0xFFFFFFFF`? [yes, the property]~~
  **ANSWERED 2026-10-14** from the exe (`EstimateIdealFunding` `0x006FD701`–`0x006FD72B`) and written into REH
  §4.10. **Yes, the property**, read into a value preset to `0xFFFFFFFF`, so a missing property also fails. The
  ratio then uses the query data's served (+0x24) and capacity (+0x14), which equal the properties at that point.
- ~~**EH10:** does the hospital query fail on missing fields, testing the staff item last? [yes]~~ **ANSWERED
  2026-10-14** from the exe (`GetHospitalQueryData` `0x006F7E80`, staff test `0x006F7F37`; school `0x006F91C0`) and
  written into REH §4.11 ("Order of the tests and partial writes"). **Yes**: capacity, effective capacity, served,
  grade, then the staff item. Also: capacity, effective capacity and served are read straight into the record, so
  a failed query leaves them written (served raw) and the rest untouched.
- ~~**EH11:** what does the ratings query's record start at? [zero]~~ **ANSWERED 2026-10-14** from the exe
  (`GetSchoolSystemRating` `0x006F9F27`–`0x006F9F79`, `GetHealthSystemRating` `0x006F9DD7`–`0x006F9E29`) and
  written into REH §4.11 ("Starting record"). **Partly zero.** Only +0x10, +0x28, +0x2C, +0x34 (and the string)
  are set; the summed +0x18 and the other fields start as stack leftovers. The leftover is reached only when the
  first building's query fails before reading +0x18, which needs a building without a capacity (none ships).
  *[choice]* 0 (the bracket).
- ~~**EH12:** do the system totals count only successful queries and add onto the caller's record? [yes]~~
  **ANSWERED 2026-10-14** from the exe (`GetSystemTotals` `0x006FA070`: reset `0x006FA0A6`–`0x006FA0BB`, sums
  `0x006FA167`–`0x006FA1CF`) and written into REH §4.11 (Totals, corrected). **No on both.** The helper first zeroes
  the caller's +0x14..+0x28, then counts and adds **every** matching building, ignoring the query result (a failed
  query contributes what it read plus the previous building's other fields). "Matching" is per building:
  `IsOccupantGroup` for any of `groups`, over every group list. Shipped data reaches it: the farmer's market's
  hospital query always fails (no health staff item) but it is still counted.
- ~~**EH13:** in `AddRadiusEffectWithCurves`, with n = 0 is pop' = pop, and is the same tract index used for all
  grids? [yes]~~ **ANSWERED 2026-10-14** from the exe (`AddRadiusEffectWithCurves` `0x005E48F0`: n test
  `0x005E4B5F`–`0x005E4B72`; reads `0x005E4A41`, `0x005E4B3F`, `0x005E4B4E`, `0x005E4BBA`, `0x005E4AF0`) and written
  into REH §4.6 (step 7, "Which tract each grid is read at", example). **Yes on both.** n = 0 skips k entirely: eff
  and pop are used unscaled. Population, coverage and target are all read at the same (x, z); the age grid is read
  at cell (x × S_t, z × S_t), the same tract when its size equals the target's. Bounds and geometry come from the
  target only; all four grids have tract size 4 in every game-written city. *[choice]* a tract outside a smaller
  source grid (hand-made save only) reads as 0.
- ~~**EH14:** P is a signed 32-bit value: is a negative P accepted on Read (`record.rs`)? [accepted]~~ **ANSWERED
  2026-10-14** from the exe (Read `0x00700A02`; the only readers, `Init` `0x006FEEB1`–`0x006FF15D`; state 4's S
  from the grid, `DoPopulationChange` `0x006FA4AE`–`0x006FA4B7`) and written into residential.md §2, §5.5 (RS15,
  corrected) and §9.2. **The original accepts it**, unchecked, as it does 0 and huge values. With every grid
  present a negative P changes nothing: state 4's S is the R$ population grid's own `GetTractSize()`, not P
  (correction: §5.5 said S = P), and P only sizes a grid `Init` must recreate. Such a grid with P < 0 hangs state 4
  at tract (0, 0). *[choice]* (changed, not the bracket) reject P or E outside 1…256 read as signed, and the loaded
  R$ population grid's size likewise; take S from that grid. No game-written save is affected (all are 4).
- Not testable by vectors: the unsigned versus signed upper-bound compare (no output differs); the §4.13 tuning
  values that are loaded but unused (minimum acceptable funding, maximum EQ/HQ boost). REH §11 test 24's calling
  conventions (slot 34 leaves the x87 stack unchanged; slots 10 to 13 return through the hidden pointer) belong
  to the residential glue.

## From the flora glue and census shadow (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/flora.md` and its `docs/abi` rows. Ini: `enable = flora` (ten `flora_*` bisection switches);
shadow `flora_shadow = 1` (registry once per city, then the deterministic part of each monthly pass; the random
part of §8 is not compared). The glue's current answer is in brackets.

- ~~**FQ1:** exemplars are enumerated with `GetAvailableResourceList` and an exemplar-type filter (the demand glue's
  pattern), not slot 18 `GetAvailableResourceListForType`, and keys are read by index (`Size`/`GetKey`), not
  `EnumKeys`. Equivalent? [assumed]~~ **ANSWERED 2026-10-14** from the exe (slot 18 `0x00973280`, its filter
  `0x00973194`, slot 17 `0x009739CB`, `EnumKeys` `0x009749CB`, `GetKey` `0x00974741`, the callback `0x006B03B0`)
  and written into flora.md §4.1 step 2 and §12. **Equivalent**, provided the filter accepts exactly the keys
  whose type is `0x6534284A` (any group, any instance): slot 18 is slot 17 with such a filter, and `EnumKeys`
  walks the same sequence that `Size`/`GetKey` index, first to last.
- ~~**FQ2 — §4.1 step 2:** the app shutdown callback is not registered: its timing against our DLL's unload is not
  documented, and the records live for the whole process. [not registered]~~ **ANSWERED 2026-10-14** from the
  exe (the callback `0x006B1070`; the app's list and its walk in `cSC4App::Shutdown` `0x0044C130`,
  `0x0044A633`–`0x0044A650`; the framework's shutdown order `0x0087AB07`) and written into flora.md §3.2. **No
  observable original behaviour; the implementer's choice stands.** The callback only drops the map's reference
  to each record and empties the map and the type array at process exit. For the record: callbacks run inside
  the app's `Shutdown`, in registration order, after PreAppShutdown and before PostAppShutdown reach the framework
  hooks, so a DLL's callback would still be loaded when it runs.
- ~~**FQ3 — §10.3:** `cIGZPersistDBSegment`'s record read/write is not in `docs/abi`, so saving and restoring an
  obliterated city's trees is off (`SaveObliterated` writes nothing and returns true; a restore is logged when the
  segment at +0x204 is non-null). Layout and slots? [off]~~ **ANSWERED 2026-10-14** from the exe (save
  `0x006B2760`, restore `0x006B0660`, the record-stream helpers `0x005D9DB0` / `0x005D9C90`, segment vtable
  `0x00AA2654`, stream vtables `0x00AA2280` / `0x00AA2310`) and written into flora.md §10.3, §12 and test 30.
  **The bracket (off) loses trees across obliteration.** Query the segment for `cISC4DBSegment` `0x499899DA`;
  slots 8/9 open/close a write stream, 5/6 a read stream; stream slots 7, 9, 18 = u16, u32, GetError.
  *Correction to §10.3:* each record starts with a u16 version 0 (count record: `u16 0, u32 count`; occupant
  record: `u16 0` then the occupant's own version-3 record); the reader ignores the u16.
- ~~**FQ4:** `cISC4OccupantFilter` is not in `docs/abi`. Instead of the filter overload (slot 36) the glue calls the
  typed box query (slot 37) for flora, building, landfill and prop, then for networks keeping those with
  `GetNetworkFlag() & 0x1F4F` non-zero. Same set? [as stated]~~ **ANSWERED 2026-10-14** from the exe (filter
  vtable `0x00AB1374`, slot 3 `0x006AFD10`; box query `0x004A0670`; typed query `0x0049DD40`) and written into
  flora.md §6.4.1. **Same set; only the order differs, and nothing observes it.** Slot 37 is slot 36 with the
  manager's one-type filter; slot 36 calls only the filter's slot 3 `bool IsOccupantIncluded(cISC4Occupant*)`
  (pop 4, AL). A network occupant that does not answer `cISC4NetworkOccupant` does not block.
- ~~**FQ5:** weather `GetMoistureType` is read as a `uint8_t` in AL (`FC4Interfaces.h`). Right? [AL]~~
  **ANSWERED 2026-10-14** from the exe (`GetMoistureType` `0x007256D0`, `GetMoistureValue` `0x00726150`, the flora
  reader `0x006B0939`) and written into flora.md §6.2. **Right for reading the original weather**: it returns
  `value >> 4` from a zero-extended byte, so EAX is 0..15 with the upper bits zero and AL loses nothing. The
  declaration should still be a 32-bit return: original flora uses all of EAX as the table offset, so a
  FreeCity4 weather must return a clean EAX (already in §13.2).
- ~~**FQ6:** "occupant in a visited block" (shadow) uses a closed box, edges included, like the host's box query.
  [closed]~~ **ANSWERED 2026-10-14** from the exe (box query `0x004A0670`, its cell mapping `0x0049E0E0` and clamp
  `0x0049DF00`, listing in `InsertOccupant` `0x0049EF60` / `MoveOccupant` `0x0049F3C0`) and written into flora.md
  §6.4.2 and test 37. **No, on two counts.** The host's test is strict (open) on x, y and z, so touching boxes
  are not returned. And it only looks at occupants listed in the query's 64 m index cells; an occupant is
  listed from its box shrunk by 0.1 m per side (boxes wider than 1 m), with coordinates up to 0.064 m below a
  64 m line counted in the cell above. A box whose low edge is 0.1–0.164 m below such a line overlaps the block
  below it but is not returned for it (this is FQ15).
- ~~**FQ7:** the shadow takes the elevation scale from the tuning exemplar's `0xC9DBDD02`; the record does not save
  it. [as stated]~~ **ANSWERED 2026-10-14** from the exe (constructor `0x006B1941`, tuning read `0x006B01B0`,
  `Write` `0x006AFEE0` / `Read` `0x006AFFE0`) and written into flora.md §7. **As stated.** Default `0.08f`;
  `16.0f / v` only when the tuning exemplar has `0xC9DBDD02`; re-read at every `Init` and on an exemplar change of
  `0x1A2FDB6A`; never saved.
- ~~**FQ8:** blocks are never freed and the registry is never cleared at app shutdown. [as stated]~~ **ANSWERED
  2026-10-14** from the exe (shutdown callback `0x006B1070`, reload in `DoMessage` `0x006B352B`–`0x006B354B`) and
  written into flora.md §13.2. **No observable original behaviour; the implementer's choice stands.** The
  original releases the registry's references and empties the map and type array only in its process-exit
  callback (FQ2), and its exemplar-change reload already drops old records unreleased.
- ~~**FQ9:** `cISC4FloraTypeFilter` (slot 3) is not in `docs/abi`. Slot 13 with a non-null filter inserts nothing and
  returns 0; with a null filter it inserts every key. [as stated]~~ **ANSWERED 2026-10-14** from the exe
  (`GetFloraTypesByFilter` `0x006B23A0`; the slot-14 callers `0x0048B860`, `0x006CD010`) and written into flora.md
  §11.1. **The null-filter half matches; the non-null half does not.** The original calls the filter's slot 3
  `+0x0C` `bool (u32 floraType)` (pop 4, AL) for every map key in ascending order, key 0 included, inserts the
  accepted keys and returns how many it accepted. No exe caller passes a filter, so only plugins notice; the
  fix is a one-line call per key.
- ~~**FQ10:** `cIGZCommandParameterSet` is not in `docs/abi`. PlaceFlora reads its parameters through slot 5
  (`operator[]`): param 0 via variant slot 24 (u32), params 1–2 via slot 36 (float). Right? [as stated]~~
  **ANSWERED 2026-10-14** from the exe (`DoMessage` `0x006B3497`–`0x006B34F6`; the set's vtable `0x00AD16C8`,
  slot 5 `0x0092A162`; variant slots `0x0091560F`, `0x0091567A`) and written into flora.md §5 and §12. **Right.**
  Slot 5 `operator[](u32)` pops 4 and never returns null (a missing index gives the set's empty variant); slot 24
  `GetValUint32()` (EAX) and slot 36 `GetValFloat32()` (ST0) wrap the converting getters with a 0 default.
- ~~**FQ11:** the cheat notification uses id `0x095BDC9C` as the spec says, but the budget and simulator glue pass 0.
  Which is right? [`0x095BDC9C` for flora]~~ **ANSWERED 2026-10-14** from the exe (flora `0x006B2C77` and
  `0x006AFDDD`; the cheat manager's `AddNotification2` `0x009212A6` and dispatch `0x00920F34`–`0x00920F3E`; budget
  `0x00693135`; simulator `0x007033B1`) and written into flora.md §4.1 step 11. **The cheat ID is right, for all
  three.** The original budget and simulator also pass their own cheat IDs. ID 0 means "every cheat": the
  manager delivers a cheat to each pair whose ID matches it or is 0, so the budget and simulator glue with 0
  receive all cheats (and must remove the pair with the same ID they added).
- ~~**FQ12:** `RegisterCommand`'s parameter info is null, as the spec says. [null]~~ **ANSWERED 2026-10-14** from
  the exe (flora `0x006B2A0C`–`0x006B2A1F`; command server slot 6 `0x0092B075`, slot 8 `0x0092AD10`) and written
  into flora.md §4.1 step 3 and §12. **Null is right.** The name is a plain `const char*` (`"PlaceFlora"`), the
  call is made only when the message server exists, and its result is ignored. With null info the server writes
  a default help text (`"Format: PlaceFlora\nOutput: <none>"`) when its help flag is set.
- ~~**FQ13:** `ToCoreAll` leaves out parent-cohort properties (existing plugin behaviour). Does flora need them?
  [left out]~~ **ANSWERED 2026-10-14** from the exe (`AddStaticFloraData` `0x006B12D0`; the `0x10` test
  `0x005FD480` in `0x006B03B0`) and a scan of the shipped `.dat` files, written into flora.md §3.3. **The original
  includes them.** Every flora property, and the `0x10 = 0x0F` test itself, is read through the effective list
  (data-formats §4.4). The 22 shipped flora exemplars have no parent cohort, so the shipped game is unaffected;
  a plugin whose flora values sit in a cohort would get defaults (or not be registered) if they are left out.
  *[choice]* include them.
- ~~**FQ14 — §6.11:** adding a record whose next type is 0 set key 0's seed type to it in the test harness. Does
  §6.11 apply when next is 0? [the test avoids the case]~~ **ANSWERED 2026-10-14** from the exe
  (`AddStaticFloraData` slot 6 `0x006B3570`, loop `0x006B3725`–`0x006B3776`; `Init` load `0x006B299D` before the
  key-0 insert `0x006B2BAA`; reload `0x006B352B`–`0x006B354B`) and written into flora.md §6.11 and §3.2. **Yes, it
  applies; there is no special case for 0.** The harness result is the original's. In the game it cannot happen:
  key 0 is never in the map during a load (the first load precedes its insert; the reload clears it and does not
  put it back), and nothing reads key 0's seed type. The test may keep avoiding the case or expect the capture.
  Side correction in §3.2: after a reload, `IsFloraTypeValid(0)` is false until the next `Init`.
- Not done (each logged once): obliterated-city save/restore (FQ3), the app shutdown callback (FQ2),
  `GetFloraTypesByFilter` with a non-null filter (FQ9).
- ~~**FQ15 — first in-game shadow (run 80, coordinator):** the registry is identical and about 2,600 trees match each
  month, but exactly one tree differs in 23 of 94 months: 21 times its birth date (ours predicts a rebirth, e.g.
  birth 2461102 kept by the original where ours predicts 2461224) and twice its state (state 0, ours 2). What in
  §6.7.2 keeps a tree from being reborn (a visit count, a random draw, the block's iterator position)?~~
  **ANSWERED 2026-10-14** from the exe (occupant manager slot 15 `0x0049E0E0`, listing `0x0049EF60` /
  `0x0049F3C0`, query `0x004A0670`; see FQ6) and the run-80 differences, written into flora.md §6.7.1 (table of the
  six trees, example) and §16 test 38. **Nothing in §6.7.2: the tree is not visited.** The monthly block query
  goes through the occupant manager's 64 m index (§6.4.2). A box wider than 1 m whose low edge is 0.1–0.164 m
  below a 64 m line is listed only from the cell above, so the block just below the line overlaps it but does
  not return it. Six trees in run 80 sit in that window (one per month, 23 tree-months in all; e.g. Oak Tree
  clump 5 `0x14C279F4`, box z from 319.844, is visited by blocks (53, 10) and (54, 10) only). Ours also visited
  them from the block below, and since wild clumps live 2–42 days, every such extra visit was a rebirth.

## From the landfill developer glue and shadow (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/landfill.md` and its `docs/abi` rows. Ini: `enable = landfill` (ten `landfill_*` bisection
switches); shadow `landfill_shadow = 1` (the original's SimNewMonth wrapped; its record read through its own Write
before and after). The glue's current answer is in brackets.

- ~~**LG1 — §4.4.5, §4.8.1:** what arguments does RemoveOccupant take for flora and base-texture removal? [(true, 3)]~~
  **ANSWERED 2026-10-14** from the exe (`0x006BC6D8`, `0x006BCA14`; `RemoveOccupant` `0x0049F240`): both are
  `(occupant, true, 3)`. The occupant is shut down after removal, and data4 of the posted `0x99EF1143` is 3. Only a
  base texture with `GetUsage() = 1` is removed. **The bracket matches.** Written into landfill.md §4.8.4.
- ~~**LG2:** how is the base-texture occupant created and given the landfill texture (SetEmptyCellTexture)? [not done]~~
  **ANSWERED 2026-10-14** from the exe (`0x006BA470`) and the saves. It calls GZCOM `CreateInstance(0xC97F987C,
  0x097F6D98)`, then Init, `SetUsage(1)`, `SetTextureSpecification` with one record (texture `0x1005`, the cell,
  `00 00 FF FF FF <zone display alpha> 01 00`, no random draw) and `InsertOccupant(occ, 0)`. All 94 usage-1
  textures in the local saves match. **"Not done" does not match**: the texture is saved, and §4.4.5 tests for it.
  Written into landfill.md §4.8.5.
- ~~**LG3:** how is the tile occupant created; is the SDK's `SetTileID` (slot 6) enough to attach the tile exemplar?
  [tiles live in FreeCity4 only; pollution's active-landfill flag is still set, which is what the simulation reads]~~
  **ANSWERED 2026-10-14** from the exe (`0x006BC7CF`–`0x006BC92A`, `SetTileID` `0x00477450`). **No.** `SetTileID`
  only renames the key of an exemplar that is already attached, and crashes when none is. The original loads the
  exemplar with `GetResource` and attaches it with `cISCExemplarPropertyHolder::SetDefaultExemplar` (QI
  `0x0AC2B5F7`). Landfill.md §4.8.1 gives the order and a GZCOM recipe. **The bracket does not fully match**: the
  occupants are also saved, block flora, are tested by `PlaceZone` and are drawn. Written into landfill.md §4.8.1.
- ~~**LG4:** who owns the reference in occupant manager slot 33's out-parameter? [slot 41 lists used instead]~~
  **ANSWERED 2026-10-14** from the exe (`0x0049DCE0` → `0x0049E690`, `AddRef` at `0x0049E7DD`; released by the
  landfill at `0x006BA454` and `0x006BC7AB`): **the caller**. A hit is `AddRef`ed; a miss leaves `*out` unwritten.
  **The bracket is equivalent** for presence tests if the slot 41 lists are released through slot 48. Written into
  landfill.md §4.8.6.
- ~~**LG5 — open question 6:** where does the zot orientation come from? [0]~~ **ANSWERED 2026-10-14** from the exe
  (`0x006BB510`–`0x006BB524`; `SC4ViewUtilities` `+0x10`, set from the camera rotation): it is the **current view
  rotation** (0 to 3) at placement, and it is saved in the prop record. **The bracket matches** while the view is at
  rotation 0. All 66 zot props in the local saves have 0. Written into landfill.md §4.7.5. The same answer closes
  lot-developer.md §12.2 Q3.
- ~~**LG6 — §3.3, foundations §13.4:** must the month run after pollution and budget? [not enforced]~~ **ANSWERED
  2026-10-14** (foundations §13.3/§13.4; the data links of §3.3): **yes**, *[compat]*. Otherwise each landfill
  intake reaches pollution's history and line 20 one month early, and each line 30 maintenance reaches the budget's
  settlement one month early. Landfill.md §3.3 has a worked example. **"Not enforced" does not match** whenever the
  landfill agent ends up before pollution or budget, which registration order does. The §13.4 deferral ends now that the
  landfill glue has landed. Written into landfill.md §3.3.
- ~~**LG7:** does FindNetworkTile's return carry a reference? [borrowed]~~ **ANSWERED 2026-10-14** from the exe
  (`0x004B0B20`; `PopulateZone` `0x006BCDF6`–`0x006BCE16`): **borrowed**. It makes no `AddRef` call, and
  `PopulateZone` only tests the result for null. The map is fetched each time from the global city's slot 40.
  **The bracket matches.** Written into landfill.md §4.4.5.
- ~~**LG8:** tiles are not saved; they are rebuilt at PostCityInit (interior orientation 0, no random draw). [choice]~~
  **ANSWERED 2026-10-14** from the exe (PostCityInit, §3.2) and the saves. The original **saves** each tile (class
  `0x8A22BEBE`, with tile, orientation and exemplar key) and **does not rebuild** on load. There is no draw. In the
  three saved landfills, all 142 cells match the §4.8 rules for the saved heaps, and the interiors have orientations
  0/2/4/6. **The choice matches** for loading original saves, except that interior orientations become 0. It
  **does not match** for a FreeCity4 save loaded by the original: with no tiles, maintenance and the terrain dezoning
  differ. Written into landfill.md §6.4.
- ~~**LG9:** is a null `GetLandfillZoneFromCell` acceptable to the city situation manager? [null]~~ **ANSWERED
  2026-10-14** from the exe (`SetTarget` `0x00528AB9`–`0x00528C0A`, `GetDistanceToTarget` `0x00526A20`). **Yes.** A
  null zone makes both functions fall back to the target's lot. A tile has none, so there is no zone marker, and the
  distance is measured to the tile. The original returns null itself for a zone's last row and column, and the effect
  is UI or mission only. **The bracket matches.** Correction: the zone rect is read as integers, not floats, so
  landfill.md §10.1 quirk 15 is withdrawn. Written into landfill.md §9.
- ~~**LG10:** city slot 97 returns a float in ST0 (`docs/abi`) where the spec says "—". Which is right? [discarded]~~
  **ANSWERED 2026-10-14** from the exe (`0x00459440`): **the spec**. Both results are stored with `FSTP` through the out
  pointers, and the x87 stack is empty at `ret 0x10`. The docs/abi row has said this since re-answers2 PR6 (`7d2f368`:
  "returns nothing"). **The bracket is harmless but not exact.** If the call is declared as returning a float, every
  discard pops an empty x87 register. That is a masked stack underflow: it sets the sticky IE and SF status flags
  (it would trap if IE were unmasked) and stores no value. Using the "result" would read the NaN indefinite. Declare
  the slot `void`. Written into landfill.md §4.2 and §8.
- ~~**LG11:** should the tile item cost be read from the tile exemplar? [no]~~ **ANSWERED 2026-10-14** from the exe
  (`getLandfillTileCost` `0x006BA3C0`). **Yes.** It reads `0xEA54D286` through the tile occupant's property holder,
  and its only source is the attached tile exemplar. `kDefaultLandfillTileCost` is used only where a cell has no
  tile, or the property is missing. **The bracket matches only if the glue's fixed costs equal the shipped tile
  exemplars** (10/20/20/30/40). Using the default for every cell gives, for example, 45 instead of 160 for a full
  3 × 3 zone. Written into landfill.md §4.9.1.
- ~~**LG12:** should the shadow clamp GetGarbageSupply? [live plus delta, clamped]~~ **ANSWERED 2026-10-14** (landfill
  §4.7.1, pollution §8.1 slot 29). This is shadow glue: **no original behaviour; the implementer's choice stands.** A
  fact that makes the clamp moot: in the original month the intakes sum to at most `t ≤ supply`, so no
  `AddToGarbageSupply(−n)` clamps, and "live plus delta" is exact. The clamp matters only for a negative supply from a
  damaged save, which `GetGarbageSupply` itself clamps and stores. Written into landfill.md §4.7.1.
- ~~**LG13:** on Shutdown, does a zone release its zot or also remove the prop? [release only]~~ **ANSWERED 2026-10-14**
  from the exe (`0x006BB766`–`0x006BB798`): it **removes the prop too**. It calls `RemovePropA` through the city's prop
  manager when there is one, then releases the zot. **The bracket does not match**, but the difference is unobservable
  in the shipped flows: every `RemoveZone` dezones cells, which already removes the zot. Only the city-close shutdown
  still holds one. Written into landfill.md §4.4.9.
- ~~**LG14:** the shadow's supply is live plus delta, clamped. [choice]~~ **ANSWERED 2026-10-14** together with LG12
  (landfill.md §4.7.1). This is shadow glue: **no original behaviour; the implementer's choice stands.** In the
  original month the supply ends at exactly `s − Σ n`, and no clamp engages, so "live plus delta" equals the
  original and the clamp is inert. The only exception is a negative supply from a damaged save.
- ~~**LG15 — §1.1:** only the city and the message server are held; terrain, pollution and budget are fetched per
  call. [as stated]~~ **ANSWERED 2026-10-14** from the exe (`Init` `0x006B9D98`–`0x006B9E87`). The original holds
  **five** services, all with `AddRef` from the global service pointers: city, terrain, pollution, budget and message
  server. It does not hold the simulator. **The bracket is acceptable**: only reference counts differ, because each
  per-call fetch names the same object for the city's life. Written into landfill.md §1.1.
- ~~**LG16:** the 3D view level is not reachable through a documented interface. [not done]~~ **ANSWERED 2026-10-14**
  from the exe (`0x006BA07F`–`0x006BA0D5`; `SL::OnView3DCreate` `0x00602120`). The original uses the 3D view global
  `0x00B43DC4`, slot 22, with no null check. The same view **is** reachable through public interfaces: app main window,
  then child `0x6104489A`, then `GetChildAs(0x9A47B417, cISC4View3DWin 0xFA47B3F9)`, as the SDK's
  `SC4UI::GetView3DWin()` does. **"Not done" does not match** (the seagulls never toggle), but the effect is rendering
  only. Written into landfill.md §3.2.
- ~~**LG17:** the traffic network map comes through city slot 40 (`GetTrafficNetwork`, `intptr_t` in the SDK). Right?~~
  **ANSWERED 2026-10-14** from the exe (`0x004596F0`; created at `0x0045B07D`; used at `0x006BCDF6`). **Yes.** Slot 40
  returns the city's `cISC4TrafficNetworkMap*`: the interface pointer created with IID `0xCA0F81B9`, **borrowed**, with
  no `AddRef`. `PopulateZone` calls its slot 8 directly, with no QI and no release. Written into landfill.md §4.4.5
  and §8.
- ~~**LG18 — §1.1/§1.2:** which simulator does the original use for AddAgent before services are acquired? [the app's
  city's simulator, held until RemoveAgent]~~ **ANSWERED 2026-10-14** from the exe (`Init` `0x006B9D98`–`0x006B9DBA`;
  `Shutdown` `0x006B95D3`–`0x006B95ED`; `SL::OnCityInit` `0x00601C20`). The **city's simulator** (city slot 50), read
  from the cache global `0x00B43D5C` that `SL::OnCityInit` fills. It is not held. `RemoveAgent` fetches it again from
  the global city. **The bracket matches**: it is the same object, and only the hold adds a reference count. Written
  into landfill.md §1.1.
- ~~**LG19 — foundations §6/§12:** the core uses its own CRT-style generator per object (state 1), not the game's
  unseeded per-thread `rand()` sequence. Acceptable?~~ **ANSWERED 2026-10-14** from the exe (`UpdateSurroundingTiles`
  `0x006BCFC0`; `rand` `0x009EFB9F` at `0x006BD201`, `0x006BD271`, `0x006BD2DD` and `0x006BD34B`). **Acceptable; it
  is foundations §12.6's decided choice.** The original draws from the per-thread CRT sequence, which every
  simulation-thread caller shares, so the values cannot be reproduced. The count (one draw per mask-15 cell) and the
  formula `2 × (rand() mod 4)` are what must match. Written into landfill.md §4.8.
- ~~**LG20:** a message from another thread that cannot get the lock is dropped. Acceptable?~~ **ANSWERED 2026-10-14**
  from the exe (`DoMessage` `0x006B9EA0`) and foundations §18.2. The lock is glue: **no original behaviour; the
  implementer's choice stands.** The original takes no lock and handles every message, and all of them arrive on the
  main thread, so it never drops one. Dropping is therefore harmless only while it never happens. A dropped zoning or
  terrain message would change the zone, so drops should be logged, and waiting is safer. Written into landfill.md
  §3.2.
- Not done (each logged once): tile models, `SetEmptyCellTexture`, zone effects (trucks, seagulls), zot
  orientation, the 3D view level, `GetLandfillZoneFromCell`. The shadow cannot compare the zots, trucks and seagulls
  the original drew, the tile occupants it changed, the expenses it booked or the history it sent.

## From the traffic simulator core (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/traffic-simulator.md`. All §11 tests 1 to 15 are unit tests and `ts11_*` vectors (29 vectors).
TN1, TN3, TN14, TN15 and TN18 hold against the merged network core. The core's answer is in brackets.

- ~~**TS1 — §3.3:** does the pacing reset keep `rerouteBucket` (pass counter 2)? [kept; the other counters are
  zeroed]~~ **ANSWERED 2026-10-14** from the exe (`0x0072180D`–`0x00721881`): `rerouteBucket` is kept, and so is
  pass counter 1 (the re-route counter), which is zeroed on entering 17 instead (`0x007218F2`). Everything else is
  zeroed on entering 16: processed, day index, `toReRoute`, `toSearch` and the other counters, including the
  source counter. That counter therefore restarts at 0 every cycle. Written into §3.3 and §4.3.
- ~~**TS2 — §3.2 state 11:** what does UpdateNeighborConnections (`0x00714BE0`) do? [nothing]~~ **ANSWERED
  2026-10-14** from the exe: not nothing. It is traffic-network.md §2.7.5 (written with PF6 after the core was cut;
  re-checked against `0x00714BE0`): per neighbour connection, copy `0xAA9EA81A + i` to `0xAA9EA81D + i` and zero it
  (i = 0…2), then arrived-last := arrived-this-cycle and arrived-this-cycle := 0 in `0x8A0B4100` (k = 0…3). One call;
  pointer added to traffic-simulator.md §3.2.
- ~~**TS3 — §4.8 state 3:** what does ReduceTransitSwitches do? [the switch load restarts from 0; CreateMaps keeps the
  previous value in counter 1]~~ **ANSWERED 2026-10-14** from the exe (`0x00713750`): the bracket holds for the
  shipped tuning. Exactly: every switch's `load := r24(densityReduction × load)` with the volume-reset factor
  `0x29136788` (shipped 0), nothing else; counter 1 (`prevUsage`) and the ratio are CreateMaps' (§4.9 step 4). Use the
  factor, not a literal 0 (a tuning mod decays the load). Written into §4.8.
- ~~**TS4 — §3.1:** what do DoRoadChanged / DoRailChanged do on occupant messages? [on insert the cell joins the damage
  block's intact set unless present; on remove it leaves]~~ **ANSWERED 2026-10-14** from the exe (`0x007155D0`,
  `0x00715790`, `0x0070EA70`, dispatch `0x00724334`): the bracket holds for the sets, but not for the counts. Insert
  adds the key and does `cellCount += 1` even when the key is already present. Remove erases from the intact set,
  or else from the damaged set. A damaged removal does `damagedCount −= 1`, pothole count −= 1, and posts
  `0x69247DC8 (x, z, 0)`. `cellCount −= 1` in every remove case. Highway and GroundHighway skip the sets; rail skips
  them on a level crossing (Road/Street flag). A rail repair decrements the road pothole count *[quirk]*. The
  per-type counts are bumped and the budgets refreshed. Written as traffic-simulator.md §3.6.
- ~~**TS5 — §3.1:** message `0x66956810` "marks the network as changed": which flag, and who rebuilds? [sets
  `network_changed`, consumed by the trip phases]~~ **ANSWERED 2026-10-14** from the exe (`0x00724558`) and the Mac
  build: the bracket is wrong on the consumer. SimBegin and every network-occupant message (data3 `0xC772BF98`, even
  a failed cast) set one byte, which is saved and loaded (the `bool` after the intersection-dirty rectangle) and
  read by nothing. No rebuild follows; the network is kept current incrementally and rebuilt only by
  DoPostCityInit. Keep the byte for the round trip, with no behaviour. Written into traffic-simulator.md §3.1 and
  §6.1, traffic-network.md §3, save-records.md §7.1.1.
- ~~**TS6 — §3.1:** what does the network part do on neighbour added / removed? [nothing]~~ **ANSWERED 2026-10-14**
  from the exe: not nothing. It is traffic-network.md §2.7.2 (written with PF6; re-checked against `0x007153C0` and
  `0x00713AB0`, args data1 type, data2 x, data3 z). Add: `FindConnection`, type bit set, trip cost raised to the
  other side's `0xCA947077` (ordered compare), one pool per established city. Remove: bit cleared; on an empty mask,
  pool 0, the pool sweep and `freeDest` decrements for the destination byte. Pointer in traffic-simulator.md §3.1.
- ~~**TS7:** DoPostCityInit steps 4 (neighbour connections) and 5 (destination-map boundary pass)? [skipped; the
  network rebuild runs, then the transit multiplier]~~ **ANSWERED 2026-10-14** from the exe (`0x00720430`): neither
  is skipped. Step 4 is an Add per neighbour connection plus one pool sweep, step 5 zeroes the destination byte of
  each ring cell without a map exit (traffic-network.md §1, §2.7.2–§2.7.3). DoPostCityInit also does more than the
  network rebuild and the multiplier: state 12/13 → 8, a resume switch on the loaded state (source iterator, lot
  iterator, ferry re-route **only in state 19**), the strategy-bucket multimap rebuilt from the lots,
  `days := lotCount div 18`, and both department-cost updates. Written as traffic-simulator.md §3.5; §3.3 gains the
  missing `dayIndex < days` guard; traffic-network.md §1 step 8 and §4.9.3 corrected.
- ~~**TS8 — §4.9 step 2:** what is the commercial traffic window? [the 4 × 4 block at (x, z); travel types car 1,
  el-train 7, monorail 8; maximum morning usage plus maximum evening usage]~~ **ANSWERED 2026-10-14** from the exe
  (`0x00714AA0`, `0x00714940`): partly. The window is a plus shape: the block widened by 2 cells each way without
  the corners, up to 48 cells, clamped at 0 and at H, but **not** at W on the block rows (x mod 256: the east
  column of a 256 city reads x = 0, 1 *[quirk]*). The per-cell value is the max of road (Car + Bus + FreightTruck,
  u16 sum), el-train and monorail `usage`. `v = (maxMorning + maxEvening) mod 2¹⁶`, rounded ties-to-even after
  the scale. Written into traffic-simulator.md §4.9 step 2 with an example.
- ~~**TS9 — §4.9:** which records are "not excluded" from the statistics? [flag bit 0 clear]~~ **ANSWERED
  2026-10-14** from the exe (`0x0071A3A6`–`0x0071A496`, end `0x0071A5D7`) and the advisor: none is excluded. A
  record with flag bit 0 (network flag `0x40000`) goes to slot 13, which the traffic advisor publishes as **bridge**.
  Others go to slot `type`. Pedestrian records count too, so a road cell adds 2. At the end of the pass the per-type
  record counts also **replace** the cell counts that price the road and transit budgets; bridges drop out. A
  read-only corpus check agrees (saved cell counts = published counts; road counts = 2 × road keys). Written into
  traffic-simulator.md §4.9, §3.6 and §6.1 row 16, and save-records.md §7.1.1.
- ~~**TS10 — slot 7 SetupPathFinderForLot:** what else is configured? [the lot rectangle as start, plus SetupTrip
  steps 3–4 only]~~ **ANSWERED 2026-10-14** from the exe (`0x00711610`): more than the bracket.
  * What it sets: SetupTrip step 4 and the start rect, without the work time. Also destination filter 0 with
    destination flag `1 << ` the **first** trip's type. Then, for commuters, the 0x1CF/0x3 travel types and the
    starting-cost array of the dominant strategy (by summed trip counts, ties to the lower index); for freight,
    flag 1 and types 0x30.
  * It returns false when `pf` or `lot` is null, when the lot has no trips even after one `UpdateTrafficVolume`, or
    for a type > 3.
  * It never sets density, work time, flag 4 or a forced start.
  Written into traffic-simulator.md §4.4.
- ~~**TS11 — slot 35 GetTransitSwitchQueryData:** the data fields? [returns the network core's query target only]~~
  **ANSWERED 2026-10-14** from the exe (`0x0071CB80`, field names from the Mac query readers): five u32/f32 fields,
  available capacity, maximum capacity, load fraction (f32), actual usage, monthly maintenance cost. A switch gives
  its capacity, ratio and previous load. A ferry terminal gives its last-period riders three times, and 1.0. A strike
  zeroes all but the maximum capacity (switch) and the cost. The cost is the purpose-`0xCA550301` budget item's
  cost × department funding, truncated. Written as traffic-simulator.md §4.16 with an example.
- ~~**TS12 — §4.12:** what does RecalculateNeighborConnectionCapacities do? [nothing]~~ **ANSWERED 2026-10-14** from
  the exe: not nothing. It is traffic-network.md §2.7.4 (written with PF6; re-checked against `0x00714E60`: the
  demand nodes `0x2010/0x2020/0x2030` at `0x10000` truncated, `GetConnectedCities(list, false)`, per-pool
  `trunc(r24(r24(E/T) × J))` with unsigned `T` and `J`, slot-19 writes, then the edge pass that sets destination bits
  and counts `freeDest`). Pointer added to traffic-simulator.md §4.12.
- ~~**TS13 — §4.5:** a route with an off-map root is encoded as the in-city cell while its unit steps are relative to
  the off-map cell. [decoded literally: shifted by one cell]~~ **ANSWERED 2026-10-14** from the exe: unreachable in
  the original. Roots are seeded on rings clamped to the map, so a neighbour start `(−1, z)` roots at `(0, z)`, and the
  ferry fallback roots on the arrival cell. The reader (`0x006D64B0`) decodes the header bytes zero-extended and
  literally, with no off-map adjustment, so the bracket is what the original would do. Keep it *[choice]*. Written
  into traffic-simulator.md §4.5.
- ~~**TS14 — §4.3:** what does ConfirmRoute check? [during a strike any route using types 2, 3, 6, 7 or 8 fails; each
  unit step leaving an in-city cell needs an exit on that side for its type, or the cell's switch for a type change;
  jumps and off-map steps are not checked]~~ **ANSWERED 2026-10-14** from the exe (`0x0070E320`): partly. The
  strike test is not by travel type. During a strike, any type change after the first cell fails, and so does a step
  that the connection word refuses at a ferry cell. A route that keeps one type passes. The unit-step test is
  `conn[t]` bit `exit + 4·entry` (any entry at the first cell). If that bit is clear, the cell passes on the
  tunnel wormhole bit, or on the ferry bit when there is no strike. Jumps are checked too: the cell before a jump
  gets an exit side from the jump's direction, with a `+z` quirk for short and same-cell jumps. The first cell has
  no strike test, the last cell is never tested, and an empty route is false. Written into traffic-simulator.md §4.3
  with an example.
- ~~**TS15 — §4.5:** is the switch fee the switch's `0x0BFC0A4C` value read as f32? [yes]~~ **ANSWERED 2026-10-14**
  from the exe: yes. DoLotActivated reads it with the converting f32 getter (`0x00723C43`; absent → 0.0), and
  ApplyRoute charges `r24(Q × toll)` into the fees (`0x00719CCC`), which become the toll income. Shipped only on the
  four toll booths (0.1). The saved f32 after the rules is this value. Written into traffic-simulator.md §4.5, §6.1
  and traffic-network.md §2.4.
- ~~**TS16 — §4.5:** the ferry-leg counters? [round(Q) to counter 1 (mode 0), 2 (mode 1) or both (mode 2), on the
  departure terminal's link to that cell, else its neighbour link]~~ **ANSWERED 2026-10-14** from the exe
  (`0x00719E8F`–`0x0071A054`): partly. Neighbour links are as bracketed (current A, current B, both). On a terminal
  link, mode 2 adds current A to the forward link and current B to the other terminal's link back. The departure
  terminal's riders also get `Qi` (twice in mode 2). The fare goes to the income, not the fees. The u16 counters
  wrap. A crossing in from a neighbour is counted at the first cell (`0x00719A4E`). Written into
  traffic-simulator.md §4.5 and traffic-network.md §2.6.
- ~~**TS17 — §4.4:** a highway-return adjustment for an off-map destination? [none]~~ **ANSWERED 2026-10-14** from
  the exe (`0x0071C858`–`0x0071C987`, `0x0070DD30`): the bracket does not match. The return leg's start cell moves
  one cell along the edge when pf ended off-map by car or bus at a Highway/Avenue/GroundHighway connection (mask
  `0x1108`). Its end cell moves the other way when pf started off-map at such a connection (any type). The
  direction follows the language's driving side (UK English and Japanese drive on the left). Also a *[quirk]*:
  with no lot at an in-city end cell, pf2's start rect is not written. Written into traffic-simulator.md §4.4.
- ~~**TS18 — §4.2:** when is the capacity property promoted from u16 to u32? [u16 until a value reaches 0xFFFF]~~
  **ANSWERED 2026-10-14** from the exe (`0x0070DE70`): yes. It is created as 12 u16 zeros, and a write of a value
  `≥ 0xFFFF` (0xFFFF included) copies all 12 elements into a u32 array, which never reverts. Any other variant type
  is accessed as u32 with no count check (*[choice]*: treat it as missing). Also found: ApplyTrafficToDestination
  stores the pool's new arrived total into the found connection's own `0x8A0B4100`. Written into
  traffic-simulator.md §4.2.
- ~~**TS19:** does the pathfinder adapter's GetLot match any cell of a lot? [yes (`false`)]~~ **ANSWERED
  2026-10-14** from the exe: yes. All three pathfinder sites pass false (`0x006D7AC7`, `0x006D8688`, `0x006D8B1D`).
  `GetLot` (`0x0072B410`) with false answers at any cell of the lot; with true it answers only at the lot's origin.
  In the simulator, only ClearTripDestinations, RefreshCommuteMap's first lookup and UpdateSubnetworkInfo pass true.
  That corrects §4.11: each lot is rolled and counted once, not once per cell. Written into traffic-simulator.md
  §4.11 and §8.
- ~~**TS20:** is the traffic-source list a snapshot? [yes: lots, then connections, when the phase is entered]~~
  **ANSWERED 2026-10-14** from the exe (`0x007107A0`, `0x007107F0`): only for connections. `Reset` (entering 16, 17,
  18, and the load resume) refills a ref-counted copy of the connection list. Lots are walked live through the lot
  manager's safe iterator, so a lot removed before it is reached is skipped and a lot added during the phase is
  visited (appended). Written into traffic-simulator.md §4.1.
- ~~**TS21:** when are `toReRoute` / `toSearch` zeroed? [on entering state 16]~~ **ANSWERED 2026-10-14** from the
  exe: yes, only there (`0x0072184B`); Read restores the saved values. But `toSearch` also grows after
  ConfirmTrips. ReRouteTrips adds 1 per trip sent back for lack of capacity (`0x0071D20D`), and FullTrips adds 1 per
  split, together with the running phase's `total` (`0x0071D75A`). Written into traffic-simulator.md §3.3 and §4.4.
- ~~**TS22:** does ReduceEdgeDensities also clear ferry terminal cells? [yes, departure and arrival; missing records
  are not created]~~ **ANSWERED 2026-10-14** from the exe (`0x007110B0`): yes, and missing records are not created.
  The departure cell is cleared by the cell scan, because the ferry phase of every rebuild gives it network-info
  records. An explicit pass after the scan (no deadline test) clears each terminal's arrival cell. Written into
  traffic-simulator.md §4.8.
- ~~**TS23:** what does DoLotDeactivated do? [destination bytes over the lot := 0, its bucket entries removed,
  `freeDest` unchanged]~~ **ANSWERED 2026-10-14** from the exe (`0x0071E2F0`): partly. The bytes over the rect do go
  to 0, but only the one bucket entry matching the lot's location is erased. `freeDest[k]` is **decremented** for
  each bit set at the rect's NW cell, unless in state 10. It also removes the switch (with a rebuild), the
  `0xCA550301` budget item (`E −= cost`), the `0x0C3BF549` item and the ferry terminal. Written into
  traffic-simulator.md §4.1.3.
- ~~**TS24:** is the direction towards a non-adjacent rectangle defined? [yes, always]~~ **ANSWERED 2026-10-14** from
  the exe: yes, for both rect cases: the first cell's entry towards `R` (`0x00719B2D`) and the mode-1 last exit
  towards `sourceRect`. In modes 0 and 2 the last exit points at the cell `(destX, destZ)` and is defined only one
  step away. Also: `R` is the destination's rect in mode 1. Other sides are the reader's. An undefined side at a
  non-last in-city cell ends ApplyRoute and loses the route's income and fees (*[quirk]*, rare). Written into
  traffic-simulator.md §4.5.
- ~~**TS25:** when is "the last switch seen" on a route updated? [at every in-city cell, so leaving and re-entering a
  station counts again]~~ **ANSWERED 2026-10-14** from the exe (`0x00719C8A`–`0x00719CDB`): yes. `last` starts at 0
  and takes every in-city cell's ID, 0 and the subway-skip case included, so a re-entry counts again. The subway
  exception compares the cell's type with the **next** record's type, not the previous one. At the last cell that
  record is the cell itself. Written into traffic-simulator.md §4.5.
- ~~**TS26:** FinishTripMap donor search order? [distance first, then −x, −z, +x, +z]~~ **ANSWERED 2026-10-14** from
  the exe (`0x00710016`–`0x007101D7`): yes. Also: the lot's own cells and same-zone non-source lots are passed over
  without closing the direction. The value is the map byte at the cell found, and no donor writes 0 over the rect.
  Written into traffic-simulator.md §4.7.
- ~~**TS27:** default `trafficUnitsPerSlice`? [16 until calibrated]~~ **ANSWERED 2026-10-14**: no original behaviour;
  the implementer's choice stands. The original slices by a 4 ms wall-clock deadline tested once per unit (§3.3 step
  1, §3.4), so it has no units-per-slice constant. Note added to traffic-simulator.md §3.4.
- ~~**TS28 — tuning reads with short or missing data:** fewer vector entries [the rest keep their value]; the
  strategy re-read with fewer than 3 values [missing ones unchanged]; missing ordinance offsets [0]; wealth above 3
  [zeros].~~ **ANSWERED 2026-10-14** from the exe (`0x00716AE0`, `0x00711810`, `0x00718BB0`). Vectors: no, each
  has its own fill (speeds to none, costs/capacities/start costs to 0.0, bools to true, background to 1.0); the
  tile costs are reset even when missing, and income reads past the array (*[choice]* 0.0). The strategy and
  ordinance arrays read past their data when short, and wealth > 3 reads the following members. The brackets stand
  as *[choice]*s there; a missing ordinance property adds nothing. Written into traffic-simulator.md §4.1.2, §4.14.
- ~~**TS29:** max funding conversion from percent? [`r24(v / 100)`]~~ **ANSWERED 2026-10-14** from the exe
  (`0x005FDAC0`, `0x00716D67`): the bracket does not match. The percent getter multiplies by the f32 constant 0.01f
  for every numeric type, `r24(v × 0.01f)`; shipped 120 gives 0x3F999999 (1.1999999), not 1.2. Written into
  traffic-simulator.md §4.14.
- ~~**TS30:** does Read commit only on success? [yes: parsed first; a failed Read leaves the simulator unchanged]~~
  **ANSWERED 2026-10-14** from the exe (`Read` `0x00721990`, ref helper `0x005D9730`): the bracket is not the
  original but stands as a *[choice]*. Past the version test (< `0x13`: false, nothing changed) the original writes
  into the simulator as it reads, nulls each ref before reading it, sizes the destination map from uninitialised
  locals on an error, and returns `!GetError()`, which the loader ignores. Written into traffic-simulator.md §6.1.
- ~~**TS31:** an unknown iterator-state byte in the record? [Done]~~ **ANSWERED 2026-10-14** from the exe (reader
  `0x005DA410`, inline GetNext e.g. `0x0071A1A5`): the bracket does not match. Read stores the byte unchecked, and
  GetNext tests only 2 (done) and 0 (fresh); any other byte acts as running, so the scan steps on from the saved
  cell, and the byte is written back unchanged until the scan ends. Written into traffic-simulator.md §6.1.
- ~~**TS32:** record versions above 0x29? [read with the 0x29 layout]~~ **ANSWERED 2026-10-14** from the exe
  (`Read` `0x00721990`): **the bracket matches.** There is no upper bound and every gate is an unsigned ordered
  test (none an equality), so versions `0x2A`…`0xFFFF` read as `0x29`; trailing bytes are skipped by the loader's
  entry framing. Written into traffic-simulator.md §6.4.
- ~~**TS33:** Read's own switch flags: is 0x40 set for a subway / el-train rule? [yes, as the re-read does; TN25]~~
  **ANSWERED 2026-10-14** from the exe (`Read` `0x0072225E`–`0x0072227D`): **the bracket matches.** Read adds `0x40`
  for (from, to) = (6, 7) or (7, 6) of any kind, as the re-read does. Its other differences stay as
  traffic-network.md §4.7.2 lists them (kind-1 raw-`to` mask, `to = 5` → `0x20`). Written into §4.7.2.
- ~~**TS34 (fixtures only):** every cell reports a network occupant; department funding defaults to 1.0; department
  tokens are 900+.~~ **ANSWERED 2026-10-14**: no original behaviour; the implementer's choice stands. These are
  test-world stand-ins. The one with an original counterpart agrees: a department created by `CreateDepartmentBudget`
  starts at 100 % funding, a fraction of 1.0 (budget.md §6). The traffic simulator's `Init` then
  sets its maximum to the TS29 value (1.1999999 shipped).
- Not covered by vectors: path searches over a real network (the vector world has no network occupants; test 10
  drives ApplyRoute with the spec's route bytes); switches, tunnels and ferries in the save record round-trip only
  as empty lists (Read's switch mask and flags have a unit test). No plugin or capi integration yet (M5 glue).

## From the power grid shadow (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/power.md`. Ini: `power_shadow = 1` (needs our simulator and the ORIGINAL power simulator: do not
enable `power`); one line per recompute cycle at state 3, plus month, bookkeeping and month-summary lines. The
shadow's current answer is in brackets.

- ~~**PW1:** `FindUnvisitedConnection` is not in the spec. The shadow takes the first connection from
  `GetConnectionsOfTypeToCity` whose visited flag is false. The original's search order? [first unvisited]~~
  **ANSWERED 2026-10-14** from the exe (`0x006CF720`): **the bracket matches.** It walks the connection list from
  the head in append order and returns the first with type `t`, visited bit clear and other-side city `city` (no
  AddRef), the same order as `GetConnectionsOfTypeToCity`. Written into power.md §6.5.
- ~~**PW2:** the log date for SimNewDay, SimIdle and SimNewMonth comes from the message's `data1` as a julian date.
  Does SimIdle's `data1` carry the date? [assumed]~~ **ANSWERED 2026-10-14** from the exe (`SimulateIdleTime`
  `0x00702E38`, `SimulateAgents` `0x007038E7`): **the bracket matches.** Both read the simulator date's number
  (`GetSimDateNumber`, julian day) once per call and pass it as `data1` of every agent message, SimIdle included.
  Written into power.md §4.3.
- ~~**PW3:** the pass import, export and demand are not saved. What does the original hold after a load, before its
  first state 0? [counted as `not_exact` until then]~~ **ANSWERED 2026-10-14** from the exe (constructor
  `0x006EF805`–`0x006EF80B`; `Read`, `Init` and `Write` never touch them): **0.** The bracket is safe but not
  needed: the shadow can start them at 0 and compare exactly. A save taken in state 2 or 3 makes the original's
  state 3 publish only the post-load share (all 0 for a state 3 save). Written into power.md §9.1 with an example.
- ~~**PW4:** `brokenFraction` is not saved; the shadow starts it at 0. The original's value after a load? [0]~~
  **ANSWERED 2026-10-14** from the exe (constructor `0x006EF8A7`; written only there and in `ApplyPoleFunding`
  `0x006EC3F0`): **the bracket matches.** It is 0 after a load, while the saved brokenCount is kept, so the first
  month repairs poles down to the new target (all of them at 100 % funding). Written into power.md §7.6.
- ~~**PW5 — §5.3:** the dirty rectangle is not saved; the shadow uses the bounding box of the changed wire bits. Can a
  larger rectangle give different wire bits?~~ **ANSWERED 2026-10-14** from the exe (`UpdateWireBits`
  `0x006ED6E0`): **yes, through zones only.** The occupant query has a 2R margin, but the zone pass re-wires only
  zoned cells inside the rect. A cell of the smaller rect wired only by a zoned cell that only the larger rect
  holds loses its bit with the smaller one. *[choice]*: take the rect our simulator merges from the same messages.
  Written into power.md §5.3 with an example.
- ~~**PW6:** the shortfall block probe tries starts 0 to min(count, 32768), assuming one `rand()` of at most 0x7FFF.
  True for networks with more than 32768 blocks?~~ **ANSWERED 2026-10-14** from the spec's block rectangle and the
  C `rand()` (foundations §12): such networks cannot exist. The block count is at most (256 / 8)² = 1,024, and the
  start is one `rand() mod count` with `rand()` ≤ 0x7FFF, so the probe range is right and covers every start.
  Written into power.md §6.3.
- ~~**PW7 — §7.5:** the ordinance factor is read live when the shadow starts, then each month. Does the original read
  it anywhere else (at load, on an ordinance change)?~~ **ANSWERED 2026-10-14** from the exe (writers `0x006EF889`,
  `0x006ED6B2`, `0x006EC580`): **at load yes, on an ordinance change no.** The tuning read in `Init` (so also after
  a load) and on `0xCA1B151C` reads it, and so does each month start. `OrdinanceUpdated` is handled only for the
  Nuclear Free Zone, so a mid-month enactment takes effect at the next month. Written into power.md §8.
- Not compared (cannot be read without changing the game): posted messages, history values, budget line items,
  demolitions and lot conditions, which poles the month's shuffle breaks (the broken count is compared), deal
  flags, the dirty rectangle.
- ~~**PW8 — first in-game shadow (run 80, coordinator):** all 213 cycles and 94 month lines are exact, but every
  bookkeeping line differs a little: the original's running counts against our from-scratch insertion of the live
  occupants, e.g. consumers 725/722, requirement 6481/6480, 4 consumption-map cells (max |d| 2). Which occupants does
  the original count that a rebuild from the live occupants does not (or the other way round)?~~
  **ANSWERED 2026-10-14** from the exe (`0x006ECF2D`, `0x006EC25F`: values read at delivery; nothing rebuilds the
  counts) and a recount of the shipped saves: **no live occupant.** The persistent +2 (cells (41,102) and (55,111))
  is in the Konradshohe save itself. The saved counts exceed its own buildings by the mayor's statue and the
  cemetery, which consume 0 in the shipped exemplars (Tegel: the same at its cemetery). The rest are posted
  occupant messages not yet delivered at the month. Written into power.md §5.2 with a test, and the removal's
  `initialized` gate added to §5.

## From the plumbing grid shadow (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/plumbing.md`. Ini: `plumbing_shadow = 1` (needs our simulator and the ORIGINAL plumbing simulator:
do not enable `plumbing`); one line per pass at the original's swap and after its state 8, one per month. The
shadow's current answer is in brackets.

- ~~**PS1:** the order in which unvisited water connections are listed is inferred; the read-only host cannot find
  them, so the shadow reproduces it over its own visited flags. The original's order? [inferred]~~
  **ANSWERED 2026-10-14** from the exe (call `0x006DD7F8` with type 5 into `FindUnvisitedConnection` `0x006CF720`,
  PW1): **append order.** The list is walked from its head and the first type-5 connection to the deal's city with
  the visited bit clear is returned. This is the order of `GetConnectionsOfTypeToCity(5, city)` without the visited
  ones, so a shadow that walks that list over its own visited flags matches. Written into plumbing.md §6.2.
- ~~**PS2:** pipe condition is a distress fraction that drops to 0 at funding of 50% or more. Intended? [as stated]~~
  **ANSWERED 2026-10-14** from the exe (`ApplyPipeFunding` `0x006DD110`): **the spec's formula is right, the summary
  is not.** The curve is a monthly change: below 50 % cond grows (0.01 a month at 0 %), at 50 % it holds, and above
  50 % it falls gradually (0.02 a month at 100 %). It is 0 at 50 % or more only when it was 0 already, as after a
  load. New quirk: a step above 1 sets 1 and skips the "no pipes → 0" test. Written into plumbing.md §7.1 with a
  400-pipe example.
- ~~**PS3:** the original's pipe condition is not saved and not observable; the shadow starts its own at 0, so the
  first month after a load assumes the original also restarts at 0. Right?~~ **ANSWERED 2026-10-14** from the exe
  (cond `+0x180`: written by the constructor `0x006DC61C` and by `ApplyPipeFunding` only): **right, the bracket
  matches.** `Read`, `Init` and `Shutdown` never touch it, so the original restarts at 0 too. The first month
  then repairs the saved distressed pipes down to the new target. The ageing flag is not saved either, so a save
  taken between a month start and its state 8 loses that month's ageing. Written into plumbing.md §7.1 with an
  example.
- ~~**PS4 — §5.3:** demand is a running sum the original builds as things are added; the shadow takes it from the
  original's record instead of recomputing it. Acceptable? [taken from the record]~~ **ANSWERED 2026-10-14** from the
  exe (demand is written only by `DoBuildingChanged` and `Read`) and the shipped saves: **yes, the bracket matches.**
  The original never recomputes it either. Take it at the comparison, not at the pass start (PS8). In the eight
  base-game saves with buildings the saved demand equals a recount of the saved buildings exactly (Konradshohe 23845),
  so here, unlike power (PW8), nothing is inherited. Written into plumbing.md §5.3.
- ~~**PS5:** the monthly funding comes from department `0xAA4014B4`, a choice the spec does not state. Which
  department? [`0xAA4014B4`]~~ **ANSWERED 2026-10-14** from the exe (`DoMonthlySimulation` `0x006DF543` reads the
  member `+0xA4`; `Init` `0x006DF238`–`0x006DF258` creates it as `0xAA4014B4`): **the bracket matches.** The funding is
  the simulator's own held department. That is `0xAA4014B4` on a new city and the saved ref after a load, and the
  saved ref is `0xAA4014B4` in every save checked. Production (§7.2) and ageing (§7.3) read the same member.
  Written into plumbing.md §7.1.
- ~~**PS6:** a pass's start is seen only when state 2 changes a probe cell; if the next working grid already matches
  what state 2 would write, the start is missed and the pass is marked not exact. [as stated]~~ **ANSWERED
  2026-10-14** from the exe (`DoIdle` state 0 `0x006DD3D9`): the probe is shadow glue, so the implementer's choice
  stands, but **the original has a better marker.** passCounter (record field 7, object `+0x9C`) is incremented once
  by state 0 and written otherwise only by the constructor and `Read`. A record read whose passCounter is one higher
  than the previous read means that call ran state 0, with no probe cell needed. *[choice]*: detect starts by
  passCounter. Written into plumbing.md §6.1.
- ~~**PS7:** when a state 8 may have run unobserved, ageing is logged as `unknown` and producer condition is not
  compared. [as stated]~~ **ANSWERED 2026-10-14** from the exe (ageing flag: set `0x006DF551`, tested `0x006DEDBD`,
  cleared `0x006DEDD9`): shadow glue, **no original behaviour; the implementer's choice stands.** Facts that can narrow
  "unknown": exactly the first state 8 after a month start ages, and an ageing changes each producer's
  `0xC910B4CF` whenever its increment is not 0, which the shadow can read. Written into plumbing.md §6.7.
- Not compared: the pipes' watered states (only reachable through a non-virtual call), redisplay, posts, history,
  line-item expenses, lot conditions and deal flags; the original's pipe condition and pass state (not saved, not
  exposed). New bursts are range-checked only (random draw).
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
- **PS9 — first in-game shadow (run 81, coordinator):** in one pass (2028-03-04, pass 357) producer 0's production
  and efficiency read 0 in ours (orig 19944 and 0.997), although the pass's produced total matches (39687) and the
  grid is identical. This looks like a shadow-side read of the wrong producer record (not an RE question); the next
  plumbing lane should look at how the shadow pairs our producers with the original's.

## From applying the crime answers (filed by the coordinator from the `apply2` lane's report, 2026-10-14)

About `docs/specs/crime.md` (CQ5, CQ8, CQ13 as applied). The glue's current answer is in brackets.

- ~~**CQ16 — CQ8:** the LTEXT string loads but the game's detokenizer returns false. [The load counts as failed;
  `0xAA12FCB0` replaces the name only if that property exists, otherwise the copied text stays.]~~ **ANSWERED
  2026-10-14** from the exe (`SC4LoadString` `0x00603040`: detokenizer call `0x00603133`, failure exit
  `0x00603202`; crime loader reset `0x006A0C9C`–`0x006A0CA6`) and written into crime.md §7 ("Name", steps 0 and
  "A failure in steps 2 or 3"). **Half right.** The load does fail, and `0xAA12FCB0` replaces the name only if it
  exists. But the copied text does not stay. `SC4LoadString`'s failure exit calls `Resize(0)` on the output, so
  the name is empty. The loader had already emptied it before the load, so every failed load leaves the name
  empty unless `0xAA12FCB0` exists.
- ~~**CQ17 — CQ8:** property `0x8A416B2E` has fewer than 3 values. [No key, no lookup; the fallback is used.]~~
  **ANSWERED 2026-10-14** from the exe (`SC4LoadExemplarString` `0x00603220` → `cSCPropertyHelp::GetPropertyValue`
  `0x005FD5B0`, variant slot 103 `0x0060CE80`) and written into crime.md §7 ("How step 1 reads the key"). **The
  original has no such rule:** it copies three `uint32` from the variant's data pointer with no type or count
  check. With fewer values it reads past the data block, so `G` or `I` is heap garbage, and a scalar's inline
  value is used as a pointer. The lookup then almost always misses, and the fallback applies. **The bracket stands as
  a *[choice]*:** treat fewer than 3 values (or a scalar) as missing, which gives that usual outcome.
- ~~**CQ18 — CQ8:** there is no language manager. [Language 0, so only (group, instance) is tried.]~~ **ANSWERED
  2026-10-14** from the exe (`SC4LoadString` `0x00603063`–`0x0060306C`; `RZ::LM` `0x00913D7A`, system service
  `0x441E5070` through `cRZSysServPtr` `0x00610800`) and written into crime.md §7 ("The language ID"). **The original
  crashes**: it calls slot 7 through the null language-manager pointer, with no test. The game always has one.
  **The bracket stands as a *[choice]*:** language 0, so only (group, instance) is tried.
- ~~**CQ19 — CQ8:** is the detokenizer returned by `GetStringDetokenizer` (`cISC4App +0xA8`) borrowed or owned?
  [Borrowed; the glue takes and releases its own reference around the call.]~~ **ANSWERED 2026-10-14** from the
  exe (`cSC4App::GetStringDetokenizer` `0x0044B4D0`; cache `SL::OnAppInit1` `0x00602323`; use in `SC4LoadString`
  `0x00603121`–`0x00603133`) and written into crime.md §7 ("The detokenizer is borrowed"). **The bracket matches.**
  The getter returns the application's member with no `AddRef`, and the game caches it once in a global, also
  with no `AddRef`. `SC4LoadString` uses that global when its caller passes none, and the crime loader passes
  none. Nobody adds or releases a reference around the call. The glue's own `AddRef`/`Release` pair is harmless.
- ~~**CQ20 — CQ8:** how does the original read the LTEXT: through the resource manager's `GetPrivateResource`, or a
  string loader of its own? [Through `GetPrivateResource`, as the SDK does.]~~ **ANSWERED 2026-10-14** from the exe
  (`SC4LoadString` `0x006030B8`/`0x006030F9`; resource manager slot 3 `0x009732EB`, slot 4 `0x00972D60`) and written
  into crime.md §7 ("How the resources are read"). **The bracket matches.** The LTEXT is requested from slot 4,
  `GetPrivateResource`, as a `cIGZString` (`0x089B7DC8`), with the last two arguments 0 and null. Slot 4 bypasses the
  cache that slot 3 (`GetResource`) uses. Step 1's exemplar is also reloaded through slot 4 (`0x006032AE`), apart
  from the loader's own cached `GetResource` load.
- ~~**CQ21 — CQ5:** do other generators seeded from `cRZTime` use the same seconds-since-1901 value, so the helper
  belongs to the RNG module? [Crime module only.]~~ **ANSWERED 2026-10-14** from the exe (34 static initialisers
  `0x00A71CB0`–`0x00A75270`, construction occupant `0x005CEDAC`, prop developer `0x006F34A3`, all calling
  `cRZTime::cRZTime` `0x00470B50`) and written into crime.md §9 ("The clock value is shared") and foundations §6.3.
  **The bracket does not match: yes, they all use it.** Every clock-seeded generator calls the same `cRZTime()`
  constructor: 34 file-local generators (each XOR its own constant), the construction occupant's plain reseed, and
  the prop developer (value × 28). The helper belongs with the RNG module.
- ~~**CQ22 — CQ13:** the crime effect property is an array whose elements are not plain numbers (strings, objects).
  [Treated as absent.]~~ **ANSWERED 2026-10-14** from the exe (`DoOccupantChanged` `0x0069F27B`–`0x0069F2B1`, variant
  raw accessor `0x0060CE80`) and written into crime.md §6.10. **Half right.** The original reads bytes 0 and 1 of the
  variant's data block whatever its type. A **string** is an `RZChar` array, so it gives the codes of its first two
  stored bytes (`"AB"`: m = 65, r = 66), a deterministic result; FreeCity4 should read it like any array, not treat
  it as absent. An **object array** can only be stored by code, never by a data file, and gives the low bytes of
  an object's address. Treating that as absent stays a *[choice]*.

## From the land value and flammability shadows (filed by the coordinator from the lane's report, 2026-10-14)

About `docs/specs/landvalue.md` and `docs/specs/flammability.md`. Ini: `landvalue_shadow = 1`,
`flammability_shadow = 1` (each needs the ORIGINAL simulator: do not enable `landvalue` / `flammability`). The land
value shadow compares on `LandValueMapChanged` (`0x87CC75B3`, end of state 8); the flammability shadow on
`SimCycleComplete` (`0x08F415FE`), with a mirror fed the original's six messages. The shadow's current answer is in
brackets.

- ~~**LV1:** with several growth developers, is the developers' address order the tuning's listing order? [not
  compared]~~ **ANSWERED 2026-10-14** from the exe (tuning reader `0x006BE110`; developer creation `0x006991A0` from
  the resource enumeration callback `0x00699E20`, load factory `0x006B7870`) and written into landvalue.md §7.4.
  **No.** The list is a map keyed by the developer pointer as an unsigned 32-bit value, walked from the smallest:
  ascending pointer order, fixed by the allocator. That matches neither the tuning order nor, in general, the
  creation order. A shadow reproduces it exactly by sorting the pointers `GetDeveloper(ID)` returns.
- ~~**LV2:** a desirability map can change between the original's pass reading it and the comparison. [rerun with the
  previous map and keep the better result]~~ **ANSWERED 2026-10-14** from the exe (growth developer slot 13
  `0x007C9FB0` returns the published grid; state 17 copies the working grid into it in one call; land value state 7
  reads and applies it in one call) and written into landvalue.md §7.4. **The bracket is sound.** The map that state
  7 sees is never half-built and changes at most once before the comparison, so there are exactly two candidates:
  the map before the developer's last state-17 copy, and the live one. The developer's slot 10 tells a shadow when to
  snapshot the first.
- ~~**LV3:** is `GetAreaSlope` on the lot developer free of side effects? [assumed read-only]~~ **ANSWERED 2026-10-14**
  from the exe (`0x006C5DC0`, its vertex reader `0x006C4310`, terrain `0x007413C0`/`0x00744980`) and written into
  landvalue.md §6. **Yes, read-only**: terrain altitude and water level reads, locals only.
- ~~**LV4:** `GetLandValueType` is sampled at each tract's first cell. Is the type uniform over a tract, or which
  cell does the original use? [first cell]~~ **ANSWERED 2026-10-14** from the exe (`0x006BD700` calls the total grid's
  `GetCellValue`, which shifts the cell to its tract) and written into landvalue.md §7.6. **Uniform**: every cell of
  a tract gives the same value and type, so the first cell is exact.
- ~~**FL1:** is 2 × tractsPerDay the right bound for tracts left unvisited at `SimCycleComplete`? It assumes the
  message is delivered queued, after the day agent. [2 × tractsPerDay]~~ **ANSWERED 2026-10-14** from the exe
  (framework `AddToTick` `0x00879BB9` / `OnTick` `0x0087A74B`; tick priorities 5,000,000 for the message server and
  −2,000,000 for the simulator; message server tick `0x0092E040`) and written into flammability.md §9. **Right for
  shipped tuning.** The message is delivered at the next frame's message-server tick, which comes before the
  simulator's. Up to then the day agent visits the rest of the wrap day (≤ `tractsPerDay`) plus one more day only at
  speed 2, where 50 ms per day fits in a 66 ms tick. A mod with a shorter day at speed 2 raises the bound.
- ~~**FL2:** the multiplier g is 0 until the first season or ordinance message; the mirror takes the original's g at
  the baseline. Right? [taken at the baseline]~~ **ANSWERED 2026-10-14** from the exe and written into
  flammability.md §6.4. **Right.** Only the constructor, `Read` and the SeasonChange/OrdinanceUpdated branch of
  `DoMessage` (`0x006AFB87`) write g.
- ~~**FL3:** a tract-size difference between the original and ours is reported as `INCOMPLETE`. [INCOMPLETE]~~
  **ANSWERED 2026-10-14**: no original behaviour; the implementer's choice stands. The original creates its grids
  with tract size 1 and never changes it, so a difference needs a save the original did not write (flammability.md §2).
- ~~**FL4:** the message server's delivery order to the original's target and to the mirror is unspecified; both read
  the same game state. Is that enough? [assumed so]~~ **ANSWERED 2026-10-14** and written into flammability.md §5.
  **Yes, and the order is not unspecified**: one send delivers to the subscribers in subscription order (foundations
  §5.1), so the original, which subscribes in `Init`, gets each message just before a later-subscribed mirror. The
  handlers read nothing that the subscribers in between write *[inferred]*.
- ~~**FL5 — first in-game shadow (run 82, coordinator):** 64 of 65 compared cycles exact; in cycle 66 (2033-03-15)
  a 2×2 block of adjusted tracts, (7..8, 53..54), is 81 in the original and 65 in ours (max |d| 21; raw grid
  identical). One lot's flammability contribution changed in a way the mirror missed or counted differently: which
  event sets an adjusted tract outside the day visit (a lot condition or occupant change mid-cycle)?~~ **ANSWERED
  2026-10-14** with FL6, from the exe (`DoDailySimulation` `0x006AF8D0`), and written into flammability.md §6.5, with a
  worked example. **No event outside the day visit: it is the visit's no-water factor.** 81 = roundHE(65 × 1.25),
  1.25 being the shipped no-water multiplier. The original visited those tracts while plumbing slot 11 said "not
  watered". The plumbing pass published a watered state for them later, so a mirror that evaluates the watered state
  at comparison time misses the factor.
- ~~**FL6 — second occurrence (run 83, coordinator):** 69 of 70 compared cycles exact; in cycle 15 (2027-08-11)
  595 adjusted tracts differ (raw grid identical; max |d| 21), starting with the column (0, 42..46): 100/80,
  106/85 original/ours, the city average 43 against 40..41. The original is about 1.25 times ours, the same ratio
  as FL5's 81/65, so a ×1.25 factor reached a whole region mid-cycle. Which event applies it (a fire, a disaster or
  an ordinance), and does it go through the day visit?~~ **ANSWERED 2026-10-14** with FL5 (flammability.md §6.5).
  **None of those: a plumbing pass that changed the watered state of a region mid-cycle, and yes, through the day
  visit** (100 = 80 × 1.25, 106 = roundHE(85 × 1.25)). Fires and disasters only read the grid, and g is never 1.25 with
  shipped tuning. The mirror must accept either watered state for tracts whose state changed during the cycle. A
  change of the plumbing simulator's `GetWateredGrid()` pointer marks each pass.

## From applying the data-format answers (filed by the coordinator from the `apply4a` lane's report, 2026-10-14)

About `docs/specs/data-formats.md` (EX1–EX20 as applied in `engine/formats` and the core variant). The current
answer is in brackets.

- ~~**EX21 — §4.3:** "multiply the scale by 0.1": the double constant 0.1 or the float `0.1f`? [double 0.1: it
  reproduces every worked example, while `0.1f` turns `0.05` into `0x3D4CCCCE`]~~ **ANSWERED 2026-10-14** from the exe
  (`fmul` with the double operand at 0x00A9EFE0 = `0x3FB999999999999A`, at 0x009104A7; 10.0 is the double at
  0x00AA2028) and written into data-formats §4.3 ("Details of the arithmetic").
  - The double constant 0.1, used as a double operand: each scale step rounds the exact product scale × 0.1 once, to
    the caller's precision. The bracket matches. With `0.1f`, `0.05` would give `0x3D4CCCCE` instead of `0x3D4CCCCC`.
- ~~**EX22 — §4.3:** is `value × 10 + digit` rounded once or twice (after the multiply and after the add)? [twice,
  each to 24 bits]~~ **ANSWERED 2026-10-14** from the exe (`fmul` then `faddp` at 0x00910476–0x0091048A, each result
  rounded under the precision control) and written into data-formats §4.3 ("Details of the arithmetic").
  - Twice, each to 24 bits on the main thread. The bracket matches. A fraction digit rounds three times (scale × 0.1,
    digit × scale, + value).
  - Worked example: `33573415` gives 33,573,412.0, where one rounding would give 33,573,416.0.
- ~~**EX23 — §4.3:** does the exponent accept a `+`, and is it read when no mantissa digit came first? [yes to both]~~
  **ANSWERED 2026-10-14** from the exe (exponent test at 0x009104C2 after both mantissa loops; sign at
  0x009104D4–0x009104DE) and written into data-formats §4.3 ("Details of the arithmetic").
  - Yes to both; the bracket matches. `+` and `-` are accepted after the `e`, and the exponent is read whatever the
    mantissa consumed, so the value is 0 × 10ⁿ: `e5` gives 0, `-e5` gives −0.0, `2.5e+3` gives 2500, and `e-32` gives
    the default NaN (`0xFFC00000`), as `0e-32` does.
  - In the §5.1 round trip the classifier stops at an `e` with no digit before it, so `e5` there never reaches the
    parser.
- ~~**EX24 — EX8:** how is −0.0 printed? [`0`]~~ **ANSWERED 2026-10-14** from the exe (the runtime's `$I10_OUTPUT`
  0x00A04942 resets the sign to positive on its zero path at 0x00A04BC1; `_fcvt` 0x009F662C reports sign = (sign is
  '-'); printer 0x0091052A) and written into data-formats §5.1 ("Negative zero").
  - `0`; the bracket matches. The sign of a zero is dropped inside the C runtime's conversion, so no `-` is printed,
    for a Float32 or a Float64.
- ~~**EX25:** signed decimal text read into a Float32: one rounding from the exact value, unlike a Sint64 scalar,
  which goes through double? [one rounding, so `-16777217` gives −16777216.0]~~ **ANSWERED 2026-10-14** from the exe
  (`ConvertFromString` signed branch: `fild` of the int64, then `fstp dword` at 0x00916BF6–0x00916BF9; scalar
  converter 0x00915B85 stores a double first) and written into data-formats §5.1 (Parsing, step 4).
  - One rounding from the exact value; the bracket matches: `-16777217` gives −16,777,216.0.
  - A Sint64 scalar rounds twice (to the stored double, then to Float32). The two differ only above 2⁵³ in magnitude:
    −(2⁵⁴ + 2³⁰ + 1) gives −(2⁵⁴ + 2³¹) from text, but −2⁵⁴ from a scalar.
- ~~**EX26:** a Bool scalar's "leading spaces trimmed": spaces only, or tabs too? [spaces only]~~ **ANSWERED
  2026-10-14** from the exe (the Bool branch of `InterpretEntry` 0x0067E780 calls `cRZString::LTrim` 0x0090E869, whose
  character set at 0x00ACE73C is space and tab) and written into data-formats §4.3 (`Bool`).
  - Spaces **and tabs**, the same trim as array values. The bracket differs: a value starting with a tab before `true`
    reads as true.
- ~~**EX27:** a String value is "trimmed" of what? [spaces and tabs]~~ **ANSWERED 2026-10-14** from the exe (the
  String branch of `InterpretEntry` 0x0067E780 calls `cRZString::Trim` 0x0090E8A6 = `LTrim` + `RTrim`, set " \t" at
  0x00ACE73C) and written into data-formats §4.3 (`String`).
  - Spaces and tabs, at both ends; the bracket matches. The trim runs before one leading and one trailing `"` are
    removed, so white space inside the quotes is kept.
- ~~**EX28 — EX8:** a Float32 signalling NaN as text? [the x87 load quiets it: `1.#QNAN`; a Float64 sNaN gives
  `1.#SNAN`]~~ **ANSWERED 2026-10-14** from the exe (call sites `fld dword` 0x00916F62/0x009170FD and `fld qword`
  0x00916F67/0x00917117, then `fstp qword`; the printer reloads at 0x00910542; runtime `1#SNAN` branch 0x00A049E6) and
  written into data-formats §5.1 ("Signalling NaN").
  - Float32 sNaN: `1.#QNAN` (or `-1.#QNAN`); the bracket matches.
  - Float64 sNaN: **also `1.#QNAN`**; the bracket (`1.#SNAN`) differs. The double is loaded onto the x87 stack before
    printing, and loading an sNaN quiets it under the masked invalid-operation exception. `1#SNAN` is unreachable from
    this printer.
- ~~**EX29 — EX8:** does the 63-character cut also apply to Float32 text (a subnormal reaches it)? [yes]~~ **ANSWERED
  2026-10-14** from the exe (`0x0091052A`, callers `0x00916F77` and `0x0091711E`, `_fltout2` `0x00A0277B`) and written
  into data-formats.md §5.1 "Floats as text". **The bracket matches.**
  - The cut applies to any text longer than 63 characters, whatever the type. The printer returns false and both
    callers ignore it, keeping the cut text.
  - A Float32 reaches it only as one of the seven negative subnormals `0x80000001` to `0x80000007` (below 1e-44 in
    magnitude). `-0.`, 44 zeros and 17 digits make 64 characters, so the last digit is dropped.
  - **Correction to EX8:** the digits are not the exact binary value cut at 24 digits. `_fcvt` gets **17 significant
    digits, rounded half-up**, from the runtime (the count 17 is passed at `0x00A027A0`), padded with zeros. The
    24-digit limit never takes effect. Float32 0.1 prints `0.10000000149011612`, not `0.100000001490116119384765`, and
    3.14159 prints `3.1415901184082031`. Any implementation of the EX8 rule needs the same change. Test 58 and the §11
    row are corrected.
- ~~**EX30 — EX17:** `get_void` with a buffer shorter than `length`? [copies what fits; the original would overrun]~~
  **ANSWERED 2026-10-14**: no original behaviour; the implementer's choice stands. The original (`0x00915714`) copies
  `min(length, n)` bytes with a plain memory copy (`0x009F19B0`) and has no way to know the buffer's size, so the
  bracket's description of it is right: it overruns a buffer shorter than `length` when the array is longer. No
  shipped record has a void array, so this is not observable. Written into data-formats.md §5.2.
- ~~**EX31 — EX15:** when do factories run? [eagerly, inside `load_package`; a factory registered later applies from
  the next `load_package`]~~ **ANSWERED 2026-10-14** from the exe (`GetResource` `0x009732EB`, loader `0x00972EAE`,
  cache offer `0x009733C6`, segment registration `0x0097437B`/`0x009743C1`/`0x009742B7`) and written into
  data-formats.md §3 "When records are read". **The bracket does not match the original's timing.**
  - Registering a segment reads nothing. A factory runs lazily inside `GetResource`, the first time an uncached key is
    requested, and reads the record from the segment that wins at that moment. A success goes to the first registered
    cache that accepts it; a failure is not kept and is read again next time.
  - So a factory registered later applies at once to uncached keys, including keys in packages loaded before it, not
    from the next package.
  - *[choice]* Eager reading stands if (1) later packages replace not-yet-looked-up reads of the same key (or all
    packages load before the first lookup), (2) failed records are retried after later packages, and (3) a late
    factory is applied to earlier packages. Which resource caches the game registers was not traced.
- ~~**EX32:** a Float32 that no text reproduces under the 24-bit parser: what does the writer do? [`to_text` fails
  with `Unsupported`]~~ **ANSWERED 2026-10-14** from the exe (`WriteEntry` `0x0067CDD0`,
  `RemoveUnnecessaryFloatingPointDigits` `0x0067C2B0`, `SerialWrite` `0x00680030`, factory constructor `0x0067E373`)
  and written into data-formats.md §4.3 "Writing text". **The bracket does not match the original**, but it stands as
  a *[choice]*.
  - The original prints each Float32 with `%f` and trims trailing fraction zeros. It never fails and is lossy: 3.14159
    is written `3.14159` and reads back 2 ulps low, 1.0000001 is written `1`, and 1e-7 is written `0`.
  - The factory writes text by default (write-mode byte 1 from the constructor; no caller of slot 12 `0x0067CB10`
    found by pattern search). So any exemplar the game writes into a save is probably text.
  - The reader accepts both forms, so FreeCity4's writer need not copy the loss. Failing with `Unsupported`, writing
    text that reads back exactly, or writing binary are all compatible.
- ~~**EX33 — §5.1:** the core property readers apply the scalar conversion steps to every array element (e.g. a
  Float32 array read as `u32`). Right for readers that use raw access plus their own conversion? [every element]~~
  **ANSWERED 2026-10-14** from the exe (`0x005FF000`, `0x005FF510`, `0x005FFB40`, `0x005FFC00`, `0x005FFCC0`,
  `0x005FFD80`, bit copy `0x005FDDC0`) and written into data-formats.md §5.2 "The vector helpers". **The bracket does
  not match the original**, but the *[choice]* stands.
  - The original's vector helpers convert nothing. They copy `GetCount()` elements of their own width from the raw
    data pointer: the same width gives the bits, a narrower stored type is over-read past its buffer, a wider one
    gives its leading bytes, and a scalar gives an empty vector.
  - On shipped data only `0xEA0232A0` (Sint32 read as `vec<u32>`) meets a different type, and there copying and
    converting give the same bits (§7.4). So converting every element is not observable, and it replaces the over-read
    with defined values.
- Not applied: the scalar RZUnicodeChar text rule (one byte when ≤ 0xFF): the reader cannot produce that scalar
  (tag `0x0D` is not read).

## From the x87 fix: demand's corrected return channels (filed by the coordinator from the `x87fix` lane's report, 2026-10-14)

About `docs/specs/demand.md` §4.1 and §5 after the ABI correction (slot 8 `CalculateJobsPerUnitOfDemand` returns
void and writes jobs[1..3] of a four-float record). The current answer is in brackets.

- ~~**DQ10 — §5:** `GetJobsBySensus` still reads `j0 + j1 + j2`, and ExtrapolateBusinessCensus reads `j[w]`,
  w = 0..2, but the outputs are now jobs[1..3]. Which elements are summed, and which one does wealth w read? [the
  three outputs, summed in order: f32(f32(out1 + out2) + out3); extrapolation reads output w for wealth w + 1]~~
  **ANSWERED 2026-10-14** from the exe (`GetJobsBySensus` `0x006A3E60`, sum `0x006A3E8C`–`0x006A3E9C`;
  ExtrapolateBusinessCensus `0x006A6BD8`–`0x006A6C83`) and written into demand.md §5 (slot 9 row and
  "`GetJobsBySensus` arithmetic") and §9.2. **The elements are right, the order is not.** The three outputs are
  summed **f32(f32(jobs[3] + jobs[2]) + jobs[1])**, last output first. Then `x := f32(supply × sum)` goes to `_ftol`
  with no `≤ 0` clamp, and the function returns EAX. The order matters for whole percents: 1, 3, 96 sum to
  0.99999994 in the original and 1.0 in the bracket's order, so supply 1000 gives 999, not 1000. The two orders
  give different sums for 1198 of the 5151 splits of 100 %. **Extrapolation:** each wealth reads its own output. $
  (`0x12010`, `0x2010`) reads jobs[1], $$ reads jobs[2] and $$$ reads jobs[3], each `f32(jobs[k] × n)`.
- ~~**DQ11 — §5:** in the original `GetJobsBySensus`, what is in the local record's jobs[0] when slot 8 returns, and
  is it really summed with jobs[3] left out? [not modelled; the sum of the three outputs]~~ **ANSWERED 2026-10-14**
  from the exe (`0x006A3E82` passes the address of a 16-byte local; the `FLD`/`FADD`s at `0x006A3E8C`–`0x006A3E9C`
  read its offsets `+0xC`, `+8`, `+4`) and written into demand.md §5. **The bracket matches.** jobs[0] is
  uninitialised stack that nothing writes or reads, and jobs[3] is summed. "j0 + j1 + j2" was a misreading from
  before the slot-8 correction. Nothing to model.
- ~~**DQ12 — §5 against §4.1:** §5 says `f32(percent) × 0.01f`, while §4.1 describes an exact FILD followed by a
  single rounding. They agree for every percent below 2^24; which is the game's? [the §4.1 link factor]~~
  **ANSWERED 2026-10-14** from the exe (slot 8 `0x006A2469`–`0x006A24CD`, the graph builder's `0x006A5514`,
  `0x006A5604`, `0x006A56F4`; the 2^32 constant at `0x00A80AA8`) and written into demand.md §4.1 and the §5 slot 8
  row. **The bracket matches; both use the same instructions.** One correction for very large percents: below 2^31 it
  is one rounding, `r24(percent × 0.01f)`. From 2^31 up, the `FADD` of 2^32 rounds the integer first, so it is
  `r24(r24(percent) × 0.01f)`. Example: 3242996796 gives `0x4BF76BC7`, not `0x4BF76BC8`. The shipped percents (at most
  1500) cannot tell any of these apart.
- Not a spec question: whether the WOW64 `CONTEXT` TagWord is the full or the abridged tag. The crash report logs
  both; the lane's host test shows the full tag (0xFFFF empty, 0x3FFF with one value) next to FXSAVE's 0x00/0x80.

## From the residential glue and shadow (filed by the coordinator from the `resglue` lane's report, 2026-10-14)

About `docs/specs/residential-education-health.md` (REH) and the residential glue (`cFC4Residential`,
`FC4ResidentialShadow`). The current answer is in brackets.

- ~~**RG1:** buzzards (the visual effects): what creates them? [not created: the effect callback returns 0, logged
  once]~~ **ANSWERED 2026-10-14** from the exe (`0x006FA649`–`0x006FA83E`; effects manager slot 7 `0x005939B0`; name
  string `0x00AB2BD8`; the shipped effect directory in `SimCity_1.dat`) and written into REH §4.7 step 3. **The
  bracket differs: the game creates them.**
  - **Who creates them.** The class calls the city's effects manager (city slot 85, cached in `0x00B43D1C`), slot 7
    `CreateVisualEffect("buzzards", &out)`. It positions the effect, starts it and releases it.
  - **Shipped data.** The effect directory defines `buzzards`, so the create always succeeds and both draws happen. An
    unknown name returns AL = 0 with `out` untouched; then only one draw is made, the tract flag stays clear and the
    next wealth level may draw again, which shifts every later strike roll.
  - *[choice]* The core asks the host. A host that cannot show effects reports success, so the core makes both draws
    and sets the flag, as the original does with shipped data.
- ~~**RG2 — REH §4.1 step 4:** building slot 11 is read as `GetBuildingProfile()`, first u32. Right? [yes]~~
  **ANSWERED 2026-10-14** from the exe (call `0x006FB9CC`; building slot 11 `0x00459250` → BDS slot 39) and written
  into REH §4.1 step 4. **Confirmed; the bracket matches.** Slot 11 returns the building's profile (or a static zero
  profile without a building development simulator), and its first u32 is the purpose: the flag is set for purpose 1
  (Residence). An early exit of the call skips the step.
- ~~**RG3:** query-record costs are 32-bit in the game but 64-bit in the core. [zero-extended in, low 32 bits written
  back]~~ **ANSWERED 2026-10-14** from the exe (school `0x006F92B7`, `0x006F92F6`; hospital `0x006F7F76`–`0x006F7FAC`)
  and written into REH §4.11 (cost fields). **The bracket matches.** The record holds the low dword of the item's
  64-bit cost, so writing back the low 32 bits is what the original does. Zero or sign extension on the way in differ
  only for a negative cost, and shipped costs are positive.
- ~~**RG4:** slots 14/15 with a null group vector: the original dereferences it. [ours returns 0]~~ **ANSWERED
  2026-10-14** from the exe (helper `0x006FA070`, dereference at `0x006FA11C`) and written into REH §4.11 (totals).
  **The bracket stands as a *[choice]*.** The original dereferences `groups` per building, so it crashes only when
  some group list holds a building; with no buildings it returns 0 after resetting the totals. Returning 0 for null
  matches the original wherever the original does not crash.
- ~~**RG5:** lot `GetCenter` goes through the FXAM-guarded raw call (crime CQ3). [as crime]~~ **ANSWERED 2026-10-14**
  from the exe (centre helper `0x006F7E00`; lot slot 15 `0x006BF560`) and written into REH §4.4 (GetCoveredTractList).
  **Confirmed; the bracket matches.** The class gets the lot from the lot manager and calls the same `GetCenter` as
  crime: it writes x, y (the lot's altitude) and z, returns the output's address in EAX and leaves the x87 stack
  empty. Without a lot it uses the occupant's `GetPosition`.
- ~~**RG6:** the shadow cannot reproduce the original's clock-seeded strike rolls in state 0. [those fields are not
  compared]~~ **ANSWERED 2026-10-14** from the exe (generator at `+0xB8`, constructor `0x007005AA`) and written into
  REH §2 (the `cRZRandom` row) and §4.9. **No original behaviour; the implementer's choice stands.** Every draw of the
  class is a double draw, which uses only `mul` (`+0xBC`). A shadow that reads that word from the original object
  before state 0 could reproduce the rolls, if it ever wants to compare them.
- ~~**RG7:** does the original re-read ordinance effects outside state 0? [the shadow re-reads them]~~ **ANSWERED
  2026-10-14** from the exe (`GetOrdinanceEffects` `0x006FE0F0`; its only references are the calls at `0x006FED0B` and
  `0x006FF5CA`) and written into REH §4.3. **No; the bracket differs.** The original reads the ordinance effects only
  in `Init` and in state 0. A shadow that re-reads them elsewhere can see an ordinance change earlier than the
  original does; it should keep the lists from state 0 to state 0.
- ~~**RG8:** slots 21/22 called off the main thread create grids through GZCOM. Allowed? [yes]~~ **ANSWERED
  2026-10-14** from the exe (slot 21 `0x006FCF30`, allocation `0x006FCF36`; slot 22 the same) and written into REH
  §4.8 (the slot 21/22 row). **No original behaviour; the implementer's choice stands.** The original does not use
  GZCOM here: it builds the float grid directly (global heap, inlined constructor). Its callers are UI code on the
  main thread *[inferred]*, so nothing in the original constrains a call from another thread.
- ~~**RG9:** setters off the main thread: deferred, and `EndStrike` off the main thread returns the published flag?
  [yes]~~ **ANSWERED 2026-10-14** from the exe (`EndStrike`, slots 8/9) and written into REH §4.9. **No original
  behaviour; the implementer's choice stands.** The original applies every setter at once on the caller's thread, and
  its callers are on the main thread *[inferred]*. `EndSchoolStrike` / `EndHealthStrike` return AL = the city strike
  flag before the call and post the end message when it was set, so returning the published flag matches.
- ~~**RG10:** department funding with no department? [1.0]~~ **ANSWERED 2026-10-14** from the exe (`0x006F8360` via
  slot 20 `0x006F84B0`) and written into REH §4.2. **The bracket differs: not 1.0.** A null department leaves the
  output untouched (both outputs for the staff department; only b for the vehicle department, after a is written). The
  class's own callers preset 0.0, so the curves see 0. The value 1.0 is used only when the budget item itself is
  missing. The host should report a missing department rather than return 1.0.
- ~~**RG11:** slots 19, 20, 21–25, 28, 47 and 49: always true? [true]~~ **ANSWERED 2026-10-14** from the exe (the 14
  call sites listed in REH §7) and written into REH §7. **Confirmed; the bracket matches.** No caller that reaches
  these slots through the residential global tests EAX or AL afterwards, so returning true is safe. Callers that keep
  the simulator pointer in a member of their own were not all traced; for them it stays *[inferred]*.
- ~~**RG12:** slot 45 returning false: are its outputs left untouched? [untouched]~~ **ANSWERED 2026-10-14** from the
  exe (`GetOccupantCoverage` `0x006FD260`; exit `0x006FD502`) and written into REH §4.11 (slot 45). **Confirmed; the
  bracket matches.** AL = 0 happens only when the building has neither capacity, and that path jumps to the epilogue
  before `dist` or `radius` is written.
- ~~**RG13:** does `SetLocalFundingPercent` set the budget item's local funding? [the shadow assumes so]~~ **ANSWERED
  2026-10-14** from the exe (`ChangeBudgetItemLocalFunding` store `0x00692DBD`; `GetBudgetItemInfo` read `0x006939E6`)
  and written into REH §4.2 with a table. **Yes, but not exactly: the bracket needs a correction.** The item's local
  funding is stored as a percentage, float32(q × 100.0f), and read back as float32(pct × 0.01f), so q = 0.43 reads
  back as 0.42999998 and 0.79999995 as 0.7999999 (0.5 and 1.2 survive). A shadow must compare the read-back value.
  budget.md §2/§7 writes the read as "pct/100"; it is a multiplication by the float 0.01f (reported for the budget
  spec's owner).
- ~~**RG14:** the dirty flag cannot be compared by the shadow. [not compared]~~ **ANSWERED 2026-10-14** from the exe
  (byte `+0xA0`; constructor `0x007005B2`; the setters listed in REH §2) and written into REH §2 (the dirty-flag row).
  **No original behaviour; the implementer's choice stands.** No interface slot returns the flag and nothing in the
  class reads it; it shows only in the save, as `dirty ∨ state ≠ 10`.
- ~~**RG15:** a building's identity is its `AsOccupant()` pointer, used for the lists, the strike message's void1 and
  IGZUnknown, and record references. Right? [yes]~~ **ANSWERED 2026-10-14** from the exe (`DoBuildingChanged` list
  inserts and removals; `BeginStrike` `0x006F84C0`, void1 `+0x30` and IGZUnknown `+0x84`; `EndStrike` `0x006F8570`)
  and written into REH §4.1 (building identity) and §5. **Confirmed; the bracket matches.** The lists hold
  `AsOccupant()` pointers and removal compares `AsOccupant()`. The strike-begin message carries that list pointer as
  both void1 and IGZUnknown, with data2 = 1; the strike-end message sets only data2 = 0 and data3 = the length.
- ~~**RG16:** per-building properties: scalars of their type (bool, u32, f32), or arrays? [scalars]~~ **ANSWERED
  2026-10-14** from the exe (helpers `0x005FE3C0`, `0x005FE4F0`, `0x005FE210`; `SetValUint32` `0x0091587D`) and
  written into REH §2 (per-building state). **Scalars; the bracket matches.** Each helper creates the property if it
  is missing and calls the variant's scalar setter, which replaces any other content (an array too) with one value of
  the property's type.
- ~~**RG17:** slots 47, 49 and 61 append to the caller's list with the game allocator. Is that so for slot 61?
  [assumed]~~ **ANSWERED 2026-10-14** from the exe (slot 61 `0x006FAEC0`: allocations at `0x006FAF30` and
  `0x006FAFBA`; slots 47/49 through `GetBuildings` `0x006FACE0`, allocation at `0x006FADA3`) and written into
  residential.md §10.1 (slot 61 row). **The bracket matches: yes.** All three take each 12-byte node from the framework
  allocator service (allocator A, the cached pointer at `0x00B207E4`, slot 3), link it at the tail of the caller's
  list and `AddRef` the occupant. This is the allocator of the SDK's `SC4List`, so the caller frees the nodes the
  usual way.
- ~~**RG18:** the simulator taken at Init step 4 is released after `RemoveAgent`; REH §4.2 has no matching release.
  [released]~~ **ANSWERED 2026-10-14** from the exe (`Init` `0x006FEC0E`; `Shutdown` `0x006F980D`) and written into
  residential.md §4.1 step 4 and §4.2. **The original holds no reference: the simulator is borrowed.** `Init` reads
  the simulator global without `AddRef` and keeps no member; `Shutdown` asks the city for the simulator again for
  `RemoveAgent` and releases nothing. A glue that takes a reference at Init and releases it after `RemoveAgent` is
  equivalent, since the city outlives both calls.
- ~~**RG19:** what goes in a strike message's data1? [0]~~ **ANSWERED 2026-10-14** from the exe (message constructor
  `0x009134D6`; `BeginStrike` `0x006F84C0`, `EndStrike` `0x006F8570`) and written into REH §5. **0; the bracket
  matches.** The constructor zeroes every field and neither strike call sets data1.
- ~~**RG20:** `GetBuildingMapKey` returns 32 bits; only the low 8 are used. Right? [yes]~~ **ANSWERED 2026-10-14**
  from the exe (calls `0x006FF863`, `0x006FF9E8`, `0x006FFB56`; seed helper `0x005E4FA0`, byte loads at `0x005E5049`,
  `0x005E5078`, `0x005E50CC`) and written into residential.md §5.3 step 1. **Confirmed; the bracket matches.** Slot 42
  sets only AL; the class passes EAX on, and the seed helper reads and compares the key as a byte.
- ~~**RG21:** the strike clock seed is `time() + 2177452800` (the cRZTime epoch), as in crime. [yes]~~ **ANSWERED
  2026-10-14** from the exe (constructor `0x007005AA` → `cRZRandom(−1)` `0x00913A5C` → `Seed` `0x00913667`; `time`
  `0x009EFB59`, `clock` `0x009F361F`) and written into REH §2 (the `cRZRandom` row) and foundations §6.1 (`Seed`).
  **The bracket differs, but it cannot be observed.** The seed is `time(NULL) XOR ((clock() << 16) | clock())`, not
  the `cRZTime` epoch value. No save or output exposes it. *[choice]* FreeCity4 uses `derive(0x4990C013)` from the
  session seed, as foundations §6.3 prescribes for member generators.
- ~~**RG22 — `docs/abi`:** residential slots 37/38 (`GetCellEQByWealth`/`GetCellHQByWealth`) are `via-call`, low
  confidence, in `interfaces.tsv`, while residential.md §10.1 says ST0 (they end by calling the float grid's
  `GetTractValue`, x87 high). Should the rows say x87? [declared float, on the spec's word]~~ **ANSWERED 2026-10-14**
  from the exe (`0x006F7A60`, `0x006F7AB0`; float grid `GetTractValue` `0x0045F210`) and noted in REH §7 (slot 37).
  **Yes: the rows should say x87, high confidence.** Each method divides x and z by the EQ grid's tract size, indexes
  `EQ[w − 1]` (HQ for slot 38) and returns through the float grid's `GetTractValue`, whose `fld` leaves the result in
  ST0; the method's `ret 0xC` does not touch it. The corrected `docs/abi/interfaces.tsv` rows are in the lane's report
  (the spec does not edit `docs/abi`).
- ~~**RG23 — first in-game residential shadow (run 86, coordinator):** every cycle, 12 of 15 daily states match
  exactly (all 25 grids, the saved fields, the per-building properties, funding and the scalars: population, EQ,
  HQ, life expectancy). States 1, 2 and 3 (the proximity grids R$, R$$, R$$$) differ in about 16,000 of 16,384
  cells: at the corner the original's grid reads 1, 0, 0, 0 along t(0,0..3) and ours 104, 103, 102, 101 (R$; R$$
  106.., R$$$ 116..). Ours looks like a distance ramp counting down from the corner; the original's is 0 or 1
  there. What do states 1–3 write into the proximity grid (a distance, a count within a radius, a flag), from
  which seed (the building map's keys `4p + w + 1`?), and how far does one state's call get? The shadow re-seeds
  this temporary grid from the building map as it stands. [ours as residential.md reads today]~~ **ANSWERED
  2026-10-14** from the exe (seed `0x005E4FA0`: row start loaded at `0x005E503B`, stores `0x005E505B`, `0x005E508A`,
  `0x005E50FB`; states 13/14/15 at `0x006FF854`–`0x006FF87C` and on; pass loop `0x006FF8D0`–`0x006FF923`) and
  written into residential.md §5.3 (worked example), §12 item 10 and §15 tests 1 and 3. **The bracket does not
  match.** The seed uses the keys `4p + w + 1` (6, 7, 8) and the passes are as written. But the seed stores every
  value in element 0 of row x, the cell (x, 0), and in the shipped equal-size case it also reads `map[x][0]`. So
  only the z = 0 edge is seeded: 0 where `map[x][0]` is the key, 255 elsewhere. Every other cell keeps the
  temporary grid's 0 (fresh grid, or the previous wealth's result, which is 0 off the edge). The passes then give
  (x, 0) = 1 (or 0 under a key) and 0 everywhere else, which is the corner's 1, 0, 0, 0. States 1 and 2 converge
  in two passes and state 3 in one, so one call normally completes the state; the time check sits between passes.
  FreeCity4 must reproduce this *[compat]*, since the growth developers' 4 × 4 floor mean of these maps is then 0
  for every tract. The commercial CO proximity maps use the same seed (simgrid §7.5).

## From applying the pathfinder, zones, landfill and neighbours answers (filed by the coordinator from the `apply4b` lane's report, 2026-10-14)

About PF1–PF15, ZQ4/ZQ7–ZQ10, LF1–LF5 and NB1–NB17 as applied in the core. The current answer is in brackets.

- ~~**TN26 — traffic-network §2.7:** the sweep and the edge pass when Z < X: where do they stop? [at the array end]~~
  **ANSWERED 2026-10-14** from the exe (sweep 0x007138B0 bound at 0x0071392D, edge pass 0x007151E0/0x0071531F, both
  `+0x194` = X − 1; arrays sized in Init 0x0071E944–0x0071E989) and written into traffic-network §2.7.3 ("When Z ≠ X")
  and §2.7.4 step 6.
  - Both loops use X − 1 for all four arrays, while the z-indexed arrays have Z records.
    - With Z > X, records X−1 … Z−1 are skipped.
    - With Z < X, the original reads past the end of those arrays. The edge pass can then write: it may clear a "pool"
      word in heap memory and set destination bits outside the map's border.
  - Unreachable in the game: every region city is square.
  - The bracket is a sound *[choice]*: stop each loop at `min(X − 1, its length)`, which is the original for every
    square city.
- ~~**TN27 — §2.7:** `FindNeighborConnection` and the pool-ID lookup for travel types above 8. [no types match]~~
  **ANSWERED 2026-10-14** from the exe (unchecked `net(t)` table u32[9] at 0x00AB31A8, read at 0x007104D7 and
  0x007114ED; callers 0x006D79D7, 0x00713703, 0x007147B8/0x007147C9, 0x00716751) and written into traffic-network
  §2.7.1 ("Travel types outside 0…8").
  - The original indexes both tables without a check, so t ≥ 9 reads the read-only data that follows: t = 9…26 give
    net 0 (the pedestrian mask `0x704`), t = 27…30 give nets 2, 1, 3, 4, t = 31 gives mask 0, and larger t read
    further unrelated words.
  - Unreachable: callers pass −1, 1 or a trip's travel type (0…8).
  - The bracket differs from the literal original for t = 9…30, but as a *[choice]* for an unreachable input it
    stands: `wanted := 0` (t = −1 keeps all bits in the pool lookup). A Ferry record still gets a pool through step 2.
- ~~**TN28 — §2.7:** `GetTrafficNeighborConnection` for a corner cell or a cell inside the city. [an empty record]~~
  **ANSWERED 2026-10-14** from the exe (`0x0070F970`; callers 0x006D87ED, 0x007104F3, 0x007153F5, 0x0071C8BC,
  0x0071C934; inline copy 0x00711500–0x0071153E) and written into traffic-network §2.7.1 (the table after
  GetTrafficNeighborConnection).
  - The original always returns a pointer. A corner cell gets index −1 or Z of the side-0 or side-2 array: heap
    memory just outside it, which Add would write. An in-city cell (x, z) gets side-3 record x, the real record of (x,
    Z), which it aliases.
  - Every caller passes the off-map cell of a neighbour connection or of an off-map trip end, after an in-city test
    where there is one, so neither case is reached.
  - The bracket stands as a *[choice]* that is never observable: an empty record, with writes to it discarded.
- ~~**ZQ11 — zones test 33:** a second draft `Place` on the same cell. [the mask bytes are merged (OR); inferred]~~
  **ANSWERED 2026-10-14** from the exe (`PlaceNetworkOccupants` 0x00629210, cache clear only in real mode at
  0x0062932F; record update 0x00633250; edge merge 0x00638160 → 0x00620F00; draft byte 0x0062591E) and written into
  zones §8.8 (the `OccupantExists` item) and test 33's stub.
  - The draft records live in the tool's cell cache, which only `Reset` and a real-mode Place empty, so a second draft
    Place finds the first one's record and updates it: flags := old OR new; the new edge directions are merged into
    the old ones. The draft byte stays set.
  - The merge keeps every connected edge and fills unconnected ones, so for the zone manager's "byte ≠ 0" contract the
    mask is the OR. The bracket matches, now from the exe. The byte values are not an OR (direction codes, minor
    types), so a replacement promises only ≠ 0.
- ~~**ZQ12 — zones test 33:** a draft `Place` over a cell that already has a network. [no draft, as a real `Place`]~~
  **ANSWERED 2026-10-14** from the exe (record from the existing network with the draft byte clear 0x00633030; flag OR
  0x00633250; draft byte on every re-solved cell 0x0062591E/0x00625BFB; draft-mode end lengthening
  0x006296D0–0x006297A7) and written into zones §8.8 (the `OccupantExists` item) and test 33's stub.
  - In the original the cell **does** become a draft once the solve succeeds. Its flags are the existing network's OR
    the tool's (8), and its mask is the existing edges of the tool's type merged with the new ones. An existing
    network cell next to the draft that the solver includes also gets the draft byte, with its own flags and the edge
    towards the draft.
  - A segment draft is also lengthened into a neighbouring earlier draft (and, in both modes, into a real network of
    its own type).
  - The bracket differs. The test-33 stub may keep it, because no checked result depends on it: in (e) the original
    also drafts (12, 9) with an edge towards (12, 10), which only `ExistsConnectionOfType` could see, and no call to
    it comes after that Place. A replacement network tool must follow §8.8.
- ~~**LF6 — LF3:** should the core add the message-server release callback (an ABI bump) to match the original's
  shutdown order? [no: one `release_services` call after the zones shut down; LF3 says the difference cannot be
  observed]~~ **ANSWERED 2026-10-14** from the exe (message server global 0x00B43CCC written only in `SL::OnAppInit1`
  at 0x006024BB) and written into landfill §1.2.
  - No original behaviour; the implementer's choice stands. The message server is the framework's application-lifetime
    service, so the developer's release of it is never the last one and its timing cannot be observed. No ABI bump is
    needed.
  - What could be observed is the unsubscription, which the original does before the zones shut down. One
    `release_services` call afterwards is equivalent while no message reaches the developer in between, which holds
    (LF3).
- ~~**NB18 — NB12:** the shim supplies the game's global `cISC4City` pointer through the new host method
  `current_city_object`. Is that the object `UpdateCommodityStats` compares against? [yes; the test hosts use a token
  distinct from every regional city ID]~~ **ANSWERED 2026-10-14** from the exe (identity test at 0x006D2A40 against
  the global 0x00B43CEC; the global is written only by the city constructor through 0x00601C10 at 0x0045A34A, and
  cleared by `SL::OnCityShutdown` at 0x0060210E) and written into neighbors §8.2.
  - Yes. The global holds the city object's own address, its `cISC4City` interface, from construction to city
    shutdown. `current_city_object` must return exactly that pointer: the test compares addresses, so another
    interface pointer or a wrapper would make the call do nothing.
  - The bracket matches. The test hosts' token is glue: no original behaviour; the implementer's choice stands.
- Not applied: PF6 beyond the record level (pools, capacities and the roll stay traffic-simulator host methods, as
  the answer says); NB12/NB17 glue (there is no neighbours glue yet).

## From applying the protection, demand and pollution answers (filed by the coordinator from the `apply3` lane's report, 2026-10-14)

About PR1–PR13, DQ3–DQ8 and PG4–PG13 as applied. The current answer is in brackets.

- ~~**PR14 — protection §5:** the dispatch active/inactive handler: which occupant does `GetDispatchEffect` read?
  [`info.station`, as UpdateUnadjusted; inferred]~~ **ANSWERED 2026-10-14** from the exe (handler `0x006F647F` →
  `DoDispatchActivated` `0x006F61F0`; `GetDispatchEffect` fire `0x006AD720`, police `0x006E0CB0`;
  `AdjustProtectionEffect` `0x006F56A0`) and written into protection.md §5 (the active / inactive handler) and §7.2.
  **`info.station` (record `+0x20`); the bracket matches, now confirmed.** Two corrections to protection.md came out
  of it:
  - **§7.2 had one caller; there are two.** `UpdateUnadjusted` and this handler both pass the record's `+0x20`.
  - **§7.2 said "no efficiency scaling"; the effect is scaled.** `GetDispatchEffect` fails (AL = 0) only for an absent
    or zero centre; otherwise it reads outer and radius (an absent one keeps the caller's value) and scales all three
    by `GetStationEfficiency(station)`.
  - **Handler quirk.** The handler ignores `GetDispatchEffect`'s AL and passes an uninitialised stack radius when it
    fails. *[choice]* FreeCity4 skips the update then.
- ~~**PG15 — PG11, slot 29:** with a negative supply and n > 0, is `0x7FFFFFFF − s` computed in int32 (wrapping) or
  exactly? [exactly, in 64 bits, so the change equals n]~~ **ANSWERED 2026-10-14** from the exe (slot 29 `0x006E32E0`,
  test at `0x006E32E9`–`0x006E32FB`) and written into pollution.md §8.1 with a worked table. **Exactly; the bracket
  matches.** The original adds first in int32 and takes `0x7FFFFFFF − s` only when that sum wrapped, which needs s >
  0, where the subtraction is exact. With a negative s and n > 0 nothing wraps and the delta is n. A damaged save with
  a negative s and n < 0 can wrap the other way (noted as a quirk).
- ~~**PR15 — PR12:** how long does the protection simulator hold the budget simulator and the zone manager? [taken in
  base init only if not already held, released at the end of Shutdown; the read-only shadow host asks the city each
  time]~~ **ANSWERED 2026-10-14** from the exe (`ProtectionInit` `0x006F6BC7`–`0x006F6C0B`; `ProtectionShutdown`
  `0x006F56F0`) and written into protection.md §4.2 (base shutdown, now traced in full). **Confirmed; the bracket
  matches.** Both are taken in base init only when not already held and released in the last step of the base shutdown
  (zone manager, then budget simulator, then city), which comes before the class releases its flammability or crime
  simulator and departments. Asking the city each time in a read-only shadow host cannot be told apart. The trace also
  fixed the base shutdown's order: dispatch unregister, station list, the nine unsubscriptions (in an order slightly
  different from the subscriptions), `RemoveAgent`, the grids, then the held services.
- ~~**PR16 — PR5:** how does a shadow recognise the original class before reading +0x84, and are unadjusted mismatches
  judged like the adjusted grid's? [by the primary vtable against the §15.1/§15.2 addresses (compat table, identified
  executable only; otherwise not read); logged, not judged]~~ **ANSWERED 2026-10-14** from the exe (constructors fire
  `0x006ADA67`, police `0x006E1F9D`; destructors `0x006ADB27`, `0x006E16ED`; base constructor `0x006F6986`;
  `ProtectionShutdown` `0x006F580D`; city getters `0x006DC690` / `0x004597C0`) and written into protection.md §2.
  **Recognition: the bracket matches. Judging: no original behaviour; the implementer's choice stands.**
  - **Recognition.** The city's getters return the object start, whose first dword is the primary vtable (`0x00AB1160`
    fire, `0x00AB2490` police). Only the class's own constructor and destructor write it, so a replacement or wrapper
    fails the test. The `+0x84` member is null before `Init` and again after `Shutdown`; the grid's `GetInstanceID()`
    (`0x89D5B91A` / `0x49D5BBCE`) is an optional extra check.
  - **Judging.** The original never compares the grids. The unadjusted grid has no copy lag, but its dispatch cones
    follow the automata, so tracts under an active dispatch can differ for reasons outside the simulator; logging only
    stays a sound choice.
- ~~**DQ13 — DQ5:** does the default `building_purpose` go through `building_profile`, and may the host's visitor be
  NULL? [yes to both]~~ **ANSWERED 2026-10-14** from the exe (`PerformCensus` `0x006A5A00`: BDS slot 39 at
  `0x006A5A53`, only `purpose` read at `0x006A5A5D`) and written into demand.md §6.3 (host shape). **Yes to both; the
  bracket matches, with one condition.** The original reads only the purpose and walks nothing, so a null visitor is
  the exact match. Routing the default `building_purpose` through `building_profile` is faithful as long as an unknown
  type yields purpose 0, as the original's inserted zero profile does.

## From the budget shadow (coordinator, run 86)

- ~~**BQ3 — budget.md, the monthly settlement total:** in run 86 (`regress rbc7f`), 30 of 31 months match. In
  2028-08 every one of the twelve tax lines is equal (1762, 767, 123, 98, 101, 30, 83, 34, 0, 1547, 290, 0), but the
  total income is 4839 in the original and 4840 in ours (net 792/793). The lines sum to 4835, so the total is not
  their integer sum: it comes from unrounded values. Which values does the original add for the total (float
  per-lot sums, or per-line floats before display rounding), in which order and width, and how is the total
  rounded? First seen in this run; runs 80–83 had none in 93–95 months. [ours as budget.md reads today]~~
  **ANSWERED 2026-10-14** from the exe (`ComputeTax` `0x00691E20`: sums at `0x0069205C`–`0x00692070`, truncations at
  `0x006920B4`–`0x006920EB`) and written into budget.md §5.2 ("The total is not the sum of the lines", worked
  example). **The bracket matches what budget.md already said, now stated exactly.** The total is a separate running
  sum over the **per-lot** tax products, not over the lines or the group sums. Each lot's tax is added to `total`
  first, then to its group's sum. Both are doubles in memory, but each `FADD` rounds to 24 bits, so they equal
  `float32` running sums. The order is the lot manager's master list. The total is truncated toward zero
  (`_ftol`, 64-bit), and so is each line. The lines drop their fractions and the total keeps them, hence 4835
  against 4839. A one-simoleon difference with all twelve lines equal is the lot-order effect: the worked example's
  four lots give 6567 in list order and 6566 reversed, with the same lines. Matching it exactly needs both the
  master-list order (creation order for lots made since the load) and a total formed lot by lot, not from the
  group sums. Which of the two ours lacks cannot be seen from the exe side.

## From the save-reference audit (filed by the coordinator from the `lvrefs` lane's report, 2026-10-14)

Run 88 (save, then reload, fourteen replacements): after an in-session save, land value's release of its three grids at
Shutdown was the last reference; on a loaded city without a save, the game still holds a reference at Shutdown. The
lane now treats every reference a glue holds from `Read` as its own (`FC4_RELEASE_CREATED`), which is correct either
way.

- ~~**SV1 — foundations §2.7:** which references does the game's load (the persistent resource manager's object table,
  `ReadGZSerializable`) keep on the objects it creates or resolves, and when does it drop them: at the end of the
  load, at the next save, or at city Shutdown? [our reading: the load keeps one until the next save; marked in the
  code as a reading, not a finding]~~ **ANSWERED 2026-10-14** from the exe (serializer resolver 0x005D2DD0, reset
  0x005D3250, segment Close 0x005D20C0, SaveCity 0x00455E20 at 0x0045641B/0x00456449/0x0045662F, PartialCloseCity
  0x00454A60 at 0x00454AF3) and written into foundations §2.7 "References the load keeps".
  - The table is the object table of the city segment's COM serializer. It keeps exactly one reference on each object
    the load **creates**, none on an object it only resolves; the loader's own local is released after `Read`, and the
    typed helper nets +1 for the destination.
  - It drops them all when the segment that loaded the city is closed: at the **next save** (the save switches to a
    fresh read-only segment with an empty table), or, if no save came first, at **city close after the city's
    `Shutdown`**, or on a failed load. Not at the end of the load.
  - The bracket matches, with one addition: in a loaded city that is never saved, the table still holds its reference
    during every `Shutdown`, so a subsystem's release there is not the last one. Treating every reference from `Read`
    as the glue's own stays correct.
- ~~**SV2 — ordinance §9.1:** does anything other than the ordinance simulator hold a foreign (plugin) ordinance read
  from a save? [assumed no; released as ours]~~ **ANSWERED 2026-10-14** from the exe (the only `cISC4Ordinance` QIs
  are the ordinance's own 0x0080C924 and the simulator's `Read` 0x0080E96B; the `GetOrdinanceByID`/`ByName` call sites
  listed in §9.1) and written into ordinance §9.1 "Who else holds a loaded ordinance".
  - Yes, one other holder: the city segment's serializer object table (foundations §2.7, SV1) keeps one reference on
    each ordinance the load created, until the next save or, in a city never saved, until city close after `Shutdown`
    and after the city is released.
  - No other exe code holds one: the messages carry the simulator, and every `GetOrdinanceByID`/`ByName` caller (LUA,
    residential, budget window) uses the pointer at once without AddRef or storing it.
  - The bracket "assumed no" differs, but "released as ours" is correct: the simulator's reference is its own. In an
    unsaved loaded city the table's release destroys the ordinance after the simulator, which nothing can observe.

## From the neighbours glue and shadow (filed by the coordinator from the `neighglue` lane's report, 2026-10-14)

About `docs/specs/neighbors.md` and the neighbours glue (`cFC4NeighborsSimulator`, `FC4NeighboursShadow`). The current
answer is in brackets.

- ~~**NG1:** Property holder semantics for our own objects (connections, deals). [sorted by property id, values copied;
  `GetPropertyList` returns false; data as a Uint8 array]~~ **ANSWERED 2026-10-14** (coordinator, run r50e2): copies are wrong. The
  holder keeps the caller's object (AddRef) and returns it borrowed; callers write its variant in place
  (foundations §10.2, "Ownership"). With copies, the traffic simulator's trip-capacity store used a freed
  property and the game crashed.
- ~~**NG2:** Object-valued properties on our connection objects. [dropped from the core sync]~~
  **ANSWERED 2026-10-14** from the exe (holder slot 11 `0x005FF950`; the connection properties seen in samples and
  code) and written into neighbors §15.4. The original holder keeps an object-valued property (a reference), returns
  it through slots 6 and 9, writes it as a reference on save and drops it on load. No shipped code puts one on a
  connection. The bracket is fine for the core sync and the save, as long as the game-facing holder keeps the object.
- ~~**NG3:** Does the region reach our connection objects non-virtually (by field offset)? [unknown; the risk is noted]~~
  **ANSWERED 2026-10-14** from the exe (image-wide scan for direct calls and pointers to the class's methods; vtable
  constants only in `0x006D4680`/`0x006D4BB0`; field-read scan of the 66 connection users and the 55 readers of
  `0x00B43D8C`; current-city record filled at `0x004AD34E`) and written into neighbors §15.3. **No; there is no such
  site.** Every access is a virtual call through the connection, traffic-source, holder or serializable interface. The
  only original code that touches the concrete class constructs its own objects (`0x006D489B`, `0x004AC994`). The
  current city's regional record holds our objects after `UpdateCityCache` and writes them through
  `QueryInterface(0xE4FDA3D4)` → `Write`. The one layout contract is `GetTripDataList`'s STL list (already met).
- ~~**NG4:** `AddDeal` with a foreign (original) deal object. [copied into a deal of ours]~~
  **ANSWERED 2026-10-14** from the exe (`AddDeal` `0x006D2870`; no caller through slot 26) and written into neighbors
  §15.4. The original keeps the caller's object (AddRef), clears its Silent flag and simulates that object. The bracket
  differs (pointer identity, later changes lost), but no shipped code calls `AddDeal`, so only mods can see it.
- ~~**NG5:** The lifetime of the deal wrapper objects. [created lazily, parked at zero references]~~
  **ANSWERED 2026-10-14** from the exe (`FindDeals` `0x006D0760`, `AddDeal`, the status-report subscribers) and written
  into neighbors §15.4. An original deal lives until its last Release. The holders are the deal list, the regional
  cities' local lists, queued status reports, `FindDeals`/`ProposeDeals` lists and the utility advisor's advice. Lazy
  creation and parking are invisible, so the bracket stands, provided a removed deal keeps answering while referenced.
- ~~**NG6:** Getters called from other threads. [the lock is tried for 20 ms, then the mirror or a neutral value is
  returned and counted as a fallback]~~
  **ANSWERED 2026-10-14**: no original behaviour (the original simulator has no lock); the implementer's choice stands.
  Noted in neighbors §15.4.
- ~~**NG7:** The shadow's state it cannot read from the original. [ready = 1, uiDisabled = 0]~~
  **ANSWERED 2026-10-14** from the exe (field writes in the simulator class; budget window `0x0077E6C5`,
  `0x0077E735`, `0x0077A216`) and written into neighbors §15.4. The fields are bytes in the original object: ready
  `+0x0C`, initialized `+0x0D`, dealsDirty `+0x3C`, statsDirty `+0x3D`, uiDisabled `+0x3E`, busy `+0x3F`. The bracket
  is wrong before ConnectionsReady (ready 0) and while the budget window's neighbour tab is open (uiDisabled 1). Read
  the bytes.
- ~~**NG8:** The volume-dirty flag. [true once after creation]~~
  **ANSWERED 2026-10-14** from the exe (`0x006D4C10`, flag at connection `+0x40`) and written into neighbors §15.4. Set
  only by the constructor; cleared at the start of the first `UpdateTrafficVolume`, even when no other side is found.
  The bracket matches.
- ~~**NG9:** `SetType` and similar setters on foreign connections. [ignored]~~
  **ANSWERED 2026-10-14** from the exe (connection vtable `0x00AB2034`; the NG3 field scan; writes in `CreateConnection`,
  `Read` and `Repair`) and written into neighbors §15.4. Type and cell never change after creation, so ignoring is
  right. `Repair`, `SetVisitedThisCycle`, `SetIsActive` and holder writes must go through the interface.
- ~~**NG10:** The return of `CreateConnection`. [our pointer, borrowed; null if the connection already exists]~~
  **ANSWERED 2026-10-14** from the exe (`0x006D4860`, NB17) and noted in neighbors §15.4. The original's value is
  garbage and every shipped caller ignores it, so the bracket stands.
- ~~**NG11:** Slots 32/33 (64-bit values). [the low 32 bits]~~
  **ANSWERED 2026-10-14** from the exe (`0x006CF860`, `0x006CF8D0` and their eleven call sites) and written into
  neighbors §15.4. Both return an int64 in EDX:EAX, and every caller (finance advisor, budget simulator, budget
  windows) reads EDX. The bracket is wrong: return the full 64 bits. `docs/abi` rows 32/33 need `edx:eax`.
- ~~**NG12:** The return channel of the regional city getters 50–64. [double (x87)]~~
  **ANSWERED 2026-10-14** from the exe (regional-city vtable `0x00A8FB20`, slots 50–64) and noted in neighbors §15.4.
  `double` on x87; the bracket matches and `docs/abi` is already right.
- ~~**NG13:** The return of deal `Copy`. [void]~~
  **ANSWERED 2026-10-14** from the exe (`Copy` `0x006CE2E0`, its only caller `Clone` `0x006CF3E0`) and noted in
  neighbors §15.4. Nothing meaningful is returned and nothing reads it; the bracket stands.
- ~~**NG14:** `RequestCreateConnection`: the yes/no dialog. [confirmed without the dialog, logged once]~~
  **ANSWERED 2026-10-14** from the exe (`0x006D15E0` → `GenericConfirmationDialog` `0x0078DFF0`) and written into
  neighbors §15.4. The original asks with a modal yes/no dialog and returns the answer; with popups disabled the dialog
  returns false, so the request is refused. The bracket differs; recommend the dialog, and "no" when popups are off.
- ~~**NG15:** `GetCity` with out-of-range cells. [passed on as uint32]~~
  **ANSWERED 2026-10-14** from the exe (`cSC4Region::GetCity` `0x004A4930`) and noted in neighbors §15.4. A hash lookup
  on the 32-bit pair; the bit pattern is what counts, so passing it as uint32 is fine and an out-of-range cell gives
  null.
- ~~**NG16:** Creating the connection marker. [`CreateProp` + `AddCityProp`, then our reference released]~~
  **ANSWERED 2026-10-14** from the exe (`0x006D4A53`–`0x006D4B15`; prop manager slots 15 and 19) and written into
  neighbors §15.4. `CreateProp`, position and orientation, the bool properties, `AddCityProp`, then `Release`. The
  bracket matches; a failed `CreateProp` just skips the marker.
- ~~**NG17:** The occupant cell of a connection. [the network's `GetOccupiedCell`, else the pole's `GetCell`]~~
  **ANSWERED 2026-10-14** from the exe (`0x006CEF62` `GetPosition`, `0x006CEF7E` `PositionToCell`) and written into
  neighbors §6.5 and §15.4. The original converts the occupant's position to a cell for networks and poles alike. The
  bracket agrees whenever the occupant lies in its own cell; recommend the original's rule.
- ~~**NG18:** A deal holder not reachable by QueryInterface. [left open]~~
  **ANSWERED 2026-10-14** from the exe (deal QI `0x006CE260`; holder vtable `0x00AB1D58` only in the constructor and
  destructor) and written into neighbors §15.4. The original deal's holder is not reachable either; it only travels with
  the record. Nothing to expose: round-trip the record's properties.
- ~~**NG19:** No city on the other side of a connection. [targets 0]~~
  **ANSWERED 2026-10-14** from the exe (`0x006D4C3B` lookup, `0x006D5030` list clear) and written into neighbors §15.4
  and traffic-simulator §4.1.4. The original clears the whole trip list. "Targets 0" agrees except for trips of a type
  above 2, which the sync never creates; recommend clearing the list.
- ~~**NG20:** The `GetSourceCells` rectangle. [(x, z, x, z)]~~
  **ANSWERED 2026-10-14** from the exe (`0x006D4820`) and noted in neighbors §15.4. `(x, z, x, z)` with the connection
  cell (outside the city); the bracket matches.
- ~~**NG21:** The return of `UpdateTrafficVolume`. [true]~~
  **ANSWERED 2026-10-14** from the exe (`0x006D4C10`; caller `0x0071E28E`) and written into neighbors §15.4. `AL` is 0,
  3 or undefined, and the only caller on a connection ignores it. "True" is safe; the bracket stands. `docs/abi`: slot
  4 of `cISC4TrafficSource` on connections is not a clean bool.
- ~~**NG22:** `cISCProperty` Write/Read. [through the segment stream]~~
  **ANSWERED 2026-10-14** from the exe (`cSCProperty` `0x005FD870`/`0x005FD8B0`; holder writer `0x005FD990`) and
  written into neighbors §15.4. Write and Read put foundations §10.1 on the given stream and return "not in error". The
  original holder calls each stored property's own `Write`, so ours must write those exact bytes. The bracket matches.
- ~~**NG23:** The size query of `GetPropertyData`. [left open]~~
  **ANSWERED 2026-10-14** from the exe (holder slot 5 `0x005FD290`, variant slot 57 `0x00915714`) and written into
  neighbors §15.4. No size query: only a raw-byte (`0x8000`) value is copied, `length := min(length, count)`, else false.
  A null buffer crashes the original; recommend false with `length` unchanged.
- ~~**NG24:** Regional cities held across calls. [AddRef until the outermost exit]~~
  **ANSWERED 2026-10-14**: no original behaviour (the original holds regional cities only within one call); the
  implementer's choice stands. Noted in neighbors §15.4.
- ~~**NG25:** The marker query type. [`0xA823821E`]~~
  **ANSWERED 2026-10-14** from the exe (prop manager slot 18 `0x004A1DB0`, filter `0x004A2210`; calls at `0x006D189A`,
  `0x006D18D6` and `0x006D0D65`) and written into neighbors §15.4. The original asks for city props (type `0xA823821E` and usage 1) in the
  cell, then tests `0xCAA46595`. The type matches; add the usage test or use `GetCityProps`.
- ~~**NG26:** Traffic source `QueryInterface`. [forwarded to the connection]~~
  **ANSWERED 2026-10-14** from the exe (thunk `0x006D47F0` → `0x006CE000`) and written into neighbors §15.4. Forwarded
  to the connection, which answers only `1`, `0x0773FEF2` and `0xE4FDA3D4`. The bracket matches.
- ~~**NG27:** `GetPropertyU32` on array properties. [the first element]~~
  **ANSWERED 2026-10-14** from the exe (holder slot 8 `0x005FD230`, variant `0x009176C8`, conversion `0x00917350`
  via text `0x00916EAA`) and written into neighbors §15.4. Non-scalars convert through text, and an array's text is its
  first element ("0" if empty). The bracket matches for unsigned integer arrays; other element types were not traced.
- ~~**NG28:** The shadow's statistics tolerance. [relative 1e-6]~~
  **ANSWERED 2026-10-14**: no original behaviour; the implementer's choice stands. Noted in neighbors §15.4.
- ~~**NG29:** Network flag bits used as type numbers. [left open]~~
  **ANSWERED 2026-10-14** from the exe (network occupant slot 22 `0x00619520`, table `0x00AA7CE8`, slot 8 `0x00604D10`)
  and written into neighbors §15.4. §6.5 passes type numbers; the occupant maps each to `1 << type` and tests it
  against its flags.
- ~~**NG30:** `ReloadCity` through the SDK `cISC4Region`, which is not in `docs/abi`. [gated, with an identity check]~~
  **ANSWERED 2026-10-14** from the exe (region vtable `0x00A8FA40`; `Init` `0x006D3B0C`–`0x006D3BEC`; `ReloadCity`
  `0x004A5030`) and written into neighbors §15.4. The SDK interface matches slot for slot. The original reloads every
  adjacent city with no gate and no identity check (the check always passes, so it is harmless; a closed gate is not).
  `ReloadCity`'s result is garbage, and after a failed read the city is gone and the holder pointer dangles. `docs/abi`
  rows for `cISC4Region` are in the re-answers8 report.
- ~~**NG31:** Status reports from deals with no host object. [left open]~~
  **ANSWERED 2026-10-14** from the exe (subscribers of `0xEA418D8F`: power, plumbing, utility advisor; status-8 posters)
  and written into neighbors §15.4. Statuses 0–7 always carry the deal; power and plumbing read it on 0, the advisor on
  3–6. Recommend creating the wrapper when posting; only status 8 is posted without a deal.
- ~~**NG32:** The shadow's connection-state comparison. [the original's after-state against our before-state]~~
  **ANSWERED 2026-10-14**: no original behaviour; the implementer's choice stands. Noted in neighbors §15.4, with the
  callers that change connection state outside the simulator.
- ~~**NG33:** A message from another thread that cannot get the lock. [dropped, returns true]~~
  **ANSWERED 2026-10-14**: no original behaviour (no lock; every message is handled and `DoMessage` returns true); the
  implementer's choice stands. Noted in neighbors §15.4.
- ~~**NG34:** A null connection reference in a save. [skipped]~~
  **ANSWERED 2026-10-14** from the exe (`0x006CFEC5`, NB7) and noted in neighbors §15.4. The original crashes; skipping
  is the recorded choice of §13, and the bracket matches it.

## From applying the aura, education/health and flora answers (filed by the coordinator from the `apply5` lane's report, 2026-10-14)

AU1–AU15, EH1–EH14 and FQ1–FQ15 are applied (the aura shadow follows the original's state field; the flora obliteration
record goes through record streams, switch `flora_obliteration`). Not applied: a call-order test for EH1, and the aura
shadow's riot roll (wall-clock RNG). The current answer is in brackets.

- ~~**AQ1:** REH: the totals record's leftover fields +0x14..+0x24. [they start at 0]~~
  **ANSWERED 2026-10-14** from the exe (totals helper `0x006FA070`, record set up once at `0x006FA0D5`–`0x006FA0E9`) and
  written into residential-education-health §4.11. The original does not start them at 0: only +0x10, +0x28, +0x2C and
  +0x34 are set, so +0x14..+0x24 hold stack leftovers until a query writes them. Shipped data reaches it: a farmer's
  market counted first adds two leftover words at +0x1C and +0x20. The bracket (0) is a *[choice]*, recommended.
- ~~**AQ2:** REH §4.8: S_EQ against member E. [E is kept]~~
  **ANSWERED 2026-10-14** from the exe (`GetCellEQ` `0x006F79C0`, `GetCellEQByWealth` `0x006F7A60`,
  `GetEQMinAndMaxCellCoords` `0x006F7380`, `GetAverageEQGrid` `0x006FCF30`) and written into
  residential-education-health §4.8 and residential §2 and §9.2. Only `GetAverageEQGrid` reads the member E, as the new
  grid's tract size. Slots 35, 37 and 43 divide by the R$ EQ grid's own `GetTractSize()`, asked on every call. Both are
  4 in every game-written city. The bracket (E) matches slot 21 only; elsewhere it is a *[choice]* that differs only in
  a hand-made save, and asking the grid is recommended.
- ~~**AQ3:** S_EQ for the HQ variants. [E]~~
  **ANSWERED 2026-10-14** from the exe (`GetCellHQ` `0x006F7A10`, `GetCellHQByWealth` `0x006F7AB0`,
  `GetHQMinAndMaxCellCoords` `0x006F7550`, `GetAverageHQGrid` `0x006FCFD0`) and written into
  residential-education-health §4.8. Slot 22 uses E like slot 21. Slots 36, 38 and 44 divide by the R$ **HQ** grid's own
  tract size, not an EQ grid's and not E. The bracket (E) matches slot 22 only; as in AQ2, the difference shows only in
  a hand-made save.
- ~~**AQ4:** FQ3 save: is flag 1 toggled only when the obliteration record opens? [yes]~~
  **ANSWERED 2026-10-14** from the exe (`cSC4FloraSimulator::SaveObliterated` `0x006B2760`: open test `0x006B2886`,
  clear, write and set `0x006B2890`–`0x006B28AB`, release `0x006B28B2`) and written into flora §10.3. Yes: the flag is
  cleared and set again only in the branch taken when the record opened. The occupant's list reference is released
  either way. The bracket matches.
- ~~**AQ5:** FQ3 restore: is the record closed right after `Read`, before `Init` and `InsertOccupant`? [yes]~~
  **ANSWERED 2026-10-14** from the exe (`cSC4FloraSimulator::LoadObliterated` `0x006B0660`: the record helper is built
  at `0x006B06DA` and destroyed at `0x006B073D`) and written into flora §10.3. No: the helper lives for the whole
  iteration, so the record is closed only after `Read`, `Init`, `InsertOccupant` and `Release`. Neither `Init` nor
  `InsertOccupant` touches the segment, so the order cannot be seen. The bracket (yes) differs from the original; either
  order is safe.
- ~~**AQ6:** FQ3: a failed `GetUint32` of the count. [the count stays 0]~~
  **ANSWERED 2026-10-14** from the exe (`LoadObliterated` `0x006B0660`, count := 0 at `0x006B067E`;
  `cSC4DBSegmentIStream::GetUint32` `0x005D1A20`; record reads `0x00975DE5`, `0x0096D967`, `0x0096DAF8`) and written
  into flora §10.3. Yes: a failed stream skips the read, and the record checks its size before it copies anything, so
  the count keeps the 0 it starts with and nothing is restored. The bracket matches.
- ~~**AQ7:** FQ6: is the 64 m listing clamped to 0..191? [not clamped; only the query is]~~
  **ANSWERED 2026-10-14** from the exe (`InsertOccupant` `0x0049EF60`, listing loop `0x0049F0B6`–`0x0049F1BA`;
  `MoveOccupant` `0x0049F3C0` through `0x0049DAC0`; the query's clamp `0x0049DF00`) and written into flora §6.4.2. Not
  clamped: the listing keeps each cell's low byte and never tests it against 192, so an edge more than a city width
  outside the city writes past the index or is listed nowhere. The bracket matches; for such boxes clamping is a
  *[choice]*, recommended.
- ~~**AQ8:** A bound on the obliteration open attempts. [choice: at most `MAX_OBLITERATED` = 2^24; the original has no
  bound]~~
  **ANSWERED 2026-10-14**: no original behaviour (the original makes `count` open attempts with no bound,
  `0x006B06C6`–`0x006B0749`); the implementer's choice stands. Noted in flora §10.3.
- ~~**AQ9:** Flora shadow: which box gives the 64 m listing? [choice: the census box taken before the pass]~~
  **ANSWERED 2026-10-14**: no original behaviour; the implementer's choice stands. Noted in flora §6.4.2 with the
  original's rule: the listing is made from the box at `InsertOccupant` or `MoveOccupant` time and kept in the occupant
  (`0x0049F094`) until the next move.
- ~~**AQ10:** Plugin: `OpenOStream`/`OpenIStream` returns false but writes a stream pointer. [treated as not open;
  nothing released]~~
  **ANSWERED 2026-10-14** from the exe (record helpers `0x005D9C90` / `0x005D9DB0`, open test `0x005D9670`, destructors
  `0x005D9C30` / `0x005D9D50`) and written into flora §10.3. Not open: the helper keeps the segment only on success, and
  the open test needs both. But the written pointer stays in the helper's stream reference, and the helper releases it
  once when it is destroyed, without a close call. The bracket's "nothing released" differs; *[choice]* release it once,
  as the original does (recommended).

## From applying the crime, traffic network and HG answers (filed by the coordinator from the `apply7b` lane's report, 2026-10-14)

CQ16–CQ22, the crime errata, TN21–TN25 and HG1 are applied (CQ16 and TN21 changed behaviour; the clock-seed helper
moved to `rng.rs`). Not done: the development simulator's own clock seed still XORs by itself (the development glue
lane owns those files); TN22/TN24 have unit tests but no vectors. The current answer is in brackets.

- ~~**CQ23:** CQ17 follow-up: what does a name key `0x8A416B2E` stored as a non-u32 array of 3 or more elements give?
  The original copies 12 raw bytes. [FreeCity4 converts the values to numbers, so a u8 array [a, b, c] gives G = b, I
  = c; a string counts as missing]~~
  **ANSWERED 2026-10-14** from the exe (`cSCPropertyHelp::GetPropertyValue` `0x005FD5B0`, called at `0x0060323E` for the
  name key and `0x006A0D41` for the icon key) and written into crime.md ("Other value types", after the CQ17 table). The
  original never converts: it takes the first 12 bytes of the array's data as three little-endian `u32`, so with at
  least 12 data bytes `G` = bytes 4–7 and `I` = bytes 8–11 (a u16 array gives `e[2] + (e[3] << 16)` and
  `e[4] + (e[5] << 16)`, a 32-bit array the bits of `e[1]` and `e[2]`, a string its bytes). A u8 array [a, b, c] has 3
  bytes, so both come from past the block. The bracket differs. *[choice]* (recommended): reinterpret when there are at
  least 12 bytes, otherwise treat the key as missing. Not observable on shipped data.
- ~~**CQ24:** CQ20 follow-up: the original reads the name key from a second, uncached load of the type's exemplar.
  [FreeCity4 reads it from the same cached load as the other properties; the two differ only if the cached copy was
  changed in memory]~~
  **ANSWERED 2026-10-14** from the exe (loader caller `0x006A1455`–`0x006A14FE`, cached load `0x006A0BC0`, uncached load
  `0x006032AE`) and written into crime.md ("The two exemplar loads agree"). Both loads use the same key (type
  `0x6534284A`, group `0xEA12F32C`), so they read the same record. They differ only if the cached copy was changed in
  memory (the loader only reads it; other writers were not traced) or if the uncached load alone fails, which sends the
  name to `0xAA12FCB0`. The bracket matches; the implementer's choice stands.
- ~~**TN29:** TN21 worked example: Street (3) is not in the §4.8 tunnel type order, so a Street portal never gets a
  tunnel record. How does a "Street tunnel cell" get the tunnel bit in the game? [any tunnel record keyed by the cell
  sets the bit; the test and vector tn11_04d put a Road portal over a Street piece]~~
  **ANSWERED 2026-10-14** from the exe (`DoConnectionsChanged` step 5, `0x0071B390`–`0x0071B3FF`; tunnel insert
  `0x007140E0`) and written into traffic-network §4.2 step 5 and §4.6. Only through its cell key: step 5 sets wormhole
  bit 2 on every cell whose key is in the tunnel table, with no network or type test. So a cell whose car record is
  typed Street gets the bit when a portal of a listed type is keyed at the same cell; a Street portal itself never gets
  a record. The bracket matches, and tn11_04d is a valid synthetic case. Whether the game's tools can build such a cell
  was not traced.

## From applying the residential, budget, demand, land value and flammability answers (filed by the coordinator from the `apply7a` lane's report, 2026-10-14)

RG23 (+RG17), BQ3, DQ10–DQ12, LV1–LV4 and FL1–FL6 are applied: the proximity seed writes only `z = 0`; the budget
total is a float32 running sum in master-list lot order; the land value shadow orders developers by pointer and
snapshots maps while slot 10 is true; the flammability shadow polls the plumbing watered grid. The current answer is
in brackets.

- ~~**RG24:** RG23: what does the proximity seed do when there is no building map? [writes FAR to `(x, 0)` only and
  keeps every other cell; a choice]~~
  **ANSWERED 2026-10-14** from the exe (seed helper `0x005E4FA0`: null map → return at `0x005E4FA8`, null grid at
  `0x005E4FB5`; state 13 call `0x006FF87C`) and written into residential §5.3 step 1. It writes nothing, not even the
  edge. A fresh grid stays all 0 and a reused one keeps the previous wealth's result, so with no map every proximity
  grid is 0. The bracket (255 on the edge, giving 1 there after the passes) differs; *[choice]* write nothing, as the
  original does (recommended).
- ~~**BQ4:** BQ3: releasing the safe lot iterator. The lane unlinks its nodes by hand, the same as `std::list::remove`
  at `0x00809580`, and frees each with allocator B, instead of calling `0x00809580`. [an equivalent unlink; the block
  is leaked, not freed, if no node is found]~~
  **ANSWERED 2026-10-14** from the exe (`ComputeTax` `0x0069207C`–`0x006920AF`; `std::list::remove` `0x00809580`) and
  written into budget.md §5.2. The unlink matches: the original removes every node whose value is the iterator and frees
  each with B. It then frees the block with C unconditionally (`0x006920AF`), whether a node matched or not, and also
  when the owner is null. The bracket's leak differs; *[choice]* free the block with C in every case, as the original
  does (recommended).
- ~~**BQ5:** BQ3: does ComputeTax hold references on the lots it visits? [borrowed, no AddRef]~~
  **ANSWERED 2026-10-14** from the exe (loop `0x00691EB0`–`0x00692077`; `cSC4Lot::GetBuilding` `0x006BFA30`) and written
  into budget.md §5.2. Borrowed: the lot is the list node's value and the building is returned without a reference; the
  loop has no `AddRef` or `Release`. The bracket matches.
- ~~**BQ6:** BQ3: the walk requires the master lot list object at lot manager + `0x5C`. [otherwise refused, with the
  `GetLotsInCellRect` fallback]~~
  **ANSWERED 2026-10-14**: no original behaviour for the refusal; the implementer's choice stands. Facts from the exe,
  written into budget.md §5.2: the original's slot 15 (`0x00734850`) always registers on its member at `+0x5C`, and
  `ComputeTax` reads no lot manager field itself. With a null owner in the block it visits nothing and frees the block.
  A walk through slot 15 needs only the block and node layout, not the manager's.
- ~~**BQ7:** BQ3: line and total values outside the int32 range. [the low 32 bits of `_ftol`]~~
  **ANSWERED 2026-10-14** from the exe (`_ftol` `0x009EEF04`; return `0x006920FF`–`0x00692101`; group stores
  `0x006920DC`–`0x006920DE`) and written into budget.md §5.2. The original never narrows: the total and every group's
  monthly tax are 64-bit `_ftol` results, so values outside the int32 range keep all their bits. Only at or beyond
  ±2^63, or for a NaN, does it give `0x8000000000000000`. The bracket (low 32 bits) differs above 2^31; *[choice]* carry
  64 bits, as the original does (recommended). Normal play never reaches 2^31 simoleons a month.
- ~~**BQ8:** The land value and flammability host tables grew (new last callbacks) without an ABI bump, so a table of
  the old size is now refused by the `struct_size` check. [no bump; refused]~~
  **ANSWERED 2026-10-14**: no original behaviour; the implementer's choice stands. The host tables and their
  `struct_size` check are FreeCity4's plugin ABI; the original has nothing to compare with.
- ~~**LV5:** LV2: a developer first seen while its slot 10 is false gets no snapshot until slot 10 turns true, so there
  is no second candidate in that window. [no snapshot]~~
  **ANSWERED 2026-10-14** from the growth developer's state machine (growth-developer §2, §3.2: slot 10 is `state < 17`,
  slot 28 is `state == 0`) and written into landvalue.md §7.4. Slot 10 is false only in states 17–20, and D changes in
  that window's first call, so a developer first seen there has already published unless it is still in state 17, which
  no slot distinguishes. The second candidate cannot be recovered. Since the state is 0 after construction, `Init` and a
  load, only a shadow that starts mid-update meets this. No original behaviour for the shadow; the implementer's choice
  (no snapshot) stands.
- ~~**FL7:** FL5/FL6: exactness assumes the plumbing pass-end message (posted in state 8) is delivered before the next
  pass swaps its grid (end of state 6). Two passes between polls would bring the pointer back unseen. [assumed; polled
  at every forwarded message and at the cycle end]~~
  **ANSWERED 2026-10-14** from the original's pass structure (plumbing §5.1 abort, §6 restart rule and pass length,
  state 6 swap and state 8 post; simulator §5.1 idle rounds; flammability §9 delivery) and written into flammability
  §6.5. Not always. A pass aborted in state 7 by a water deal status message has swapped but posts nothing, and a pass
  that ends with the recompute flag set restarts at once and can reach its next swap in the same frame, before its
  pass-end message is delivered. Either way the pointer can come back unseen. *[choice]* (recommended): compare the
  published grid's watered bits at every poll instead of the pointer.
- ~~**FL8:** FL5/FL6: a pass first seen by the comparison's own poll, after the wrap day's early visits. [those tracts
  are accepted by the existing previous-value rule, at most 2 × tractsPerDay "unvisited"; marks are not carried into
  the next cycle]~~
  **ANSWERED 2026-10-14** from the original's timing (flammability §6.5 and §9) and written into flammability §6.5. The
  original visits the new cycle's first tracts (at most 2 × `tractsPerDay`) under the same pre-pass state as the old
  cycle's last ones, and they keep those values for a whole cycle. The bracket is right for the comparison in hand, but
  with marks dropped at the boundary those tracts mismatch at the next comparison. *[choice]* Carry the pass's marks for
  the tracts that the new cycle has already visited.
- ~~**FL9:** FL5/FL6 with FreeCity4's own plumbing: detection relies on its published grid alternating too, which it
  does today. [pointer detection only]~~
  **ANSWERED 2026-10-14**: no original behaviour; the implementer's choice stands while FreeCity4's plumbing keeps
  alternating. Note that plumbing §12.2 allows a replacement to keep one fixed published grid, which would defeat
  pointer detection. The content comparison recommended for FL7 (flammability §6.5) does not depend on alternation.

## From the graphics formats spec (re-formats2 lane, 2026-10-14)

graphics-formats.md specifies FSH and S3D as the 1.1.641 exe loads them. These are what it could not settle from the
exe and the shipped data alone.

- ~~**GF1 — graphics-formats §2.6:** under DX7 (the default renderer) the FSH codes `0x64`, `0x65`, `0x6D`, `0x78`,
  `0x7B` and `0x7E` load through the factory but `TranslateGDFormatToSourceFormat` (`0x0088D0B0`) cannot create a
  texture for them. What does a model show then (untextured, white, previous texture, not drawn)? None is shipped
  on a model; plug-in textures may use them. [unknown; FreeCity4 decodes them and shows the texture, *[choice]*]~~
  **ANSWERED 2026-10-14** from the exe (DX7 stage set-up `0x00884320`, disable path `0x008844A6`, translation
  `0x0088D0B0`) and written into graphics-formats §2.6. **Drawn untextured.** The texture has no surface, so the driver
  disables the stage's colour and alpha operations and the model is drawn in its diffuse colour and alpha. It is not
  skipped, and the previous texture is not sampled. Under OpenGL five of the six codes upload; `0x78` fails there too.
  The bracket (decode and show) differs from DX7 but matches the OpenGL renderer for five codes; it stays *[choice]*.
- ~~**GF2 — §2.6, §2.11:** for `0x6D`, `0x7E` and `0x78` the factory asks for BGRA/BGR with the non-reversed packed
  types (4-4-4-4, 5-5-5-1, eGDType 11), which under OpenGL would read the channels in a different order from the
  Direct3D layouts (A4R4G4B4, A1R5G5B5, R5G6B5) that FSH tools write. Is there any renderer path where the game shows
  them correctly, and which layout should FreeCity4 follow? [the Direct3D layouts, *[choice]*]~~
  **ANSWERED 2026-10-14** from the exe (the format and type tables of §2.6, DX7 `0x0088D0B0`) and written into
  graphics-formats §2.11. **No renderer path shows them correctly.** DX7 cannot create them (GF1). OpenGL reads
  `0x6D`/`0x7E` with the channel order reversed and alpha in the low bits, and `0x78` fails under both. The bracket
  (the Direct3D layouts) stands *[choice]*.
- ~~**GF3 — §2.4, §2.11:** DXT1 with A = 0 (13,762 shipped entries) asks for "DXT1 RGB", but DX7 translates both DXT1
  formats to the same surface. Does DX7 then show 3-colour-mode index-3 texels as transparent (so alpha-tested
  materials cut them out)? An in-game check on a model with such texels would settle it. [yes, inferred from the
  translation; FreeCity4 decodes them transparent]~~
  **ANSWERED 2026-10-14** from the exe (`TranslateGDFormatToSourceFormat` `0x0088D0B0`: formats 7 and 8 both become
  source format 10, the one Direct3D DXT1 surface) and written into graphics-formats §2.11. **Yes.** A Direct3D DXT1
  surface always decodes 3-colour-mode index 3 as (0, 0, 0, 0), whatever A says. Materials with alpha test or blending
  drop those texels; a material with neither shows them black. The bracket (decode them transparent) matches. Not
  checked in game.
- ~~**GF4 — §2.13:** the 24 `0x7B` entries (64 × 64 and similar, identity grey-ramp `0x24` palettes) are in group
  `0x891B0E1A`, used by terrain brushes, water and underground code (`0x0074D730`, `0x00752240`, `0x0065D810`).
  Which of them reads these, and does it go through the texture path (which DX7 cannot create) or read the pixels
  from `cS3DShapeFile` directly? [inferred: CPU reads of intensity maps]~~
  **ANSWERED 2026-10-14** from the exe (`cSTETerrainView3D::InitializeCityEdgeLayerHeights` `0x00757300`,
  `cSTETerrain::ApplyBrushComponents` `0x0074D730` with resource interface `0x1AB50C3F`) and written into
  graphics-formats §2.13. They are read on the CPU, never through the texture path. The two 257 × 1 ramps (`0xCA5CC9C0`,
  `0xCA5CC9C1`) are read pixel by pixel into the city-edge layer height tables. The brushes sample their bitmaps on the
  CPU; that the 22 square entries are those brushes is inferred, not traced to the brush data. The bracket matches.
- ~~**GF5 — §3.8:** `SetTexFiltering` (`0x007D4280`) overrides the material's filters with (nearest, nearest) or
  (linear, linear-mipmap-linear) when one of two draw settings is on, and `0x007D41E0` can override wrap. Which
  settings are these, what are their defaults, and which is on in normal play? Without an override, 137,480 shipped
  1.5 textures have filters (0, 0), i.e. nearest. [unknown; probably the trilinear override is on by default]~~
  **ANSWERED 2026-10-14** from the exe (`SetTexFiltering` `0x007D4280`, wrap `0x007D41E0`, render-property
  registrations `0x007DF320`, `0x007DF400` and `0x007E0120`, DX7 filter states `0x00884363`–`0x008843BB`) and the shipped
  `Graphics Rules.sgr`, and written into graphics-formats §3.8. The settings are the boolean render properties
  `kForcePointSampling` (nearest, nearest) and `kForceBilinear` (linear, linear-mipmap-linear); wrap is
  `kForceTextureRepeat`. All three default to false and no shipped rule sets them. So in normal play none applies, and
  (0, 0) means nearest with no mipmaps. The bracket's guess (trilinear on by default) is wrong.
- ~~**GF6 — §3.5:** byte order of the 32-bit vertex colour element (only `0x80004101`, 2 shipped blocks in model
  `0xCB93E2B4`). [inferred: Direct3D colour, bytes B, G, R, A]~~
  **ANSWERED 2026-10-14** from the exe (DX7 vertex format `0x008891C0`, diffuse flag at `0x008891EA`; copy for
  `0x80004101` `0x0088C7D0`) and written into graphics-formats §3.5. DX7 copies the 32-bit colour unchanged into the
  vertex's Direct3D diffuse colour, so the bytes are B, G, R, A. The bracket matches. OpenGL not traced.
- ~~**GF7 — §4.2:** layouts of RKT4 (`0x27812824`) and RKT5 (`0x27812825`), read by `GetAllModelInstanceIDs`
  (`0x00496C90`, `0x00497120`) and `IsStaticBuilding` (`0x00583B10`), not by `GetModelInstanceID`. [out of scope
  here; for the building-rendering lane]~~
  **ANSWERED 2026-10-14** from the exe (`GetAllModelInstanceIDs` `0x00496C90`: zoom mask `0x00496D06`–`0x00496D25`,
  RKT4 walk `0x00496F2E`–`0x00497078`, RKT5 `0x0049707D`–`0x004970CB`; occupant form `0x00497120`;
  `IsStaticBuilding` `0x00583B10`) and a sweep of the shipped `.dat` files, and written into graphics-formats §4.2
  ("All the models of an occupant"). RKT4 is a list of records, each (selector, x, y, z, kind) followed by an
  RKT0–RKT3 value set of that kind (3 values for RKT0/RKT1, 30 for RKT2, 5 for RKT3). Every record whose selector
  equals the occupant's state gives one instance and an offset (x, y, z as signed 16.16 fixed point). An unknown
  kind or a short record ends the walk. RKT5 is RKT1 plus state × `0x10000`. Property `0x0ABFC024` is a zoom mask
  checked first, and the occupant's flag `0x80` switches all six keys to `0x278129xx`. `IsStaticBuilding` only
  tests that the keys are present. Shipped: RKT4 on 618 props, RKT5 on 5 flora; four streetlights carry a stray
  value that stops their `0x27812924` walk after one record. The bracket (out of scope) is replaced by this answer.
- ~~**GF8 — §4.2:** where the model detail global `0x00B216FC` (zoom remap table `0x00B094A4`) is set, and from which
  option. [unknown; 0 in the image]~~
  **ANSWERED 2026-10-14** from the exe (`cSC4ModelMaker::Init` `0x004948BE`–`0x004948DF`, message handler
  `0x00496466`–`0x00496495`, render-property getter slot 7 of vtable `0x00ABB6C0`, registration `0x007E2330`) and the
  shipped `Graphics Rules.sgr`, and written into graphics-formats §4.2. The global is the integer render property
  `kModelLODOffset` (22) clamped to 0–2. It is read at model-maker `Init` and again on message `0xC9F775BB`, which
  `cSC4App::ApplyVideoPreferences` posts. The option is Texture Quality: High 0, Medium 1, Low 2; the default is 0.
- ~~**GF9 — §2.8:** 167 shipped DXT1 entries store their mip levels without the 16-byte padding, and the game reads
  the smallest two or three levels from the wrong offsets (the last one past the pixel data). Reproduce, or read
  them as stored? [reproduce, zero-filling past the entry, *[choice]*]~~
  **ANSWERED 2026-10-14** from the exe (level table in `SerialRead` `0x00800860`; the levels are read from the
  `size`-byte buffer of `Open` `0x00808F00`) and a sweep of all 45,387 shipped entries, and written into
  graphics-formats §2.8. All 167 are in group `0x1ABE787D` and each is followed by a `0x70` name block. The last level
  always reads that block's first 8 bytes, and no read passes the record end. Reproduce, as the bracket says, but with
  the record's bytes rather than zeros: zero only past `size`, which no shipped entry reaches *[choice]*.
- ~~**GF10 — §3.9:** semantics of ANIM frame rate (30, 10, 3, 2, 1, 0 shipped), mode (always 3), flags and
  displacement (always 0), as the animated model instance uses them. [not traced; for the rendering spec]~~
  **ANSWERED 2026-10-14** from the exe (animated instance `0x007FD870`, `SetFrame` `0x007FDAF0`, `Step`
  `0x007FDB80`, `SetSpeed` `0x007FC170`, `Advance` `0x007FC1C0`, `SetAnimMode` `0x007FAFC0`; prop driver
  `0x005ECFB0`) and written into graphics-formats §3.9. The rate is frames per second (speed = rate × 0.001 per
  ms); rate 0 never advances. The file's mode, flags and displacement are never read. The instance has its own
  mode: 3 (default) loops, 2 ping-pongs, other values play once and hold; automata attach with mode 1. `Advance`
  steps at most one frame per call.
- ~~**GF11 — §3.10, §3.11:** consumers of PROP (`skeleton`/`suit` pairs on 58 people models) and REGP (`fx_…` names on
  1,810 models), and the meaning of REGP's seven floats. [inferred: effect attachment points, position and
  orientation; not traced]~~
  **ANSWERED 2026-10-14** from the exe (`cSC4ModelRegEffects` `0x005E8320`, `0x005E8810`, `0x005E6160`; fire crew
  `0x00548F10`, `0x00548BD0`; airplane `Init` `0x005017C0`) and a sweep of the shipped models, and written into
  graphics-formats §3.10 and §3.11. PROP has no consumer. REGP `fx_` names become visual effects on automata,
  helicopters, tanks and bridges (not on buildings, props or flora); `regpoint01` gives fire crews a point. There
  is one record per animation frame (C = F in all 3,025 entries). The data hold a position and a unit quaternion,
  but the exe reads the last four floats as an axis and an angle in radians. The bracket is right about the
  position, not about the orientation.
- ~~**GF12 — §2.4:** does anything outside the 3D texture path read the FSH centre fields (non-zero in 722 entries) or
  bits 12–27 of `+0x0C` (non-zero in 294)? [none found]~~
  **ANSWERED 2026-10-14** from the exe (entry-header decoder `0x00808D70` and its callers `0x008008EE`,
  `0x00808FBB`, `0x008090A0`) and written into graphics-formats §2.4. The decoder never copies the centre fields or bits
  12–27; it keeps code, width, height, the directory tag, the pixel pointer, A and M. No other code parses FSH. The
  bracket (none) matches.

## From the zones glue and shadow (filed by the coordinator from the `zoneglue` lane's report, 2026-10-14)

About `docs/specs/zones.md` and the zone manager glue (`cFC4ZoneManager`, `FC4ZonesShadow`; phase 1, the zone
developer stays original). The current answer is in brackets.

- ~~**ZG1:** The tuning exemplar type: `0xA52160F5` or §4's `0x6534284A`? [`0x6534284A`]~~
  **ANSWERED 2026-10-14** from the exe (`ReadSimulatorTunableValues` `0x004DC460`: key `0x004DC49B`–`0x004DC4AB`,
  resource call `0x004DC4B6`) and written into zones §4. The type is `0x6534284A`; `0xA52160F5` is the interface ID
  asked of the resource manager with that key. The bracket matches.
- ~~**ZG2:** An oversized region or a bad §11.4 layout passed to PlaceZone. [fails with error 0 and touches nothing]~~
  **ANSWERED 2026-10-14** from the exe (`PlaceZone` cell loop `0x004DD8B0`–`0x004DD913`) and written into zones §6
  (pseudo-code and a new note). The original has no city-bounds test. It skips only cells outside the region's own
  bitmap or with the bit clear, then reads (and on a commit writes) the zone grid unchecked. A rect past the city edge
  therefore reads and writes outside the grid, and a bad bitmap is dereferenced. The bracket (fail with error 0, touch
  nothing) is a *[choice]*, recommended. Also corrected: §6's "only cells inside the city".
- ~~**ZG3:** The zone display alpha after `Read`. [the alpha read]~~
  **ANSWERED 2026-10-14** from the exe (`Read` `0x004DD58E`; `Init` `0x004DE192`; `ReadSimulatorTunableValues`
  `0x004DC65E`–`0x004DC675`) and written into zones §10.1. Right after `Read` the alpha is the saved byte, as
  bracketed. But `Init`, which follows every load, overwrites it with the exemplar's Zone Display Alpha (shipped 170)
  and sets the default alpha to the same value. The saved alpha survives only when the exemplar or the property is
  missing.
- ~~**ZG4:** A second `Read`: the services the first one set. [replaced; the old ones released]~~
  **ANSWERED 2026-10-14** from the exe (`Read` reference reads `0x004DD43F`, `0x004DD714`–`0x004DD75B`; reference
  reader `0x005D9730`, null store `0x005D9742`) and written into zones §10.1. The original does not release the old
  references: the reader nulls the field and reads the new one, so the old grid and services leak one reference each.
  Releasing them (the bracket) is the implementer's choice, recommended.
- ~~**ZG5:** ZoningChange `void1` / `IGZUnknown`. [SetVoid1 plus SetIGZUnknown on the zone manager; the message holds a
  reference]~~
  **ANSWERED 2026-10-14** from the exe (`PlaceZone` `0x004DDFDB` message slot 12, `0x004DDFE3` slot 33; standard
  message vtable `0x00A92D40`) and written into zones §5.3. `SetVoid1` stores the zone manager raw and `SetIGZUnknown`
  AddRefs it, so the message holds one reference. The bracket matches.
- ~~**ZG6:** A dezone arriving during a PlaceZone. [runs at once and is counted]~~
  **ANSWERED 2026-10-14** from the exe (`DoMessage` `0x004DCA70`, no re-entry guard; `PlaceZone` `0x004DD780`)
  and written into zones §5.1. It runs at once. The InsertOccupant is delivered synchronously, so a network occupant
  inserted during a `PlaceZone` commit (for example a street drawn by the parcelling) runs the dezone `PlaceZone` nested
  inside it. The original counts nothing; the counting is glue, the implementer's choice.
- ~~**ZG7:** When are `outLots` inserted? [at the pricing call]~~
  **ANSWERED 2026-10-14** from the exe (`PlaceZone` `0x004DDC94`–`0x004DDCC6`, after the pricing `DemolishLots`
  at `0x004DDC75`) and written into zones §6. The lots are inserted right after the pricing call, before the landfill
  cost and the funds test, so also on a dry run, after an abort and on error 1. The bracket matches.
- ~~**ZG8:** What is released after a failed `Read`? [the grid, at Shutdown or destruction]~~
  **ANSWERED 2026-10-14** from the exe (`Read` `0x004DD415`–`0x004DD425`; `Shutdown` `0x004DC230`; `Release`
  `0x004DC441`–`0x004DC44E`; destructor body `0x004DCC70`; `Init` `0x004DE220`, `0x004DE26A`) and written into zones
  §10.1. A failed `Read` stops at the version and acquires nothing. The grid and services held are released by
  `Shutdown` only, which the final `Release` runs before the destructor; the destructor frees only the two count maps.
  The bracket matches, with "destruction" meaning the final `Release`.
- ~~**ZG9:** Glue behaviour with switches off or calls from another thread. [`zones_budget` off: unlimited funds,
  nothing charged; no budget simulator: funds 0, withdraw false; PlaceZone from another thread without the lock:
  false, error 0; such a message: dropped; SetZoneDisplayAlpha: unchanged; GetZoneType fallback: 0 outside the grid]~~
  **ANSWERED 2026-10-14** from the exe (`PlaceZone` `0x004DD78A`; `GetZoneType` `0x004DC740`) and written into zones
  §6: no original behaviour; the implementer's choice stands. The original has no switches and no threads. Where it
  meets these cases it does not cope: `PlaceZone` calls the budget simulator with no null test (a crash without one),
  and `GetZoneType` reads outside the grid unchecked (§11.1).
- ~~**ZG10:** Is the loaded flag set before the version check, even on failure? [yes; the core changed to do this]~~
  **ANSWERED 2026-10-14** from the exe (`Read` `0x004DD415`: the flag at +0x14 is set before the version read at
  `0x004DD41D` and the compare at `0x004DD420`) and written into zones §10.1. Yes: the flag is set first, also when the
  version check fails. The bracket matches.
- ~~**ZG11:** Occupant manager slot 33. [called through the SDK overload, relying on MSVC's reversal of overloaded
  virtuals]~~
  **ANSWERED 2026-10-14** from the exe (occupant manager vtable `0x00A8F7F0` slots 28–35; slot 33 `0x0049DCE0`
  forwarding to slot 32 `0x0049E690`) and written into zones §6 and §12. Slot 33 is the SDK's typed
  `GetFirstOccupantByStandardCityCell(out, x, z, uint32)`, declared before its filter overload and placed after it by
  MSVC's reversal; every typed/filter pair in slots 28–35 follows the same pattern. The bracket matches; it holds for
  MSVC and clang-cl, not GCC.
- ~~**ZG12:** Shutdown followed by the final Release. [Shutdown runs twice]~~
  **ANSWERED 2026-10-14** from the exe (`Release` `0x004DC441`–`0x004DC44E`; `Shutdown` guard
  `0x004DC234`–`0x004DC246`) and written into zones §1. Yes, `Shutdown` runs twice: the final `Release` calls it before
  the deleting destructor. The second call finds the initialised flag clear, does nothing and returns 1. The bracket
  matches.
- ~~**ZG13:** Shadow: "our core's bookkeeping after it has loaded the same grid". [the core loads the original's own
  record through its Write]~~
  **ANSWERED 2026-10-14**: no original behaviour; the implementer's choice stands. The original has no shadow. For
  reference, its `Write` (`0x004DC7A0`) record (zones §10.1) carries the counts, both count maps, the tuning arrays and
  the alpha, but not the max slopes or the default alpha; `Init` re-reads the tuning after a load (§4, ZG3).
- ~~**ZG14:** Is a PlaceZone dry run (commit=false, outLots null) on the original safe? `CanRemove(2)` is an inferred
  name. [yes, per §6 and §13.2; `CanRemove(2)` assumed to be a query]~~
  **ANSWERED 2026-10-14** from the exe (`PlaceZone` commit gate `0x004DDE5E`–`0x004DDE67`, `outLots` test
  `0x004DDC9B`; `DemolishRegion` `0x00468410`; `cSC4Occupant::CanRemove` `0x005E8AE0`) and written into zones §6 and
  §12. Yes, it changes no game state: no withdrawal, grid write, demolition, parcelling or message, and a null
  `outLots` is skipped. It writes only the caller's region, the occupant manager's shared filter type and the
  demolition utility's scratch fields. `CanRemove` is the SDK name and a pure query of a removal counter. The bracket
  matches. Not every callee of the pricing pass was traced.
- ~~**ZG15:** Dry-run sample regions and flags (the spec names no UI caller). [four seeded regions: 1×1 checkSlope with
  size check, 2×3 checkSlope without size check and a lot inside, 4×4 the building-development form, 7×3 checkerboard
  without flags]~~
  **ANSWERED 2026-10-14**: no original behaviour; the implementer's choice stands. The original's own callers of
  `PlaceZone` in this spec (the dezone, §5.1; the BDS, §6) both commit. The UI zoning tool, not traced, is the likely
  dry-run caller; tracing its calls to slot 11 would show which flags it uses.
- ~~**ZG16:** Are `*outCost` and `*outError` written on an early size-check failure? [the shadow fills the original's
  with 0 first; our core writes 0]~~
  **ANSWERED 2026-10-14** from the exe (`PlaceZone` outputs `0x004DE02B`–`0x004DE052`; ZG17's trace) and written into
  zones §6. Yes: both are written on every exit, the size-check failure included. `*outError` is 3 or 4, or 1 when the
  cost exceeds the funds; `*outCost` is 0 for every type except 14, which gets the landfill zoning cost of the whole
  region (0 with `skipFunds`). The shadow's pre-fill is harmless and is overwritten. Our core's 0 matches for every type
  but 14 (ZG17).
- ~~**ZG17 — zones §6, in game (regress rzones, coordinator):** the zones shadow's 1×1 PlaceZone dry run (checkSlope,
  size check, commit=false) with zone type 14 fails with error 3 on both sides, but the original writes a cost
  (36–208, varying by month and place) and ours writes 0. 15 of 92 months differ, only in this case. What does the
  original write to `*outCost` when PlaceZone fails with error 3, and for which zone types? [ours: 0]~~
  **ANSWERED 2026-10-14** from the exe (`PlaceZone` `0x004DD780`: size failures `0x004DD82A`/`0x004DD846` jump to the
  abort path `0x004DDC31`; landfill cost `0x004DDCC8`–`0x004DDCF5`; funds test `0x004DDE34`; outputs
  `0x004DE02B`) and written into zones §6 (notes, worked example). **A failed size check is an abort, not an early
  return.** The cell loop is skipped but the post-loop block runs: for type 14 only, the cost is
  `landfillDeveloper.GetZoningCost(region)` of the whole request region (0 with `skipFunds`; every other type: 0).
  `*outCost` gets that cost and `*outError` 3 (or 4), unless the cost exceeds the funds, which turns the error into
  1. Also corrected: a type ≥ 16 is replaced by 0 (dezone), not a failure.

## From the FSH and S3D readers (filed by the coordinator from the `formats3` lane's report, 2026-10-14)

About `docs/specs/graphics-formats.md` and `engine/formats` (`fsh`, `s3d`, `keys`). All §8 tests 1–43 pass; the
release sweep reproduces the §8.4 counts. Not done: the info-only open of §2.9; a DXT1 A = 0 opaque-black decoder
variant (GF3). The current answer is in brackets.

- ~~**FS1:** §2.8 says bytes "beyond the entry" read as zero, but not where an entry ends. [it ends at the first of: its
  attached block (`next`), another entry's header, or `size`; only positions at or after its pixel start count]~~
  **ANSWERED 2026-10-14** from the exe (level table `0x00800A60`–`0x00800AF5`: offsets only, no end test) and
  written into graphics-formats §2.8. The original has no entry end: it reads each level at its aligned offset from the
  `size`-byte buffer, whatever lies there. The bracket's end (attached block, next entry header or `size`, at or after
  the pixel start) is the implementer's choice for the acceptance test, and it reproduces the 167. But the bytes decoded
  past that end are the record's own, not zero (GF9).
- ~~**FS2:** Q2 rejects what does not fit "except the mip-padding case": how is that case told from a truncated entry?
  [if the levels fit packed back to back but the 16-byte-aligned layout overruns, zero-fill; if even the packed levels
  do not fit, reject. Shipped data: exactly 167 zero-filled, 0 rejected]~~
  **ANSWERED 2026-10-14** from the exe (`SerialRead` `0x00800860` checks no sizes) and a sweep of all 45,387 shipped
  entries, and written into graphics-formats §2.8. The original makes no such distinction: it accepts every layout and
  reads whatever follows. The bracket's test (packed fits, aligned overruns: accept; not even packed: reject) is the
  implementer's choice and stands. The sweep reproduces it independently: 167 accepted that way, 0 rejected. In every
  one of the 167 the aligned layout overruns by exactly 8 bytes.
- ~~**FS3:** A record with `count` = 0 has no frame 0. [rejected]~~
  **ANSWERED 2026-10-14** from the exe (`SerialRead` `0x008008D0`–`0x008008DB`: the count from the header, 0 →
  fail) and written into graphics-formats §2.7. A zero count fails the load before any entry is read, in both the full
  and the info-only open. The bracket (rejected) matches.
- ~~**FS4:** The original checks the `0x7B` palette on frame 0 only. [every `0x7B` frame needs a 256 × 1 palette with
  all 256 entries inside the record]~~
  **ANSWERED 2026-10-14** from the exe (`SerialRead` reads the palette info and calls `GetTypeAndFormat` for frame 0
  only, `0x008008DE`–`0x0080090D`; the frame loop compares headers only) and written into graphics-formats §2.6. Only
  frame 0 is checked, and the palette is never applied. The bracket (every frame needs one) is stricter than the
  original. Check frame 0 only, and decode a later frame with no usable palette as the grey the original uploads
  *[choice]*.
- ~~**FS5:** For DXT1, the field A could differ between frames. [each frame reports its own (format, type);
  `Fsh::format()` is frame 0's]~~
  **ANSWERED 2026-10-14** from the exe (`GetTypeAndFormat` called once, from `SerialRead` `0x0080090D`, with frame 0;
  `SetBindingImageData` reads the stored pair at `0x00809BA7`) and written into graphics-formats §2.10. Every frame uses
  frame 0's (format, type); a later frame's A is never read. The bracket (each frame its own pair) differs. Report frame
  0's pair for every frame. The decoded pixels are the same either way, since the decoder ignores A (§2.11).
- ~~**FS6:** Negative sign-extended dimensions. [the power-of-two test is on the 32-bit sign-extended height, so a
  negative height fails; max(W, m) is signed, so a negative width becomes m]~~
  **ANSWERED 2026-10-14** from the exe (`SerialRead` `0x0080094E`–`0x0080095A` power-of-two test on the 32-bit
  sign-extended height; `0x00800964`/`0x00800976` signed max) and written into graphics-formats §2.7. A negative height
  fails the test, and a negative width becomes m. The bracket matches.
- ~~**FS7:** A DXT width that is not a multiple of 4: the level size from the formula is smaller than the whole blocks.
  [the missing bytes read as zero, whole blocks are decoded, and the image is cropped]~~
  **ANSWERED 2026-10-14** from the exe (`nSGLDX7::cTextureManager::LoadTextureLevel` `0x0088ACD0`, `CopyToSurface`
  `0x0088A380`: a compressed surface is filled with its whole linear size from the level pointer) and written into
  graphics-formats §2.8. DX7 reads the whole blocks from the level's start, so the missing bytes are the record bytes
  that follow, not zeros. Decoding whole blocks and cropping stands; take the missing bytes from the record (zero only
  past `size`), as for GF9. OpenGL not traced.
- ~~**FS8:** The DXT5 alpha divisions (/7, /5) have no stated rounding. [rounded down, as for DXT1]~~
  **ANSWERED 2026-10-14**: no original behaviour; the original never decodes pixels on the CPU, and the GPU does
  the DXT interpolation. The implementer's choice (round down, as for DXT1) stands; noted in graphics-formats §2.11.
  No DXT5 entry is shipped.
- ~~**FS9:** A vertex format with stride below 12, or with no position, while the bounding box reads 3 floats per
  vertex. [stride < 12 is rejected; no position with stride ≥ 12 is accepted and its box uses the first 12 bytes]~~
  **ANSWERED 2026-10-14** from the exe (bounding box `0x00801040`: three floats at each vertex start, step S, no
  format test) and written into graphics-formats §3.4. The original boxes the first 12 bytes of each vertex whatever the
  format, so with S < 12 the last vertex reads past the vertex data. The bracket (reject S < 12; accept no position with
  S ≥ 12 and box its first 12 bytes) matches where the original is defined.
- ~~**FS10:** A legacy VERT stride S below the format's standard stride, or a legacy block with an unknown format. [both
  rejected]~~
  **ANSWERED 2026-10-14** from the exe (legacy VERT keeps S as read; `RenderMesh` `0x007D4A80` draws with the standard
  stride) and written into graphics-formats §3.4. The original loads both. With S below the standard stride the draw
  reads past the vertex data. An unknown legacy format is read and boxed with S but drawn with stride 0 and no known
  layout. Rejecting both (the bracket) stays the choice.
- ~~**FS11:** PROP is "sorted after reading", but the sort key is not given. [file order kept]~~
  **ANSWERED 2026-10-14** from the exe (PROP reader `0x00807EED`–`0x00808264`; sort `0x00806B40` compares the `s16` at
  +2, the frame) and a sweep of the shipped models, and written into graphics-formats §3.10. The key is the frame,
  signed and ascending, by `std::sort`. Equal frames keep file order up to 16 entries, not beyond. Every shipped entry is
  (−1, 0). The bracket (file order) matches the shipped data but not the original in general. Sort stably by frame
  *[choice]*.
- ~~**FS12:** REGP lower-casing: ASCII or locale-aware? Which encoding for byte names? [ASCII lower-casing; names
  decoded as UTF-8 when valid, else Latin-1]~~
  **ANSWERED 2026-10-14** from the exe (`cRZString::MakeLower` `0x0090EAC5`; `setlocale` from
  `cRZLanguageManager::SetCurrentLanguage` `0x00928021`) and a sweep of the shipped models, and written into
  graphics-formats §3.11. An all-ASCII name is lowered with `tolower`. Any other name goes UTF-8 → UTF-16 → `towlower`
  → UTF-8 in the game language's locale, so its non-ASCII letters are lowered too. All 3,025 shipped names are ASCII.
  The bracket matches for them. For a valid UTF-8 name, also lower non-ASCII letters *[choice]*.
- ~~**FS13:** §4.1 when the fallback texture itself does not load. [the fallback key is returned anyway]~~
  **ANSWERED 2026-10-14** from the exe (`GetModelFrame` `0x00806DF5`–`0x00806E02`: the fallback lookup's result is not
  tested) and written into graphics-formats §4.1. The slot is left with no binding and its ID set to 0, so it is never
  retried and draws without a texture *[inferred]*. The bracket (the fallback key returned anyway) differs. Resolve the
  slot to no texture *[choice]*.
- ~~**FS14:** RKT lookups with a zoom above 4, a detail above 2, a rotation above 3, or too few values. [no model
  (`None`); RKT0 ignores zoom and detail; the RKT1 addition wraps]~~
  **ANSWERED 2026-10-14** from the exe (`GetModelInstanceID` `0x00496830`–`0x00496937`, table `0x00B094A4`, RKT2
  index `0x005F8AB0` = 4z + r) and written into graphics-formats §4.2. The original checks nothing. A large zoom reads
  the next table row or the pointers after the table. Too few values read past the array. r is a signed remainder, so it
  is −3 to 3. Detail is clamped when it is set. The bracket (no model) stands for zoom and value count. Clamp the detail
  to 0–2 and take the rotation mod 4 rather than refusing *[choice]*. RKT0 and the RKT1 wrap match.

## From applying the traffic simulator answers (filed by the coordinator from the `apply6a` lane's report, 2026-10-14)

TS1–TS34 are applied to the traffic simulator core (vectors ts11_27–ts11_46 new; ts11_10a and ts11_25 adjusted). The
host gained connection, region, budget and drive-side methods. The current answer is in brackets.

- ~~**TS35:** Does entering state 18 zero pass counter 1? [yes]~~
  **ANSWERED 2026-10-14** from the exe (`DoDailySimulation`: state 17 `0x007218B4`–`0x007218F2`, state 18
  `0x007217BF`–`0x007217FD`; the 16/17/18 entry action in `WorkOnSimCycle` `0x007210B0`) and written into
  traffic-simulator §3.3. No: entering 18 zeroes the day index, `processed` and `passTotal`, not pass counter 1
  (+0x94C). Only entering 17 zeroes it, so it keeps state 17's re-route count through state 18. The bracket (yes) is
  wrong.
- ~~**TS36:** After a dx = ±1 jump, what entry does the cell carry? [none]~~
  **ANSWERED 2026-10-14** from the exe (route reader `GetNext` `0x006D6510`: the none exit at `0x006D665F`, the skipped
  update at `0x006D666F`; `SetData` `0x006D64FD`) and written into traffic-simulator §4.3. The landing cell carries the
  entry side that the cell before the jump had on arrival. The reader blanks it only in that cell's own record and keeps
  it for the next one. It is none only when the cell before the jump is the first cell. The bracket (none) is wrong.
- ~~**TS37:** Does a missing funding percent leave the default? [unchanged]~~
  **ANSWERED 2026-10-14** from the exe (percent getter `0x005FDAC0`: missing property `0x005FDAD6` → `0x005FDCBE`, other
  types `0x005FDAF3` → `0x005FDCBD`; calls `0x00716D67`, `0x00716D79`) and written into traffic-simulator §4.14. Yes:
  the getter returns without a store, so the code default 1.2 stays. The bracket matches.
- ~~**TS38:** §8 and §4.1.4 name slot 13 differently from §2.7.2. Which is right? [the code keeps its existing naming]~~
  **ANSWERED 2026-10-14** from the exe (vtable `0x00AB1EE0` slot 13 `0x006CDFA0`, pop 4; slot 14 `0x006CEDE0`, pop 12;
  regional-city slot 20 `0x004A7B40`) and written into traffic-simulator §8. §4.1.4 and traffic-network §2.7.2 are
  right: slot 13 is `FindConnectionOnOtherSide(conn)`. It takes the connection's type and cell and calls slot 14, which
  returns the matching connection from the neighbour's regional record, or null. §8's "property holder" named what the
  caller does with the result; corrected. The naming in code is the implementer's choice.
- ~~**TS39:** What is the "jump flag" in TS16? [the 0x80 flag of the next jump record]~~
  **ANSWERED 2026-10-14** from the exe (route reader `0x006D65C6`–`0x006D65CC`, run tokens `0x006D6593`; ApplyRoute test
  `0x00719E97`) and written into traffic-simulator §4.5. It is bit 7 (`0x80`) of the jump token that leads to the next
  record. The reader stores it in the record the jump arrives at, and run tokens clear it. The bracket matches.
- ~~**TS40:** What does a statistics slot with no record publish? [zeros]~~
  **ANSWERED 2026-10-14** from the exe (state-2 entry `0x00721196`–`0x007211D6`; record update
  `0x0071A3D7`–`0x0071A40C`; publish `0x0071A5D7`–`0x0071A67A`) and written into traffic-simulator §4.9. Zeros: average
  0.0, congested 0, max cell (0, 0), max 0.0. The −1.0 start is a separate comparison value, not the published max. The
  bracket matches.
- ~~**TS41:** A pool ID repeated in row 19: which entry wins? [the last one]~~
  **ANSWERED 2026-10-14** from the exe (traffic `Read` `0x00722F60`–`0x00722FB3`; map find-or-insert `0x00714560`) and
  written into traffic-network §2.7. The last one wins: each entry's holder is stored through the map's find-or-insert,
  overwriting the earlier pointer, which is never released. The bracket matches.
- ~~**TS42:** Pool ID overflow. [pool 0]~~
  **ANSWERED 2026-10-14** from the exe (`AddNeighborConnection` ID search `0x0071555A`–`0x00715595`, store `0x007155A6`)
  and written into traffic-network §2.7.2. The count wraps to ID 0: the record gets pool 0, the default pool, and the
  new holder is stored in the map under key 0. If key 0 is taken too, the search never ends. Unreachable in the game.
  The bracket (pool 0) matches.
- ~~**TS43:** A missing regional demand node. [0]~~
  **ANSWERED 2026-10-14** from the exe (`0x00714E60`: `GetDemand` at `0x00714ED9`, its result used at once at
  `0x00714EDC`) and written into traffic-network §2.7.4. The original crashes: `GetDemand` returns null for a missing
  node and the value query goes through it. The shipped data define the nodes. The bracket (0) is a *[choice]*,
  recommended.
- ~~**TS44:** Which row-14 field holds the saved source cell? [`phase_positions` s16×2, the lot location]~~
  **ANSWERED 2026-10-14** from the exe (writes `0x0071E276`, `0x0071D0FD`, `0x0071D51C`; source iterator `0x007107F0`;
  resume `0x00720B9A`–`0x00720C2E`) and written into traffic-simulator §3.2, §3.5 and §6.1. It is the two s16 phase
  positions. The trip phases store the cell the source iterator returns: a lot's location (lot slot 21) or a neighbour
  connection's cell. The 16–18 entry action zeroes them (the table's "pass counters reset" was these two). The bracket
  matches.
- ~~**TS45:** Which cell is saved for the state-15 resume? [the same `phase_positions`]~~
  **ANSWERED 2026-10-14** from the exe (FinishTripMap `0x0070FFD3`–`0x0070FFE7`; resume `0x00720B2C`–`0x00720B46`) and
  written into traffic-simulator §3.5. The same two s16 phase positions: FinishTripMap stores each lot's location as it
  takes it, and the resume passes them to `GetLot`. The bracket matches.
- ~~**TS46:** How is §4.16's `float(item.cost)` computed? [converted to f32 first, then × funding, r24]~~
  **ANSWERED 2026-10-14** from the exe (`GetTransitSwitchQueryData` `0x0071CC55`–`0x0071CC5B` and
  `0x0071CD6F`–`0x0071CD75`) and written into traffic-simulator §4.16, with a worked example. The int64 is loaded
  exactly (`fild`), multiplied by the f32 fraction with one rounding to 24 bits, then truncated. The bracket (f32 first,
  then r24) rounds twice and differs only for costs above 2^24 (16,777,217 at 0.75: 12,582,913 against 12,582,912).
- ~~**TS47:** A strategy above 2 in SetupPathFinderForLot. [ignored; a choice in code]~~
  **ANSWERED 2026-10-14** from the exe (`SetupPathFinderForLot` `0x00711680`–`0x00711687`, sums allocated at
  `0x0071165E`) and written into traffic-simulator §4.4. The original adds the count to `sum[strategy]` unchecked, so a
  strategy above 2 writes past the 12-byte array into the heap. Only a damaged save has one. The bracket (ignore it) is
  a *[choice]*, recommended.
- ~~**TS48:** A damaged travel type above 8. [adds nothing in ApplyRoute and fails ConfirmRoute; a choice]~~
  **ANSWERED 2026-10-14** from the exe (ApplyRoute `0x00719C2F`, `0x00719C4B`, `0x00719EE9`; type mask `0x3F` in the
  reader `0x006D6572`) and written into traffic-simulator §4.5. The original checks nothing: a type up to 63 indexes the
  9-entry network, generates-traffic and income tables and reads what follows them. ConfirmRoute reads unrelated record
  words (§4.3). The bracket (add nothing, fail ConfirmRoute) is a *[choice]*, recommended.
- ~~**TS49:** A switch ID with no switch behind it in GetTransitSwitchQueryData. [returns false; a choice]~~
  **ANSWERED 2026-10-14** from the exe (`GetTransitSwitchQueryData`: find-or-insert `0x0071CBB7`, building read
  `0x0071CC12`–`0x0071CC14`) and written into traffic-simulator §4.16. The original inserts a default switch, writes
  four of the five fields, then crashes reading the null building. Only a damaged save has such an ID. The bracket
  (return false) is a *[choice]*, recommended.

## From applying the residential, protection and save-reference answers (filed by the coordinator from the `apply8a` lane's report, 2026-10-14)

RG1–RG22, RG13 in the budget core, PR14–PR16, PG15, DQ13, SV1–SV2, LF6 and NB18 are applied or cited. RG1 creates
the buzzards through the effects manager (new switch `residential_effects`, 160 switches). The current answer is in
brackets.

- ~~**PR17:** Protection §5.1: what does `UpdateUnadjusted` do when `GetDispatchEffect` fails? [the return value is
  ignored and the record is zeroed, so the centre adds 0]~~
  **ANSWERED 2026-10-14** from the exe (`UpdateUnadjusted` `0x006F5D83`–`0x006F5DFE`; `GetDispatchEffect` fire
  `0x006AD720`, police `0x006E0CB0`) and written into protection.md §5.1 with a worked example. The AL is ignored,
  as in the bracket, but **the record is not zeroed**. The dispatch loop reuses the station loop's stack record, and
  a failed call zeroes only x, z and centre. The cone is drawn with `c = 0` and the leftover outer and radius from
  the previous dispatch or the last station: an inverted cone, 0 at the centre rising towards the old outer.
  **The bracket differs**, but only for custom stations: every shipped fire and police station has a non-zero
  dispatch centre, so with shipped data the call never fails. *[choice]*: keep one record across both loops to
  match; the zeroed record is acceptable as a known difference.
- ~~**PR18:** Protection §7.2: what is in the caller's record? [a zeroed record]~~
  **ANSWERED 2026-10-14** from the exe (`0x006F5C87`, `0x006F5DA9`; handler `0x006F6221`) and written into
  protection.md §7.2. **Never a zeroed record.** In `UpdateUnadjusted` it is the record shared by every station and
  dispatch of the rebuild: the previous dispatch's values, else the last station's, else uninitialised stack. In
  the active/inactive handler it is a fresh, uninitialised stack record. **The bracket differs** when the call
  fails, and on success when the station lacks the outer or radius property (that field keeps the leftover).
- ~~**PR19:** The older "PR14/PR15" labels in the jail vectors, `protection/mod.rs` and `algo.rs` clash with the
  QUESTIONS numbering. [left as they are]~~
  **ANSWERED 2026-10-14**: glue-only, no original behaviour; the implementer's choice stands.
- ~~**RG25:** RG1: the buzzard transform's bytes +2 and +3 are not stated. [0]~~
  **ANSWERED 2026-10-14** from the exe (`0x006FA814`–`0x006FA819`; effect `SetTransform` `0x005BE880` →
  `0x005BEC20`) and written into residential-education-health.md §4.7 step 3. The original never writes +2 and
  +3: they are uninitialised stack bytes, the alignment gap after the two flag bytes. The effect copies only byte
  +1 and the dwords +0x04 to +0x34, so it reads neither +0, +2 nor +3. **The bracket (0) is equivalent.**
- ~~**RG26:** RG10: how does a host report "cannot be read"? [NaN or a NULL callback means missing; the outputs are left
  alone]~~
  **ANSWERED 2026-10-14**: glue-only, no original behaviour; the implementer's choice stands. Written into
  residential-education-health.md §4.2. The only original "missing" is a null `GetDepartmentBudget` for an
  existing item. A missing item gives 1.0 and must not be reported as missing. A NaN is a safe sentinel, because
  a real funding fraction is never NaN.
- ~~**RG27:** RG1: does "cannot show" (a NULL callback or `FC4_EFFECT_NOT_SHOWN`) count as created? [yes: the second
  draw is made and the flag set, but there is no altitude read, start or release]~~
  **ANSWERED 2026-10-14** from the exe (`0x006FA6AA`–`0x006FA83E`) and written into
  residential-education-health.md §4.7 step 3. **Yes.** After a successful create, only the second draw and the
  flag change simulation state. The altitude read is a pure query, and `Start` and `Release` act only on the
  effect. **The bracket matches**, as long as the second draw is never skipped.
- ~~**RG28:** RG7: which ordinance effects does the shadow use before its first state 0? [it reads them as they stand]~~
  **ANSWERED 2026-10-14** from the exe (callers `0x006FED0B` and `0x006FF5CA`, RG7) and written into
  residential-education-health.md §4.3 with an example. Before its first state 0, the original uses the lists that
  `Init` built: the ordinances on in the save, or none in a new city. The shadow has no original counterpart.
  **The bracket is acceptable** while no ordinance has changed since `Init`. *[choice]*: to match always, read the
  effects once at `Init` and then only at the original's state 0.
- ~~**NB19:** NB18: does `cISC4App::GetCity` return the same primary cISC4City address the game's city global holds?
  [assumed yes]~~
  **ANSWERED 2026-10-14** from the exe (`cISC4App` slot 37 `+0x94` `0x0081F910`; `cSC4App::NewCity` `0x00454908`;
  `PartialCloseCity` `0x00454AEC`; the global is cleared at `0x00602100` from the city destructor `0x0045A468`) and
  written into neighbors.md §8.2. **Yes.** `GetCity` returns the application's city member, borrowed. NewCity
  stores in it the address the constructor gave to the global `0x00B43CEC`: the primary `cISC4City`. The two can
  differ only after the city's `Shutdown`. **The bracket matches.** The ABI row is in the re-answers9 report.
- ~~**PG16:** PG15: should the n<0 wrap quirk on a damaged save be reproduced? [no, clamped: delta −s, supply 0]~~
  **ANSWERED 2026-10-14** from the exe (`0x006E3301`–`0x006E332F`) and written into pollution.md §8.1 with a table.
  The original wraps only when a damaged save holds a supply within `|n|` of `−2^31`. It then stores the wrapped
  positive sum. Every other negative supply goes to 0, the same as the clamp. Only a damaged save reaches this, so
  the implementer's choice stands. **The bracket is acceptable**; reproducing the wrap is what a conformance test
  against the original would expect.

## From applying the power, plumbing and landfill answers (filed by the coordinator from the `apply6b` lane's report, 2026-10-14)

PW1–PW8, PS1–PS8 and LG1–LG20 are applied or confirmed, except LG5 (orientation stays 0; there is no documented route
to the view rotation) and LG16 (seagull effects are not implemented). LG1 and LG2 need an in-game check. The current
answer is in brackets.

- ~~**AO1:** Foundations §13.4 ranks 2 and 5: what AddAgent names do the two port developers and the commercial
  simulator use? [they count as unranked and keep registration order ahead of the month tail]~~
  **ANSWERED 2026-10-14** from the exe (`cSC4PortDeveloper::Init` `0x006EA950`, string `0x00A89650`;
  `cSC4CommercialSimulator::Init` `0x0069EF90`, string `0x00A89920`) and written into foundations §13.4 (tail agent
  names). Both port developers register `"cSC4PortDeveloper"` at level 6 through their shared base `Init`; the
  commercial simulator registers `"cSC4CommercialSimulator"` at levels 0, 10, 4, 6. **The bracket differs**: with the
  names known, rank them (both port developers share rank 2; their mutual order is free, *[choice]* registration
  order) instead of leaving them unranked.
- ~~**AO2:** LG6 with the game's own simulators: the month order is heap-address order, which the glue cannot change.
  [the order is logged at AddAgent, with a WARNING if landfill would run before pollution or the budget]~~
  **ANSWERED 2026-10-14** from foundations §13.1 (the original's set is ordered by target pointer) and written into
  foundations §13.4. No original behaviour can be imposed on the original simulator; the glue's rank follows its
  object's address. **The bracket stands** as the minimum. *[choice]*: allocate the glue's month target from the game's
  own heap at the original object's place in city `Init`, which reproduces the original's usual placement but cannot
  guarantee it.
- ~~**LG21:** LG5: is there a documented route to SC4ViewUtilities `+0x10`, for example a docs/abi row for
  `cISC43DRender` slot 8? [orientation 0]~~
  **ANSWERED 2026-10-14** from the exe (`SL::OnView3DInit` `0x00602590`; `0x0043FD00`; `0x007C9FB0`) and written
  into landfill.md §4.7.5 and §8. **Yes, there is a route.** The global `0x00B43DD8` is filled from the city view as
  `cISC4View3DWin` (IID `0xFA47B3F9`) slot 4 `GetRenderer`, then `cISC43DRender` slot 8 `GetViewUtilities`. Both
  are pop 0, return EAX and give a borrowed pointer. The rotation is the 32-bit integer at `+0x10` of the view
  utilities object, which is not a GZCOM object. Neither interface has a rotation getter, so that field read is
  the only route. **The bracket (orientation 0) matches only at the first rotation.** *[choice]*: glue reads the
  rotation through this route when it places the zot, and uses 0 when there is no view. ABI rows are in the
  re-answers9 report.
- ~~**LG22:** Landfill §4.8.5: is the in-memory record that SetTextureSpecification takes laid out in the saved 14-byte
  order, and what is its stride? [saved order; one record in a 16-byte buffer with zeroed padding]~~
  **ANSWERED 2026-10-14** from the exe (`SetTextureSpecification` `0x005E0B20`, `Write` `0x005DF1E0`,
  `SetEmptyCellTexture` `0x006BA470`) and written into landfill.md §4.8.5. **Yes: the saved order, with a
  16-byte stride.** The texture is at `+0x0`, x and z at `+4` and `+5`, orientation at `+6`, the configuration
  byte at `+7`, the five unnamed bytes at `+8`–`+0xC`, the frame at `+0xD`, and padding at `+0xE`–`+0xF`. The
  padding is copied into the vector but is never read or saved. The original's own record leaves it unset (stack
  garbage). **The bracket matches.**
- ~~**LG23:** LG8: a FreeCity4 save made while tiles were kept in FreeCity4 has no game tiles. [PostCityInit rebuilds
  only the cells without a tile]~~
  **ANSWERED 2026-10-14** from landfill.md §6.4 (LG8: the original saves its tiles and never rebuilds them on load)
  and written into landfill.md §6.4. **No original behaviour exists for this case**, so the implementer's choice
  stands. The original loading such a save shows no tiles until each heap next changes. **The bracket is
  acceptable.** Define the cells to rebuild as those whose rule gives a tile other than `0x02F10000` and where
  slot 33 finds no landfill occupant. On an original save that rebuilds nothing. The one exception is a cell left
  without a tile by `AddRegionToZone` (§4.4.4 step 4), which the original fills only when its heaps next change.
- ~~**LG24:** In the original, a zone that was never initialised never releases its zot at Shutdown. [we release our
  reference at the developer's Shutdown, without RemovePropA]~~
  **ANSWERED 2026-10-14** from the exe (developer `Shutdown` `0x006B94B0`, zone `FinalRelease` `0x006BB8C0`, zone
  destructor `0x006BBBC0`) and written into landfill.md §4.4.9. **The premise is incomplete.** The zone's `Shutdown`
  skips the zot, but the developer's `Shutdown` then releases the zone. Its last reference runs the destructor,
  which releases the zot without `RemovePropA` and without nulling the field. **The bracket matches.**
- ~~**LG25:** `cISCExemplarPropertyHolder` has only one docs/abi row (slot 3); slot 4 `GetDefaultExemplar` has none.
  [a reviewed slot count of 5 against 4; only slot 3 is called]~~
  **ANSWERED 2026-10-14** from the exe (vtables `0x00A8CD08` and `0x00A871A0`) and written into landfill.md §4.8.1
  and §8. **The vtable has five slots**: 0 QI thunk, 1 `AddRef` thunk (`0x005E06A0`), 2 `Release` thunk
  (`0x005E06B0`), 3 `SetDefaultExemplar` (`0x00458E60`, pop 4, void) and 4 `GetDefaultExemplar` (`0x0060CE80`,
  pop 0, EAX, borrowed). A zero dword follows at `0x00A8CD1C`, then the next vtable. **The reviewed count of 5 is
  right.** A declaration that stops before slot 4 is safe only for calling. The landfill calls only slot 3. The
  missing rows (1, 2) are in the re-answers9 report; main's interfaces.tsv already has slots 0, 3 and 4.
- ~~**LG26:** LG3: when InsertOccupant fails after Init, should the occupant be shut down? [released without Shutdown;
  the tile is kept in FreeCity4]~~
  **ANSWERED 2026-10-14** from the exe (callers `0x006BD234`/`0x006BD2A4`/`0x006BD310`/`0x006BD37E`;
  `cSC4Occupant::Shutdown` `0x005E8980`; `Release` `0x005E8DF0`) and written into landfill.md §4.8.1. **No
  Shutdown.** For a new tile the insert cannot fail, and no caller tests the result. Had it failed, the tile would
  be freed when the queued `0x99EF1140` message released it, without `Shutdown` or its `0x99EF1141`. The original
  keeps no tile list, so its later cell lookups would find no tile. **"Released without Shutdown" matches.** "Kept
  in FreeCity4" has no counterpart. *[choice]*: keep it and log the failure.

## From applying the exemplar, zones draft and traffic-network answers (filed by the coordinator from the `apply8b` lane's report, 2026-10-14)

EX21–EX33, ZQ11–ZQ12 and TN26–TN28 are applied or cited. EX29 replaces the EX8 printer (17 significant digits,
half-up, trimmed, cut at 63 characters, Float32 included); TN27 fixed a real bug (travel type −1 matched every
connection type). The current answer is in brackets.

- ~~**EX34:** `_fcvt(x, 64)`: does it round again at the 64th place when the 17 digits reach past it (below 1e-47)?
  [yes, half-up again; 9.5e-65 prints `0.` and 61 zeros after the cut]~~
  **ANSWERED 2026-10-14** from the exe (`_fcvt` `0x009F662C` → `_fpcvt` `0x009F65CF` → `_fptostr` `0x00A0264A`,
  rounding at `0x00A0268E`–`0x00A0269E`) and written into data-formats.md §5.1 ("Below 10⁻⁴⁸ the digits are
  rounded a second time"). Yes: `_fptostr` keeps 64 + decpt digits and rounds half-up again on the next one. It
  happens below 10⁻⁴⁸ (decpt ≤ −48), not below 1e-47: from 1e-48 to 1e-47 all 17 digits still fit. 9.5e-65 prints
  `0.` and 61 zeros (**the bracket matches**). The second rounding can show: 2e-49 prints `0.`, 48 zeros and `2`
  instead of `…1999999999999`, and everything below 5e-65 prints `0`. Two findings on the way, also in §5.1: a tiny
  negative value keeps its sign (−4e-65 prints `-0`, unlike −0.0), and a cut that falls among the fraction digits
  strips trailing zeros (1.0000000000000002e-47 prints 49 characters, not 63).
- ~~**EX35:** Negative exponents: §4.3 says "divided", but EX23 explains `e-32` as 1/0, then 0 × ∞ (a reciprocal and
  a multiply, two roundings). Which is it? [one divide, one rounding; the NaN comes out the same]~~
  **ANSWERED 2026-10-14** from the exe (`fld1`/`fdivrp` at `0x00910507`, `fmul` at `0x0091050B`; 10ⁿ from
  `0x0091040D`) and written into data-formats.md §4.3 (Float32). A reciprocal and a multiply: 1/10ⁿ is rounded to
  the caller's precision, then multiplied in, so there are two roundings. **The bracket differs** ("one divide, one
  rounding"): at 24 bits `9e-1` gives `0x3F666667` (one division: `0x3F666666`), `5e-2` gives `0x3D4CCCCC`
  (`0x3D4CCCCD`) and `2.5e-3` gives `0x3B23D70B` (`0x3B23D70A`); `1e-1`, `3e-1` and `1e-3` agree. The NaN of
  `e-32` is the same either way, as the bracket says. A positive exponent is one multiply.
- ~~**EX36:** A leading `-` on a NaN result (`-e-32`): does the sign flip the NaN, as `fchs` would? [yes: `0x7FC00000`]~~
  **ANSWERED 2026-10-14** from the exe (`fchs` at `0x00910526`, after the exponent step) and written into
  data-formats.md §4.3 (Exponent). Yes: the sign is applied last and flips the NaN, so `-e-32` gives `0x7FC00000`;
  `-1e-32` gives −∞. **The bracket matches.**
- ~~**EX37:** EX31 lookup cache: the original keeps a looked-up record; our store keeps no cache, so a lookup between
  two package loads sees the later package here and the cached record in the original. [no cache; the two agree
  when every package loads before the first lookup]~~
  **ANSWERED 2026-10-14** from the exe (the manager's constructor `0x009744B2` creates and registers one default
  cache, vtable `0x00AD9428`, at `0x00974544`–`0x00974574`; lookup `0x0097C36E`, insert `0x0097C595`, purge
  `0x0097C48F`) and written into data-formats.md §3 ("Cache") and §12 item 2. The original does keep every record
  that loads, with a reference, and returns it before consulting any segment; registering or removing a segment
  never touches the cache. An entry leaves only by key removal, a clear, or a purge of entries no one else holds
  *[unsettled: their callers were not traced]*. **The bracket's condition holds**: the two agree whenever every
  package is registered before the first lookup, which the shipped game does in one pass at startup, the city's
  save going to the back (a lookup during that pass was not ruled out). *[choice]* No cache stands; a store that
  registers packages after lookups must keep the first record of each key looked up.
- ~~**EX38:** EX31 late factory: may a factory added after packages are loaded be refused instead of applied to the
  earlier packages? [refused; `add_factory` returns false]~~
  **ANSWERED 2026-10-14** from the exe (the loader, slot 46 `0x00972EAE`, picks the factory by type at every
  uncached request from `GetResource` `0x009732EB`) and written into data-formats.md §3 ("A factory added late").
  The original never refuses a factory: a late one serves every later request for its type, records in earlier
  segments included. **The bracket differs from the original** but stands as *[choice]* on one condition: every
  factory is added before the first package is loaded (condition 3 of §3 is then vacuous), which holds when
  FreeCity4 registers all its factories at startup. A refused factory must be logged as an error.
- ~~**EX39:** EX33 scalar: the `props` vector readers give a scalar's one value where the original gives an empty
  vector. This matters for the shipped `Uint8:0:{}` values of `0x88EDC795` and `0x88EDC796`, which no core reader
  reads yet. [kept as one value; the doc says such a reader must treat it as empty]~~
  **ANSWERED 2026-10-14** from the exe (vector helpers `vec<u8>` `0x005FFB40` and `vec<s64>` `0x005FFD80`; readers
  `0x0047A0EC`, `0x0047A16C`, `0x0047A20C`, `0x0060F690`, `0x0060F710`, `0x0069112B`, `0x0069131A`, `0x00693899`)
  and a sweep of the shipped `.dat` files, written into data-formats.md §5.2 ("Shipped scalars that only a vector
  helper reads"). **The bracket differs.** The scalar is not limited to the Living Mall text: 172 binary lot
  configurations store `0x88EDC796` as a Uint8 scalar 0 (161 of them `0x88EDC795` too, 10 also `0x88EDC793`), and
  two landfill tiles store the budget cost `0xEA54D286` as a Sint64 scalar, which the budget simulator reads with
  `vec<s64>`. The original reads every one of them as empty. A vector reader must return an empty vector for a
  scalar; only the converting getters keep the value (the landfill cost reads 1 and 10 that way).
- ~~**ZQ13:** Does a real-mode Place empty the draft cache even when the Place fails, and before the solve? [yes, at
  the start of every real-mode Place; no vector depends on it]~~
  **ANSWERED 2026-10-14** from the exe (`PlaceNetworkOccupants` `0x00629210`: real-mode test and
  `cSC4NetworkWorldCache::Clear` `0x00647B20` at `0x00629327`–`0x00629334`) and written into zones.md (the draft
  cache's "Lifetime"). Yes: right after the bounds setup and before any solve, every real-mode Place empties the
  cache, and nothing before that point can return early, so a Place that then fails has emptied it too. **The
  bracket matches.**

## From the development glue and shadow (filed by the coordinator from the `devglue` lane's report, 2026-10-14)

The building development simulator and the growth developers are replaced as one unit (`enable = development`;
building-development.md §9, growth-developer.md §12), with a read-only `development_shadow`. Each question is marked
in the code; the implemented answer is in brackets.

- ~~**DG1:** The return channel of `AddToDemandValue`. [bool in AL; no x87]~~
  **ANSWERED 2026-10-14** from the exe (`cSC4Demand::AddToDemandValue` `0x005D4C70`: load, add, store-and-pop, AL = 1,
  `ret 4`) and written into growth-developer §5.3, §11 and §13.1 (quirk 11 withdrawn). It adds the amount to the
  node's float32 demand value and returns true in AL; the x87 stack is left as found. **The bracket matches**; the spec's
  old "ST0, never popped" was wrong (docs/abi already says `eax-or-void`). Bears on the run 102 record: the same
  section now records that the original's constructor never initialises total capacity (`+0x108`), so the saved
  `capacity=1036996237` (`0x3DCF4E8D`, a float bit pattern) is the original's own garbage, not a misread; zone, purpose,
  wealth and source are tuning set by `Init` after `Read`, and five grid references of six is the record's shape
  (growth-developer §2, §9). *[choice]* total capacity 0 for a new city.
- ~~**DG2:** The timer that slices state 18. [a real timer, QueryPerformanceCounter]~~
  **ANSWERED 2026-10-14** from the exe (adaptive helper `0x0068CC20`/`0x0068BF50`, growth developer step `0x00B49098`)
  and written into growth-developer §5.2. The original reads the processor's time-stamp counter and converts the
  budget with the simulator's `MillisecsToCycles` (simulator §5.5). **The bracket matches in effect**: any monotonic
  real-time clock gives the same behaviour, since slicing changes only how much of state 18 runs per call.
- ~~**DG3:** The layout behind `GetLotConfigurationMap`. [lot configuration manager slot 9 `GetLotConfigurationsByFilter`
  with an accept-all filter, into a B hash_set]~~
  **ANSWERED 2026-10-14** from the exe (slot 20 `0x00489720`, map `0x00B216C8`, begin `0x004B0390`; slot 9
  `0x0048B1A0`, null-filter path `0x0048B2E7`) and written into building-development §4.7.4 (Inputs). Slot 20 returns
  the manager's own STLport `hash_map` (allocators.md hash_set shape, 12-byte nodes `{next, key, config*}`), with no
  reference. **The bracket matches**: slot 9 with a null (or accept-all) filter gives the same configurations, each
  AddRef'd into a B `hash_set`; the caller must release each one. Order does not matter (the table keeps maxima).
- ~~**DG4:** The budget item cost at +0x38. [0]~~
  **ANSWERED 2026-10-14** from the exe (`LoadBuildingProperties` `0x0069BE00`–`0x0069BE3F`; profile readers after
  BDS slots 39/40 and occupant slot 11 scanned) and written into building-development §4.7.1. The original stores the
  sum of the `cost` fields of the exemplar's budget items (budget simulator slot 50), e.g. 120 + 80 = 200. **The
  bracket differs**, but no reader of `+0x38` was found in the exe *[inferred]*, so 0 is not observable by the game;
  *[choice]*: fill the sum when the budget simulator is reachable, else 0.
- ~~**DG5:** How a building occupant's properties are loaded. [a GZCOM occupant, `SetBuildingType`, `GetResource`, QI
  `0x0AC2B5F7` and `SetDefaultExemplar`; a failure gives false]~~
  **ANSWERED 2026-10-14** from the exe (slot 7 `0x00698DE0`, loader `0x00698490`, constructor `0x00458E90`, GZCOM
  factory `0x00458F40`, QI chain `0x004587D0` → `0x005E89C0` → `0x005FD310`) and written into building-development
  §4.9.1. The original creates the building with the class's own allocator and constructor (count 0), AddRefs, then
  `SetBuildingType`, the loader, `GetBoundingBox` / orientation swap / `SetBoundingBox`, the altitude, `SetPosition`,
  `SetOrientation`, `Init`, the insert and one Release. The loader QIs `0x0AC2B5F7` on the `cISC4Occupant`, falls back
  to `GetBuildingType` for type 0, looks the type up in the key map, `GetResource(key, 0xA52160F5)`,
  `SetDefaultExemplar`, then sets the box to `(0,0,0)`–occupant size `0x27812810`; **it returns true on every path**.
  The bracket differs in two points: a failed load does not give false (the building is still inserted, with an empty
  box), and the box from `0x27812810` is part of the load. QI `0x0AC2B5F7` succeeds for buildings because their QI
  chain ends in the property-holder base, which the landfill occupant's does not (in-game: 154 of 154 succeeded).
  Bears on the five references on `0xA9BD882D`: `CreateInstance(0xA9BD882D)` already returns count 1 (the director's
  QI AddRefs), so an AddRef copied from the BDS's `new` path leaves one reference per building. *[choice]*: no extra
  AddRef after GZCOM creation; keep the original's always-true load.
- ~~**DG6:** BDS slots 23, 26 and 29. [false, logged once]~~
  **ANSWERED 2026-10-14** from the exe (slot 23 `0x00695AF0`, slot 26 `0x006982B0`, slot 29 `0x006983C0`, cell helper
  `0x00695610`; callers: the lot developer's foundation code and the wall factory `0x005E3122`) and written into
  building-development §4.9.2. **The bracket differs**: the original implements all three, and the original lot
  developer and wall code depend on them; false gives no foundations and no retaining walls. Slot 26/29 set the
  default exemplar from the foundation (`0x11`) / wall (`0x15`) key maps (26 also reads maximum height `0x88FCC3E3`);
  slot 23 builds the foundation occupant (size from the lot developer's highest/lowest cell altitudes, height ≥ 0.5)
  and inserts it. *[choice]* implement them as specified, or forward to an original BDS; never return false for valid
  input.
- ~~**DG7:** The static cleanup through app slot 69. [not registered, logged]~~
  **ANSWERED 2026-10-14** from the exe (callback `0x0069B850`, registration in `Init`, app slots 69/70) and written
  into building-development §1.3 step 1. The callback runs only at application shutdown: it clears the static flag,
  unregisters itself and empties the nine catalogue containers. No in-session behaviour depends on it, so **the
  bracket stands** (not registering is unobservable while the process lives).
- ~~**DG8:** "Declined by stage" in the month line. [counted by wealth only]~~
  **ANSWERED 2026-10-14**: glue-only (a log line). No original behaviour; the implementer's choice stands. (The
  original counts nothing by stage; `ExecuteDeclinePlan`, building-development §4.5, knows each item's developer and
  so its wealth.)
- ~~**DG9:** When the decline plan line is written. [at BDS state 4 → 0]~~
  **ANSWERED 2026-10-14**: glue-only (a log line). No original behaviour; the implementer's choice stands. State 4 → 0
  is where the original finishes executing a plan (building-development §4.2), so the choice is consistent.
- ~~**DG10:** The `cSC4LotConfigurationObject` layout. [the SDK header]~~
  **ANSWERED 2026-10-14** from the exe (`0x00477B10` stride `0x34`, type byte at `+0`; reads at `0x0069C43E`,
  `0x0069C6D0`–`0x0069C71E`) and written into building-development §4.7.4 (Inputs). **The bracket matches**: the
  SDK's `cSC4LotConfigurationObject` is the exact `0x34`-byte layout (rotation `+0x02`, x `+0x04`, z `+0x0C`, ID
  vector `+0x28`). Slot 24 returns the first object of type 0 (building), or null.
- ~~**DG11:** An empty capacity vector against a present-but-empty one. [`n_capacities > 0` gives one map]~~
  **ANSWERED 2026-10-14** from the exe (`0x0069BBB7` property test, pair loop `0x0069BC10`–`0x0069BC44`, push
  `0x0069BC46`–`0x0069BC7E`) and written into building-development §4.7.1. **The bracket differs**: a present
  `0x27812834` always pushes one map, even with no values (an empty map); only a missing property gives no map. The
  difference reaches the lot's copied vector (`cSC4Lot::SetState`) but no capacity sum. *[choice]*: follow the
  original (present → one map); ignore an unpaired last value.
- ~~**DG12:** A zero profile inserted in advance. [replaced in place]~~
  **ANSWERED 2026-10-14** from the exe (`AddBuildingData` passes true at `0x0069D0F2`; erase only for false,
  `0x0069BF29`) and already in docs/abi/layouts.md (Building profile). **The bracket matches**: a zero profile
  inserted earlier by `GetBuildingProfile(unknown)` is assigned over in place, so pointers to it stay valid.
- ~~**DG13:** The cheat notification ID. [the cheat ID]~~
  **ANSWERED 2026-10-14** from the exe (`Init` `0x0069D5E2`–`0x0069D649` and the third pair) and written into
  building-development §1.3 step 10. **The bracket matches**: each cheat is registered (slot 3) and subscribed with
  `AddNotification2(target, id)` using its own ID; the delivered message is `0x230E27AC` with data1 = that ID.
- ~~**DG14:** The name fallback. [GameString, otherwise cRZBaseString]~~
  **ANSWERED 2026-10-14** from the exe (`LoadBuildingProperties` `0x0069BA55`–`0x0069BAD1`; `SC4LoadExemplarString`
  `0x00603220`) and written into building-development §4.7.1. Order: localized LTEXT via `0x8A416A99`, else property
  `0x899AFBAD`, else "". The original stores both names as `cRZString` values in its name maps and returns borrowed
  `cIGZString*`; the string class is not observable through the interface, so **the bracket stands** as long as the
  content follows that order and the object outlives the call.
- ~~**DG15:** The lot slope read. [slot 19]~~
  **ANSWERED 2026-10-14** from the exe (`0x0069879C` lot `+0x4C`, `0x006987AA` lot developer `+0x78`) and written
  into building-development §4.11. `lot.GetSlope()` is lot slot 19 (ST0); `GetLotSlope(lot, false)` is lot developer
  slot 30 (pop 8, ST0). **The bracket matches.**
- ~~**DG16:** The advice subject. [not created]~~
  **ANSWERED 2026-10-14** from the exe (`DoBuildingChanged` `0x0069CA80`) and written into building-development §4.10.
  The original creates and fills a subject, then pushes the event with a null subject and drops it. Not creating it is
  unobservable, so **the bracket stands**, provided `highestStage` is updated and the event is still pushed with null.
- ~~**DG17:** The building type filter's slot 3. [bool(uint32)]~~
  **ANSWERED 2026-10-14** from the exe (`0x00699C8F`–`0x00699C97`: one push, AL tested) and written into
  building-development §4.7.2. **The bracket matches**: `bool(uint32 type)`, pop 4, AL. The exe has no filter class and
  no caller of BDS slot 17 *[inferred]*, so only plug-ins use it.
- ~~**DG18:** The `GrowthItem` stage width. [masked to uint8]~~
  **ANSWERED 2026-10-14** from the exe (growth developer writes the whole `int32` at `0x006B6FA8`; `Grow` reads the
  low byte at `0x0069B349`) and written into building-development §4.4. **The bracket matches**: the tract developer
  receives the low byte; shipped stages are 1..8.
- ~~**DG19:** A zone type out of bounds. [0]~~
  **ANSWERED 2026-10-14** from the exe (`GetMaxCapacity` `0x00696C20`: map lookup, 0 when absent; zone loop bound
  `0x0069C975`) and written into building-development §4.7.4. **The bracket matches**: a zone outside 0..15 is never a
  key of the table, so the lookup misses and the result is 0.0; `GrowthStageFitsInZone` / `BuildingFitsInZone` give
  false for unknown zones (§4.7.3). No original path indexes an array by zone.
- ~~**DG20:** Lot configuration zones: 0..15, where `fc4.h` allows 0..16. [0..15]~~
  **ANSWERED 2026-10-14** from the exe (`0x0069C974`–`0x0069C978`: `z < 16`) and written into building-development
  §4.7.4. **The bracket matches**: the BDS asks `IsCompatibleWithZoneType` for zones 0..15 only. A wider range in
  `fc4.h` is harmless as long as the table loop stops at 15.
- ~~**DG21:** An off-thread message arriving without the lock. [dropped]~~
  **ANSWERED 2026-10-14**: glue-only. No original behaviour (the original's getters take no lock); the implementer's
  choice stands.
- ~~**DG22:** ABI numbering after a merge. [renumbered by the coordinator: ABI 35]~~
  **ANSWERED 2026-10-14**: coordinator bookkeeping, not a behaviour question. No original behaviour; the coordinator's
  numbering stands.
- ~~**DG23:** The `wall_clock_seed` for the `0xFFFFFFFF` key seed. [the development session seed]~~
  **ANSWERED 2026-10-14** from foundations §6.1 (`Seed` `0x00913667`) and growth-developer §7. The original derives
  such a seed from `time()` and `clock()` (non-deterministic, probability 2⁻³² per tract and update). No original
  behaviour can be matched; **the implementer's choice stands** (a session-derived seed keeps runs repeatable).
- ~~**DG24:** A null property holder. [false]~~
  **ANSWERED 2026-10-14** from the exe (`Init` `0x006B85EF`–`0x006B85F8`: AddRef on the holder without a null test) and
  already in growth-developer §1.2 step 3. The original crashes on a null holder; **the bracket matches the spec's
  *[choice]*** (false).
- ~~**DG25:** The shadow's `building_purpose`. [slot 40, no insert]~~
  **ANSWERED 2026-10-14** from building-development §4.7.2 (slots 39/40). The original's own readers use slot 39, which
  inserts a zero profile for an unknown type; slot 40 returns null instead, and a null read as purpose 0 equals the
  zero profile's purpose. **The bracket stands**: same value, and the read-only shadow does not grow the catalogue.
- ~~**DG26:** Growth developer slots 10 and 28. [the full EAX, 0 or 1]~~
  **ANSWERED 2026-10-14** from the exe (`0x006B7530`, `0x006B75D0`: EAX zeroed, then set from the compare) and already in
  growth-developer §10.1 ("EAX 0/1"). **The bracket matches**: slot 10 = `state < 17` (signed), slot 28 = `state = 0`,
  both as the full EAX.
- ~~**DG27:** The off-thread fallback for slot 30. [−4 / 20]~~
  **ANSWERED 2026-10-14**: glue-only. No original behaviour; the implementer's choice stands. −4 / 20 are the values the
  original's slot 30 gives for an index outside its vectors (growth-developer §10.1), so the fallback is consistent.

## From the apply6c follow-up (landfill game tiles; filed by the coordinator, 2026-10-14)

Regress r6b (run 100): every LG3 game tile failed to insert (`gameTiles=0 gameTileFailures=12`). apply6c logs each
step and adds a fallback. The current answer is in brackets.

- ~~**LG27:** Does the landfill occupant's QueryInterface (`0x00477130`) answer `0x0AC2B5F7`? If not, which object
  does: the sub-object at +0x20, or the occupant's property holder? [QI on the occupant, then on its property holder]~~
  **ANSWERED 2026-10-14** from the exe (QueryInterface `0x00477130` and its thunks `0x00477210`, `0x00477230`,
  `0x00477240`, `0x00477260`; `CreateLandfillOccupant` `0x006BC845`–`0x006BC84C`; `cSC4Occupant` constructor
  `0x005E9220`) and written into landfill §4.8.1. No, and no other object of the tile does: every sub-object's QI is a
  thunk to `0x00477130`, which answers only `0x2A15F262`, `1`, `0x26BB767E` and `0xE4FDA3D4`. The original never asks.
  It calls `SetDefaultExemplar` (slot 3) directly on the sub-object at +0x20, after the constructor and before `Init`
  and the insert. The bracket cannot work, as run 103 shows. *[choice]* Use the holder at `cISC4LandfillOccupant*` +
  0x20 (= `AsPropertyHolder()` + 0x14), or the layout-free `Read` route in §4.8.1.
- ~~**LG28:** Does the GZCOM default constructor `0x004774C0` set type `0x6A11BD2B`, as the tile constructor
  `0x00477270` does? If not, does InsertOccupant accept the type it leaves? [left as the constructor sets it; the
  type is logged]~~
  **ANSWERED 2026-10-14** from the exe (default constructor `0x004774C0`, type stored at `0x00477553`; `cSC4Occupant`
  constructor flag byte at `0x005E9265`) and written into landfill §2.3 and §4.8.1. Yes: it sets type `0x6A11BD2B` like
  the tile constructor. Flag 4 is set by the base constructor for every occupant, so the two constructors differ only in
  tile and orientation (the spec's "flag 4 not set" was wrong and is corrected). InsertOccupant never reads the type
  (LG29). The bracket matches.
- ~~**LG29:** When does InsertOccupant (`0x0049EF60`) return false: already inserted, not initialised, its box, or its
  type? [each failure is logged with those values]~~
  **ANSWERED 2026-10-14** from the exe (`InsertOccupant` `0x0049EF60`: flag test `0x0049EF8A`, holder test `0x0049EFBB`,
  first-cell duplicate scan `0x0049F158`–`0x0049F182`) and written into landfill §4.8.1. Only for a null occupant, an
  occupant whose flag 1 ("inserted") is already set, a null `AsPropertyHolder()` (never for game occupants), or an
  occupant already listed in its first cell. Never for the type, the initialised state (the insert calls `Init` itself)
  or the box. Logging these is the implementer's choice; the bracket stands.

## From the commercial, industrial, civic, park, history and clock specs (re-specs9 lane, 2026-10-14)

Open points from commercial-industrial.md, civic-buildings.md, parks.md, history.md and clock24.md. The current
answer is in brackets; each entry says what would settle it.

- ~~**CM1:** AO1, the commercial part: which AddAgent name does the commercial simulator use, and does it count at the
  month level? [`cSC4CommercialSimulator`. It registers a one-shot, an idle, a day and a month agent; the month agent
  ignores its message but still takes a place in the month order (commercial-industrial.md §3). AO1 and
  foundations §13.4 can take this name. The two port developers are still open]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (commercial-industrial.md §3, foundations §13.4). The name is `cSC4CommercialSimulator`, and it is in the §13.4 tail table with levels 0, 10, 4, 6. The port developers' name is settled too (AO1, AO3: both use `cSC4PortDeveloper`).
- ~~**CM2:** If FreeCity4 ever corrects RG23 (the proximity seed covers only the z = 0 edge), the CO$$ and CO$$$ maps
  stop being constant and offices start to affect growth. Should the correction be offered? [no: compatibility
  keeps the defect. A correction would be a separate *[choice]* behind an option, off by default. This is a
  coordinator decision, not an RE question]~~ **ANSWERED 2026-10-15 (re-answers11):** Not an RE question: the original's behaviour is the defect, and RE has nothing more to add. The bracket stands as the coordinator's policy (compatibility keeps it; any correction is an off-by-default *[choice]*).
- **CM3:** The demand nodes `0x3000` and `0x3800` are held but never read, yet `Init` fails without them. Can the
  shipped game reach `Init` without them? [no case found: the demand simulator creates them first, and FreeCity4
  keeps the failure path. A load test with a city whose demand record is missing would settle it]
- ~~**IN1:** Does anything call the industrial simulator beyond the city's lifecycle calls? [no other reference to its
  interface or class ID was found, so the implementation is a stub with a version record. A runtime call trace on
  its vtable would settle it]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES as far as static analysis reaches. The interface IID `0x477A9E7D` and class ID `0xA990C022` occur only in `cSC4City::CreateCityComponents` (`0x0045AE5E`), `cSC4City::Read` (`0x0045CF15`), the class's own `QueryInterface`/`GetGZCLSID` (`0x006B8975`, `0x006B8961`) and the class table (`0x00B08E78`); the vtable `0x00AB1530` is referenced only by its constructor and destructor (`0x006B8A4A`, `0x006B8AF0`). A caller through the city getter (slot 60) cannot be ruled out statically; a runtime trace would.
- **CB1:** Lot placement and exclusion IDs. The civic simulator has no preview-occupant test, so a preview occupant
  of a building with `0xEA2E078B` adds that ID and then erases it. If the preview's removal came after the real
  building's insertion, the ID would be lost and the building could be placed twice. [in the shipped game a placed
  reward building is disabled in the menu, so either the preview's removal comes first or the preview is never
  inserted. FreeCity4 must keep the original message order. An in-game check (place the Mayor's House, look at the
  menu) with a message log would settle it]
- **CB2:** A lot that returns to state 3 (for example after abandonment) is charged a second time and listed in
  `maintained` a second time. Does the shipped game send state 3 twice for a civic lot? [reproduced as is. The
  corpus holds 7 `maintained` entries in 5 cities and no duplicate. A lot-state message log for an abandoned and
  restored civic building would settle it]
- ~~**CB3:** Status bytes 4, 5 and 6 occur in the corpus (15, 8 and 32 entries). The SDK calls bit 4 "advisor event
  data set". Who sets it, and does anything read it back? [carried through `Read` and `Write` unchanged; the
  advisors write through `GetConditionalBuildingStatus`. A trace of the writers of the status byte would settle it]~~ **ANSWERED 2026-10-15 (re-answers11):** Answered (civic-buildings.md §2). Bit 4 is the reward advice's "already announced" mark: advisors.md §6.6 sets status 6 when the condition first returns 1 and fires only while bit 4 is clear. Later results keep it (a string gives 4, number 0 gives 5), which accounts for the corpus values 4, 5 and 6. The advisor reads it back; nothing else does.
- **CB4:** `buildingCount` is a u16 that wraps. A removal without a matching insert gives 65,535: for example a
  building inserted before its status entry existed, or removed before activation. [reproduce the wrap. Counts
  above 0 occur in only 13 entries of 5 cities, and none has wrapped. A save that loads with a building but
  without its status entry would show it]
- ~~**CB5:** The income line is keyed by the building type, not by the item's line. The line that
  `CreateLineItemForBuildingType` returns is used without a null check. [keep the keying. *[choice]* FreeCity4
  skips the income when creation fails, which the shipped budget never does for a new type]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (choice harmless). `CreateLineItem` always creates and stores a new item and returns it (budget.md, the department budget methods), so the null case does not arise in the original apart from an allocation failure.
- ~~**PK1:** The shipped distress threshold 25.0 is compared with a fraction. So the first monthly pass of each session
  marks every park lot run down (condition 2), and the park manager never restores it. Yet in the corpus every park
  lot is at condition 0 (one at 1). What resets the condition? [the lot developer's condition test
  (lot-developer.md §5.5) *[inferred]*. An in-game check would settle it: play one month with a park, then save,
  or watch the lot condition in the query tool]~~ **ANSWERED 2026-10-15 (re-answers12):** Answered: there is no quirk (parks.md §4.4, PK10). The threshold is read with the percent getter (`0x005FDAC0`, × 0.01f), so the shipped 25.0 becomes 0.25 and parks are marked run down only at 25 % funding or less. The corpus's condition 0 needs no other reset.
- ~~**PK2:** The zoo escape runs only when the park list is not empty. A park budget item that names a missing
  department crashes the original. [the shipped zoo is in both groups, so the first point cannot show in the
  shipped game. *[choice]* FreeCity4 skips the charge for a missing department (parks.md §4.2)]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (choice confirmed). Shipped data cannot reach the crash; skipping the charge is a safe *[choice]*.
- ~~**PK3:** Park line items are never removed. A removal message releases the occupant once even when it was not
  listed. [reproduced; an empty line stays in the budget panel at 0 and 0. A budget-panel check after demolishing
  every park of one type would confirm the visible effect]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (reproduced from the exe, parks.md §4). What remains is a visual check of the budget panel, not a behaviour question.
- ~~**HW1:** The yearly thinning keeps the last point of every `Dispose` range and leaves the last group of every
  `Coalesce` range open. Is that an off-by-one in the original, or intended? [reproduced as is; Konradshohe's 296
  points match this model exactly]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES. The intent cannot be recovered from the code; the behaviour is as reproduced, and the 296-point corpus match confirms it. Keep it as *[compat]*.
- **HW2:** `GetPoint` at an interior point returns the regression value through three points (a smoothing), and
  `GetCount` counts the widened range. Do the graph windows depend on this? [reproduced; the graph and Lua callers
  are not traced yet. A trace of the graph window's calls would settle it]
- **HW3:** The regression sums are `double` variables, but they follow the caller's x87 precision: 24-bit from the
  simulators, 53-bit inside Lua calls (foundations §18). [FreeCity4 evaluates them per caller. A differential test
  through a Lua advisor call would confirm the 53-bit path]
- ~~**HW4:** Version 1 records turn `maxCount = 100` into 0, duplicate dates overwrite the last point, and nothing in the
  executable posts `0x89EFA537`. Why `100 → 0`, and does any plugin post `0x89EFA537`? [reproduced; the message stays
  handled for plugins. The city getter is settled: slot 32 (`+0x80`) returns the field that `CreateCityComponents`
  fills with the warehouse (history.md §1); docs/abi still shows a folded name for it]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (history.md §3). The four bytes of `0x89EFA537` occur only in the warehouse's `Init` (`0x00476726`), `Shutdown` (`0x00475900`) and `DoMessage` (`0x004770BE`), so nothing in the executable posts it. The reason for `100 → 0` is not recoverable from the code.
- **CK1:** Does the clock run while the player pauses the simulation (pause button or speed 0)? [yes *[inferred]*:
  only hidden pauses (through `SC4GamePause` and the scrubber) stop it, and nothing links the simulator's `Pause()`
  or speed to the clock. An in-game check would settle it: pause, then watch the clock display and the lighting]
- **CK2:** Is the clock timer's unit 3 microseconds? [yes *[inferred]* from the `1e-6` factor, which gives a
  640-second day at magnification 135. The unit factors from `0x00B54360` onwards are calibrated at runtime.
  Reading that table in a running game, or timing one game day, would settle it]
- ~~**CK3:** What is the object at `+0x8C` that `Shutdown` hands the string at `+0x78` to? [dead: nothing in the clock
  sets it, so FreeCity4 omits both fields. A write breakpoint on `+0x8C` would settle it]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (dead; clock24.md §2). The clock's only writes to `+0x8C` are the constructor's 0 (`0x00445A15`) and `Shutdown`'s clear (`0x0044516B`), and no accessor hands it out.
- ~~**CK4:** A saved `pauseCount > 0` leaves the clock stopped after the load, and only `Stopwatch resume` restarts it.
  [reproduced; the corpus has no such save]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (reproduced from the exe; the corpus has no such save, so there is nothing more to settle).
- ~~**CK5:** Each step loses the fraction of a game second left after truncation, so the day is up to about 1.6 %
  longer than 86,400 / magnification real seconds, depending on the idle cadence. [FreeCity4 reproduces the formula. The
  visible day length follows FreeCity4's own idle cadence, which conformance does not test]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (formula reproduced; the visible day length is FreeCity4's own cadence and outside conformance).
- ~~**CK6:** The SDK header declares `GetTimePercentage` and `GetVisualTimePercentage` as `float (void)`, but the game
  takes an out pointer and pops 4 bytes. `GetEventHour` takes a dword index, not a `bool`. [the game's form is
  binding (clock24.md §8), and docs/abi should note it. Plugins built on the SDK declaration would corrupt the
  stack in the original too]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (clock24.md §8). Re-checked: `GetTimePercentage` and `GetVisualTimePercentage` end with `ret 4` (`0x0044543E`, `0x0044546E`), and `GetEventHour` tests the whole dword (`0x00445024`, `0x0044502E`). The docs/abi note is the coordinator's to add.

## From the power shadow under the turbo clock (filed by the coordinator from the `powerturbo` lane's report, 2026-10-14)

The power shadow's bookkeeping lines were exact at normal speed but not under `sim_turbo` (regress gsT1: 306 of 352).
The cause was the shadow's classifier, not the power core: building messages still in flight at a month line, which
turbo makes more frequent. The lane fixed it in the shadow (exact under turbo and at normal speed) and implemented
the brackets below, marked *[choice]* in code.

- ~~**PW9:** power §5.2 takes the first bookkeeping line as the baseline of differences inherited from the save, but
  that line can have building messages in flight. Should the spec say how the baseline is chosen? [the first line
  equal to its predecessor; never edited afterwards; an entry that reads 0 is listed as absent and counts as
  inherited again when it returns; a non-inherited difference is a mismatch only when it is unchanged at a line
  where nothing else changed, or unchanged across three lines]~~ **ANSWERED 2026-10-15 (re-answers10):** glue, accepted, with a better rule now in the spec. A posted occupant message reaches every subscriber in the same `MessageSend`, so a shadow subscribed to `0x99EF1142`/`0x99EF1143` has seen exactly the messages power has handled. The occupants in flight are those whose listing in the occupant manager differs from the delivered messages. Recommended: build the expected counts from the delivered messages, which removes case 2. Otherwise take the baseline at the first line with nothing in flight. The bracket's heuristic is accepted. power §5.2.
- ~~**PW10:** given §5.3 (PW5: a partial rebuild leaves bits set outside the rebuilt area, and no insertion filters
  run during a rebuild), should the bookkeeping line compare wire bits at all? [listed as "wire bits, not counted";
  kept out of the baseline and the mismatch count. The cycle lines still compare every rebuild's wire bits exactly.
  No counted check then covers the wire bits that §5.1 insertion sets directly]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES (accepted): the original's wire bits depend on history. Insertion sets them with filters; a rebuild clears and re-wires only the dirty rect, without filters and with no margin for zoned cells; bits outside a rebuilt rect only accumulate. So no counted comparison against a rebuild is exact. Only a mirror that replays the same sequence covers the §5.1 bits. power §5.2.
- ~~**PW11:** under turbo the baseline can settle late (the 9th line in one run), so a real difference that appears
  earlier is taken as inherited; and a frame that opens two month days keeps the first day's messages in flight at
  the second, which then looks settled. Is a count of queued occupant messages (from the message server) a
  documented, reliable signal that the shadow could use instead of this guesswork? [not used]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES (not used). The queue count exists: `cIGZMessageServer2` slot 10 (`0x0092D0F4`) returns the number of queued messages, excluding the one being delivered. But it counts every type and is never 0 at a month line, because the day's SimNewDay/SimNewMonth are queued in the same tick. The reliable signal is the delivered occupant-message stream (PW9). power §5.2.

## From applying the graphics-format answers (filed by the coordinator from the `apply9c` lane's report, 2026-10-14)

- ~~**FS15:** FS4: a later `0x7B` frame that has its own valid 256 × 1 palette. Does the original use that palette,
  frame 0's, or upload grey? §2.6 covers only a frame with no usable palette. [its own palette; a palette that is
  missing, the wrong size, outside the record, or behind a chain that leaves the record counts as "no usable
  palette" → grey, and the record is still accepted]~~ **ANSWERED 2026-10-15 (re-answers10):** **DIFFERS** (accepted as a choice): the original applies no palette to any frame. `SetCurrentFrameByIndex` stores each frame's own palette (`+0x10`), but its only reader (`0x00808DE0`) is called once, by the frame-0 check in `SerialRead` (`0x008008FA`). Every `0x7B` frame uploads its indices as luminance (grey). The bracket (own palette, else grey; record accepted) equals that for the shipped grey ramps. graphics-formats §2.6.
- ~~**FS16:** FS12: the original's `towlower` works on UTF-16 code units, so it probably does not lower characters
  outside the BMP. Should FreeCity4 match that? [no: every character is lowered by Unicode simple case mapping, as
  §3.11 says]~~ **ANSWERED 2026-10-15 (re-answers10):** **DIFFERS** for non-BMP or invalid names only; the bracket is accepted. The original's UTF-8 decoder (`0x00910E18`) makes no surrogates: a 4-byte character becomes one unit, its code point mod 0x10000 (U+1F600 → U+F600), which `towlower` may lower and which is written back. The decoder also stops at the first invalid sequence, cutting the name (`Café` → `caf`). Shipped names are all ASCII. graphics-formats §3.11.
- ~~**FS17:** §2.13 does not record the 15 shipped frames, in multi-frame DXT1 records, whose own A field would give the
  other DXT1 format (they report frame 0's format since FS5; the pixels are the same). Should it? [no code depends
  on it]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES (no code depends on it); the count is recorded in the §2.13 survey table as the lane's measurement (15 frames, all in multi-frame records, given frame 0's format). graphics-formats §2.13.

## From the development glue fixes (filed by the coordinator from the `devglue2` lane's report, 2026-10-14)

devglue2 made the development replacement work in game: the catalogue loads 744/744 types and 281 families (a
building's own property list holds only its own properties; most building data sits in parent cohorts), the
shadow is exact (305/305 lines), and the replacement grows like the original at equal pacing without going
bankrupt. QI `0x0AC2B5F7` on new building occupants works (154 created, 0 failures), unlike the landfill tile (LG27).

- ~~**DG28:** the growth developer's special-building copy is released last, in its destructor; in an unsaved city
  that release is the last reference (the five warnings on class `0xA9BD882D` in run 102). Does the original also
  free those occupants there, after the load's object table has dropped its references? [yes, assumed; the release
  is marked expected (`FC4_RELEASE_AT_DESTRUCTION`), and still warns if the reference was never taken] *Lead from re-answers9 (DG5):* `CreateInstance(0xA9BD882D)` already returns a count of 1, so an extra AddRef after it leaves one reference too many per building; the five warnings may come from that, not from destructor timing. *Tested by apply10a (2026-10-15): the lead is disproved.* Our BDS takes no extra AddRef after `CreateInstance(0xA9BD882D)`, yet with `FC4_RELEASE_AT_DESTRUCTION` removed run `a10a-after-dev` gave the five warnings on class `0xA9BD882D` again, all from the growth developer's destructor releasing the special-building copy (§1.3) at shutdown. Still open: does the original's growth developer destructor drop those occupants' last references in an unsaved city? [assumed yes; the workaround stays, with a comment citing that run]~~ **ANSWERED 2026-10-15 (re-answers10):** **DIFFERS**: the city's `Shutdown` (`0x0045B1D0`) shuts down and releases the BDS (`+0x164`) before the occupant manager (`+0x12C`). The BDS releases each developer (destroying it, so its copy is released) before its own special list. Both happen while the occupant manager still holds every building in the city, so the copy is the last reference only for a special building that left the city since the last state-14 copy. Our last release means our developer outlives the original's order, or a reference is missing. An extra `AddRef` (the DG5 lead) would leak instead, which agrees with apply10a's run: the warnings point to the release order or a missing reference. building-development §1.4.
- ~~**DG29:** data-formats §4.4 says `EnumProperties` gives the effective property list, but in game it lists only a
  holder's own properties. Is there an effective enumerator, and what does the original's lookup do with cohorts?
  [the glue walks the parent cohort chain itself (`EffectiveProperties`), with cycle detection]~~ **Answered (re-answers9, data-formats §4.4):** `EnumProperties` (slots 19/20) walks only the own list; every lookup binary-searches a sorted vector that already holds the inherited properties; the effective enumerator is slot 4 `GetPropertyList` (ascending ID order, one reference). The glue's own chain walk is acceptable when own properties win.
- ~~**DG30:** the suitability comparison tests each building type at a cell whose zone fits it, rotating through lots
  in state 1, lots in other states and empty cells. Tooling only. [accepted as tooling]~~ **ANSWERED 2026-10-15 (re-answers11):** Not an RE question (tooling only); accepted as tooling.
- ~~**DG31:** with the original simulators, a city run at about 85 game days per real second (`sim_turbo`) ends with
  lower income (3540-3647 against 4202-4593 at 8-20 days/s) and negative funds. Which original work is bound to
  frames or real time rather than game days? [unknown; compare runs only at equal pacing. Separately, our
  development unit costs about 3× per game day: 23-29 days/s under turbo against the original's 80-90]~~ **Answered (re-answers9, simulator §5.7):** four kinds of original work are paced by frames or real time: idle cycles with a per-day bound (growth developers, aura, plumbing); idle cycles restarted only when finished (demand census, land value, crime, power, the BDS monthly rebuild: whole months are skipped when they fall behind); real-time budgets inside day work (the BDS 3 ms cut); and posted messages delivered frame by frame. Original runs compare only at equal pacing up to about 20 days/s (cheetah is 50 ms per day); 85 days/s measures the pacing, not the simulation. **Per-day amounts, 2026-10-15 (re-answers10), simulator §5.7.1:** each round of `OnTick` opens at most one day and makes one idle pass, so under a clock that opens a day every round each idle agent gets K ≤ 1 idle delivery per day plus its day delivery. Calls per cycle: growth developer 20 + E (E = special-building effects), crime 6, power 5, land value 21, aura 17 + N (landmark departments), plumbing 2W + 7 (W = networks) plus the time-sliced cell states, demand census 2 plus time-sliced work over every cell. Only the census (time-bound: it skips a month when its CPU time exceeds about D × (5 + K·m) ms) and crime (if it gets fewer than 6 deliveries a month) can lose months. Land value and power defer; aura, plumbing, the growth developers and so the BDS hold days open instead (the BDS's developers end by about day 21).

## From the traffic glue, phase 1 (filed by the coordinator from the `trafficglue` lane's report, 2026-10-14)

Our traffic simulator (with the pathfinder) replaces the original in game (`enable = traffic`): clean under turbo,
at normal speed for 84 months and across two saves, with 0 reference warnings. There is no traffic shadow yet.
The lane's implemented answers are in brackets; most are glue choices that need only a confirmation or a spec line.

- ~~**TG1:** `FindPath` with a route deque. [returns 0, logged once, counted; the deque is untouched]~~ **ANSWERED 2026-10-15 (re-answers10):** the original fills the deque (pathfinder §4.13 step 5, also after the flag-2 fallback of §4.12); the automata, emergency and MySim dispatch and the query tool read it. The bracket **DIFFERS**: fill it. The deque's 0x28-byte layout and the clear / push_front / map-regrowth rules are in pathfinder §7 (traffic-simulator §9.1).
- ~~**TG2:** host callbacks left NULL: `path_info` and its helpers, `sim_clock_ms`, `update_network_costs`,
  `lot_strategy_byte`, `take_connection_volume_dirty`, `occupants_in_cells`, `occupant_filter_includes`,
  `cell_has_network_occupant`, `tunnel_paired_portal`, `view_cell_rect`, `create_accident`. Which of these does
  the original need for the simulation (not the view)? [all NULL in phase 1]~~ **ANSWERED 2026-10-15 (re-answers10):** by what the original reads (traffic-simulator §9.1 tables): **needed** for the simulation are the occupant query over a rectangle (`occupants_in_cells`), path connections and path info (`path_info` and helpers), the tunnel's paired portal (`tunnel_paired_portal`), the neighbour connection's volume-dirty test-and-clear (`take_connection_volume_dirty`; one sync per connection object), the roads and mass-transit department cost updates (`update_network_costs`, if that is what it does) and the lot's strategy byte at lot + 0x88 (`lot_strategy_byte`, only if our core computes the lots' trip targets). **Not needed**: the wall-clock deadline (`sim_clock_ms`), the accident check (`view_cell_rect`, `cell_has_network_occupant`, `create_accident`: monthly, wall-clock gated, visible cells only; it costs only the visible crashes) and `occupant_filter_includes` (filter 2 unreachable). So all-NULL **DIFFERS** for the first group.
- ~~**TG3:** the trip record layout. [FreeCity4's own `FC4TripData`]~~ **ANSWERED 2026-10-15 (re-answers10):** the lots' and connections' own `std::list<cSC4TripData>` (0x2C-byte records, allocator B) are read by their `Write`/`Read`, by the query tool's route display (`0x004CA460`, `0x004C5E20`, `0x004CFD20`) and by `SetupPathFinderForLot`. `FC4TripData` is acceptable only if those lists hold the trips in that layout whenever those run (copied in at load, written back before the lots save, kept current for the query tool); otherwise **DIFFERS** (a save loses every trip). traffic-simulator §9.1.
- ~~**TG4:** `Init` after state was loaded by `Read`. [the loaded state is kept]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES. In a load `Read` always runs before `Init` (foundations §3.7), and `Init` creates each grid, the iterators, the generator, the departments and the pathfinders only when the member is still null; the cycle state is reset only with a new scanning iterator (`0x0071F4FE`). Init rebuilds only unsaved structures (tuning, subscriptions, edge vectors, cell array). traffic-simulator §1.
- ~~**TG5:** slot 34's record size. [0x23C]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES: 0x23C bytes at simulator +0x1E4 (last field u8 at +0x238; next member +0x420; working set at +0x440). traffic-simulator §7.
- ~~**TG6:** slot 22. [returns null, logged]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES: slot 22 returns the lot iterator only in cycle state 1, which nothing enters (a loaded 1 becomes 0 at PostCityInit). traffic-simulator §9.1.
- ~~**TG7:** departments. [`Get`, else `Create(id, 0)`; `SetMaxAllowedFunding`; line items 1–6]~~ **ANSWERED 2026-10-15 (re-answers10):** partly **DIFFERS**: `CreateDepartmentBudget` returns an existing department, so get-else-create is right, but the original uses group `0xAA369059`, sets the department and line-item names, makes mass-transit line 2 and roads line 5 Income, and runs these only when the member is null after Read; lines 4–6 are created on every Init when missing. Exact calls in traffic-simulator §9.1.
- ~~**TG8:** `GetLanguageUtility`. [borrowed, not AddRef'd]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES: the language manager returns a pointer to its own member (`0x00924B50`) and nobody AddRefs or releases it. traffic-simulator §9.1.
- ~~**TG9:** `DoLotActivated` budget items. [copied, then `AddBudgetItemToBudget(lot holder, item.index)`]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES: `GetBudgetItemForPurpose(building holder, …)`, then `CopyBudgetItemProperties(building holder, lot holder)` and `AddBudgetItemToBudget(lot holder, item.index)` (index at record +0x18). traffic-simulator §9.1, §4.1.3.
- ~~**TG10:** lot order. [the master list through the safe iterator; a rect query as fallback]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES for the safe iterator, which is the original's only order; the rectangle fallback has no counterpart and must not run in normal play. traffic-simulator §4.1, §9.1.
- ~~**TG11:** `Shutdown`. [unsubscribe; `RemoveAgentAllLevels`]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES: unsubscribe the nine messages, then `RemoveAgent(this + 4)` on every level through the city's simulator (skipped when null). traffic-simulator §1.
- ~~**TG12:** `department_funding` with a null department. [1.0]~~ **ANSWERED 2026-10-15 (re-answers10):** no original behaviour (the original never checks; only a failed Read leaves a null department). 1.0 accepted. traffic-simulator §9.1.
- ~~**TG13:** slot 12. [a ring of 64 records]~~ **ANSWERED 2026-10-15 (re-answers10):** accepted as a choice. The original returns a live pointer into the hash node (or the shared zero record), valid until the record is erased; the ring gives call-time copies and must keep each pointer valid until the caller's next call. traffic-simulator §9.1.
- ~~**TG14:** slot 29. [a vector kept per tile pair]~~ **ANSWERED 2026-10-15 (re-answers10):** accepted as a choice if each call returns the current route as an STLport `vector<u8>` (allocator B) and null in exactly the original's cases; the original points into the terminal link's stored vector. traffic-simulator §9.1.
- ~~**TG15:** slots 20/21 called off the main thread. [a direct property read]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES: slots 19–21 use no simulator state, only the holder's property `0x8A0B4100`, with no lock in the original. traffic-simulator §9.1.
- ~~**TG16:** a lot's `AsTrafficSource`. [borrowed]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES: lot slot 4 is a cast without AddRef; the source iterator AddRefs the lot or connection object itself. traffic-simulator §4.1.
- ~~**TG17:** `CreatePathFinder(null)`. [false]~~ **ANSWERED 2026-10-15 (re-answers10):** no original behaviour (the original writes through the null pointer). false accepted. traffic-simulator §9.1.
- ~~**TG18:** `SetupPathFinderForLot` with a pathfinder the original created. [false]~~ **ANSWERED 2026-10-15 (re-answers10):** **DIFFERS** in principle (the original configures any `cISC4PathFinder` through its interface), but unreachable when ours is the city's simulator. Accepted. traffic-simulator §9.1.
- ~~**TG19:** stale lot or connection tokens. [refused; a connection removed mid-phase is skipped]~~ **ANSWERED 2026-10-15 (re-answers10):** lots MATCH (skipped by the safe iterator). Connections **DIFFER**: the original's connection list is a snapshot of counted references, so a connection removed mid-phase is still visited until the next Reset (entering 16, 17 or 18). traffic-simulator §9.1, §4.1.
- ~~**TG20:** which messages carry an object. [only the lot message `0xE8BDA2E7` and the occupant messages
  `0x99EF1142`/`0x99EF1143`]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES: only the lot message (IGZUnknown, QI `0xE818FCED`) and the occupant messages (void1, QI `0xA821EF94`) carry an object the handler reads; `0x08DAC3E7` reads data2 and the simulator's own department. traffic-simulator §9.1.
- ~~**TG21:** a message on another thread while the lock is busy; slot 19 while the lock is busy. [dropped; slot 19
  counted as a fallback]~~ **ANSWERED 2026-10-15 (re-answers10):** messages: the original never drops one (no lock, handled on the delivering thread), so a drop **DIFFERS** whenever it happens; keep it counted and rare. Slot 19 needs no lock at all (TG15): the direct write is the normal path. traffic-simulator §9.1.
- ~~**TG22:** units per slice against the original's 4 ms deadline (traffic-simulator §3.4). [16 deterministic units
  per slice; our turbo runs at about 43–46 game days/s with traffic against about 76 without]~~ **ANSWERED 2026-10-15 (re-answers10):** no original constant (TS27); 16 accepted. The original's per-day traffic work is the share `total/days` plus the last slice's overshoot, so N changes only the overshoot. traffic-simulator §9.1, §3.4.

## From applying the REH, flora, residential and flammability answers (filed by the coordinator from the `apply9d` lane's report, 2026-10-14)

apply9d applied AQ1–AQ10, RG24 and FL7–FL9 (merged `ccfe2b4`). Every in-game run stayed clean and exact:
flammability 18/18 under turbo and 117/117 at normal speed, residential 384/384 and flora 25/25. Under turbo the
flammability cycle-end message arrived up to 17 days after the wrap day.

- ~~**AQ11:** AQ2/AQ3: when there is no R$ EQ or HQ grid, which tract size do the R$$ and R$$$ variants and the
  minimum and maximum coordinates use? [member E]~~ **ANSWERED 2026-10-15 (re-answers10):** no original behaviour: the original never tests the R$ grid pointers (`+0x7C`, `+0x88`). Slots 35–38 call the R$ grid's `GetTractSize` at once (`0x006F79C4`, `0x006F7A63`); the wealth variants never ask the R$$ or R$$$ grid. Slots 43/44 return the six zeros when the population is 0 (`0x006F7389`), and otherwise take the walk's tract counts and the size from the R$ grid. The quotient helper (`0x006F6FC0`) reads all six grids untested, so a missing grid crashes. Member E (with E's tract counts) is accepted. residential-education-health §4.8.
- ~~**AQ12:** AQ10: how should the stray reference from a failed `OpenOStream`/`OpenIStream` be audited? [as
  acquired, then released as created-for-us]~~ **ANSWERED 2026-10-15 (re-answers10):** glue, with the original's ownership as the guide: an `Open*Stream` out-pointer is a reference the callee hands to the caller (the game's segment gets it by `QueryInterface` and drops its own, `0x005D27B3`–`0x005D27C0`). The helper adds none and releases it once. Audit it as handed over (created for us) at the call, whatever the call returned, and the helper's release as that reference's release. The bracket's "acquired" is right only if the audit counts both kinds together. flora §10.3.
- ~~**AQ13:** AQ7: is each end of a listing clamped to 0..191 on its own? [yes]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES the query's rule, which is the model for the recommended clamp: `0x0049DF00` clamps each of the four values on its own, as a signed 32-bit cell number, to ≥ 0 and then ≤ 191. The original's listing itself does not clamp (AQ7). flora §6.4.2.
- ~~**FL10:** FL8: the tracts a new cycle visits come from a `rand()` draw. Which marks carry into the next cycle?
  [every tract changed after the wrap day, a superset]~~ **ANSWERED 2026-10-15 (re-answers10):** glue; the bracket (a superset) is safe and accepted. The exact set can be read from the original: the iterator at flammability `+0x30` keeps index `+0x24`, current `+0x28` and step `+0x2C` (`0x0049CB10`), so the new cycle has visited the `index` positions `(current − j × step) mod count`. The marks to carry are the tracts in that set that changed at a poll from the wrap day's up to the comparison's. flammability §6.5.
- ~~**FL11:** FL8: the day count starts when the shadow is created (the first event, before the original's first
  day) and uses our own tractsPerDay. It is checked only against the number of cycle ends delivered, so an offset of
  whole cycles is not detected. [once it is off, it stays off for that city and only the comparison's own poll
  carries]~~ **ANSWERED 2026-10-15 (re-answers10):** glue, accepted as a fallback. The day count drifts (it starts before the original's `Init` wrap and uses our `tractsPerDay`), while the original's iterator index (`+0x24`) is the new cycle's exact position at the comparison. Prefer reading it. flammability §6.5.
- ~~**FL12:** FL1/FL8: what is the bound on unvisited tracts under the turbo clock? [max(2 × tractsPerDay, the
  new-cycle tracts the count gave at the previous comparison); in game the bound reached 7098 and the unvisited
  count stayed at most 155]~~ **ANSWERED 2026-10-15 (re-answers10):** **DIFFERS** in kind: the original has no fixed bound. The number of new-cycle tracts at the comparison is the remainder of the wrap day plus `tractsPerDay` per further SimNewDay in the same simulator tick, at most one day per round of the tick's loop, so it grows with the frame budget under a short day. It is exactly the iterator's index (`+0x24`). The bracket's heuristic is acceptable where the iterator cannot be read. flammability §6.5, §9.
- ~~**FL13:** FL7: what polls when there are no day polls, for example when the game's own simulator delivers the days?
  [every forwarded message plus the cycle end. Polling still needs the host's `watered_grid` to be non-NULL, although
  content polls read only slot 11]~~ **ANSWERED 2026-10-15 (re-answers10):** glue, accepted. The original reads the watered state only through plumbing slot 11, never `GetWateredGrid`. Every message the shadow sees, including posted SimNewDays, arrives at the message-server tick, after all of the previous simulator tick's visits, and intermediate plumbing states between polls cannot be seen from outside the simulator. Polling at every forwarded message plus the cycle end is the finest grain without an agent. The non-null `watered_grid` requirement is a glue limitation: fall back to slot 11. flammability §6.5.

## From applying the zones, landfill, budget and crime answers (filed by the coordinator from the `apply9b` lane's report, 2026-10-14)

apply9b applied ZG1–ZG17, LG27–LG29, BQ4, BQ7 and CQ23 (merged `969527d`). In game the zones shadow is exact (392/392,
296/296), and landfill game tiles insert with no failures (`gameTiles=12 gameTileFailures=0`; the tile exemplar is set
through the holder at occupant + 0x20).

- ~~**ZG18:** ZG3: with no exemplar, or no Zone Display Alpha property, does `Init` still copy the alpha to the default
  alpha? [no: the saved alpha survives and the default alpha stays as it was (0xFF from the constructor); only the
  exemplar's value is copied]~~ **ANSWERED 2026-10-15 (re-answers10):** partly **DIFFERS**. With no exemplar the tuning reader returns before both alphas (`0x004DC4BB`): matches. With the exemplar present but no Zone Display Alpha, the copy to the default alpha still runs (`0x004DC675` is unconditional), so the default alpha becomes the current alpha (the saved byte after a load). Also, the constructor never sets the default alpha (only the alpha, 0xFF at `0x004DCEB8`), so 0xFF there is our choice. zones §10.1, §4.
- ~~**ZG19:** ZG3 / §10.1: does `Read` set the default alpha? [no: the default alpha is not saved, so after `Read` and
  before `Init` it is 0xFF]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES: `Read` writes only the alpha (`0x004DD590`); the default alpha is neither saved nor read. Its value before the first `Init` is uninitialised in the original; 0xFF is our choice. zones §10.1.
- ~~**ZG20:** zones §15.2 still says "Type 16 → false", which conflicts with §6 and ZG17. [ZG17 followed: type 16
  dezones; `zn15_02` updated. The spec text needs the fix]~~ **ANSWERED 2026-10-15 (re-answers10):** the bracket MATCHES and the spec text was wrong: test 2 now says type 16 becomes type 0 and dezones with type 0's limits (zones §15 test 2, §6).
- ~~**ZG21:** ZG2 and the §5.1 dezone: under the ZG2 choice, a network whose bounding cells cross the city edge has its
  whole dezone refused. Before, the inside cells were dezoned, and the original dezones them too while touching
  memory outside the grid. [refused whole, with error 0; not seen in game]~~ **ANSWERED 2026-10-15 (re-answers10):** **DIFFERS**. The original dezones every in-city cell of the rectangle and also touches zone-grid memory outside the grid (no clamp in the handler, in `GetBoundingCityCells` `0x005E8E20`, or in `PlaceZone`). Recommended choice for this internal call: clip the region to the city and dezone the inside cells; refusing the whole call leaves them zoned. zones §5.1.

## From applying the neighbours answers (filed by the coordinator from the `apply9a` lane's report, 2026-10-14)

apply9a applied NG4–NG27 (merged `bf0e4b1`, ABI 45). The main in-game change: with our neighbours simulator active
the original budget now reads slots 32/33 as int64 in EDX:EAX (NG11). Before, it read garbage
(`income orig/ours=844424930136029/4061`); now 25/25 budget lines are exact. Konradshohe has no deals, connection
requests or marker changes, so NG4, NG14, NG16, NG17, NG19, NG23 and NG25 are tested only outside the game.

- ~~**NG35:** NG14: there is no interface for `GenericConfirmationDialog`. What should happen when popups are on?
  [confirm without asking; refuse when popups are off]~~ **ANSWERED 2026-10-15 (re-answers10):** **DIFFERS**. `GenericConfirmationDialog` (`0x0078DFF0`) is a free cdecl function `(title, text, yesLabel, noLabel)` (labels null for Yes/No), true only when closed with Yes (`0x8A8DFD4E`). It returns false at once when popups are off, otherwise it pauses and asks the player. With popups on, the original asks; confirming without asking lets the player never decline. Recommended: call it at its build-gated address and return its answer; refusing when popups are off already matches. neighbors §15.4 NG35.
- ~~**NG36:** NG14: with no app reachable, is the popup flag taken as enabled? [enabled]~~ **ANSWERED 2026-10-15 (re-answers10):** **DIFFERS**. `SC4GetPopupModalDialogsEnabled` (`0x005F9090`) returns false when the app global `0x00B43C94` is null, so the original behaves as popups off (request refused). It cannot happen in the game; only tests without an app see it. Treat a missing app as off. neighbors §15.4 NG36.
- ~~**NG37:** NG23: what does the original's `AddPropertyData` (holder slot 10) store? [ours stores a Uint8 array, so
  `GetPropertyData` does not read it back]~~ **ANSWERED 2026-10-15 (re-answers10):** **DIFFERS**. Holder slot 10 (`0x005FFA40`) stores raw bytes of memory type `0x8000`, with count = length, through variant slot 58 (`0x00915743`). It reuses the buffer when the type and count already match; otherwise it allocates with allocator C. It skips the copy for null data. `GetPropertyData` reads exactly that type back. A `Uint8` array is refused by the getter and is saved with a different tag. Store type `0x8000`. neighbors §15.4 NG37.
- ~~**NG38:** NG17: should a failing `GetPosition` be checked? [unchecked: a zero vector, so cell (0, 0)]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES. Occupant slot 8 cannot fail in the original: every implementation writes the vector, the base class (`0x005E8A20`) writes (0, 0, 0), and the caller ignores `AL` (`0x006CEF62`). A zero vector, and so cell (0, 0), is the original's result for an occupant with no position of its own. neighbors §15.4 NG38.
- ~~**NG39:** NG25: what is the rectangle form of `GetCityProps` (prop manager slot 18)? [not called; the usage test is
  added to the cell query]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES in practice. Prop manager slot 18 (`0x004A1DB0`): false without an occupant manager. Otherwise it sets the filter's usage to 1 and calls the occupant manager's cell-rectangle query (slot 40, `0x0049E250`) with the two inclusive corners. That query clamps the corners to the city, converts them to world coordinates (16 m cells, the far corner +1), shrinks the box by 0.1 m per side, opens the height range and runs the world-box query (slot 36). Ours differs only for a prop reaching less than 0.1 m into the cell or for a cell outside the city. neighbors §15.4 NG39.
- ~~**NG40:** NG7: what if the original's state bytes cannot be read (an unidentified vtable)? [ready = 1, everything
  else 0, logged once]~~ **ANSWERED 2026-10-15 (re-answers10):** glue choice, accepted. The original's bytes are always readable, and an unidentified vtable means a mod replaced the simulator. Its usual values between Init and Shutdown are ready 1 and initialized 1, with uiDisabled and busy 0 outside the budget tab and `simulate_deals`. The bracket's initialized 0 is unusual but unread (NG41). neighbors §15.4 NG40.
- ~~**NG41:** NG7: the `initialized` byte has no shadow input. Should it be compared? [read and logged only]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES. Only `Init` (`0x006D38FA`) and `Shutdown` (`0x006CFBC4`) read the byte, each returning true at once when already done; nothing else looks at it. Reading and logging it is enough. neighbors §15.4 NG41.
- ~~**NG42:** NG4: is the foreign deal's reference released exactly when it leaves the list, or when nothing in the
  core still holds it? [when the core object goes, which is list removal unless something else in the core holds it]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES. The original's list releases its reference exactly when the deal is erased (`0x006D1588` → `0x004216F9`). Every other holder (regional cities, queued reports, caller lists) has its own reference on the same object. The bracket is equivalent as long as each core holder that outlives removal stands for one of those. neighbors §15.4 NG42.
- ~~**NG43:** NG4: what should `AddDeal` return when the core cannot list the deal? [true, and the reference is given
  back]~~ **ANSWERED 2026-10-15 (re-answers10):** partly **DIFFERS**. `AddDeal` (`0x006D2870`) cannot fail and returns true. It clears Silent, appends without a duplicate check, pushes deals and statistics to the region regardless of the dirty flags, then clears both flags. A null deal crashes. "True" matches; not listing the deal differs (the original lists it and lets the simulation judge it). It is accepted as a logged, mod-only deviation. neighbors §15.4 NG43.
- ~~**NG44:** NG27: how do signed, 64-bit and float values convert, and what about an empty character array? [as C
  converts them; the empty character array stays false]~~ **ANSWERED 2026-10-15 (re-answers10):** partly **DIFFERS**. Scalars convert numerically (`0x00915AA5`): signed values sign-extend, 64-bit values keep their low 32 bits, and floats truncate toward zero through `_ftol` (NaN, ±∞ and values of 2^63 or more give 0). Arrays go through their first element's text: signed and 64-bit elements give the same bits. A float element of 2^63 or more wraps (Float32 {1e30} gives `0x19058000`), and a NaN element parses as ±1. An empty character array gives the empty text, which parses as 0, so the getter returns **true** with 0, not false. "As C" matches only for in-range values. neighbors §15.4 NG44; data-formats §5.1.

## From the history warehouse and 24-hour clock cores (filed by the coordinator from the `cores10a` lane's report, 2026-10-14)

cores10a implemented both cores (merged `0757437`, ABI 46): every §12 test of history.md and clock24.md is a unit
test and a conformance vector (`hw12_*`, `ck12_*`). The 421-save corpus checks (history §7.1, clock §12.12) are
left for the coordinator or a glue lane, since engine lanes cannot read the saves.

- ~~**CK7:** clock §4.1: the percentage factor's bit pattern `0x37C22E45` is 1/43200 (1.0 at noon), but the decimal
  value and §12.1 say 1/86400. [`0x37422E45`, 1/86400; a test pins it]~~ **ANSWERED 2026-10-14 (re-answers10):** the exe's constant (`0x00A86508`) is `0x37422E45` = f32(1/86400); the spec's `0x37C22E45` was a typo. `GetTimePercentage` at noon writes exactly 0.5 (`0x3F000000`); the product is rounded once to float. The bracket MATCHES (clock24 §4.1, test 14).
- ~~**CK8:** the clock step's signedness. [u32: a negative product wraps, then the 1200 clamp applies]~~ **ANSWERED 2026-10-14 (re-answers10):** the step is the low 32 bits of `_ftol`'s int64 truncation toward zero, compared unsigned with 60 and 1200: a product of −1 or less wraps and is clamped to 1200; between −1 and 0, or NaN, gives 0 (no step). The bracket MATCHES (clock24 §4.3, test 15).
- ~~**CK9:** an elapsed time of 2^31 or more. [the float(uint32) idiom]~~ **ANSWERED 2026-10-14 (re-answers10):** the unsigned-to-float idiom (exact load, plus 2^32 when the top bit is set); below 2^31 the product uses the exact `e`, rounded once. The bracket MATCHES (clock24 §4.3).
- ~~**CK10:** `atol` overflow (`Stopwatch magnify`). [wraps]~~ **ANSWERED 2026-10-14 (re-answers10):** the CRT `atol` (`0x009EF7C2`) skips leading white space, takes one sign and accumulates in wrapping 32-bit arithmetic. The bracket MATCHES. Also: `WhatTimeIzIt -1` gives hour 15, and `magnify` converts the signed result (clock24 §4.8, test 18).
- ~~**CK11:** how cheat text is split. [on blanks; double quotes group and are removed]~~ **ANSWERED 2026-10-14 (re-answers10):** the separators are exactly space, tab and newline, and runs count as one. A separator splits only when an even number of quotes has been seen in the current argument, and only an argument that both starts and ends with a quote loses that outer pair. The bracket MATCHES for ordinary text but **DIFFERS** if FreeCity4 removes inner quotes or splits on other white space (clock24 §4.8).
- ~~**CK12:** the cheat registration order. [register, then subscribe, one cheat at a time]~~ **ANSWERED 2026-10-14 (re-answers10):** register `WhatTimeIzIt`, subscribe it, then register `Stopwatch`, subscribe it. The bracket MATCHES (clock24 §4.2).
- ~~**CK13:** `Shutdown` without a cheat manager. [it unregisters only when a manager exists]~~ **ANSWERED 2026-10-14 (re-answers10):** without a cheat manager nothing is unregistered; with one, each cheat in turn has its notification removed, then is unregistered. The bracket MATCHES (clock24 §4.2).
- ~~**CK14:** a saved pause count of −1. [signed i32 counters: Resume does nothing, `IsPaused` is false, so the clock
  runs]~~ **ANSWERED 2026-10-14 (re-answers10):** the counters are signed, `Pause(false)` does nothing at −1 and `IsPaused` is false, but `Init` starts the timer only when both counters are **exactly** 0. So the timer never starts, every step is 0 and the clock stays frozen until `Stopwatch pause` brings the count to 0 and a hidden pause cycle (or pause and resume) starts the timer. The bracket **DIFFERS** ("the clock runs") (clock24 §4.7, test 16).
- ~~**CK15:** timer `Restart` on a stopped timer. [starts it]~~ **ANSWERED 2026-10-14 (re-answers10):** `Restart` sets the elapsed time to 0 and the start tick to now, whether or not the timer ran, so a stopped timer starts. The bracket MATCHES. Also: `Start` does not test whether the timer runs (clock24 §4.3 timer model).
- ~~**HW5:** history §4.7: how `r` is grouped. [the product of the two standardised values, summed, then divided by n]~~ **ANSWERED 2026-10-14 (re-answers10):** the grouping MATCHES (the product of the two standardised values, summed, then divided by n), but the standardised values are `(x − meanX) × (1/sx)` and `(value − meanY) × (1/sy)`, with each reciprocal rounded once; dividing by `sx` changes the result (history §4.7, test 12).
- ~~**HW6:** ranges: how they are walked, and a widened range with from > to and no point in it. [walked like a list;
  the widened case takes the point before the first one at or after `from`, and that point]~~ **ANSWERED 2026-10-14 (re-answers10):** walked like a list from the first point, with unsigned dates. With `from > to` the widened range is the **first point after `to` and its predecessor** (after the two pre-checks: the last point if `from` is after it, the first point if `to` is before it). The bracket (the point before the first at or after `from`, and that point) **DIFFERS** whenever a point lies between `to` and `from` (history §4.6, test 11).
- ~~**HW7:** a `Read` failure. [stops at the first failed read and returns false; series read completely stay, the
  partial one is dropped; a date not after the previous one, with no point yet, is dropped]~~ **ANSWERED 2026-10-14 (re-answers10):** `Read` tests no individual read and runs on with the values the failed reads left, so the series being read keeps the points read before the failure. A duplicate key makes `Read` return false **at once** (the rest of the record is unread); the old spec said it was only discarded. The bracket's stop-and-drop is accepted as *[choice]* for corrupt records; it **DIFFERS** from the original only in the partial series. A date ≤ prev with no point (only date 0) is dropped: MATCHES (history §7, test 14).
- ~~**HW8:** `Store`'s full-queue test. [unsigned: a count at or above maxCount drops one point; maxCount 0 means
  unlimited, so 0xFFFFFFFF never trims]~~ **ANSWERED 2026-10-14 (re-answers10):** the u32 count is compared unsigned with `maxCount`, and 0 means unlimited. The bracket MATCHES (history §4.1).
- ~~**HW9:** `GetMinMaxAverage` with from == to and no point on that date. [`GetPoint` is called twice and its value
  counted twice, as written]~~ **ANSWERED 2026-10-14 (re-answers10):** both flags are tested after the loop, so with `from = to` and no point on that date, `GetPoint` runs twice and its value counts twice. The bracket MATCHES (history §4.7, test 13).
- ~~**HW10:** precision values other than 24. [all treated as 53 bits]~~ **ANSWERED 2026-10-14 (re-answers10):** glue-only: the game never selects the 64-bit mode, so only 24 and 53 occur. The choice is confirmed (history §4.7).
- ~~**HW11:** 24-bit sums of int32 operands. [the exact sum, rounded once]~~ **ANSWERED 2026-10-14 (re-answers10):** each addition takes the exact integer operand and the stored sum and rounds once at the caller's precision. The bracket MATCHES (history §4.7).
- ~~**HW12:** `RecordHistoryData`'s return value. [true]~~ **ANSWERED 2026-10-14 (re-answers10):** the original returns the final `Release`'s EAX, which is 0 (false) because nothing keeps the message. No shipped caller reads it (48 call sites checked). The bracket **DIFFERS**: return false *[compat]* (history §4.1).

## From the commercial, industrial, civic and park cores (filed by the coordinator from the `cores10b` lane's report, 2026-10-14)

cores10b implemented the four cores (merged `ab3c1c3`, ABI 47). Every listed spec test is a unit test and a
conformance vector (`ci13_*`, `cb12_*`, `pk12_*`). The 421-save corpus tests (commercial-industrial §13.9, civic §12.10,
parks §12.8) wait for a glue lane's save sweep, which the coordinator runs.

- ~~**CM4:** commercial `Init` after a failed `Init` (§4.1 sets `initialized` in step 1). [returns true at once and
  creates no grids]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES. `Init` sets `initialized` before the demand test (`0x0069EF9C`), so a second call returns true at once (`0x0069F196`) and creates no grids. commercial-industrial §4.1.
- ~~**CM5:** commercial rebuild with no building map: RG24 says the shared seed helper `0x005E4FA0` writes nothing; do the
  passes of helper `0x005E5130` still run? [yes: nothing is seeded, then the passes run, so a fresh grid stays 0 and a
  loaded grid is only lowered]~~ **ANSWERED 2026-10-15 (re-answers10):** **DIFFERS**. Helper `0x005E5130` tests both pointers first and returns at once for a null building map (`0x005E5136`) or a null grid (`0x005E513F`): no seed and **no passes**, so the grid is left exactly as it was (a loaded grid is not lowered either). The seed helper `0x005E4FA0` is only reached with both present. `DoIdle` still advances the state. commercial-industrial §4.4.
- ~~**CM6:** a commercial grid absent at a rebuild (failed `Init`, or an unresolved save reference). [the rebuild does
  nothing; the state machine still advances]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES. The helper returns at once for a null grid (`0x005E513F`) and `DoIdle` advances the state anyway. commercial-industrial §4.4.
- ~~**CM7:** a commercial state outside 0..3. [nothing happens]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES. The dispatch is an unsigned bounds test (`ja` at `0x0069ED50`), so any value outside 0..3, negative ones included, does nothing. commercial-industrial §4.3.
- ~~**CM8:** commercial `Read`. [atomic; the old grids are released before being replaced (§8.1 quirk)]~~ **ANSWERED 2026-10-15 (re-answers10):** the original is **not atomic**: `Read` (`0x0069ECB0`) sets `dirty` from the flag before the references (`0x0069ECF6`), each reference read (`0x005D9730`) first stores null without releasing the old grid, and the result is the stream's error state after both reads (`0x0069ED16`). A wrong version changes nothing. The bracket (read into temporaries, release the old grids on success) is a *[choice]* that agrees with the original for every valid save read on a fresh object. commercial-industrial §8.1.
- ~~**CM9:** the commercial message server. [unsubscribe and release only when one is held]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES. `Shutdown` (`0x0069EBE0`) unsubscribes and releases the message server only when one is held. Note: `RemoveAgent` goes through the global city `0x00B43CEC`, not the held city. commercial-industrial §4.2.
- ~~**IN2:** industrial `Shutdown`: the spec gives only "AL (1)". [clears `initialized`, returns true]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES. Industrial `Shutdown` (`0x006B8A10`) clears `initialized` and returns true, nothing else. commercial-industrial §2.2.
- ~~**CB6:** civic `Read` order: inserting the file's entries in order, as STLport does, would reverse each bucket chain,
  so §12 test 9 could not be byte-identical. [grow the table first, then insert in reverse; a key already present, or
  repeated in the file, keeps its first value]~~ **ANSWERED 2026-10-15 (re-answers10):** **DIFFERS**. The original inserts in **file order**, each at the head of its chain (set `0x004F0940` → `0x00624DD0`, map `0x0069E720` → `0x0069E5F0`; `resize(n + 1)` before each insert changes nothing below 194 entries), so `Read` **reverses** every chain of two or more entries, and write → read → write is not byte-identical for such chains; a second round trip restores them. A repeated or already present status key **overwrites** the stored value (last value wins, `0x0069E944`–`0x0069E96D`); a repeated exclusion ID is a no-op. civic §7, and §12 tests 9 and 10 are corrected to expect the reversal (test 10: corpus re-write = the record with multi-entry chains reversed; a second round trip byte-identical).
- ~~**CB7:** civic `Release` (slot 2). [sibling rule: above 1 it decrements; otherwise Shutdown, destroy, return 0]~~ **ANSWERED 2026-10-15 (re-answers10):** **DIFFERS**. `Release` (`0x0069D7A0`) decrements above 1; at 1 or less it calls the deleting destructor (`0x0069E700`, destructor `0x0069E500`) **without** `Shutdown` and returns 0. The destructor releases the Landmarks department but not the list entries. The city always shuts the object down before its last release, so the bracket (Shutdown, then destroy) agrees in the shipped game and is accepted as a *[choice]*. civic §8.
- ~~**CB8:** civic null list entries. [the monthly pass skips them; an ape escape that draws one does nothing]~~ **ANSWERED 2026-10-15 (re-answers10):** the original crashes: the monthly pass dereferences each `maintained` entry without a test (`0x0069E0F0`–`0x0069E10B`), and the ape escape passes a null draw on to `AddControllersFromOccupantGroup` (`0x00517150` → `0x005163D0`) untested (the controllers' handling of a null target was not followed). The bracket is accepted as a *[choice]*. civic §4.6.
- ~~**CB9:** a business-deal income line that cannot be created (CB5). [skipped]~~ **ANSWERED 2026-10-15 (re-answers10):** the original crashes: `SetType(1)` is called on a line that could not be created (`0x0069DF6D`–`0x0069DF78`). Skipping is a *[choice]*. civic §4.4.
- ~~**CB10:** a civic department that cannot be created in `Init`. [no name or maximum is set and nothing is held]~~ **ANSWERED 2026-10-15 (re-answers10):** partly. A Landmarks department that cannot be created crashes the original (name set on null, `0x0069DC69`); Building Maintenance and Business Deals are null-tested (`0x0069DCAB`, `0x0069DCF1`) and set nothing: MATCHES for those, a *[choice]* for Landmarks. civic §4.1.
- ~~**CB11:** a civic department whose funding cannot be read in the monthly pass. [that entry is skipped]~~ **ANSWERED 2026-10-15 (re-answers10):** the original crashes: it reads through the funding pointer without a test (`0x0069E146`). Skipping the entry is a *[choice]*. civic §4.6.
- ~~**CB12:** civic `DoBuildingRemoved` step 3. [releases only when a matching entry was found and erased]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES (`0x0069DBEB`–`0x0069DC02`): the release happens only when a matching entry was found and erased. civic §4.5.
- ~~**CB13:** civic `Read`. [atomic; the old Landmarks department is released, the old list entries are not]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES except atomicity. The old Landmarks department is released before its reference is read (`0x0069E9A1`–`0x0069E9AB`); the lists are cleared without releasing their old entries (`0x005DB3C0`). The original is not atomic (the containers fill entry by entry and the stream error is tested at the end); atomicity is a *[choice]*. civic §7.
- ~~**CB14:** the civic temporary generators' counter n is not saved. [seed derive(S, 0x0A413481 ^ n), n counting the
  generators this object has made]~~ **ANSWERED 2026-10-15 (re-answers10):** accepted (glue, nothing in the original: it seeds each temporary from the clock). `n` counts the generators the simulator object has made, from 0 at construction, not saved. foundations §6.3's "in this session" is reworded to "by the owning object"; civic §5.
- ~~**PK4:** park tuning with no exemplar. [threshold 0.25 and an empty curve, which gives 0, so no zoo escapes; the
  tuning is read once per object]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES with one *[choice]*. Without the exemplar `ReadTuningParams` (`0x006D5990`) reads nothing and returns false; the threshold keeps the constructor's 0.25 (`0x00B0D3D8`, `0x3E800000`) and the curve stays empty (`0x007D0410`). `Init` sets `tuningRead` whatever the result (`0x006D5B1C`–`0x006D5B21`): read once per object. The original's evaluator reads an empty curve through a null array (`0x005F4289`) and crashes, so "empty gives 0" is a *[choice]*. parks §4.1, §4.4, §6.
- ~~**PK5:** park insert when the item names a missing department, or a line item cannot be created. [steps 5–7 are
  skipped; the line item is skipped]~~ **ANSWERED 2026-10-15 (re-answers10):** the original crashes in both cases: a missing department is dereferenced at `0x006D571F` (after step 5 has already changed the item's line), and a line item that cannot be created is dereferenced by `SetDisplayFlag` at `0x006D573E`. The bracket is accepted as a *[choice]*; FreeCity4 still runs step 8 (release the building) and the zoo step. parks §4.2.
- ~~**PK6:** park monthly pass with no held department or unreadable funding. [stops after step 1]~~ **ANSWERED 2026-10-15 (re-answers10):** the original crashes: no null test on the department (`0x006D58C4`) or on the funding pointer (`0x006D58CA`). Stopping after step 1 is a *[choice]*. parks §4.4.
- ~~**PK7:** park zoo removal of an occupant that was not listed. [released once anyway: the literal reading]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES. The list removal (`0x006D5200`) frees the matching nodes without releasing; the single release follows unconditionally (`0x006D52C3` parks, `0x006D5366` zoos). parks §4.3.
- ~~**PK8:** park `Read`. [atomic; the lists are replaced without releasing their old entries]~~ **ANSWERED 2026-10-15 (re-answers10):** the lists are replaced without releasing their old entries (`0x005DB3C0` clears first): MATCHES. The original is **not atomic** (both lists read with no error test in between; the result is the stream's error state, `0x006D51EE`); atomicity is a *[choice]*. A wrong version changes nothing. parks §7.
- ~~**PK9:** the park generator. [`cRZRandomSuper` seeded with derive(S, 0xAA3FE6BC)]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES as a *[choice]*. The original builds `cRZRandomSuper(−1)` in the constructor (`0x006D5850`–`0x006D5855`, `0x00913B74`): wall clock, once per object, not reseeded by `Init`. FreeCity4's `derive(S, 0xAA3FE6BC)` follows foundations §6.3; bit 0 of the seed is ignored (RN1). parks §5, foundations §6.2.
- ~~**RN1:** foundations §6: in a mutation check, session seeds `0x1234` and `0x1235` gave identical park draws. Since
  `derive` is an XOR, the generator seems to ignore bit 0 of its seed. Does the original `cRZRandomSuper` (or its
  seeding) also drop bit 0, and does it matter for any other generator? [not investigated]~~ **ANSWERED 2026-10-15 (re-answers10):** yes, and it is the original's own behaviour: `cRZRandomSuper::Seed` (`0x00913A72`) ORs the seed with 1 (`0x00913A97`) before the seeding loop, after the clock test, so `s` and `s XOR 1` give identical streams. This matches our generator. `cRZRandom::Seed` (`0x00913667`) keeps all 32 bits. It matters only for `cRZRandomSuper` users (the file-local generators of §6.3, the park generator, …). Seed-mutation tests should flip another bit (e.g. `S XOR 2`), and `derive`'s XOR 1 fix-up is invisible to them. foundations §6.1.

## From applying the development answers (filed by the coordinator from the `apply10a` lane's report, 2026-10-15)

apply10a applied DG4, DG5, DG6 and DG11 (merged `b398049`, ABI 49): the building loader, the BDS foundation and
retaining wall slots 23/26/29, the capacities-present flag and the budget item cost sum. In game the replacement is
clean (476/476 foundations, 35/35 wall loads, `ref_warnings=0`) and the development shadow exact (927/927). DG28 is
updated above (the AddRef lead is disproved). The lane's choices are in brackets.

- ~~**DG32:** DG5 loader: property `0x27812810` with fewer than three values, or values not stored as float32 (the
  original reads three floats through the raw value pointer). [the missing values are 0; other numeric types are
  converted]~~ **ANSWERED 2026-10-15 (re-answers11):** **DIFFERS** (unreachable with shipped data; the bracket is accepted as a *[choice]*). The loader passes the variant's value field (slot 111, `0x0060CE80`, returns `+0x04` whatever the type) to `SetBoundingBox` as the address of three floats (`0x00698582`–`0x00698592`): a Float32 array of 1 or 2 values is read past its heap block (garbage, not 0), a scalar's value bits or an Empty value's null are used as an address (crash or garbage), and integer arrays give their bits as floats (no conversion). All 743 shipped building exemplars hold a Float32 array of 3, own or inherited. building-development §4.9.1 step 6.
- ~~**DG33:** BDS slot 26: `0x88FCC3E3` is present but has no values. [max height is set to 0]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES. The read is the converting Float32 getter (`0x005FD4F0` → variant slot 37 `0x00917852`); an array of count 0 (or an Empty value) takes the text round trip, gives the text `0`, so 0.0, and the getter returns true, so `SetMaxHeight(0.0)` is called (`0x00698397`–`0x006983A4`). A missing property leaves the constructor's value. Shipped: 46 of 50 foundation exemplars have no `0x88FCC3E3`, 4 have one Float32. building-development §4.9.2.
- ~~**DG34:** BDS slots 26 and 29 with a null occupant (the original does not test for it). [return false]~~ **ANSWERED 2026-10-15 (re-answers11):** the original **crashes**: both slots call `occ.AsOccupant()` through the occupant first, with no test (`0x006982C7`, `0x006983D0`). Returning false is accepted as a *[choice]*. building-development §4.9.2.
- ~~**DG35:** BDS slot 29: its fallback for type 0 needs `GetRetainingWallType` (wall slot 11), but docs/abi gives no
  IID for `cISC4LotRetainingWallOccupant`, so the wall cannot be reached from an occupant token. What is the IID?
  [the plugin keeps the wall the slot 29 call received and answers only for that wall; any other token gives 0]~~ **ANSWERED 2026-10-15 (re-answers11):** **IID `0x295D1162`**. The wall occupant's `QueryInterface` (`0x005E28C0`) answers it with the primary interface (`+0x00`, vtable `0x00AA3080`) and passes every other IID to the `cSC4Occupant` base at `+0x04` (whose slot 0 `0x005E29A0` adjusts back), so the QI works from an occupant token; the game itself reaches walls from occupants this way (`0x005E3766`, `0x005E385A`). Slot 11 (`+0x2C`, `0x0080BC00`, pop 0, EAX) returns the wall type at `+0x38`. The bracket (answer only for the wall the call received) **DIFFERS** for slot 29's type-0 fallback on any other wall; with the IID the plugin can QI the token. building-development §4.9.2; ABI row proposed in the lane report.
- ~~**DG36:** foundation creation: the spec only implies that GZCOM class `0x49C05C8F`, like `0xA9BD882D`, returns with
  the caller's reference already counted. [it is not AddRef'd again; in game `ref_warnings=0` over 476 foundations]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES. Class `0x49C05C8F` is the foundation occupant (`GetGZCLSID` `0x005D6130`, registered at `0x004607F7`); its factory `0x005D64C0` uses slot 23's pool and constructor (count 0) and returns the `+0x04` sub-object without AddRef, and the director's `GetClassObject` (`0x0090DFAC`) then QIs it (`0x0090DFFC`–`0x0090E00B`), which AddRefs: the caller receives count 1, as for `0xA9BD882D`. building-development §4.9.2.
- ~~**DG37:** `SetDefaultExemplar` on an occupant with no `cISCExemplarPropertyHolder` (QI `0x0AC2B5F7`), which the
  original does not test for. [false, which the building loader treats as success with an empty box]~~ **ANSWERED 2026-10-15 (re-answers11):** **DIFFERS** (unreachable with the game's classes; the bracket is accepted as a *[choice]*). The holder's `SetDefaultExemplar` is called without a null test in all three loaders (building `0x0069853A`–`0x00698545`, foundation `0x0069836A`–`0x00698375`, wall `0x00698452`–`0x0069845D`), so such an occupant crashes the original. Buildings, foundations and walls all answer `0x0AC2B5F7` through the property-holder base. building-development §4.9.2.
- ~~**DG38:** DG4: `GetBudgetItemInfo` needs the building exemplar as a property holder. Does the original pass the
  resource exemplar or the occupant? [the type's exemplar is loaded from the catalogue key and passed; 0 when it does
  not load or the budget simulator is not reachable]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES for the holder: the original passes the building **exemplar's** property holder, loaded once per type at catalogue build (`AddBuildingData` `0x0069D0A0`: `GetResource` then exemplar slot 7, `0x0069D0CF`–`0x0069D0FF`), the same holder every other profile field is read from; never an occupant. The budget simulator (`0x00B43D64`) is called with no null test (`0x0069BDCF`–`0x0069BE00`), so "0 when the budget simulator is not reachable" is a *[choice]* (the original crashes). building-development §4.7.1 table.

## From applying the exemplar, graphics, protection, landfill and agent-order answers (filed by the coordinator from the `apply10b` lane's report, 2026-10-15)

apply10b applied EX34, EX35, EX38, EX39, GF7, PR17, PR18, LG21, LG23 and AO1 (merged `0f3ad91`, ABI 50) and cited
EX36, EX37, PR19, AO2, LG22, LG24-LG26, RG25-RG28, NB19 and PG16. EX39 is applied as the spec says: the C ABI slice
setters now always store an array, and the vector readers read a scalar as empty. In game the shadows are exact
(development 928/928, protection 288/288, landfill 72/72) and a replacement run that zones a new landfill reaches the
LG21 view route (rotation 0). GF13 and GF14 are reserved by graphics-formats §8.5, so these questions start at GF15.
The lane's choices are in brackets.

- ~~**EX40:** EX34 (data-formats, `format_float`): when the 63 kept characters hold no significant digit (the first one
  falls at the 64th or 65th character), does the trailing-zero strip also remove the `.`? [only `0` characters are
  stripped, so it leaves `0.` or `-0.`; marked [literal reading]]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (data-formats.md §5.1, the 63-character cut). The strip clears only `0` characters, walking back from the 63rd character and never testing the first (`0x00910686`–`0x00910697`), so it stops at the point and leaves `0.` or `-0.`. Examples: 1.5e-62 prints `0.`, −1.5e-61 prints `-0.`. The case needs more digits after the first significant one; a lone digit at the 64th character is cut without a strip.
- ~~**GF15:** GF7 (graphics-formats §4.2): is the zoom mask's bit number the zoom index as passed (0-4, before the
  detail remap), and is the mask read as the property's first integer value whatever its stored type? [yes to both; a
  mask with no value counts as 0; [literal reading] in `keys::occupant_models`]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES, with the stored-type rule made exact (graphics-formats.md §4.2, "Details of the read"). The bit is the zoom as passed, before the detail remap (`0x00496D16`–`0x00496D25`; shift count mod 32). The mask is read with the converting getter (`0x005FD480` → variant slot 25 `0x009176C8`): scalars converted by data-formats §5.1 (floats truncated), an array by the text round trip (first element), and count 0 or Empty gives 0. All 1,009 shipped masks are Uint32 scalars.
- ~~**GF16:** GF7 (§4.2): for b+0..b+3 (RKT0-RKT3) and b+5 (RKT5), does `GetAllModelInstanceIDs` write T and G to its
  out pointers, as it does for a matching RKT4 record? [yes, v[0] and v[1]; [inferred]]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES. T and G are written for every form: RKT0 at `0x00496D80`/`0x00496D8A`, RKT2 and RKT3 at `0x00496EFC`–`0x00496F06`, and RKT1, RKT5 and matching RKT4 records through the rule helper `0x00496830` (`0x00496929`–`0x00496933`), each time v[0] and v[1].
- ~~**GF17:** GF7 (§4.2): when a rule needs a value past the end of the whole property (too few values), or the zoom
  is above 4, what is appended? [nothing, neither an instance nor an offset, matching `model_key`'s "no model"; inside
  the array an RKT3 record at zoom 3-4 does read into the next record, as the quirk says]~~ **ANSWERED 2026-10-15 (re-answers11):** **DIFFERS** (choice accepted). The original checks nothing for RKT0–RKT3 and RKT5: it always appends one instance, plus offset (0, 0, 0) if asked (`0x004970EA`), reading past the property's array, or past the remap table for a zoom above 4. An in-range RKT1 base of 0 also appends a 0 instance (`0x00496E1E`–`0x00496E35`). Appending nothing in the out-of-range cases is a *[choice]*, since the original's values are garbage; shipped data never reaches them. Only RKT4 checks lengths.
- ~~**GF18:** GF7 (§4.2): which stored types do the model keys and the zoom mask accept (the original's read
  conversion)? [the integer types up to 32 bits, converted; anything else counts as absent; shipped keys are Uint32
  arrays; [choice] in `keys::property_u32s`]~~ **ANSWERED 2026-10-15 (re-answers11):** **DIFFERS** (choice accepted). The keys are read with `vec<u32>` (`0x005FF510`): the raw 32-bit words of any stored type with no conversion; a scalar gives an empty vector, and RKT0–RKT3 and RKT5 then read through a null data pointer *[inferred: crash]*. The zoom mask alone uses the converting getter (GF15). Every shipped key is a Uint32 array and every mask a Uint32 scalar, so treating other key types as absent is a safe *[choice]*.
- ~~**LG30:** LG21 (landfill §4.7.5): should the view rotation be read when the zot is placed (what the glue does) or
  cached at view init, as the original's global is? [read at placement through the documented route; 0 without a
  view. Only rotation 0 has been seen in game: no script action rotates the view]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (landfill.md §4.7.5, "When the rotation is read"). The global caches the view-utilities object pointer, not the rotation; `AddZot` reads `+0x10` at placement (`0x006BB510`, `0x006BB51F`), so the original also uses the rotation current at placement. Without a view the original reads through a null pointer, and the glue's 0 is a *[choice]*.
- ~~**AO3:** AO1 (foundations §13.4): the two port developers share the name `cSC4PortDeveloper` and rank 2. Is their
  relative order registration order? [yes, registration order]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES as a choice (foundations.md §13.4). The original ranks both agents by heap address and the logs do not separate them, so there is no original order to follow. Neither agent reads the other's results (separate port lists and history keys, no random draws, additive demand updates); the only visible difference is the order of two upgrade advisor messages in the same month. Registration order is confirmed.
- ~~**PR20:** PR18 (protection §5.1): the answer covers `UpdateUnadjusted`'s caller record. Does the separate
  dispatch-message handler also carry leftovers into its own record? [no: it keeps a zeroed record]~~ **ANSWERED 2026-10-15 (re-answers11):** **DIFFERS** in the letter, choice accepted (protection.md §7.2, PR18 table). The handler's record is in its own stack frame (`0x006F61F0` reserves 0x44 bytes and writes nothing to the effect record before the call at `0x006F6221`), so it does not carry `UpdateUnadjusted`'s values. It holds stale bytes from unrelated earlier calls. Only the radius is used; a successful call with the radius property writes it, and FreeCity4 skips the update on failure, so a zeroed record is equivalent on shipped data.

## From the city situation, demolition, dispatch, port, prop developer and city detail specs (re-specs10 lane, 2026-10-14)

Open points from city-situation.md, demolition.md, dispatch.md, port-developers.md, prop-developer.md and
city-detail.md. The current answer is in brackets; each entry says what would settle it. "Turbo run" means the
in-game harness with the FreeCity4 turbo clock and a message log.

- ~~**CS1:** what does 3D view slot 57 do when the situation popup is shown (`0xEBB67EF7`)? [hides the mission status
  panel]~~ **ANSWERED 2026-10-14** in this lane from the exe (3D view vtable slot 57, `+0xE4`,
  `HideMissionStatusPanel`) and written into city-situation.md §3.
- ~~**CS2:** the argument meanings of city situation manager slots 22–25 (`InitiateSituationForOccupant`,
  `InitiateSituationByAutomataGroup`, `StartJoyride`, `TreatAutomatonAsGroup`). [the symbol port's names; the
  argument counts are verified from the pops (8, 12, 12, 8 bytes); nothing FreeCity4 replaces calls them. A call
  trace of the four slots while the shipped situations run (car chase, UFO, joyride) in a turbo run would settle it]~~ **ANSWERED 2026-10-15 (re-answers11):** Answered from the exe and the Mac build's symbols (city-situation.md §8, slots 22–25). Slot 22 `(cISC4Occupant*, cISC4CitySituationAdvice*)`: QI for the automaton, then `InitiateSituation(advice or the first matching active entry, automaton, true)`. Slot 23 `(uint32 group, bool, bool search)`: starts the active entry that accepts the group, else (only when `search`) looks for the group's automata. Slot 24 `(cISC4Automaton*, bool, cIGZMessageTarget2*)`: sets `justActivated`, posts `0xABBBCA64`, enters the drive view. Slot 25 `(cISC4Automaton*, uint32 group)`: sets the automaton's group and tracks it. MATCHES: nothing FreeCity4 replaces calls them.
- ~~**CS3:** who posts `0x2C16BDE7` (the situation popup without an advice)? [no code in the executable; only a script
  or a plugin could. A turbo run that triggers every shipped situation should log 0 posts of `0x2C16BDE7`]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES. The ID's four bytes occur once in the whole executable, at the handler's compare (`0x0052EE1F`), so no code and no data table can post it (city-situation.md §3). Only a script or plugin could.
- ~~**CS4:** (cores12a) tick step 4 runs only while the status is 3; elapsed time grows by `t × 0.001` s in `double`;
  a time limit above 0 gives status 2 when reached; the 30-second warning fires once. Is that right? [as stated]~~ **ANSWERED 2026-10-15 (re-answers15):** **DIFFERS** on three points (city-situation.md §4.4). (1) Step 4 runs on every tick with a mission held, at any status (also 4 and 5). It is skipped only in a tick that handles `justActivated`, decrements a countdown that stays > 0, or decrements a positive `endDelay`. It does run in the tick in which the countdown expires. Within it, the built-in success checks and the clock act only at status 3, but the evade-list check, the Lua check (`+0xAE4 ≠ −2`, called whatever the status), the warning and the pursuit update run at any status. (2) `elapsed := elapsed + t × 0.001f` is `float` arithmetic (`0x0052CA4A`, constant `0x00A867A4`; 24-bit x87), not `double`. (3) The limit fires only when `limit ≥ 1.0f` and `limit ≤ elapsed`, so a limit in (0, 1) never fires. The warning MATCHES: it fires at most once (the flag is cleared by `InitiateSituation` and `ActivateSituation`), when `limit ≥ 30.0f` and `limit − 30.0f ≤` the pre-increment `elapsed`, through active automaton slot 61 with no null test.
- ~~**CS5:** (cores12a) does `InitiateSituation` return false only for "could not start" (status 5)? [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** **DIFFERS** (city-situation.md §4.3). It also returns false, with nothing changed, when the advice is already the active one (null included). For a null advice it deactivates the current mission and returns false with no popup. And without `showBriefing`, a successful start still sets status 5 and posts `0xEBB67EF7(advice, 0)`, but returns **true** (`0x0052E8E3`) *[quirk]*. The slot-21 caller then activates (status 3), so the queued popup opens the briefing window *[inferred]*.
- ~~**CS6:** (cores12a) `DeactivateSituation` sends `0xABBBCA64` (data1 0), releases the advice, resets to status 4, ends a
  running countdown and unfreezes the automata; `EndSituation` is the same call. Is that right? [as stated]~~ **ANSWERED 2026-10-15 (re-answers15):** **DIFFERS** (city-situation.md §4.3.1). `DeactivateSituation` (`0x005294C0`, only with a mission held) does the send (synchronous), the release and status 4, but it does **not** touch the countdown, `endDelay`, the time limit, `elapsed` or the indicators. The automata are unfrozen only when the countdown runs out on a later tick. `EndSituation` (`0x00529ED0`) is a separate call. It releases the `+0xA9C` effect, sets the time limit to 0, and stops and releases the countdown effect, though not the countdown value. With a mission held, it untracks the active vehicle from the advice's entry and clears it, then calls `DeactivateSituation`. It always calls `ShowAllIndicators(4, indicatorsVisible)`.
- ~~**CS7:** (cores12a) with max CSI reached, a situation does not take over another entry's vehicle. Is that right?
  [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES in effect (city-situation.md §4.2). The original has a takeover path, used at the limit or with no candidate. It scans the entries **after** the new one for a holder of strictly lower advice priority (`0x00524A50`) and moves that holder's vehicle and handle across. But every insert re-sorts the list ascending by priority (`0x00527920`), so no later entry has a lower priority unless priorities changed since the sort. The path is dead in shipped play *[inferred]*.
- ~~**CS8:** (cores12a) `failure_money` is deposited only when it is non-zero, like the success money. Is that right?
  [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES (city-situation.md §4.5). The test is in the advice's getter: slots 48 and 51 (`0x004237B0`, `0x00423820`) return false for 0, and the manager deposits only on true (budget slot 7). A negative amount is a withdrawal that the budget refuses when funds are too low.
- ~~**CS9:** (cores12a) the outcome aura position is the active vehicle's, else the current target's, else no aura. Is that
  right? [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** **DIFFERS** for success (city-situation.md §4.5). Failure matches: the active automaton (slot 7), else the target (occupant slot 8), and with neither it skips the effect, the aura **and the money** (`0x00526771` → `0x00526931`), running only the failure script. Success (`0x0052C190`) has no target fallback and no skip: the position is the active automaton's, else the zero vector `0x00B227E0` (0, 0, 0). The effect, the aura (when radius > 0.0f) and the money then all happen.
- ~~**CS10:** (cores12a) automaton notify subtypes 0xB, and 0xC with data3 0x14, drop the automaton from tracking like a
  removed occupant. Is that right? [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** **DIFFERS** (city-situation.md §4.7). Both go to `HandleAutomatonDisabled` (`0x0052C020`; 0xC with any other data3 returns true). It first sets automaton flag `0x10000000` (slot 30), which also makes `CanPlayerDrive` refuse it. With a mission held, the active automaton gives status 2 whatever the current status. Otherwise the target's automaton gives "next target" (types 1 and 3; status 1 when none is left) or status 2 (other types), followed by `HandleStatusChanged`. Only then is a tracked automaton dropped as for a removed occupant.
- ~~**CS11:** (cores12a) slots 23–25 are left inert. Blockers: slot 23's search path (`0x00525640`) is not traced;
  slot 24 posts `0xABBBCA64` without naming the IGZUnknown (which advice?); slot 25 "tracks the automaton
  (`0x005291C0`)" without saying whether that is the §4.2 indicator tracking. [inert]~~ **ANSWERED 2026-10-15 (re-answers15):** all three traced (city-situation.md §8 rows 21–25). Slot 23: when no entry accepts the group and `search` is set, `0x00525640` collects the group's automata from the four automata managers (slot 30 tests the group, slot 32 collects). The first one `CanPlayerDrive` accepts is taken; otherwise `CreateAutomatonForPlayerDrive` (`0x005292F0`), and with none, false. Then `StartJoyride(automaton, true, null)` and true. Slot 24 **sends** `0xABBBCA64` with data1 = 1 and **no** IGZUnknown (a joyride names no advice), then enters the drive view; status and active advice are untouched. Slot 25 runs `HandleInsertAutomaton` (`0x005291C0`), the `0x99EF1142` handler's body: yes, the per-automaton side of the §4.2 indicator tracking (`TrackAutomaton`, kind 4, listener). Inert slots **DIFFER**; the three can now be implemented.
- ~~**DM1:** demolition filter step 7: in a view level other than 1, networks with any flag of `0x1F4F` are refused.
  Is that mask the surface networks, so that only pipes and subways can be demolished underground? [yes, from the bit
  layout. In a turbo run, demolish a rectangle over a road crossing a pipe in the underground view: the road must
  stay and the pipe go]~~ **ANSWERED 2026-10-15 (re-answers11):** **DIFFERS** in part (demolition.md §4.2 step 7). `0x1F4F` is every network type bit except 4 (water pipe), 5 (power line) and 7 (subway), so in a view level other than 1 a network passes only when it carries none of the other nine types: pipes, subways and also type-5 power line pieces. Power poles are a separate occupant type and step 7 does not test them. The bracket's "pipes and subways" is right for the underground networks.
- ~~**DM2:** `ModifyTerrainHeight` (slot 9) returns a vertex count and never writes `*cost`. [as specified; no shipped
  caller passes a cost pointer. A call trace of slot 9 with the level and raise tools would confirm both]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES, now verified (demolition.md §4.7). `0x00467BB0`–`0x00468191` (`ret 0x28`) never loads its tenth argument; the ninth (`reason`) is read for the redisplays (`0x00467F59`, `0x00468157`). It returns the vertex count.
- ~~**DM3:** `0xA9DED179` has a handler and no poster. [no poster in the exe; FreeCity4 keeps the handler. A turbo
  run with demolitions, disasters and terrain edits should log 0 posts]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (demolition.md §3). The ID's bytes occur only at `0x00466A03` (Shutdown), `0x00466F29` (Init) and `0x00467002` (DoMessage) in the whole file.
- ~~**DM4:** occupant flag 1 means "inserted in the occupant manager", so poles removed earlier in the same call are
  skipped. [yes, inferred from the occupant manager's insert and remove paths; flora.md agrees. A breakpoint on the
  occupant manager's insert and remove watching flag 1 would settle it]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES, now verified (demolition.md §4.6). `InsertOccupant` returns early when flag 1 is set (`0x0049EF8E`) and sets it before posting `0x99EF1142` (`0x0049F202`); `RemoveOccupant` needs it set (`0x0049F264`) and clears it after posting `0x99EF1143` (`0x0049F382`).
- ~~**DM5:** (cores12a) is an occupant of foundation type that lacks `cISC4FoundationOccupant` rejected by the filter
  (step 5)? [rejected]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES (choice; demolition.md §4.2 step 5). The original does not test the QueryInterface result (`0x00466817`–`0x00466835`) and calls slot 9 through a null pointer, so it crashes. Every shipped foundation has the interface. Rejecting is the safe *[choice]*.
- ~~**DM6:** (cores12a) should the 0.05 level tolerance (§4.7) be compared after widening `float` to `double`?
  [widened]~~ **ANSWERED 2026-10-15 (re-answers15):** **DIFFERS** (demolition.md §4.7). Both modes compute `|altitude − h|` and compare it with the `float` constant 0.05f (`0x00A880D8` = `0x3D4CCCCD`): area mode counts `> 0.05f` (`0x0046802A`–`0x0046803E`), per-vertex mode skips `< 0.05f` (`0x00467D45`–`0x00467D59`). At 24-bit x87 precision the subtraction rounds to `float`. Use `f32` throughout. A `f64` compare with 0.05 disagrees within one `float` step of the threshold.
- ~~**DM7:** (cores12a) in area mode, is the redisplay rectangle the same as the `SetAltitude` rectangle? [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES (demolition.md §4.7). `SetAltitude` (terrain slot 49, `0x00468172`) and `RedisplayTerrain` (slot 68, `0x00468187`) get the same local rectangle `(max(x0,0), max(z0,0), min(x1,cellsX), min(z1,cellsZ))`. Area mode returns `n` also when the area is locked; an immovable occupant gives 0.
- ~~**DM8:** (cores12a) group effect: are the size thresholds strict or inclusive, and is the centre cell truncated or
  rounded? [the literal reading of §4.8]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES (demolition.md §4.8). The thresholds are strict and unsigned: `+2` if `n > medLarge` (`0x00B07F4C`, `jbe` at `0x004675BB`), else `+1` if `n > smallMed` (`0x00B07F48`). The occupant effect's model thresholds are strict too (`0x0046740E`, `0x0046743C`). The centre is `(x0 + x1) / 2`, `(z0 + z1) / 2` in signed division truncating toward zero (`0x00467555`, `0x00467568`), the same as Rust's `/`.
- ~~**DM9:** (cores12a) which network kinds does `UpdateNetworks` queue, by occupant type? [the literal §4.6 list]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES (demolition.md §4.6), provided the test is the occupant type alone. Type `0x2890D4DE` (pole) is sent `0xAA7A014D` and removed. Every occupant of type `0xC772BF98` is queued (AddRef, then neighbours slot 8), whatever its network kind: there is no QueryInterface and no network-type filter. Any other type is skipped. Flag 1 (slot 32) gates both. The list goes to the network manager's slot 6 (`0x00468396`).
- ~~**DM10:** (cores12a) is the deferred-lot message's lot released even when the lot is not initialized? [always
  released when non-null]~~ **ANSWERED 2026-10-15 (re-answers15):** **DIFFERS** in the letter; the choice is accepted (demolition.md §3). The original releases the lot only on the initialized path (`0x004670A0`). A null or uninitialized lot jumps to freeing the record (`0x004670A7`), and its reference leaks *[quirk]*. Nothing posts `0xA9DED179`, so releasing on both paths cannot be observed.
- ~~**DM11:** (cores12a) pole orphans: the core neither holds nor releases references on them. Is that right? [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES in net effect (demolition.md §4.3, §4.4 step 8). The original holds references: the remembered neighbour poles sit in a vector of auto references, released at the end. Each orphan's QueryInterface reference goes into `orphans` without a further AddRef, or is released when `orphans` is null (`0x0046937B`). Step 8 pops each orphan, inserts it into `occs` and `demolished` (each set takes its own reference), and releases the list's reference (`0x00469031`). The balance is zero. The glue must keep the sets' own references.
- ~~**DM12:** (cores12a) is the view filter still applied when flag `0x2000` (lots only) is set? [skipped for
  lots-only calls; the bare filter is used for the lot-blocking test]~~ **ANSWERED 2026-10-15 (re-answers15):** **DIFFERS** in part (demolition.md §4.4 step 2, §4.5, §4.9). Two parts match: with `0x2000` (or privilege 0), Φ has no view filter, and the lot-blocking test always runs Φ with the view filter cleared. The original still fetches the 3D view's filter (`0x00468473`) and uses it directly in two tests. One is the empty-lot qualification, states 1 and 6 (`0x00468960`–`0x00468974`, view filter slot 4 with `0x278128A0`). The other is the step-5 effect gate (`0x00468D02`–`0x00468D12`, slot 3). §4.5's "no view filter is used" was wrong and is corrected.
- ~~**DM13:** (cores12a) occupant types not listed for effects get effect nibble 0. Is that right? [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES (demolition.md §4.8). The nibble helper (`0x00466A90`) leaves 0 for foundation, construction and other types, and returns true, so the effect still goes ahead with key `cause << 8 | size`; the lookup falls back to the key with low nibble 0. Only a prop whose `0x8A5E5DB8` is missing or false returns false (no effect).
- ~~**DM14:** (cores12a, "DM choice") a `DemolishLots` list that holds only null entries is treated as empty (returns
  true, cost untouched). Acceptable? [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES (choice; demolition.md §4.5). The original calls lot slot 14 on the first entry (`0x004670F9`) and on every entry, with no null test, so any null entry crashes it and there is no original result to match. Treating it as the empty list is the natural choice; with mixed entries the union covers only the non-null ones. Note: a non-empty list whose lots are all non-demolishable is different. It still calls `DemolishRegion` with an empty mask, which writes `*cost := 0` and, when `doIt`, runs `QueueRemovals(true/false)`.
- ~~**DP1:** dispatch types 2, 3, 4 and 7: no shipped source handles them. [unused in the shipped game; a plugin may
  use them, so FreeCity4 keeps them as plain numbers]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (dispatch.md §2). The four `cISC4DispatchSource` classes handle 0 (police `0x006E0210`), 1 (fire `0x006ACFA0`), 5 and 6 (air sources: slot 3 `0x006DF760` compares `+0x14`, set to 5 at `0x006ACE33` and 6 at `0x006DFFA3`). The Mac symbols list no other `CanHandleDispatchType`.
- ~~**DP2:** 409 template saves hold `toolType` 4, while the shipped tool's "none" is 8. [written by an earlier build;
  every value loads the same and is overwritten when the tool opens. Nothing more to settle unless a value other
  than 4–8 turns up]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (dispatch.md §7). This build writes `toolType` only with 8 (`0x0046A116`, `0x0046A4F6`, `0x0046A56B`) or with a handled type in `BeginDispatchTool` (`0x0046B0AA`), so 4 can only come from an earlier build. All values load the same.
- **DP3:** dispatch states: 0 idle, 3 dispatched and 5 working are inferred names; who sets the others? [the automata
  controllers; only state 5 changes `active`. A trace of `UpdateDispatchState` while a fire engine answers a fire in
  a turbo run would list the states in order]
- ~~**DP4:** (cores12a) clearing the available pylons shuts down and releases every pylon. Is that right? [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES (dispatch.md §4.1, "Dropping pylons"). `0x00469E10` walks the vector from the back; each non-null pylon gets `Shutdown` (pylon slot 5, which also releases its source occupant when initialized), then `Release`, then the pop. A pylon that is also the active one is shut down too; the active pointer keeps its own reference.
- ~~**DP5:** (cores12a) clearing the active pylon is `SetActivePylon(null)`: UI state 1, then release. Is that right?
  [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES for `SetActivePylon` (`0x00469630`: old pylon `SetUIState(1)` and `Release`, no `Shutdown`; new pylon AddRef and `SetUIState(2)`) and for `Init` when loaded. Two other paths do different things. The manager's `Shutdown` shuts the active pylon down and releases it, with no UI state. `EndDispatchTool` does not touch the active pylon at all. Station removal and `EndDispatch` clear it with `SetActivePylon(null)`.
- ~~**DP6:** (cores12a) returning a pylon with no source occupant only shuts it down; `EndDispatch` releases the
  record's reference. Is that right? [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** **DIFFERS** in the letter; the choice is accepted (dispatch.md §4.4). `ReturnPylonToStation` (`0x0046AA20`) shuts such a pylon down **and releases it** (`0x0046ACBA`), and `EndDispatch` then releases it again: an over-release *[quirk]*. It is unreachable in shipped play *[inferred]*, so the single release is the safe *[choice]*. Separately, a null pylon or a null dispatch source makes the return do nothing at all.
- ~~**DP7:** (cores12a) when a station is removed, its vehicles are untracked and despawned but not released. Is that
  right? [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** **DIFFERS** (dispatch.md §3). For each record of the station, `DoMessage` (`0x0046A320`) untracks the vehicle in the vehicle view (when the view exists), calls automaton slot 13 (`Remove`, with 0) **and releases the record's automaton reference**. The pylon is cleared from active if needed, shut down and released; the station is released and the record erased. The available pylons of that station are then shut down, released, cleared from active and erased.
- ~~**DP8:** (cores12a) does `RevokeDispatchByPylon` return the pylon to its station only when some source handles the
  pylon's dispatch type? [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES in effect (dispatch.md §4.4). `0x0046AE70` always calls `ReturnPylonToStation(GetDispatchSource(p.GetDispatchType()), p)`, also when no record holds `p`, and the return does nothing when that source is null. AL is `RevokeDispatchByID`'s result for the first record holding `p`, else 0. `RevokeDispatchByID` erases the record whatever `EndDispatch` returned.
- ~~**DP9:** (cores12a) a re-targeted vehicle that already has a view handle is not tracked again. Is that right?
  [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES (dispatch.md §4.2 step 5). On re-targeting, `BeginDispatch` (`0x004698F0`) calls automaton slot 25 (`SetDispatchState(3)`), automata manager slot 17 (a new path, mode 2) and automaton slot 39 (event target) again. It does not track again, so the record keeps its handle. It then sets the pylon's dispatch state from automaton slot 26 and posts **begin** again with the same ID.
- ~~**PD1:** `GetLeastEfficientPort` loops forever with two or more ports. Does anything call it? [no caller in the exe
  and no Lua binding found; FreeCity4 returns the least efficient port (*[choice]*). Searching the shipped Lua
  (`SimCity_1.dat` advisor scripts) for a binding, then a turbo run with two airports and every advisor open, would
  settle it]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (choice confirmed; port-developers.md §4.7). No direct caller, and no Lua binding: the only port names the executable exposes to scripts are the `g_*airport*` and `g_seaport_*` count and volume globals, and no string contains `LeastEfficient`.
- ~~**PD2:** the port's monthly `UpdateOccupantEffects(building, old ratio, new ratio)` adds the usage-ratio change on
  top of the demand effects the building already has as an occupied building. Do both apply? [yes: the port call
  moves the effect by the ratio change only. Logging the demand caps (demand.md §8) before and after an airport's
  first month with known office jobs in a turbo run would settle it]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (port-developers.md §4.3). `UpdateOccupantEffects` removes the effects at the old ratio and adds them at the new one (demand.md §7.1), starting from 0, on top of the effects the occupied building already adds, so a port gives (1 + usage) × its effects *[inferred: port lots take the common occupancy path]*.
- **PD3:** what does lot condition 2 mean on a port lot (set when the port is stressed for more than 90 days)?
  [what it means on any lot (lot-developer.md §5.5): the presentation of a distressed lot; the port developer resets
  it to 0 each month it is not stressed. A turbo run with an airport above 96 % usage for four months, logging
  condition and state changes of its lot, would show whether anything else reacts]
- ~~**PD4:** the seaport type declares an 11 × 5 terrain grid but ships 72 values (12 × 6). Data error, or are the
  dimensions counted in cells while the values are vertices? [data error as far as the code goes: the grid is
  rejected, so seaports never re-level terrain. Plopping a seaport on an uneven coast and comparing vertex heights
  before and after would settle it]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (data error; port-developers.md §4.8). The code counts cells in both places: load needs rows × columns = value count (`0x006EA676`–`0x006EA690`), and use needs rows × columns = the lot rectangle's cell area, one vertex per cell (`0x007013D7`–`0x0070141A`). The declared 11 × 5 is wrong for 72 values; 12 × 6 would load.
- ~~**PV1:** the prop developer's night batch size (`+0xA34`) is not initialised; 225 shipped records hold
  `0xBAADF00D`. [never read while the night mode is 0; *[choice]* FreeCity4 writes 0 for a new city and writes a
  loaded value back unchanged. A new city saved by the original would show what the GOG build writes]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (choice confirmed; prop-developer.md §2). The constructor never writes `+0xA34`, so the original saves whatever the heap block held (`0xBAADF00D` is the debug heap's fill *[inferred]*); there is no fixed value to match, and the field is never read while the mode is 0.
- **PV2:** a tagged prop whose tag names no object of its lot's configuration lowers the block's slot count by one.
  Which props have such tags in practice? [props left over from a different configuration (a lot redeveloped with a
  new configuration, or a lower detail level before the city detail manager's reduce); reproduced. Counting, in a
  corpus city, lot props whose tag matches no object of their lot's configuration would show how common it is]
- ~~**PV3:** `UpdateAllOptionalProps` (slot 7) has no caller. [none in the exe; FreeCity4 implements it as specified
  (every block, x outer). A plugin scan or a call trace in a long turbo run would settle it]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (prop-developer.md §4.2). No direct call targets `0x006F4DB0`; it is reachable only through vtable slot 7. Implementing it as specified is harmless.
- ~~**PV4:** the block step runs on every Idle delivery, also while paused, so props develop at a frame-rate-dependent
  speed. Does the original really grow optional props while paused? [yes, from simulator.md §5.4. In a turbo run,
  pause a fresh city with zoned growth for 60 s of real time and log `0x99EF1142` for props carrying `0x89A1C16C`:
  posts while paused confirm it]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES. The simulator delivers Idle whenever `hiddenPauseCount ≤ 0`, with data2 = (pauseCount > 0) (simulator.md §5.4), and the prop developer's Idle handler ignores the message data (prop-developer.md §3), so player pause does not stop the block step.
- ~~**PV5:** the requester index stored on a prop is not bounds-checked, and `RemovePropRequester` shifts later
  indexes. [only pollution removes its requester, at `Shutdown`; an index ≥ the requester count cannot occur in the
  shipped game. *[choice]* FreeCity4 treats it as "no index". Nothing to settle in game]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (choice confirmed). Static: only pollution removes its requester, at Shutdown; nothing to settle in game.
- ~~**PV6:** a power change switches night-lit **street** props at once but leaves night-lit **lot** props until the
  next dusk, dawn or switch. [reproduced, from the exe. At night in a turbo run, cut the power line to a lit block and
  watch a lit lot prop and a lit street lamp: the lamp goes dark at once, the lot prop at dawn]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (reproduced from the exe, prop-developer.md). What remains is a visual check, not a behaviour question.
- ~~**CD1:** which lot configuration objects are dropped at each city detail level? [the lot configuration manager
  filters objects by the detail preference when it builds configurations (it passes the preference to the
  configuration's reader); the exact field belongs to a lot configuration spec. Comparing one lot configuration's
  object count at detail 1 and 3 would settle it]~~ **ANSWERED 2026-10-15 (re-answers11):** Answered (city-detail.md §3, "Which objects a detail level keeps"). `ConvertPropertiesToObjectArray` (`0x00479D10`) keeps an object when its type byte is 0 (building) or `(value1 & 0x30) < 16 × d` (`0x00479EB1`–`0x00479EC6`): level bits `0x00` at every detail, `0x10` from medium, `0x20` only at high, `0x30` never. Network lot configurations use d = 3 (`0x0060F80E`). MATCHES the bracket; the field is value 1's bits 4–5. Confirmed independently by re-specs13d (prop-lotconfig.md §B3.3, with the shipped counts: 73,832 / 39,781 /
  23,265 objects kept at detail 3 / 2 / 1, and the rebuild at the next city `Init` after the preference changes).
- ~~**CD2:** after a detail change every network lot is redeveloped (network lot manager slot 12). Does that change
  saved state, such as street props drawn from the network lot manager's generator? [yes, inferred: redevelopment
  re-creates the network lot's objects. Saving a city, reopening it at another detail level and saving again, then
  comparing the street prop records, would settle it]~~ **ANSWERED 2026-10-15 (re-specs13c):** the bracket
  MATCHES. `RedevelopLot` (`0x0060D5E0`) removes the cell's street props (prop manager slot 33) and flora (flora
  slot 20), then `CreateLotObjects` draws again from generator `0x00B44870` for every prop and flora object with
  candidates, so the prop manager's and the flora's saved state change; the network lot record (component 11)
  does not. network-manager.md §8.6 and §17.

## From lot creation, the plop test, the lot developer and the graphics formats (re-specs11 lane, 2026-10-15)

Open points from zones.md (§5.2, §7.1.1, §8.4, §8.5.1), lot-developer.md §17 and graphics-formats.md §3.9 and
§3.11. The current answer is in brackets; each entry says what would settle it. "Turbo run" means the in-game
harness with the FreeCity4 turbo clock and a message log.

- ~~**LD1:** is the occupant manager's box query inclusive at the box edges? Lot developer slot 50
  (`GetFoundationByPosition`) queries a degenerate box (a vertical line), which finds anything only if the test
  keeps a point inside the occupant's box. [inclusive or strict, unknown]~~
  **ANSWERED 2026-10-15** in this lane from the exe (slot 36 `0x004A0670`: one-cell path `0x0049D8B0`, many-cell
  path `0x004A08CC`–`0x004A0938`; six `fcomp` tests, each keeping the occupant only when its minimum is below the
  query's maximum and the query's minimum is below its maximum) and written into lot-developer.md §17.5.1 and test
  34. **Strict on every axis**, as flora.md §6.4.2 already states: the line at `(x − 0.05, z + 0.05)` finds a
  foundation only when it lies strictly inside the foundation's box, and only through the index cell of that
  point. A foundation whose edge is exactly on the line is not found.
- ~~**GF13:** what does a visual effect do with a NaN orientation? An identity REGP record (zero axis) makes the
  model-effects update divide by zero, so `SetTransform` receives a NaN rotation (graphics-formats.md §3.11; 344
  shipped records). [unknown; *[choice]* FreeCity4 uses no rotation for a zero axis. Watching one of the 1,320
  `blue_blinking_light_slow` effects on a bridge in the original would settle it: drawn in place, drawn elsewhere or
  not drawn. The effect side (`cISC4VisualEffect` slot 8 and the effect renderer) is not specified]~~ **ANSWERED 2026-10-15 (re-answers11):** **DIFFERS** in part (graphics-formats.md §3.11, "What the effect does with it"). The effect's `SetTransform` (slot 8, `0x005BE880`) composes the incoming transform with its own placement in its transform node (`0x005BEC20`): the composed translation never uses the incoming rotation and stays exact (`0x005BEDF3`–`0x005BEEA9`), but the composed rotation and the reference point are NaN (`0x005BF307`–`0x005BF388`, even with the zeroed offset). *[inferred]* particles placed through the rotation are not drawn; the bracket's "no rotation" draws them at the right place. The choice is acceptable as a cleaner reading, since the in-game look is still the only way to settle what the player sees; it is marked *[choice]* and not *[compat]*.
- ~~**GF14:** the automata's per-tick animation driver. graphics-formats.md §3.9 traces the prop occupant's driver
  (animation tick `0x86AD10EE`, milliseconds × time dilation) and `SetAnimMode(1)` from `cSC4Automaton::AttachAnim`,
  but not what an automaton passes to `Advance` (slot 15) each tick, nor when it calls `SetFrame` (slot 10) or
  `SetSpeed` (slot 14). [inferred: the same tick message and dilation as props; nothing in saves depends on it.
  Settled by tracing the automaton's `DoAnimationTick` or equivalent and its calls through instance slots 10, 14
  and 15; the fire crew's `DoAnimationTick` (`0x00549CF1`) is the place to start]~~ **ANSWERED 2026-10-15 (re-answers11):** **DIFFERS** (graphics-formats.md §3.9, Automata). The automata manager calls each automaton's `DoAnimationTick(µs, ms)` from the same tick `0x86AD10EE` (`0x00506AFC`–`0x00506B14`, call `0x0050A89C`); the base step (`0x005089F0`) passes the tick's milliseconds with **no time dilation**, does nothing while paused (unless emergency-paused), calls `Advance(ms)` on each instance, sends the end-of-animation notification (slot 102 with 1, 0) when events are queued and the first instance is on its last frame, and adds ms to `+0x1C0`. Pedestrians (`0x00544C90`) call `SetSpeed(0.01)` before each `Advance` (10 frames per second). No per-tick `SetFrame`. Nothing in saves depends on it.
- ~~**ZQ14:** who posts InsertOccupant with data4 = 1, 2 or 3? The zone developer joins dead-end stubs on 1 and 2
  (`JoinDeadEndStubs`, zones.md §5.2.1) and joins across a rail line on 3 (`JoinAcrossRail`, §5.2.2). [inferred:
  the network tools' real-mode Place, with a value per network kind; not traced. Settled by finding the callers of
  occupant manager slot 20 (`InsertOccupant`) that pass a non-zero data4, or in a turbo run by logging data4 of
  `InsertOccupant` messages while laying a road, a street and a rail line]~~ **ANSWERED 2026-10-15 (re-answers11):** **DIFFERS** (zones.md §5.2). data4 is `v` of occupant manager slot 20 (`0x0049EF60`, stored at `0x0049F205`–`0x0049F214`); the network tool passes its field `+0x208`: 1 for a network the player draws (all kinds), 3 inside tool slot 7 `PlaceNetworkOccupants` (`0x0062924A` sets 3, `0x00629980` resets 1), i.e. every network laid by code (zone developer, zone manager PlaceZone, network manager lot connections, tract developer, regional connections, BDS terrain change); 0 for the remove-and-reinsert in `InsertOccupantsForSolvedCells` (`0x00625AEE`) and other managers. Nothing passes 2. It depends on the call path, not on the network kind.
- ~~**ZQ15:** two values of the zone developer's preview and plop test are not explained. (a) The terrain view's slot 4
  (zones.md §7.1.1) takes seven arguments; the last two are always 3 and 0 here. (b) `CanCreatePloppedLot`
  (§8.4.1) rejects a cell whose network has every bit of `0x200000`, and blocks an outward connection to a network
  with any bit of `0x600000`. [(a) unknown, *[inferred]* a draw layer and a flag; the preview is UI and a headless
  replacement leaves it out. (b) the flag meanings belong to traffic-network.md (its Q3); the codes are reproduced
  from the bits without needing the names. Settled by the terrain view's slot 4 and by the network spec naming the
  two bits]~~ **ANSWERED 2026-10-15 (re-answers11):** (a) **DIFFERS** (zones.md §7.1.1): slot 4 is the textured-cell `MarkSelected` (`0x007526E0` → `0x00751A70`, pop 28); 3 is the blend (combiner) op, replaced by op 1 at alpha 0.5 when a renderer capability byte is set (`0x00751B57`–`0x00751B83`), and 0 means "do not clear the marked cells first" (`0x00751A7C`). Still UI only; leaving it out headless is fine. (b) **MATCHES** (the codes need only the bits), and the names are now in zones.md §8.4.1 and traffic-network.md Q3: `0x200000` = connection node (set by `CreateConnectionNode`, `0x00611486`); `0x400000` = pre-built-model piece not placed by the network tool (set by the `cSC4NetworkOccupantWithPreBuiltModel` constructor at `0x0061E2BC`, cleared by `InsertNetworkOccupant` at `0x006279F2`; name *[inferred]*).
- ~~**ZQ16:** lot-records.md §16 Q2 (vacate-reason values above 6 in saves) has an answer that lot-records.md does not
  carry yet. [**answered** in zones.md §8.5.1 *[inferred]*: the lot constructor `0x006C1E50` does not write the
  vacate reason (`+0x83`) and the pool's free path writes only the block's first dword, so a new lot inherits the
  byte of whatever last used its block, and a lot that has never been vacant saves that stale byte. The owner of
  lot-records.md should cite zones.md §8.5.1 under Q2. Settled for good by creating lots in a fresh city in a turbo
  run and reading field 17 of never-vacant lots: values that change with the order of earlier demolitions confirm
  it]~~ **ANSWERED 2026-10-15 (re-answers11):** **MATCHES**. Re-checked: the lot constructor `0x006C1E50` writes the byte at `+0x80` (`0x006C1F19`) but nothing at `+0x83`. lot-records.md §16 Q2 now cites zones.md §8.5.1.

## From the zone developer / lot manager glue, Stage A (filed by the coordinator from the `zones2` lane's report, 2026-10-15)

About `docs/specs/zones.md` §1.2, §7-§11.3 and the replacement `cFC4ZoneDeveloper` (`enable = zonedev`) with its shadow
(`zonedev_shadow = 1`); merged as `9a9b0df`, ABI 51. The lane numbered these ZONES-ZD1..ZD13. The current answer is in
brackets. ZG23, ZG24 and ZG27 were asked before re-specs11 merged; zones.md §8.5.1, §7.1.1 and §8.4.1-§8.4.3 now cover
them, and Stage B applies those sections.

- ~~**ZG22:** lot manager slot 19 `GetLotSavvyCellAltitude`: does it write `*ok`? §8.7 does not say. [`*ok` left
  untouched]~~ **ANSWERED 2026-10-15 (re-answers11):** **DIFFERS** (zones.md §8.7, slot 19). When `ok` is non-null it is always written: 1 for a lot with flag bit 0 (`0x0072AAEF`), 0 for the terrain path (`0x0072AB06`).
- ~~**ZG23:** lot construction (§8.5 step 1): the lot object's allocation and constructor arguments. [Stage A: `new_lot`
  builds nothing, CreateLot fails and is logged; **answered by re-specs11** in zones.md §8.5.1 (pool `0x00B0D3AC`,
  allocate `0x006BF430`, constructor `0x006C1E50`); Stage B implements it]~~ **ANSWERED 2026-10-15 (re-answers11):** Answered by re-specs11 (zones.md §8.5.1: pool `0x00B0D3AC`, allocation `0x006BF430`, constructor `0x006C1E50`); nothing to add. Stage B implements it.
- ~~**ZG24:** `HighlightParcels`' `*out` region (§7.2, §13.2): its allocator and the caller that frees it are not
  traced. [`*out = null`, logged once; zones.md §7.1.1 now describes the highlight textures; check whether it also
  settles the region's owner]~~ **ANSWERED 2026-10-15 (re-answers11):** Answered (zones.md §13.2, the HighlightParcels row). The only caller that passes `out` is `cSC4ViewInputControlZone::DoCursorUpdate` (`0x004DBDCE`), which keeps the region in its `+0x58` and frees the previous one (rows and object) when it stores the next (`0x004DBDD1`–`0x004DBDF3`). `ClearHighlights` (`0x004DAE2D`) and `ComputeAndMarkSelectedArea` (`0x004DB734`) pass null. So the zone tool owns it; `*out = null` is safe only for a glue with no zone tool.
- ~~**ZG25:** §5.2 step 3, the street-joining helpers `0x00731F60` / `0x00732450`, are not specified. [no-op plus a log
  line; zones.md §5.2.1-§5.2.2 (`JoinDeadEndStubs`, `JoinAcrossRail`, ZQ14) may cover them]~~ **ANSWERED 2026-10-15 (re-answers11):** Answered by zones.md §5.2.1 (`JoinDeadEndStubs`) and §5.2.2 (`JoinAcrossRail`), with the data4 values in §5.2 (ZQ14): 1 for player-drawn networks, 3 for networks laid by code. The no-op is now a gap Stage B can close.
- ~~**ZG26:** the PlopAllLots / PlopAllBuildings cheats (§1.2 step 7): their bodies are not specified, and the host has
  no cheat-manager registration call. [not registered]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES (zones.md §5.4, now specified). Both cheats are registered only when the debug-functionality flag (app slot 21) is set, so a shipped game without it never reaches them. `PlopAllLotTypes` (`0x0072C8E0`) places one lot per lot configuration in rows from (0, 0). `PlopAllBuildingTypes` (`0x0072BE50`) places one generic lot per building type on a spiral from the map centre. Both use `CreateLotIfPossible(…, true, false, cfg)` followed by `ConstructDefaultLot`. Leaving them unregistered is a safe *[choice]*.
- ~~**ZG27:** slot 21 `CanCreatePloppedLot` (§14.3 Q2). [Stage A returns 0, logged; **answered by re-specs11** in zones.md
  §8.4.1-§8.4.3 (codes 0, 1, 3-8); Stage B implements it]~~ **ANSWERED 2026-10-15 (re-answers11):** Answered by re-specs11 (zones.md §8.4.1–§8.4.3), with the two network flags now named (ZQ15); nothing to add.
- ~~**ZG28:** slots 11 and 24-26 leave EAX undefined in the original. [return true]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (choice). The original leaves EAX undefined, so no caller can depend on it; returning true is safe.
- ~~**ZG29:** the budget and traffic glue walk the original lot manager's raw list (the list object at `lm+0x5C`). Our
  master list keeps the same raw layout and list object, but not at `+0x5C` inside our object. Does any caller read the
  member directly rather than through slot 15? [assumed none; not traced]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (zones.md §8.3). No load of the lot manager from `0x00B43D08`, or from city slot 33 (`+0x84`), is followed by a read of `+0x5C` anywhere in the executable, and the budget's tax pass reaches the list through slot 15 (budget.md BQ6). Every other class goes through the interface.
- ~~**ZG30:** parcellization has no dry run (`HighlightParcels` draws, posts and places street drafts). [the shadow
  compares the lookups and CanCreateLot only]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES (zones.md §7.1). The executable has no side-effect-free parcellization. `HighlightParcels` is the closest, but it still draws random numbers (§9), posts both DeferOccupantOperations messages and places street drafts. A shadow cannot call it without moving the zone developer's generator `0x00B4B398`. Comparing the lookups and `CanCreateLot` is the right scope.
- ~~**ZG31:** a traffic insertion-point iterator from a foreign list at CreateLot (§8.5 step 4). [the lot is appended,
  logged]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES (zones.md §8.5 step 4). The iterator is the traffic simulator's safe lot iterator, taken at its `Init` from slot 15 of the lot manager installed at that time (traffic-network.md §4.14). It is returned only in work-cycle state 1, which the cycle never enters, so it is null in practice and appending is exact. A foreign iterator is reachable only if the traffic simulator's `Init` ran against the original manager.
- ~~**ZG32:** reference audit for the manager's last lot reference at DeleteLot / Shutdown. [`FC4_RELEASE_AT_DESTRUCTION`,
  as DG28]~~ **ANSWERED 2026-10-15 (re-answers12):** **DIFFERS** in timing (zones.md §1.2 Shutdown). The original deletes every lot in `Shutdown`, not in its destructor: `0x0072B8D0` (called at `0x00733663`) walks the cell map x outer, z inner, and calls its own `DeleteLot` (slot 24) on each non-null cell. That runs the full §8.6 sequence: zot prop, base texture, cells, master list, business unit, `lot.Shutdown()`, and the manager's release. The expected-release marker therefore belongs at `Shutdown`'s `DeleteLot`, as at any `DeleteLot`.
- ~~**ZG33:** another thread that cannot take the lock within 20 ms. [neutral answers, except the snapshotted lot count
  and grid; counted as a fallback]~~ **ANSWERED 2026-10-15 (re-answers12):** Not an RE question; the bracket stands. The original lot manager has no lock: every caller runs on the simulation thread, so there is no original behaviour to match.
- ~~**ZG34:** with `enable = zonedev` and CreateLot refused, the original tract developer's Grow (2026-01-30 on
  Konradshohe) gives two "over-release of game object [class 0xC9BD5D4A] at ReleaseToken" warnings (DeleteLot's
  manager release, §8.6 step 6) and then an access violation at `0x0070690C` / `0x00AB1995` (runs `zonedev-enable-1`,
  `-2`). Does the game's growth path tolerate a failed CreateLot after DeleteLot, or must lot creation never fail
  there? [not resolved; re-test after Stage B builds lots, then audit Grow's delete-and-recreate reference flow (slot 7
  caller references against the manager's); **Stage B (2026-10-15):** with lots built it does not reproduce (`zonedev-enable-final` clean, ref_warnings=0, 68 lots built, 0 refused); the refusal path is still logged. Open part: is a refused CreateLot on the growth path ever expected?]~~ **ANSWERED 2026-10-15 (re-answers12):** **Answered: lot creation must not fail there** (tract-developer.md §4.15.3). After a failed `CreateLotIfPossible`, `Build` returns 0 and the pass continues with the next candidate (§4.3). Candidate lot sets hold borrowed pointers (`GetLot`, no AddRef), and overlapping aggregation candidates share lots, so the next aggregation runs `PerformAggregation` step 1 on lots that were already deleted and freed. That gives the two over-release warnings at the second `DeleteLot` of the same lot, then the AddRef at `0x0070690C` through a stale vtable (`0x00AB1995` is data, not code). The original clears the rectangle itself first, so in practice step 4 succeeds there. Stage A must not let Grow reach aggregation while it cannot create lots; Stage B's working `CreateLot` removes the cause. Stage B's clean run (`zonedev-enable-final`) agrees: with `CreateLot` working the path is not reached, so a refused CreateLot on the growth path is never expected.

## From the zone developer / lot manager glue, Stage B (filed by the coordinator from the `zones2` lane's report, 2026-10-15)

About `docs/specs/zones.md` §5.2.1-§5.2.2, §7.2.2, §8.4.1-§8.4.3, §8.5.1 and their implementation in `cFC4ZoneDeveloper` /
`FC4ZoneDevHost` (`enable = zonedev`); merged as `3cb9b43`, ABI 55. The lane numbered these ZONES-ZD1..ZD19; ZD1-ZD13
repeat ZG22-ZG34. Stage B did not apply two re-answers11 answers: ZG22 (slot 19 always writes `*ok`; ours leaves it
untouched) and ZG24 (the zone tool owns the highlight region; ours returns null with the zone tool present). The current
answer is in brackets.

- **ZG35:** §8.5.1: the lot constructor does not write the vacate reason at `+0x83`, and the pool block is not zeroed, so a
  new lot carries a stale byte there. [choice: FreeCity4 writes 0 after construction]
- **ZG36:** §8.4.1 `RotateLotRect` with a facing outside 0..3. [choice: the rect is left unrotated]
- **ZG37:** §8.4.2 / §8.4.3: a constraint or network configuration object whose data vector has fewer values than the
  check reads. [choice: missing values read as 0]
- **ZG38:** §8.7 `PositionToCell`: the core computes `floor(p / 16)`; does the original round differently at negative
  positions or at the map edge? [floor]
- **ZG39:** §1.2 Shutdown and §5.2 PostCityInit: `GetNetworkTool(type, 1)` returns a tool whose only reference is the zone
  developer's AddRef, so Shutdown's Release frees it (the lane's game runs show this). The spec does not state the
  ownership. [`FC4_RELEASE_CREATED`]
- **ZG40:** `UpdateHighlightTextures` (§7.1.1), which the spec allows to be omitted, is not implemented. [omitted]

## From the history warehouse and 24-hour clock glue (filed by the coordinator from the `histclock` lane's report, 2026-10-15)

About `docs/specs/history.md` and `clock24.md` and the replacements `cFC4HistoryWarehouse` (`enable = history`) and
`cFC4Clock24` (`enable = clock24`) with their shadows (`history_shadow = 1`, `clock24_shadow = 1`); merged as
`efac89a`, ABI 52. The lane numbered these history 1, 7, 8, 11, 12 and clock24 2-6, 9, 10. The current answer is in
brackets.

- ~~**HW13:** `Release` on a warehouse that was initialised but never shut down (history §1): does `Release` at a count
  of 1 or less run `Shutdown` first? [`Shutdown` first, then delete]~~ **ANSWERED 2026-10-15 (re-answers11):** **DIFFERS** (history.md §1). The original's `Release` (`0x00476AA0`) does not call `Shutdown`: its destructor frees only the series map (`0x00476918`–`0x00476952`) and leaves the subscriptions, relying on the city's `Shutdown` call. Running `Shutdown` first is an accepted *[choice]* that changes nothing in the normal order.
- ~~**HW14:** when the history shadow sees the original's 5 ms timer split a yearly-pass step (the original answers
  false where the shadow answered true), what is the intended comparison? [the shadow skips the original's repeats
  until the original answers true and counts `split_steps`; 0 in every lane run]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES (history.md §4.3). The pass's timer is a local `cRZTimer` with unit 4, milliseconds (`0x0088FEDF(4)` at `0x00475E56`; no longer *[inferred]*). It is read after every cursor advance (`0x00475F1B`), and the call stops at 5 or more (unsigned, `0x00475F27`). The split depends only on real time; the coalesced keys, their order and the final state do not. Skipping the original's repeats until it answers true, and comparing the result, is the right comparison.
- ~~**HW15:** the warehouse's raw save record cannot be read in game from outside the class store. [the load comparison
  uses the original's own `Write` into memory; the save sweep (`engine/formats/tests/save_records.rs`) checks the raw
  records of the 417 stock saves]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES (history.md §7). No slot returns the raw record, and `Write` (`0x00475D2C`) is the only serialiser, so the original's own `Write` into a memory stream is the read path. The in-memory layout is now in §2 for a direct read.
- ~~**HW16:** queue lengths set through slot 13 (the producers' `Init`, not a message) are invisible to the history
  shadow. Should there be a read path for them? [a series that differs only by its queue length is named and counted
  as a mismatch; none occurred]~~ **ANSWERED 2026-10-15 (re-answers12):** Answered (history.md §2). No slot returns a queue length, but the shadow already has two read paths: the original's `Write` (HW15) writes each series' `maxCount`, or the tree can be read directly (key at node `+0x10`, points list `+0x14`, `maxCount` `+0x18`; `0x00476FF2`, `0x00475EC1`). Naming such a series as a mismatch until then is fine.
- ~~**HW17:** 403 of the 417 stock saves hold version-1 history records, which cannot be rewritten byte for byte (the
  writer always writes version 2; a queue length of 100 reads as 0; unset or repeated dates are folded). Is converting
  on the first save the intended behaviour, since the original also writes version 2? [yes: the sweep reports them as
  converted and checks that the conversion is stable]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (history.md §7). The original's `Write` always writes version 2 (`0x00475D2C`), so it converts a version-1 record on its first save too.
- ~~**CK16:** `cIGZSystemService::OnTick`: the value the clock's system service returns is not in clock24.md. [true, the
  SDK default]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (clock24.md §8). The sub-object's slot 10 is the shared default `0x0091559A`: AL = 1, pops 4.
- ~~**CK17:** `GetServiceTickPriority` is not specified. [the service priority, -2,000,000]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (clock24.md §8). Slot 12 (`0x009D4714`) returns the tick priority, which the base constructor (`0x0091DE0F`) sets to the same −2,000,000 as the priority (`0x0091DE34`, `0x0091DE37`).
- ~~**CK18:** does the system-service base's `QueryInterface` answer any IID besides `0x287FB697` (for example IID 1
  through the +0xC sub-object)? [only `0x287FB697` through the base; the object's QI table answers the other four
  IIDs]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES, with one addition (clock24.md §8). Slot 0 of the `+0x0C` sub-object is a thunk to the clock's own `QueryInterface` (`0x00445A80` → `0x00444E40`), so every pointer answers the same five IIDs: the four of §1, and `0x287FB697` through the base (`0x0091DD67`). IID 1 gives the primary pointer, and any other IID returns false with `*out` unwritten (`0x009457DC`).
- ~~**CK19:** slots 5, 8 and 11 have no meaningful return value in the spec. [return 1]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (choice; clock24.md §8). The original leaves EAX as a by-product (slot 8 the day quotient at `0x0044549B`, slots 5 and 11 the last call's result), so returning 1 is safe.
- ~~**CK20:** if a tuning property (`0x897E709F`, `0x09B73421`, `0x09B73422`) is stored as an array, which value is used?
  [the first value]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (clock24.md §6). All three are read with the converting getters (Float32 `0x005FD4F0`, Uint32 `0x005FD480`), so an array gives its first value. An array of count 0 gives 0, and a float hour is truncated (data-formats §5.1).
- ~~**CK21:** the original's timer at +0x50 (a `cRZTimer`) has no read path or layout in the spec, so the clock shadow
  cannot replay real elapsed time. [the shadow copies each of the original's steps through a synthetic elapsed time at
  magnification 1 (a game-time jump of 61 to 1200 s between idle passes is one step); any other game-time change is a
  jump (`SetHour`, `SetMinute`, `SetSeconds`); messages and derived values are compared]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES, and the layout is now specified (clock24.md §4.3 timer layout). `cRZTimer`: `+0` unit, `+8` start tick (u64, 0 = stopped), `+0x10` accumulated ticks (u64). Unit 3 is QueryPerformanceCounter ticks × 10^6 / frequency, so microseconds (`0x0088FFFF`; no longer *[inferred]*), less a calibrated per-interval overhead (`0x00B54358`). After each step the start tick at clock `+0x58` holds the step's moment, so a shadow can bound each step's real elapsed time. The value read inside the step is not stored, so the synthetic-elapsed replay stays the exact method.
- ~~**CK22:** under the turbo clock the framework idle loop is starved (about 24 `OnIdle` passes in 29 s), so the clock
  advances only about one game hour per turbo run and reaches no day/night boundary. Is that acceptable for the
  harness, or should turbo give the idle loop time? [nothing changed; normal-speed runs cover the clock (598 steps,
  323 posts in 330 s)]~~ **ANSWERED 2026-10-15 (re-answers12):** Not an RE question; the bracket stands. Exe fact for the decision (clock24.md §3.1): the clock advances only in `OnIdle`, and a pass advances it by at most 1200 game seconds (`0x00445874`). So a starved idle loop slows the clock in game time too; nothing else advances it. So 24 passes advance the clock by 8 game hours at most, and 12 game hours need at least 36 passes. Normal-speed runs covering the clock is a sound choice.

## From the commercial, industrial, civic and park glue (filed by the coordinator from the `cmglue` lane's report, 2026-10-15)

About `docs/specs/commercial-industrial.md`, `civic-buildings.md` and `parks.md` and the replacements
`cFC4CommercialSimulator` and `cFC4IndustrialSimulator` (`enable = commercial, industrial`),
`cFC4CivicBuildingSimulator` (`enable = civic`) and `cFC4ParkManager` (`enable = parks`) with their shadows
(`commercial_shadow = 1`, `civic_shadow = 1`, `parks_shadow = 1`); merged as `58b6d11`, ABI 53. The lane numbered
these CIV-1..CIV-6, PKG-1, PKG-2 and SWEEP-1. CIV-4 is the CB6 answer, applied: civic `Read` inserts in file order at
the head of each chain (a repeated exclusion is a no-op, a repeated status key's last value wins in place), conformance
`cb12_09` and `cb12_10` follow it, and the civic shadow pre-reverses the chains of the original's captured record. The
current answer is in brackets.

- ~~**CB15:** Lua `civic.get_landmark_count` has no published registry node (the budget and aura glues stub
  `register_lua` too). [logged as not published (stub)]~~ **ANSWERED 2026-10-15 (re-answers12):** **DIFFERS** (civic-buildings.md §4.1 step 6). The original publishes a game-data registry node, as the budget does: node `"civic"` under the root (registry slot 5, key = the simulator) and a call entry `"get_landmark_count"` (slot 3, key and callback `0x0069D740`, flags 1, context = the simulator). The callback writes `GetLandmarkCount()` as a Uint32 variant into output parameter ID 1 (`cIGZCommandParameterSet` slot 7). `Shutdown` removes the node (slot 7). The stub can become this recipe once the CB16 rows exist.
- ~~**CB16:** `docs/abi` lists only slots 3, 5 and 6 of `cIGZCommandParameterSet`, so a Lua getter cannot append its
  output parameter. [not published until the rows exist]~~ **ANSWERED 2026-10-15 (re-answers12):** Answered (rows in `build-runs/re-answers12-abi-rows.md`; `docs/abi/vtables.tsv` already lists the whole vtable `0x00AD16C8`). The two slots a getter needs: 7 `SetParameter(uint32 id, const cIGZVariant&)` (`0x0092ABEB`, pop 8; find-or-insert by ID, then assign) and 8 `AppendParameter(const cIGZVariant&)` (`0x00929E19`, pop 4; ID = last ID + 1, or 0 when empty). The civic callback uses slot 7 with ID 1 (civic-buildings.md §4.1).
- ~~**CB17:** civic slot 7 `UpdateConditionalBuildingStatus`: the spec does not say how to read the caller's tooltip.
  [read as a `cRZString`-layout begin/end pair; the status is copied, never written back]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES (civic-buildings.md §4.7). The source struct is read in place: byte `+0`, u16 `+2`, and the `cRZString` at `+4` whose STLport begin / end pointers are at `+8` / `+0xC`; the range is assigned unless it is the same string (`0x0069EA1D`). Nothing is written back.
- ~~**CB18:** civic `Write` returns true when the stream refuses strings (`SetGZStr` false), so a stream without string
  support writes a short record with success. Is that the original's behaviour? [as the spec says: the result is the
  stream error state only]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES (civic-buildings.md §7). `Write` (`0x0069D920`) ignores every stream call's result, `SetGZStr` included (`0x0069DA1E`), and returns `!stream.GetError()` (stream slot 18, `0x0069DA88`). Whether a stream that refuses strings also sets its error flag is the stream's own behaviour.
- ~~**CB19:** the in-memory layout of the 193-bucket status map beyond the bucket order (node allocation, erase) is not
  specified. [a game-layout hash map whose values keep a fixed address, merged field by field with the core; the core
  keeps the 193 order]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES (civic-buildings.md §2). Nothing outside the class touches the table; slot 6 hands out the node's value address. Layout for reference: table at `+0x2C`, buckets at `+0x30`/`+0x34`, count at `+0x3C`; 0x20-byte nodes from allocator B (`+0` next, `+4` key, `+8` status, `+0xA` count, `+0xC` `cRZString`), so a value's address never moves while its node lives.
- ~~**PK10:** in game the original park manager's threshold reads 0.25 (the code default), not the exemplar value, so
  the PK1 quirk is not observed. Why does the original not take the exemplar tuning? [the glue follows the spec
  (exemplar tuning at the first `Init`); the shadow follows the original "as without exemplar" and says so on every
  line]~~ **ANSWERED 2026-10-15 (re-answers12):** **DIFFERS**: the spec was wrong, and the original does take the exemplar (parks.md §4.4, §6, tests 2 and 6 corrected). The threshold is read with the **percent** getter `0x005FDAC0` (`0x006D59EB`), which multiplies by `0.01f`: 25.0 becomes 0.25 exactly, which is what the game shows. The glue and the shadow should read `0xCA480830` × 0.01f; with the shipped data both give 0.25, so only a modded exemplar would differ.
- ~~**PK11:** `fc4_park_destroy` and `fc4_civic_destroy` do not release the list entries of a never-initialised object
  (as the original's destructor, CB7), which leaks the references a `Read` took if `Init` never runs. [as the spec
  says]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES (parks.md, civic-buildings.md §8). The park manager's destructor (`0x006D5860`) frees the nodes of both lists (`0x00413FE0`) without releasing their entries, and the civic one does the same (CB7). A never-initialised object that read references therefore leaks them in the original too.
- ~~**CM10:** the save sweep for the commercial, industrial, civic and park records runs in `fc4-capi`
  (`capi/src/save_sweep_tests.rs`, over the C ABI); histclock's record mode is in `fc4-formats`
  (`formats/tests/save_records.rs`). Should the four records move there, over core-level hosts (about 100 host methods
  and object-ID-keeping streams)? [they stay in `fc4-capi`; both read `FC4_SAVES_DIR` and both READMEs point at each
  other]~~ **ANSWERED 2026-10-15 (re-answers12):** Not an RE question: the original has no counterpart. Keeping the four records' sweep in `fc4-capi` is the coordinator's choice; the bracket stands.

## From the lot developer glue (filed by the coordinator from the `lotglue` lane's report, 2026-10-15)

About `docs/specs/lot-developer.md` and the replacement `cFC4LotDeveloper` (`enable = lotdev`, host
`FC4LotDevHost`) with its shadow (`lotdev_shadow = 1`); merged as `5212df2`, ABI 54. The lane numbered its first
questions LD-Q1..LD-Q11; they are filed here as LD2..LD11 (LD-Q9 is left out: it was superseded by LD1, answered
strict, and the conformance world's slot 37 query is now strict). LD12..LD30 keep the lane's numbers, which the code
comments and the game log cite. The current answer is in brackets.

- ~~**LD2:** §16 test 30 says a 2-cell-wide rectangle makes no slot 19 call, but §17.4 step 2 gives it one interior
  column of vertices. [implemented as §17.4 is written: only a 1-cell-wide rectangle gives no call]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES (lot-developer.md §17.4, §16 test 30 corrected). Slot 19 (`0x006C43D0`) walks `x1+1 … x2`, `z1+1 … z2`, so only a 1-cell-wide rectangle has no interior vertex; a 2-cell-wide one checks its one interior column. Test 30 was wrong: the "2-cell" skip is slot 55's own guard (`x2 − x1 > 1` and `z2 − z1 > 1`) before it calls slot 19.
- ~~**LD3:** the retaining-wall helpers `0x005E3070`, `0x005E3960` and `0x005E37F0` have no rows in direct-calls.md.
  [host calls the plugin cannot make, so no walls are built; logged once]~~ **ANSWERED 2026-10-15 (re-answers12):** Answered (lot-developer.md §7.2, ABI rows in `build-runs/re-answers12-abi-rows.md`). The three are `__cdecl` statics of `cSC4LotRetainingWallOccupant` taking the `cISC4Lot*`, caller pops: `AddRetainingWallOccupant(lot, bool add)` `0x005E3070` (8 bytes, AL; callers `0x006C8A3D`, `0x006C8C23`), `UpdateRetainingWalls(lot)` `0x005E37F0` (4 bytes, AL a leftover), `UpdateSurroundingRetainingWalls(lot)` `0x005E3960` (4 bytes, no return). They use the global occupant manager, lot manager and BDS, so a plugin can call them by address behind the executable check. Not building walls stays a gap until the host calls them.
- ~~**LD4:** what does slot 42 do with an empty boundary map, before `Init`? [nothing is written, and slot 43's record
  gets texture 0]~~ **ANSWERED 2026-10-15 (re-answers12):** DIFFERS, choice stands (lot-developer.md §17.7). With an empty map the key-0 retry also returns the end node (`0x006C6D2F`), and the three outputs are read from the head node's never-initialised value area: they are written, with indeterminate values. Writing nothing and giving slot 43 texture 0 is a safe *[choice]*; after `Init` key 0 always exists.
- ~~**LD5:** what does `PickLotFoundationAltitude` do with a facing outside 0-3? [taken mod 4]~~ **ANSWERED 2026-10-15 (re-answers12):** DIFFERS, choice stands (lot-developer.md §17.5.4). The facing is not masked (`0x006C53E6`): `corners[f]` and `corners[(f + 1) % 4]` (C signed remainder) read outside the 4-float array for a facing outside 0-3. Facings are 0-3, so `mod 4` is a safe *[choice]*.
- ~~**LD6:** what does slot 40 return when the lot has no configuration or no primary object? [position (0,0,0),
  orientation 0]~~ **ANSWERED 2026-10-15 (re-answers12):** **DIFFERS** (lot-developer.md §17.6, verified `0x006C3CF0`). With no configuration (lot slot 9) or no primary object (cfg slot 24) slot 40 returns EAX = 0 and writes neither `pos` nor `orient`: the caller's values stay. Do not write (0,0,0) / 0.
- ~~**LD7:** what does slot 58 do with a null configuration? [treated as having no primary object]~~ **ANSWERED 2026-10-15 (re-answers12):** DIFFERS, choice stands (lot-developer.md §17.5.5). `cfg` is called through its vtable without a null check (`0x006C40F5`), so the original faults on a null configuration. Treating it as having no primary object is a safe *[choice]*.
- ~~**LD8:** where does the orientation of a §6.6 normal base-texture record come from? [from slot 39: object
  orientation plus facing]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES (lot-developer.md §6.6). The record's orientation byte is slot 39's `orient` output, `(object byte +0x02 + lot facing) mod 4` (stored at `0x006CA9F7`).
- ~~**LD9:** is there a limit on slot 16's edge sweep for huge rectangles? [capped at `MAX_EDGE_SAMPLES` = 1<<20
  samples]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES, the cap never binds (lot-developer.md §17.2). The original has no cap; after the clamp both ends lie in `[0, extent − 0.1]`, so an edge has at most `extent / 4 + 2` samples (1,026 for a 4,096 m city), and a NaN end skips the sweep.
- ~~**LD10:** interfaces.tsv has no return-channel rows yet for slots 18, 19, 20, 21, 39, 40, 42 and 61. [these slots
  follow §11.1 and §17]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES. interfaces.tsv now has `CORRECTS` rows for slots 18-21, 39, 40 and 42 (re-specs11, `be30d89`). Slot 61 returns the AL of slot 55 / 54, or the flags' low byte when the lot has no foundation flag (lot-developer.md §17.5.4); the proposed row is in `build-runs/re-answers12-abi-rows.md`. Slot 22 has no result (LD27).
- ~~**LD11:** carried over from earlier stages (as in the earlier stages' code comments): the C++ runtime `rand()`
  state; the off-thread fallback; refusing re-entrant calls; base texture usage 2; hash set order; the terrain cell
  width read through city slot 91; the class ID behind `building_ref`; the reference returned by `unit_create`; side
  effects of `Write` in shadow mode; tuning that cannot be read from the original. [as in those comments]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES (glue choices; nothing in the executable contradicts them). Facts where the executable has one: base-texture usage 2 is set on every path of slot 41 (`0x006C41FB`, `0x006C426F`; lot-developer.md §6.6); the unit reference is LD13; the tuning cannot be read back (LD30). The C runtime `rand()` state, the off-thread fallback, re-entrancy refusal and shadow-mode `Write` effects have no original counterpart.
- ~~**LD12:** the view rotation used by the lot content and the base textures is not specified. [the host reads none;
  rotation 0]~~ **ANSWERED 2026-10-15 (re-answers12):** **DIFFERS** for zots only (lot-developer.md §5.6, §12.2 Q3). The lot developer reads the view rotation once, for the zot prop's orientation (`SC4ViewUtilities` `0x00B43DD8` +0x10 at `0x006C99ED`); base textures, props, flora and buildings use only the lot facing and the object orientation. Rotation 0 is right except for zots, which must follow the view (0 without a view).
- ~~**LD13:** creating a business unit: the GZCOM class and IID for the new unit are not specified. [no unit is
  created; joining and leaving existing units works (units=2 in a 6-year run)]~~ **ANSWERED 2026-10-15 (re-answers12):** **DIFFERS** (lot-developer.md §5.7 step 4). The unit is `cSC4BusinessUnit`, class `0x4A232DA8`, IID `0x6A233069` (`cISC4BusinessUnit`; QI `0x006BF390` also answers 1 and `0xE4FDA3D4` at +4). The original builds it directly (0x20 bytes from C, constructor `0x006C0800`, count 0, no AddRef: the member lots hold the references), and the class is registered (factory `0x006C0EB0`). A replacement creates it with `CreateInstance(0x4A232DA8, 0x6A233069)`, adds the candidate and the lot, then releases its own reference.
- ~~**LD14:** §7.3: the burnt-building prop and the rubble decal (type IDs, placement) are not specified. [not added;
  logged once]~~ **ANSWERED 2026-10-15 (re-answers12):** **DIFFERS** (lot-developer.md §7.2 ExplodeLot steps 2 and 4; §7.3 already gives the rubble types). Burnt building: class `0xC9EFB016`, constructor `0x005EF810(1)`, non-virtual `SetBoundsFromOccupant` `0x005EEB70` (`__thiscall` on the base, pop 4), occupant sub-object at +8 (flag `0x40`, `Init`). Decal: `cSC4PropOccupantTerrainDecal`, class `0x2977AA48`, IID `0xE982078A`, slots 3 `SetDecalTexture(key*, 2s)` and 4 `SetOpacity(0.7)`, occupant sub-object at +0xC, key `{0x7AB50E44, 0x1ABE787D, burnt ? 0x49C80EC7 : 0x2A2EF55D}`. Both classes are registered in the COM directory; both go in through prop manager slot 25.
- ~~**LD15:** slot 38's body is not specified beyond its signature. [answers neutrally from the host side; logged once]~~ **ANSWERED 2026-10-15 (re-answers12):** **DIFFERS** (lot-developer.md §7.1). Slot 38 (`0x006C9100`): the building occupants (`0x278128A0`) in the lot's bounding rectangle from occupant manager slot 41 (flags 1); only the first entry is QueryInterfaced for `0x87DDA3A9`, that reference is released at once, the list is released (slot 48), and the pointer (no reference) or null is returned.
- ~~**LD16:** slots 41 and 45 lack details (arguments, effects). [host side, neutral answer; logged once]~~ **ANSWERED 2026-10-15 (re-answers12):** **DIFFERS** (lot-developer.md §6.6). Slot 41: `out := null`; the first base-texture occupant at the lot's location cell (lot slot 21; occupant manager slot 33) is QueryInterfaced for `0x097F6D98` and given `SetUsage(2)`; otherwise, with `create`, `CreateInstance(0xC97F987C, 0x097F6D98)` (fallback `new`, constructor `0x005E05B0`), `Init`, `SetUsage(2)`, not inserted; AL = `out ≠ null`, caller owns the reference. Slot 45: slot 41 without create, then `UpdateVertexAltitudes(true, force, null)` (base-texture slot 9): rebounds from the terrain and, when forced or the box changed, notifies the occupant manager (slot 24) and rebuilds the models.
- ~~**LD17:** `UpdateLotObjectAlitudes` and `Add/RemoveNetworkConnectionNodes` (§6.5 b) are named but not specified.
  [nothing is done; logged once. The completions counter counts the `AddNetworkConnectionNodes` calls: 116 in a
  2-year diagnostic run]~~ **ANSWERED 2026-10-15 (re-answers12):** **DIFFERS** (lot-developer.md §6.5). All three use interfaces only. `UpdateLotObjectAlitudes` (`0x006C91F0`): for each building, prop and flora occupant of the lot whose y differs from its slot 39 position (or the terrain without a configuration object) by more than 0.1, set only y, notify the occupant manager (slot 24, 1) and, for a building, redo its foundation (slots 48, 60, 56, 59 / 58). `AddNetworkConnectionNodes` (`0x006C9C80`): per type-7 object with ≥ 3 IDs, network manager slot 20 `CreateConnectionNode(&n, ids[0], &cell, rotl(ids[1], 8·orient), rotl(ids[2], 8·orient), ids[3] or 0, orient)`, then slot 21 `AutoconnectLotNodes(lot, &list)`. `RemoveNetworkConnectionNodes` (`0x006C9FB0`): the lot's network occupants with flag `0x200000` go to network manager slot 6 `DestroyNetworkOccupants`.
- ~~**LD18:** what is AL from city slot 95 (used by the host)? [read as a bool, 0 / non-zero]~~ **ANSWERED 2026-10-15 (re-answers12):** **DIFFERS** (lot-developer.md §17.2). City slot 95 `PositionToCell` (`0x004599E0`) returns nothing; AL is the low byte of the x87 status word from its last `fnstsw` (sticky exception flags: 0x20 precision, 0x10 underflow, 0x02 denormal, 0x01 invalid). Treat it as void and do not read AL.
- ~~**LD19:** flora placement: is the occupant manager's blocking filter applied before the typed flora query, or
  after? [typed query first, then the filter. Konradshohe's configurations list no flora (`cfgFlora=0`), so no game
  run has exercised this]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES (lot-developer.md §6.2). The filter is passed into occupant manager slot 36 and tests only the type (`IsOccupantIncluded` → `IsOccupantTypeIncluded(o.GetType())`, accept = `0x74758926`, `0x006C3C20`), so typed query then filter gives the same set; the order is the manager's box-query order.
- ~~**LD20:** lot slot 61 `GetOccupancy`: its ABI (return channel and arguments) is not in interfaces.tsv. [treated as
  returning in EAX]~~ **ANSWERED 2026-10-15 (re-answers12):** **DIFFERS** (lot-developer.md §11.4, §12.3). Slot 61 is `void GetOccupancy(float& out)`, pop 4: it writes the float through the pointer (`0x006C0E8A`; 0.0 from the BSS constant `0x00B49114` when the capacity is 0) and leaves EAX = the out pointer, so reading EAX gives an address, not the occupancy.
- ~~**LD21:** the night deque and the shadow: the original's record has 670-740 night-deque buildings while the mode
  stays 0 in turbo, and ours matches (`roundtrip=same`, `night=L`). Should the deque be filled on insertion even when
  the visual time never changes? [yes, as recorded]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES (lot-developer.md §8.1). Insertion (`0x006C8F80`) appends every building to the deque with no mode test; turbo with mode 0 still fills it.
- ~~**LD22:** the §5 tests (`TestCondition` / `TestAbandonment`) have no read-only entry in the original, so the shadow
  cannot compare them. Is there a getter, or a message that exposes their result? [not compared]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES (not compared). There is no read-only entry. The outcomes are visible only as lot changes: `cSC4Lot::SetCondition` posts `0xE99B3D4D` (`0x006C2773`) and `cSC4Lot::SetState` posts `0xE8BDA2E7` (`0x006C347C`), so a shadow can compare the condition / state after each daily pass instead.
- ~~**LD23:** safe lot iterator fallback order: is `GetLotsInCellRect`'s snapshot order (used when the executable is
  not identified) acceptable in place of the master-list order? [used only as a fallback; `fallbacks=0` in every run]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES as a fallback only (glue choice). The original always walks the master list through its safe iterator (§4.1); any other order changes which lots draw first from the file RNG, so the fallback is not exact. Keep it counted and logged.
- ~~**LD24:** which base-texture occupant belongs to a lot when a cell has several? [the first occupant of type base
  texture whose lot pointer matches]~~ **ANSWERED 2026-10-15 (re-answers12):** **DIFFERS** (lot-developer.md §6.6 slot 41, §16 test 43). The original asks occupant manager slot 33 for the first base-texture occupant overlapping the lot's location cell (lot slot 21, bytes +0x7A / +0x7B), in the cell bucket's order, with **no lot-pointer check**.
- ~~**LD25:** the construction occupant: §6.2 creates it through the BDS. When that answers null, should it be created
  directly with `new`? [no direct `new`; construction is refused and counted]~~ **ANSWERED 2026-10-15 (re-answers12):** **DIFFERS** in the premise (lot-developer.md §6.3). The construction occupant does not come from the BDS: it is `CreateInstance(0xA97F909E, 0x496E7636)` (`0x006CC7E9`), with a direct `new` (0x1A0 bytes, constructor `0x005D00F0(1)`) only if that fails. The class is registered (factory `0x005D0BC0`), so the fallback is never reached in the shipped game. Use `CreateInstance`; refusing when it fails is acceptable.
- ~~**LD26:** the base-texture record layout (`extra[5]`, frame) beyond the documented fields. [the undocumented bytes
  are copied unchanged]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES (lot-developer.md §17.7, lot-records.md §6). The in-memory record is 16 bytes: texture, x, z, orientation, flip / object byte, five bytes (constructor `0x005CA770` sets `0xFF`), frame (`0xFF`), 2 bytes padding. For empty lots the five bytes are the zone drag colour R, G, B, the display alpha and 3; for normal textures they stay `0xFF`. Copying them unchanged is right.
- ~~**LD27:** slot 22 `GetOccupantCellCoverage`'s AL is the raw AL of its second `PositionToCell`. In game it is
  0x32 / 0x33 for 62-64 of the 64 sampled buildings per month (`rawAL=62-64`), not 0 / 1. Do callers rely on the raw
  byte, or only on whether it is true? [the core returns a bool, and the shadow compares only the truth value;
  rectangles match. If callers need the byte, `fc4_lotdev_occupant_cell_coverage` returns it and the shadow's
  `CompareLot` compares it raw]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES (lot-developer.md §17.2). The byte is the x87 status flags left by `PositionToCell` (LD18), not a result. Slot 22's only callers are `ChangeBuilding` (`0x006C52DB`), `GetOccupantSlope` (`0x006C5F64`) and `StartConstruction` (`0x006CC764`), through `this`; none reads AL. Comparing only the rectangle is right.
- ~~**LD28:** the night batches (§8.3) never ran in any turbo run: `nightBatches=0` and `mode=0` throughout to 2032.
  Does a turbo city ever send the day/night change `0x44778234` without the 24-hour clock running? [not exercised;
  lights are set at insertion instead (`lights=203`)]~~ **ANSWERED 2026-10-15 (re-answers12):** Answered (lot-developer.md §8.2). Only the 24-hour clock posts `0x44778234` (`ChangeVisualTime` `0x00445701`, `EnableDayNightTransitionNotifications(true)` `0x0044566E`); every other use is a subscription. Without a visual-time crossing (turbo starves the idle loop, CK22) there is no message and no batches; insertion lighting is the only path, as observed.
- ~~**LD29:** slot 25 `IsAreaFlat` disagrees with "altitude range within `0xE900EA24`" (shipped 1.0). In the shadow,
  the original answers flat for lots whose slot 27 range is bit-identical on both sides and > 1.0. Observed: 1.0355,
  1.0623, 1.0735, 1.0886, 1.0948, 1.1135, 1.1373, 1.1757, 1.1858, 1.2204, 1.2268, 1.2937, 1.3282, 1.3630, 1.4001,
  1.4131, 1.4340, 1.4482, 1.4526, 1.5149, 1.6784, all on both the dry and with-water variants; slot 26 follows. What
  exactly does slot 25 compare: another measure, another threshold, or a scaled tuning value? [range <=
  `kMaxCellAltitudeDeltaForFlatness`; 0-24 mismatches per shadow line, all of them this. Settled by the exe; then
  only `LotDeveloper::is_area_flat` (core `lot_developer/terrain.rs`) changes, with a §16 vector, and
  `lotdev_shadow = 1` should show mismatches=0 on every line (the `s25r` entries give the ranges)]~~ **ANSWERED 2026-10-15 (re-answers12):** **DIFFERS** (lot-developer.md §17.2, §16 test 42). Slot 25 (`0x006C5C30`) compares each vertex with the **first vertex** `(x1, z1)`, not the range: `ref := f32(slot 15(x1, z1, true))` (always with water, literal 1 at `0x006C5C4B`), then for x outer, z inner over `x1 … x2+1`, `z1 … z2+1`: `|slot 15(x, z, withWater) − ref| > +0xD0` (kMaxCellAltitudeDeltaForFlatness, strict, NaN no) → false. A range up to twice the tuning value is flat, which explains every observed range 1.0355-1.6784.
- ~~**LD30:** the tuning cannot be read back from the original (no getter), so the shadow cannot check that both sides
  use the same `kMaxCellAltitudeDeltaForFlatness` and the other §3.5 values. Is there an accessor? [not compared]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES (lot-developer.md §3.5). No accessor: `ReadSimulatorTunableValues` (`0x006C4A20`) has one caller, `Init` (`0x006CB9C7`), and nothing else writes the members (`kMaxCellAltitudeDeltaForFlatness` at `+0xD0`). A shadow reads the same exemplar with the same rules to get the original's values.

## From the network manager and network lot manager spec (re-specs13c lane, 2026-10-15)

Open points from network-manager.md §15. The current answer is in brackets; each entry says what would settle it.

- ~~**NM1:** are the globally managed network occupants (bridges, tunnels, prebuilt-model pieces) also saved by the
  occupant manager, or only through component 53? [both hold refs to the same objects; the occupant manager spec
  (re-specs13b) settles it]~~ **ANSWERED 2026-10-15 (re-specs14b):** both. The occupant grid (component 4) and
  component 53 save refs to the same objects; each ref resolves to one object, so `managed` holds each piece once.
  MATCHES. network-manager.md §15.
- ~~**NM2:** does the occupant manager post `0x99EF1142` while a city loads? If it does, the network manager's managed
  list (read first) gets every piece twice. [no, inferred. A turbo run that saves, loads and saves again and compares
  the component 53 counts would settle it]~~ **ANSWERED 2026-10-15 (re-specs14b):** no. Loading posts no Insert. Since
  the base occupant `Read` (`0x005E8D60`) sets bit 1 of the state byte, the base `Init` (`0x005E8C90`) posts no
  Create either. The network occupant `Init`s never insert. MATCHES. network-manager.md §15.
- ~~**NM3:** meanings of network flags `0x40000`, `0x80000`, `0x40000000` and `0x80000000`~~ **ANSWERED 2026-10-15
  (re-specs15b):** `0x40000` bridge-step piece (MATCHES), `0x80000` tunnel portal (MATCHES), `0x40000000` elevated
  with supports (MATCHES), `0x80000000` draft (preview) occupant. network-geometry.md §10.3. History: ~~and of the terrain kinds 3,
  4 and 6 in the network manager's terrain handler. [unknown; logging `data4` of `0x278E6003` for each terraforming
  brush settles the kinds]~~ **PARTLY ANSWERED 2026-10-15 (re-specs14a):** the kinds are settled from every call
  site of `RedisplayTerrain` (terrain.md §8.2): 0 internal (sea level, flat areas, shore cliffs, texture-only),
  1 terraforming in god mode, `ChangeTerrainLevel` and the edge reconciliation, 2 terraforming in mayor mode and
  the `ApplyTerrainBrush` script command,
  3 disasters and water (earthquake, pipe-burst sinkhole, effect brushes such as the volcano, lava solidification,
  the surface water's animation, and the `tBrushParameters` default), 4 the network height solver (road and rail
  grading, `0x00644530`), 5 levelling (demolition `ModifyTerrainHeight`, lot developer, landfill), 6 the tunnel cut
  (`cSC4NetworkTool::InsertTunnelPiece`). Flag `0x40000` is the **bridge** flag *[inferred from use]*: the surface
  water's `IsNavigable` reads a deck height from a piece with that flag and requires 20 m of clearance over the
  water, and its `0x89D3E9F3` handler tests it with "no adjacent piece" (terrain.md §10.3–§10.4). Still open: `0x80000`,
  `0x40000000`, `0x80000000` (a network-tool spec settles them; `0x80000` is the piece kept by a tunnel cut,
  *[inferred]* a tunnel portal).
- **NM4:** the exact visited and rejected set tests of the avenue flood in `UpdateAvenueStyle`. [a cell is visited
  once; styling an avenue loop through an intersection and logging the styled cells settles it]
- ~~**NM5:** the effect-prop slots 6, 7 and 8 used for potholes. [set the effect name, start, stop; the prop occupant
  spec should confirm]~~ **ANSWERED 2026-10-15 (re-specs14b):** confirmed: `SetEffectType(char const*)`,
  `StartEffect(bool)`, `StopEffect(bool)` (vtable `0x00AA4ED8`, overloads in reverse SDK order). MATCHES.
  network-tools.md §13.2.


## From the network geometry spec (re-specs15b lane, 2026-10-15)

Open points from network-geometry.md §17. The current answer is in brackets; each entry says what would settle it.

- **NG1:** the satisfier's tie-break between vertices with equally narrow ranges follows the hash-map iteration order
  of vertex ids. [reproduce the SGI `hash_map<int>` bucket order; comparing one hill crossing in both engines shows
  whether ties occur]
- **NG2:** the bridge internals: the step and edge tests, `AdjustBridgeSpan`, `FindBestBridgeType`,
  `SelectBridgeElevation`, specialised and parametric bridge models. [not specified]
- **NG3:** who fills the tool's extra-cell list (`+0xF0`), and whether `CheckForGoodTerrain`'s inverted extra-cell
  test can be reached. [empty in an ordinary drag, inferred]
- **NG4:** `IdentifyLandBridgeCells` and which types set `+0x354`. [not specified]
- **NG5:** the rule-driven piece solver (`SolveUsingEvaluator`, `ApplyNetworkRules`, tile subsets) and
  `IdentifyHighwayIntersections`. [a later spec]
- **NG6:** the readers of type info `+0x110` and of the p+0xA0 id list, and the source of road's support info.
  [only the flat-cell test is known]
- **NG7:** `DestroyOccupantsAndResolve` builds one neighbour key as `(x << 16) + z` while the tool's sets use
  `(z << 16) + x`. [unchecked; a destroy next to an off-diagonal neighbour would show it]
## From the traffic glue, phase 2: the traffic shadow (filed by the coordinator from the `traffic2` lane's report, 2026-10-15)

About `docs/specs/traffic-simulator.md` and the same-process traffic shadow (`traffic_shadow = 1`; core
`traffic_simulator/shadow.rs`, capi `traffic_shadow.rs`, plugin `FC4TrafficShadow`, seam op 51); merged as `40d9eab`,
ABI 56. With `traffic_shadow_hypotheses = 7` (bits 1, 2, 4 below) the shadow matches the original on every delivery on
Konradshohe for 7 game months (261/261, resyncs=0); with 0 (the spec as written) 25 of 92 deliveries are clean. The
lane numbered these TS1-TS17 (TS50-TS66 here; engine comments and `engine/README.md` use the lane's numbers). The
current answer is in brackets.

- ~~**TS50:** §6 record: the record does not save the last-day field, so a shadow (or a resync) cannot know the original's
  day pacing state. Is lastDay intentionally unsaved, and what does the original do with it on load? [the copy starts
  at the first delivery]~~ **ANSWERED 2026-10-15 (re-answers13):** MATCHES. The last day-of-month seen is member `+0x1AC`, set to −1 by the constructor (`0x0071FA3B`) and written only by DoDailySimulation (`0x00721603`); Write, Read, Init and DoPostCityInit never touch it. The simulator is built with each city, so the first call after a load always takes the new-day branch, even on the save's own day: on day 1 the two incomes are set again, and the strategy re-draw test of that date runs a second time (more draws). traffic-simulator §3.3 step 2.
- ~~**TS51:** §4.1.2 lot strategy byte: there is no getter for a lot's travel strategy byte. Is the last byte of the lot's
  own `Write` record (§6.3) the byte §4.1.2 uses, and is there an interface slot for it? [the shadow reads that byte]~~ **ANSWERED 2026-10-15 (re-answers13):** MATCHES, no slot. The byte is lot member `+0x88`: UpdateTrafficVolume reads it at `0x006C2986`, and `cSC4Lot::Write` writes it as the record's last byte, after the trip list (`0x006C0347`). No `cSC4Lot` vtable has a getter for it; the copy constructor copies it (`0x006C25AA`). traffic-simulator §4.1.1.
- ~~**TS52:** §4.6 DeactivateBudgetItems: the shadow needs the cost of the holder's `0xCA550301` budget item without
  removing it; passing the holder token to OccHolder's BudgetItemForPurpose broke the stack under /RTC1. Is reading
  `item.cost` through the city budget simulator (GetBudgetItemForPurpose) the right lookup? [that lookup; a choice]~~ **ANSWERED 2026-10-15 (re-answers13):** MATCHES, and it is the original's own lookup, not a choice: DoLotDeactivated calls the budget simulator's slot 51 (`0x00691880`) with the lot's own property holder (lot slot 3) as a `cISCPropertyHolder*` (`0x0071E4BD`–`0x0071E4CA`), and slot 51 only fills the record. It copies a whole `0x20`-byte `BudgetItem` (8 dwords, `0x006918EE`; cost s64 at `+0x10`, index at `+0x18`, funding f32 at `+0x1C`), so a buffer smaller than `0x20` bytes is overrun, a likely cause of the /RTC1 failure. traffic-simulator §4.1.3.
- ~~**TS53:** §6 record variants: the original's WriteVariant output is captured through the core's writer
  (`fc4_props_write_variant`, foundations §9.2: u32 0, type tag, value). Is that the byte form the original's segment
  stream writes for every type that occurs in the traffic record? [assumed]~~ **ANSWERED 2026-10-15 (re-answers13):** MATCHES. The only variants in the record are in the pool holders of row 19: plain `cSCPropertyHolder`s whose own `Write` (holder `+0x5C`, called at `0x00713324`) writes the foundations §10.2 record, each property through `WriteVariant`: u32 marker 0, u32 disk tag, u32 count, elements. The traffic code puts only `0x8A0B4100` on a pool: a u16 array (tag `0x8002`) or, once promoted, a u32 array (`0x8003`). traffic-simulator §6.2.
- ~~**TS54:** §6.2 Read: the SGI hash containers (damage sets, edge densities) iterate bucket ascending with head
  insertion, and our Read rebuilds the chains in reverse order. Which order does the original's Read leave, and does
  any later pass depend on it? [the shadow restores the original's order: smallest prime >= max(len, 100), non-decreasing
  keys]~~ **ANSWERED 2026-10-15 (re-answers13):** Partly **DIFFERS**. Read inserts into fresh containers of 193 buckets (hint 100, `0x00829BC0`). For each key in file order, the table first grows to the smallest listed prime ≥ count + 1 when that exceeds the bucket count (`0x00710DD0`). A grow walks the old buckets in ascending order, moving each chain's nodes from the head, each to the head of its new chain. Then the key goes to the **head** of its chain (`0x0060D270` damage sets, `0x006D6FD0` edge densities). The bracket's bucket count is the final one. Building the final table directly and ordering each chain by key gives the original's order only when no grow happens during Read (193 keys or fewer) and the file listed each chain in decreasing key order. Otherwise replay the inserts. Two things depend on the order: ApplyPotholeChanges' pick of the `r`-th cell, and the next Write's key order. Nothing else does. traffic-simulator §6.2.
- ~~**TS55:** §4.3 ConfirmTrips merge: in game a routed trip is set back to status 0 only when it absorbed a later trip.
  As written (every routed status-0 candidate) the copy gets toReRoute 41 where the original has 0 (Konradshohe
  2026-01). Confirm. [hypothesis bit 1; clean in game, 261/261]~~ **ANSWERED 2026-10-15 (re-answers13):** MATCHES hypothesis bit 1; it becomes the rule. In a re-route source each trip that is status 3 when reached absorbs every later trip of the same type and strategy, whatever that trip's status. Its status becomes 0 **only when it absorbed at least one trip**: the store is inside the match (`0x0071D9B6`). Each such trip adds 1 to pass counter 5 (`0x0071DA0D`). A routed trip with no partner keeps status 3 and goes on to the capacity test. traffic-simulator §4.3.
- ~~**TS56:** §6.1 row 19 / §4.4 end of FullTrips: the travel-type count pairs appear to be (published, building), and the
  end of FullTrips swaps the two vectors instead of copying (the original's row 19 is (130,132) right after the end;
  the spec reading gives the reverse). Confirm. [hypothesis bit 2; clean in game]~~ **ANSWERED 2026-10-15 (re-answers13):** MATCHES hypothesis bit 2; it becomes the rule. Write and Read store the pairs as (published `+0x424`, building `+0x430`) (`0x00713370`–`0x00713398`, `0x00722FD0`). The end of FullTrips **swaps** the two vectors' begin, end and capacity words (`0x0071D806`–`0x0071D84E`), so building holds the previous published counts until entering 18 refills it with 9 zeros (`0x007210E6`). traffic-simulator §4.4, §6.1, §2; save-records §7.1.1.
- ~~**TS57:** §4.4 ReRouteTrips merge: the merge also absorbs later trips of the same type and strategy that are routed
  (status 3) or status 4, not only status 0 (lot (38,96) goes from [2x st0, 1x st3] to [3x st0] in the original; the
  spec reading leaves the st3 trip). Confirm. [hypothesis bit 4; clean in game]~~ **ANSWERED 2026-10-15 (re-answers13):** MATCHES hypothesis bit 4, and wider. The ReRouteTrips merge absorbs into each status-0 trip every later trip of the same type and strategy, whatever its status (0, 1, 3, 4 or 5). The test meant to skip routed trips reads the **absorbing** trip's status (`0x0071D2D7`), which is always 0 there, so it never skips. The absorbing trip stays status 0 and no counter moves. traffic-simulator §4.4.
- ~~**TS58:** §4.4/§4.5 reversible trips: the original keeps a reversible trip's returnRoute where the spec clears or
  rewrites it (about 100 lists a month). Which rule applies? [counted as a known difference]~~ **ANSWERED 2026-10-15 (re-answers13):** The original never clears or rewrites `returnRoute` on a freight or reversible result. It fetches only the outbound route (pathfinder `+0x80` into `route` and `travelTypesUsed`), so `returnRoute` keeps what it held: an earlier two-leg result of the same record, the vector copied into a split copy (`0x0071D6EA`), or the save. Nothing reads it while the reversible bit is set. The spec's "empty" (§2) was wrong and is corrected; keep the vector untouched, and the difference goes away. traffic-simulator §2, §4.4.
- ~~**TS59:** §10 Q4 pass counters: counters 0, 5 and 7 advance in ConfirmTrips in ways §10 does not give; counter 4 also
  advances in ConfirmTrips (4 right after entering 17 with processed 0). What are the increment points of counters 0
  and 4-8? [counted as a known difference]~~ **ANSWERED 2026-10-15 (re-answers13):** The ten counters (`+0x948` + 4·i) are named, with every increment point, in the traffic-simulator §10 Q4 table. In short: 0 confirmed `0x0071DBF5`; 1 re-routes found `0x0071D289`; 2 `rerouteBucket`; 3 sources `0x0071D93F`; 4 destination full, in ConfirmTrips `0x0071DC01` and in ReRouteTrips `0x0071D221`; 5 merges `0x0071DA0D`; 6 return leg not confirmed `0x0071DBA0`; 7 status-0 trips met before the merge `0x0071D95D`; 8 status 5 → 0 `0x0071DC11`; 9 splits `0x0071D77F`. All are zeroed on entering 16 except 1 and 2. All ten are saved. Only counter 2 drives behaviour. The bracket (a known difference) can become exact.
- ~~**TS60:** §6.1 rows 14-15 unnamed fields: the original changes an i32 that toggles 0/1 with the pacing, and the
  re-route context's x,z. What are they? [the copy keeps them as read]~~ **ANSWERED 2026-10-15 (re-answers13):** The i32 is **`dayIndex`** (`+0x918`), the s32 right after `processed` in row 15 (`0x00721912`–`0x0072193E`). The field the spec and save-records called "dayIndex" (the u32 after `days`, `+0x930`) is never used, nor are the next two (`+0x934`, `+0x938`). The re-route context's x, z (`+0x83C`, `+0x840`) are the cell of the source being worked, written as each source is taken in 16–18 (`0x0071E26D`, `0x0071D104`, `0x0071D523`) and not reset on phase entry. Keeping them as read **DIFFERS**: model `dayIndex` at that position and write the context cell as sources are taken. traffic-simulator §3.3, §6.1; save-records §7.1.1.
- ~~**TS61:** statistics sets: the last, unnamed byte of a statistics set increments once per cycle (1C to 1D) in the
  original. What is it, and where is it incremented? [not modelled]~~ **ANSWERED 2026-10-15 (re-answers13):** **DIFFERS** (it should be modelled). It is `freightByte`: freight trip length as a fraction of `commuteMaxTime` × 255, truncated, low 8 bits. FinalCalculations recomputes it once per cycle (`0x0070E84A`, `0x0070E862` when there is no freight) as the last byte of the **published** set (`+0x41C`, block offset `0x238`); the 1C to 1D step was a recomputation. The working set's byte (`+0x678`) is written only by the constructor and Read and is never copied. traffic-simulator §4.12.
- ~~**TS62:** §3.4 deadline test points: in game the original also stops at the end of a source's list, in 17 and 18
  after the merge or CompleteTripSourceCalculations and before the next source is taken; in ReRouteTrips it never
  stops before a trip that is not status 1 (shadow-14: 4 FullTrips days with the original's saved cell before ours,
  next-day attempted/routed +4, +5, +97, +34; 2026-02-24 lot (40,82) merged by the original). Which test points does
  the original have (`0x0071D1D9`, `0x0071D607`), and is one at the list end? [the shadow follows the saved cell and
  takes a list end only when the record or trip list shows it, 0-2 a month]~~ **ANSWERED 2026-10-15 (re-answers13):** The test points are now in a table in traffic-simulator §3.4. ConfirmTrips tests before each source (`0x0071E222`). ReRouteTrips and FullTrips test at the top of the source loop (`0x0071D094`, `0x0071D4B3`): at each call's start and after each finished source, that is after its merge or CompleteTripSourceCalculations and before the next source is taken. **Yes, that is a list-end stop.** They also test before every trip whatever its status (`0x0071D1D9`, `0x0071D607`). A point reads the clock only when the per-call counter, stepped by the phase's adaptive `step`, is a multiple of 1024. "Never before a non-status-1 trip" is timing, not code. A stop inside a list redoes the list from its head next call; the merge or CompleteTripSourceCalculations has not run. The bracket's rule (follow the saved cell) agrees with this.
- ~~**TS63:** §4.7 FinishTripMap: in game state 15 can stop part-way, after a lot, on the deadline (2026-01-30, 18 to 15
  in one day; saved cell lot (17,51); the trip-length grid differs in 12 tracts until the copy stops there too). Does
  FinishTripMap test the deadline per lot, and at which address? [the shadow can stop after the lot at the saved cell]~~ **ANSWERED 2026-10-15 (re-answers13):** MATCHES. FinishTripMap tests the deadline before each lot is taken (`0x0070FF84`). The lot's location becomes the phase position as it is taken (`0x0070FFE0`), and the lot is then handled completely. So a stop falls after a whole lot, at the saved cell. traffic-simulator §4.7, §3.4.
- ~~**TS64:** §3.2 states 12/13, scanning iterator: after 11 to 13 the original's scanning iterator reads reset (all
  zero) or Running at (34,87), and Done at (0,128) after 13 finishes; the spec gives 12 and 13 no use of the iterator
  and ours keeps phase 10's Done (shadow-19 2026-02-24, shadow-20 2026-07-30). Does UpdateSubnetworks or
  UpdateSubnetworkInfo reset and run the scanning iterator, and is it deadline-stoppable? [ours keeps Done; 1-2
  deliveries differ in that field]~~ **ANSWERED 2026-10-15 (re-answers13):** **DIFFERS**. Entering 13 resets the simulator's scanning iterator (`+0x818`) to state 0 at (0, 0), keeping its rectangle and step (`0x00721308`–`0x0072132A`). UpdateSubnetworkInfo steps it per cell and tests the deadline before each step (`0x00716212`), so a call can stop mid-scan (state 1 at the last cell). When the scan ends the iterator reads state 2 at `(x0, z1 + 1)`, which is (0, 128) in a 128-cell city; the neighbour pass then runs in the same call. State 12 and its entry do not touch the iterator. DoPostCityInit uses a local iterator. traffic-network §4.12, traffic-simulator §3.2.
- ~~**TS65:** §4.4 suspended searches: SetupTrip calls SetMaxWorkTime(remaining slice ms), so the original can suspend a
  search, and a suspended search seems to write the trip's destination (a status-0 split copy at the day's stop: lot
  (56,99) original (28,37) against ours (18,40) on 2026-03-02; lot (31,86) on 2026-05-10; same count, length and
  route). "The copy is made before the search" was refuted in game. Does a suspended PerformPathSearch write
  destX/destZ, and what else does it leave in the trip? [ours never suspend (§3.4 choice); counted as a known
  difference for status-0 trips]~~ **ANSWERED 2026-10-15 (re-answers13):** Yes. When the outbound leg is found, `destX, destZ := pf.GetEndCell()` at once (`0x0071C752`). On the two-leg path flags bit 0 is then cleared (`0x0071C858`). If the return leg suspends, the trip keeps only those two changes; `route`, `returnRoute`, `tripLength`, `travelTypesUsed`, `count` and the status are unchanged and `processed` is not counted. An outbound suspension writes nothing. A resumed search skips SetupTrip; a network change restarts it, and the written destination stays. The bracket is accepted as the §3.4 choice: the difference is confined to those fields of status-0 trips at a day's stop. traffic-simulator §4.4.
- ~~**TS66:** §4.15 draws: the pothole and accident draws and the strategy draws (property `0x4A611773`) cannot be compared
  because the original's generator state is not readable. Is there a readable generator, or a deterministic seed path?
  [not compared]~~ **ANSWERED 2026-10-15 (re-answers13):** There is no interface, save field or deterministic seed. The generator is the heap object at simulator member `+0x6FC` (`0x9C8` bytes: `u32 state[624]`, the next-word pointer at `+0x9C0`, the words-left count at `+0x9C4`). Init creates it and seeds it once from the local date and time (`0x0071F132`–`0x0071F1A9`). A same-process shadow can read that state after each delivery and compare draws. It could also call `Seed` (`0x00913A72`, thiscall, one u32) on it after Init, but that is a write into the original, so a test-only choice. traffic-simulator §4.15.

## From the terrain and nation specs (re-specs14a lane, 2026-10-15)

Open points from terrain.md §17. The current answer is in brackets; each entry says what would settle it.

- **TR1:** the exact operations of brush component kinds 2, 3, 5, 6 and 8–10 (smoothing, levelling, erosion, lake,
  rain, spring). [summarised only; only the god-mode terraforming tool uses them. A trace of `0x00745DE0` and
  `0x00745BA0` settles 2 and 5] **PARTLY ANSWERED 2026-10-16 (re-answers17):** kinds 2 and 5 are traced in the bytes and specified in terrain §9.4.1. Kind 5 levels toward the target by `step = b × strength / 256` (sets the target when within a step, else raises or lowers by a step; `b = 0` or `alt = target` is no change). Kind 2 is talus smoothing: for each neighbour in the offset order, the excess drop over a threshold (`tan(TalusAngle) × 16`, × √2 on diagonals) times the step moves from the vertex to the neighbour. Kinds 3, 6 and 8–10 and the shipped `TalusAngle` stay open (god-mode only).
- **TR2:** the brush values of the `volcano_control` effect (level, strength and radius ranges, repetitions per
  update). [effect brushes use `data4 = 3` and redisplay on the last repetition; the effects manager spec, INVENTORY
  rank 7, settles the values]
- ~~**TR3:** `GetLerpedWaterLevel` (terrain slot 32) and `AltitudeAtWhichFloodingWillOccur` (surface water slot 10)
  are from pseudocode only. [bilinear over the four corners; maximum over wet neighbours. Check the bytes if a
  simulation consumer appears]~~ **ANSWERED 2026-10-16 (re-answers17):** both checked in the bytes. `GetLerpedWaterLevel` MATCHES (bilinear over `depth + alt`), with no clamp at `x = sizeX` (the `ix + 1` corner wraps to the next row) (terrain §5.1). `AltitudeAtWhichFloodingWillOccur` **DIFFERS** from the round-1 text: it takes `(x, z)`, starts at the sea level, counts the vertex's own level when it is wet, then the wet neighbours in the offset order, strict maximum (terrain §10.2).

## From the disaster layer core (filed by the coordinator from the `cores13` lane's report, 2026-10-15; merged `333ecde`)

cores13 implemented disasters.md §2-§14 as a Rust core (`engine/core/src/disasters/`), a C ABI (`fc4_disasters`,
ABI 57) and conformance vectors (`ds18_*`, `ds00_*`). Q4 (the riot's removal lock) is answered by network-manager.md
§10 and implemented. There is no plugin glue yet, so none of these readings has been checked in game. The lane's own
numbers (its Q6-Q25, continuing disasters.md §17) are given in parentheses; the current choice is in brackets.

- ~~**DS1:** (Q6) the fire generator's class and seed are unspecified. [the core takes them from the host generator]~~ **ANSWERED 2026-10-15 (re-answers14):** MATCHES in effect. The original's fire generator is a `cRZRandom` member of the fire instance constructed with seed `0xFFFFFFFF` (clock-seeded, `0x0057AD90` → `0x00913A5C`), shared only with the toxic spill. It is not reproducible in the original, so foundations §6.3 lets the host seed it, provided it is a `cRZRandom` private to the fire with both the `lcg` and `mul` paths. disasters.md §1, §13.
- ~~**DS2:** (Q7) which draw method (raw, u32, double, range, gaussian) does each random use? [the spec text as far as it
  goes]~~ **ANSWERED 2026-10-15 (re-answers14):** **DIFFERS** where the old spec text was wrong or silent. Raw `RandomUint32Uniform()` for every fire ID; `RandomDoubleRangeUniform(0, 100)` for `IgniteCell`, with ignition iff `r ≤ p` (`p` a `float`); `RandomUint32Uniform(k + 1)` for the resistance; `RandomDoubleRangeUniform(0, 1)` for the spread, compared `r < 1.0f/D4[s]` and drawn **only when the step reaches the spread** (the old text said always); two `RandomDoubleRangeGaussianFast` (x then z) for the spread cell; `RandomUint32Uniform(4000)` for `ExtinguishFire` only when no time is set; and a draw the old text missed, `RandomUint32Uniform(n)` per insertion into a non-empty toast list. Riot: two `RandomDoubleUniform()` (x then z, × 16 m from the cell corner) per created leader on `0x00B22B80`. Pipe: `RandomSint32RangeUniform(lo, hi)` µs per leak effect (re)creation on `0x00B22B60`. Earthquake: `RandomDoubleRangeUniform` on `0x00B22AE8` for the fault line. disasters.md §5.5–§5.10, §6.4, §7, §8, §13.
- ~~**DS3:** (Q8) is the fire ID drawn once per IgniteCell, also when the cell does not ignite?~~ **ANSWERED 2026-10-15 (re-answers14):** Once per `IgniteCell` call that passes the probability, **even when no occupant then ignites**, and only when `fireID` is 0; a call refused by the probability draws nothing, and all occupants of the cell share that ID (`Ignite` never draws then). A core that followed the old text (the draw inside `Ignite`, per occupant) **DIFFERS**. disasters.md §5.5.
- ~~**DS4:** (Q9) are timer seconds truncated or rounded?~~ **ANSWERED 2026-10-15 (re-answers14):** Truncated where the integer `GetElapsedTime` is used (`0x00890198`: float multiply, then `_ftol`): the fire's milliseconds and the riot's once-per-second gate (units 5). The damage assessment uses the float overload (`0x00890377`), so its seconds are fractional, not truncated. disasters.md §2.5, §5.8, §6.5.
- ~~**DS5:** (Q10) where does GoToNextDisaster's cursor start?~~ **ANSWERED 2026-10-15 (re-answers14):** The cursor (`+0xC0`) is set to the newest entry's index on **every** `RegisterDisasterLocation` (`0x005783D0`). `GetNextDisasterLocation` (`0x00577770`) clamps it to `count − 1`, uses that entry, then decrements, wrapping below 0 to `count − 1`; unregistering does not touch it. So the first call after a registration goes to the newest entry and later calls walk backwards. The old text (advance, then use) **DIFFERS**. disasters.md §2.7, test 11.
- ~~**DS6:** (Q11) the effectiveness with no timers, or with an overall total of zero. [0]~~ **ANSWERED 2026-10-15 (re-answers14):** **DIFFERS** in the formula; the zero cases MATCH. `EndDispatchTimers` (`0x00577AF0`) divides the summed (fractional) seconds by `float(GetTotalDispatchCount()) × overall seconds`, the count being fire protection slot 25 or police slot 26 (the number of dispatch units), **not** the number of timers; the result is 0 when that count is 0, and an empty map gives 0. Nothing is passed to the simulators; the values are only stored at `+0xAC`/`+0xB0`. disasters.md §2.5, test 10 (rewritten).
- ~~**DS7:** (Q12) what is the dispatch key's data2, and does any message create an entry?~~ **ANSWERED 2026-10-15 (re-answers14):** data1 is the dispatch type (1, 5 → fire map; 0, 6 → police map; others ignored) and data2 is the key, the **dispatch ID** (a dispatch record, not a station). Any of the four messages with a mapped type creates the entry when absent (a new units-5 timer, not started); `0xE94671EC` starts it if stopped (keeping accumulated time) and the other three stop it if running. disasters.md §2.5.
- ~~**DS8:** (Q13) instance Shutdown: does the instance stay in its manager's list, with no DisasterHasEnded sent? [yes,
  it stays and nothing is sent]~~ **ANSWERED 2026-10-15 (re-answers14):** **DIFFERS.** The fire (`0x0057A540`), riot (`0x0057D420`), pipe burst (`0x0057C460`), toxic spill (`0x0057EB80`) and volcano (`0x0057F920`) remove themselves from their manager in `Shutdown`; the earthquake is removed by its state 4 just before. No fire or riot `Shutdown` calls `DisasterHasEnded`, but the fire's `KillAllFires` posts `0x49E3DB1D` data2 0 per fire and both unregister their locations; the toxic spill calls `DisasterHasEnded` and the volcano calls it through `EndVolcanoLava` if it had not ended yet. disasters.md §3.1 (table).
- ~~**DS9:** (Q14) D3-D5 property arrays of other lengths. [the D4 index is clamped]~~ **ANSWERED 2026-10-15 (re-answers14):** MATCHES on every defined input; clamping is accepted as a *[choice]* for short arrays, which are undefined in the original (it reads past the array). The cumulation always reads D3[0..4] and the stage lookup (`0x005796B0`) returns 0..5; D4 is indexed by that stage, D5 by the fire's current stage; extra entries are ignored. Note a related **DIFFERS** in the old §5.1: the defaults apply **only when the whole exemplar is missing**; with the exemplar present, a missing array stays empty and D6/zoom are left unset. disasters.md §5.1.
- ~~**DS10:** (Q15) which fires count for ExtinguishCell and GetFireCountInCellRect?~~ **ANSWERED 2026-10-15 (re-answers14):** `ExtinguishCell` (`0x0057B350`) takes the occupants of the single cell (x, z) through the occupant manager (slot 40) with an "active fire" filter: fire occupants with a non-null target, **including** ones already extinguishing. `GetFireCountInCellRect` (`0x00579A60`) counts every fire in the list, extinguishing or not, whose fire occupant's bounding cells overlap the rect (inclusive). `GetFireCount` counts the list. disasters.md §5.7.
- ~~**DS11:** (Q16) riot defaults when the exemplar is missing (max riots, max leaders, zoom). [no leaders-spawned
  default]~~ **ANSWERED 2026-10-15 (re-answers14):** **DIFFERS.** The instance sets defaults first and lets present properties override them (`0x0057E3D0`): 6 stages, 30 s per stage, `0xEA400622` = 3, **leaders spawned 0, 1, 0, 1, 0, 0**. The police curve defaults to (64, 0) (65, 1) (255, 3) when the exemplar exists but the curve is empty, and stays empty when the exemplar is missing. The zoom byte has no default (uninitialised; use 0). The manager statics default to max riots 3 and **max leaders 5** (shipped 10). disasters.md §6.1.
- ~~**DS12:** (Q17) what does message `0xEA400622` mean?~~ **ANSWERED 2026-10-15 (re-answers14):** It is not a message: `0xEA400622` is a riot tuning property, stored at instance `+0x54` (default 3) and **never read**. A port may drop it. disasters.md §6.1.
- ~~**DS13:** (Q18) the exact spiral order of the cell search.~~ **ANSWERED 2026-10-15 (re-answers14):** `FindNearestRoadTile` (`0x0057D4C0`) uses the city's `SpiralIterator` in its clockwise mode, exactly as plumbing.md §7.2: centre first, ring r from (x + r, z − r + 1) running +z first, rings 0 to 7, inside the whole-city box; the first cell whose traffic-network tile occupant has network type 0, 3 or 6 wins, and (x, z) is rewritten to it. disasters.md §6.4, test 13.
- ~~**DS14:** (Q19) does the riot leave its manager right after phase 3?~~ **ANSWERED 2026-10-15 (re-answers14):** No. Phase 1 → 3 (`0x0057D390`) only unregisters the locations and calls `DisasterHasEnded`; the instance stays in the list, keeps ticking, and leaves only at the riot manager's `Shutdown`. So the removal locks are released only then (network-manager.md §10 note added; disasters.md Q4 corrected). Also **DIFFERS** from the old §6.2: `AttemptDisasterStart` (`0x0057EA10`) **always returns null**. Ended riots do not count as active, so they never block new ones. disasters.md §6.2, §6.7.
- ~~**DS15:** (Q20) the tuning defaults of the short forms (earthquake, pipe burst, volcano, effect).~~ **ANSWERED 2026-10-15 (re-answers14):** Earthquake: read once per process, only if the exemplar exists; static defaults spacing 40, curviness 30, slope 0, speed 25, shaker 10000 µs, repulsor spacing 100, curves empty; the slope (× π/180) and speed (× 1e-6) conversions run whenever the exemplar exists, so a missing speed means 25 m/s, and with no exemplar the speed stays 25 m/µs *[quirk]*. Pipe burst: 5 s and 10 s, zoom 0. Volcano (read at every `Init`): T 20 s, scale 1, radius 100, curves empty, lava disabled. Effect disasters: no tuning. disasters.md §7, §8, §10, §11.
- ~~**DS16:** (Q21) the earthquake's fault line geometry.~~ **ANSWERED 2026-10-15 (re-answers14):** `ComputeFaultLine` (`0x00576640`), all on the earthquake generator `0x00B22AE8`: a base angle in [0, π] drawn once per process; angle = base + uniform(±slope); L = SizeX/3; two four-point polylines from the epicentre in opposite directions, each step L along ±dir plus a perpendicular uniform(±curviness) offset (first step one shared draw, +d and −d; second and third steps one draw each, A first). disasters.md §7.
- ~~**DS17:** (Q22) the earthquake's transition from rumble to deformation.~~ **ANSWERED 2026-10-15 (re-answers14):** The manager's `AttemptDisasterStart` (`0x00576D80`) sets state 2 and calls `StartDeformation` in the same call as `Init`, so the rumble state never ticks; nothing else drives state 1 in this build. The intensity keeps its constructor value 0.1 (strength 0.19, width 0.55) and "QuakerShaker" never plays. Deformation, settle (3 s) and end as in disasters.md §7, which also corrects the redraw to terrain slot 78 then slot 68 per side.
- ~~**DS18:** (Q23) the sinkhole brush parameters.~~ **ANSWERED 2026-10-15 (re-answers14):** `0x0057C1A0`: terrain brush `0x14` (terrain slot 57) with start = end = (x × w, z × w), w = terrain slot 6, strength 1.0, width 1.0, and fields `+0x10` = 0, `+0x14` (byte) = 1, `+0x20` = 3, `+0x34` = 100.0 whose meaning is not established. It is dug only when the cell's pipe was found, and even without a demolition utility. disasters.md §8.
- ~~**DS19:** (Q24) when is the pipe-burst occupant inserted and removed?~~ **ANSWERED 2026-10-15 (re-answers14):** The pipe burst inserts **no occupant** of its own. Leaks are records keyed by cell (`StartLeakAtPipe` `0x0057C7C0`), removed when they burst, when a pipe occupant on their cell is inserted (non-preview) or removed (`0x99EF1142`/`0x99EF1143`), or at `Shutdown`. The only occupant handled is the existing pipe: the layer location holds it from `Init`, and `BurstPipe` demolishes it. disasters.md §8.
- ~~**DS20:** (Q25) the volcano: the timing of states 3 to 5, its end request, and state 7.~~ **ANSWERED 2026-10-15 (re-answers14):** States 1 → 3 → 5 all happen inside the start call. State 3 starts the "volcano_control" effect, whose completion posts `0xCA493C1D` (the end request) for this instance. State 5 flows for T seconds (clock > T, strictly), then `EndVolcanoLava` (which calls `DisasterHasEnded` and unregisters the locations) and state 3, or state 8 if the end was already requested. State 7 is the end request: it marks it, keeps a flowing lava going, else goes to 8. State 8 ends the lava, removes the instance from the manager and shuts it down. `GetPhase` maps states 1 and 5–8 to phase 2. disasters.md §3.2, §10, test 14.
- ~~**DS21:** is the toast list processed only when Update runs? [yes]~~ **ANSWERED 2026-10-15 (re-answers14):** MATCHES, with a consequence: `ProcessToastList` (at most 5 per call, from the front) runs only inside `Update`, which runs only while an episode keeps the instance on the animation tick (plus once at `Init`). Toasts queued while no fire burns wait for the next episode. disasters.md §5.7.
- ~~**DS22:** the contents of the special-building record.~~ **ANSWERED 2026-10-15 (re-answers14):** {name, cost}: the name is the BDS `GetBuildingName` (BDS slot 38) of the building's type (building occupant slot 12), and the cost is the occupant's int64 `0x8A1C3E72` (0 if absent). A record is pushed when the BDS profile flags (`GetBuildingProfile`, slot 39, `+0x1C`) have any bit of `0x1800400`. disasters.md §2.5.
- ~~**DS23:** the network default cost. [taken from the host]~~ **ANSWERED 2026-10-15 (re-answers14):** **DIFFERS.** It is one int64 constant, **10** (`0x00B0A1C0`, read only at `0x00578D41`), used for any network occupant whose `0x8A1C3E72` lookup failed. It is not per network type and comes from no host data. disasters.md §2.5.
- ~~**DS24:** the order in which the riot releases its removal locks. [insertion order, a choice the spec does not
  make]~~ **ANSWERED 2026-10-15 (re-answers14):** MATCHES in effect. `ClearRiotNetworkOccupants` walks the manager's `std::set` keyed by occupant pointer (ascending address), calling `SetRemovalFlags(2)` then Release on each. The order is not observable, so insertion order conforms. disasters.md §17.

## From the terrain core (filed by the coordinator from the `cores16` lane's report, 2026-10-16; merged `d651d74`)

cores16 implemented terrain.md as a Rust core (`engine/core/src/terrain/`), a C ABI (`fc4_terrain`, ABI 58) and
conformance vectors (`tr18_*`, `tr00_*`, `nt05_economy_modifier`), and nation.md GetEconomyModifier. There is no
plugin glue yet, so none of these readings has been checked in game. Each choice is marked `[choice]` in the code
and the vectors. The current choice is in brackets.

- ~~**TR4:** (terrain §11.3/§11.5) in what order are the lava neighbour offsets scanned? When several neighbours tie
  for the largest drop, the first one wins. [west, east, north, south, then the diagonals]~~ **ANSWERED 2026-10-16 (re-answers17):** **DIFFERS.** The offsets at terrain `+0x84`…`+0xA0` are W, NW, N, NE, E, SE, S, SW (`Init` `0x007500E0`–`0x00750129`; north is z − 1). The edges are scanned k = 0, 2, 4, 6, so **W, N, E, S**, then the diagonals NW, NE, SE, SW (`0x0075DB09`–`0x0075DBD6`). The first wins ties (strict `>`), as in the bracket; only the N/E order changes. terrain §11.3, test 15.
- ~~**TR5:** (terrain §11.5) does SetThingsOnFire call SetCause(3) after each ignited cell, or once per call? [per cell]~~ **ANSWERED 2026-10-16 (re-answers17):** MATCHES: `SetCause(3)` follows each ignited or burned cell, and only flammable cells (`0x0075DF3D`–`0x0075DF47`); the cells go x outer, z inner. terrain §11.5, test 16.
- ~~**TR6:** (terrain §10.2) what does GetWaterType return for a vertex with a NaN depth? [the comparisons fail, so it
  falls through to fresh water (2)]~~ **ANSWERED 2026-10-16 (re-answers17):** MATCHES: the first test (`depth ≤ 0`) is false for NaN, the level is NaN, and the second test returns 2 (`0x0073D811`–`0x0073D83E`). terrain §10.2.
- ~~**TR7:** (terrain §9.3) ComputeSpansForRectangle's footprint spans are not fully specified. [an inclusive vertex
  rectangle clamped to the terrain]~~ **ANSWERED 2026-10-16 (re-answers17):** **DIFFERS.** `ComputeSpansForRectangle` (`0x00740BB0`) scan-converts the rotated rectangle: rows `trunc(z / 16)` of the corners (unsigned clamp to `cellsZ`), half-open edge crossings per row `Z = 16z`, each span from the leftmost crossing column to one past the rightmost (unsigned clamp to `cellsX`), visited inclusive. The full rule, the corner order and a worked example are in terrain §9.3.
- ~~**TR8:** (terrain §9.1/§9.4) what is a brush description's starting angle, and is it stored per terrain or per
  description? [0, per description]~~ **ANSWERED 2026-10-16 (re-answers17):** MATCHES for what the brushes use. Each description has two angles: the stroke angle (`+0x6C`, used by oriented components) starts at 0; the walk angle (`+0x68`) starts at `RandomRange(0, 2π)` of the terrain generator when the walk step is non-zero. The step property `0x8A020102` defaults to 0.5° when missing, which is the case for every shipped brush, so each in-bounds application draws `RandomUint32Uniform(5)` (`0x00747EC0`, `0x0074D7BD`–`0x0074D855`). Stored per description in the static map, so shared by every terrain. terrain §9.1, §9.3.
- ~~**TR9:** (terrain §2.3) what is the starting value of the default brush level at `+0xBC`? [0]~~ **ANSWERED 2026-10-16 (re-answers17):** MATCHES at start, **DIFFERS** in game: 0 from the constructor (`0x0074C408`), then `DoPostCityInit` copies the 3D camera's zoom (`0x0074955F`) and every view-update message `0xA6B79602` with a new zoom updates it (`0x0074AAE2`). A harness with no view keeps 0; the glue should feed the zoom. terrain §2.3.
- ~~**TR10:** (terrain §13.1) no record holds the sea level, so what is it after a load? [the tuning's `new_sea_level`]~~ **ANSWERED 2026-10-16 (re-answers17):** MATCHES: `Init` sets `+0x7C` from the tuning value (`0x007500D5`) and `Load` never writes it. terrain §13.1, test 23.
- ~~**TR11:** (terrain §8.1) how are the data words of texture message `0x870BC918` packed? [no data, all zero]~~ **ANSWERED 2026-10-16 (re-answers17):** **DIFFERS.** data1 and data2 pack the render rectangle like the other two messages (the rect padded by 1 and clamped when the terrain flag is set), data3 is `(trunc(lo) << 16) + trunc(hi)` of the 3D view's altitude band, and data4 is the caller's (`0x00749894`–`0x007499EE`). Render-only; a view-less replacement may use the plain minimum and maximum. terrain §8.1.
- ~~**TR12:** (terrain §3/§15) what do the slots with unchecked indices (GetAltitudeAtIndex, SetAltitude(x, z) and
  others) do out of range? [a read returns 0 or NaN, a write does nothing]~~ **ANSWERED 2026-10-16 (re-answers17):** there is no defined original value: the listed slots form a flat index with no check and read or write past the array when it is out of range. The bracket is accepted as a *[choice]*, refined: reads return 0.0 (never NaN, matching the checked siblings), `GetWaterType` 0, `GetWaterVelocity` a shared zero vector, writes are ignored; and while the flat index is inside the array the access must happen (coordinates past the last column wrap to the next row). Also corrected: slots 30, 31, 33, 55 and 92 are bounds-checked. terrain §5, §10.5.
- ~~**TR13:** (terrain §10.4) does a power pole block navigation only on a cell with depth > 0? [yes: depth > 0 means
  GetWaterDepthForCell > 0]~~ **ANSWERED 2026-10-16 (re-answers17):** MATCHES: the pole handler sets the dirty flag when `GetWaterDepthForCell(x, z) > 0`, strict (`0x00740294`–`0x007402A4`). The flag lives at object `+0x50`, not `+0x48`. terrain §10.3.
- ~~**TR14:** (terrain §8.4) does SetWaterSurfaceDisplayState post on every call or only when the state changes?
  [every call]~~ **ANSWERED 2026-10-16 (re-answers17):** **DIFFERS**: only on a change. Slot 84 (`0x007434E0`) compares with `+0xDD` and returns when equal; otherwise it stores and posts. terrain §8.4, test 21.
- ~~**TR15:** (terrain §9.3) is a kind-0 brush reported as a change even when every touched vertex was locked or
  clamped? [yes: the out rectangle and the redisplay still happen]~~ **ANSWERED 2026-10-16 (re-answers17):** MATCHES: kind 0 returns true for every `b ≠ neutral` after calling `RaiseVertex` or `LowerVertex`, which skip a locked vertex silently (`0x00745C60`). terrain §9.4.
- ~~**TR16:** (terrain §9.4) with an angle of exactly pi, sin(pi) in f32 is not 0, which gives a one-pixel fringe on the
  earthquake stroke. Is that intended? [kept as computed]~~ **ANSWERED 2026-10-16 (re-answers17):** MATCHES: the original computes `fcos` / `fsin` of the f32 angle and stores f32 (`0x00742216`–`0x0074221E`), with no snapping, so it has the same fringe. The stroke angle itself is `f32(atan2 + f32(π/2))`. terrain §9.3.
- ~~**TR17:** (terrain §10.4, plumbing §7.2) for IsNavigable, are the city bounds behind slot 99 the terrain's cells,
  and does the search follow the plumbing §7.2 spiral? [yes to both]~~ **ANSWERED 2026-10-16 (re-answers17):** MATCHES: city slot 99 (`CellIsInBounds` `0x004594D0`) is `0 ≤ x < cellsX`, `0 ≤ z < cellsZ` with the same counts the terrain takes, and the walk is the plumbing `SpiralIterator` (`0x0043FE10` / `0x00440850`) over the unclipped square. terrain §10.4.
- ~~**TR18:** (glue contract) the lava and brush host callbacks run while the core holds the terrain mutably, so a
  callback that calls back into the same `fc4_terrain` handle is refused (and counted). Should the glue lane queue
  such work instead? [refused and counted; for the glue lane to decide]~~ **ANSWERED 2026-10-16 (re-answers17):** on the simulation's paths the original never calls back into the terrain from a lava or brush callee: the flammability and fire calls do not touch the terrain, and every terrain message is queued. Refuse-and-count is compatible (the count should stay 0); no queue is needed. Only the god-mode kinds 1 and 8–10 reach other objects that may read the terrain or are the surface water. terrain §15.4.

## From applying the re-answers10/11 DIFFERS entries (filed by the coordinator from the `apply12` lane's report, 2026-10-16; merged `463cca7`)

apply12 applied ZG18, ZG21, CM5, NG35-NG37, NG43, NG44 and DG35 (ABI 59, taken as 66 on the lane; FC4TestSeam op
66 installs a fake confirmation dialog). DG28 is not applied: the audited release in the growth developer's
destructor (`FC4_RELEASE_AT_DESTRUCTION`) stays. The lane's choices are in brackets.

- **NG45:** (neighbors.md §6.2, NG35) RequestCreateConnection's title and text are LTEXT keys, but the spec does not
  say which loader reads them or which group the two instance-only keys (`0x0A8E152D` text, `0x0A8E15F9` fallback
  name) belong to. [SC4LoadString's rule (group + language ID, then the group alone, U+2022 becomes U+200B), with
  both keys in the title's group `0x6A231EAA`; the sprintf template takes the name at the first `%s` and `%%` gives
  `%`; a missing LTEXT gives empty text (choice)]
- **DG39:** (building-development §1.4, DG28) after loading a saved city (Konradshohe, run `apply12-dev-1`), our
  growth developers are destroyed only after the simulator's Shutdown, not inside the BDS Shutdown, although the
  core shuts each one down and releases it there. Does the original's load-time object table (or anything else)
  hold a reference to a growth developer it resolved for the BDS record, and when is that reference released?
  [the destructor's audited release stays until this is answered]

## From the occupant manager and automata cores (filed by re-answers16 from the coordinator's task, 2026-10-16)

The occupant manager questions OM4–OM8 continue occupant-manager.md §16. AT2–AT5 are automata.md §13 Q2–Q5,
filed here under their own prefix. The implementers' choices are in brackets.

- ~~**OM4:** is the 0.1 m shrink constant (occupant-manager §5.1 step 5 and the standard-cell boxes of slots
  32/34/40) `float` or `double`, and at which address? [float]~~ **ANSWERED 2026-10-16 (re-answers16):** MATCHES.
  It is `float` `0x3DCCCCCD` at `0x00A8F768`, read as a 4-byte operand in Insert, Move, the FindOccupant helper and
  slots 32/34/40/47. The 1.0 width limit is `float` at `0x00A81228`. Every shrunk bound is stored as a `float`.
  occupant-manager §5.1, §7.
- ~~**OM5:** slot 34's clamp to the standard counts: 0..=count or 0..=count−1? [0..=count]~~ **ANSWERED 2026-10-16
  (re-answers16):** **DIFFERS**. A non-null min or max is clamped to 0…count − 1 (`0x0049E827`–`0x0049E8B7`). A null
  max becomes (count X, count Z), one standard cell past the edge, as in slot 40. occupant-manager §7, test 21.
- ~~**OM6:** out-of-grid and crash paths: Insert before Init; manager boxes with bytes past 191 or negative cells;
  slot 23 with a null filter; slot 28 outside the grid; FindOccupant on an occupant without a property holder. [no
  nodes / flag 1 set and Insert posted / true and no-op / none / no guess]~~ **ANSWERED 2026-10-16 (re-answers16):**
  occupant-manager §5.7 and §6.3. Where the original is defined, the brackets MATCH:
  - a **wrapped** box (low corner byte > high, e.g. a cell below 0 stored as 255) gets no nodes, while flag 1 is set
    and Insert is posted (loops `0x0049F0C8`/`0x0049F0EA`);
  - FindOccupant without a property holder guesses from two uninitialised locals; Insert and Move never place such
    an occupant, so skipping the guess gives the same results.

  The remaining cases crash or touch wild memory in the original, so the brackets are accepted as choices:
  - Insert before `Init` reads through the null grid at `0x0049F0F7`, after Create and the stored box. Insert after
    `Shutdown` works, because the rows survive.
  - Bytes 192–255 inside a running loop read past the grid in Insert, Remove and Move.
  - Slot 23 writes the type to address `0xC` (`0x0049DB87`).
  - Slot 28 (and slot 32) index the grid unclamped.
- ~~**OM7:** does slot 44 `IterateOccupants` skip the duplicate check for single-cell nodes like slots 36/38? [checks
  every node]~~ **ANSWERED 2026-10-16 (re-answers16):** **DIFFERS** in mechanism, with identical results:
  - it skips the check for bit-0 nodes (`0x004A1689`–`0x004A1695`);
  - a single-cell walk has no check at all;
  - the spec's §9.2 was also wrong about callbacks: slot 44 collects an AddRef'd batch per cell under an inner lock
    and calls back after dropping it, all inside the outer lock. So a callback may remove the occupant it was given.
    occupant-manager §9.2, test 23.
- ~~**OM8:** do `Write`/`Read` and slot 25 take the manager's lock? [yes]~~ **ANSWERED 2026-10-16 (re-answers16):**
  **DIFFERS**: none of them does (`0x0049ECF0`, `0x004A1A20`, `0x0049E490`). The extra recursive lock is not
  observable and may stay. occupant-manager §13, §14.
- ~~**AT2 (automata.md §13 Q2):** the meaning of the two `vec3`s and the `bool` of a controller-manager automaton
  set.~~ **ANSWERED 2026-10-16 (re-answers16):** it is an occupant's **queue**:
  - the line of automata, front first;
  - the queue point, 1 m outside the middle of the lot edge that touches a network (traffic network map slot 14);
    the lot's facing picks the first side;
  - the unit direction in which the line grows;
  - "point found".

  State 4 (`QUEUE`) appends, steers each automaton behind its leader, and removes (`0x0051BB60`–`0x0051BD50`; point
  search `0x00516070`). automata §3.3.
- ~~**AT3 (Q3):** the type numbers of RandomWalk, PlopJump, Plummet, PullOver, Bumped and Collide, and the meaning
  of corpus state types 0, 2, 3, 7, `0x0D`.~~ **ANSWERED 2026-10-16 (re-answers16):**
  - `0x0B` RandomWalk, `0x0E` Plummet, `0x0F` PlopJump, `0x11` Bumped, `0x14` Collide, `0x15` PullOver: the Windows
    factory `0x0050B7F0` sizes match the Mac classes, and the game's Lua `automata_states` table agrees;
  - 0 DEFAULT, 2 BEE_LINE, 3 CROWD, 7 IDLE;
  - 13 is the traffic automaton's blocked wait, pushed at `0x0054E322` for (tuning `0x00B21C74`) × 1000 ms.

  automata §6.2.
- ~~**AT4 (Q4):** the records of aircraft and watercraft automata.~~ **ANSWERED 2026-10-16 (re-answers16):**
  - aircraft base `0xC96E75DC`: `u16` 1 + automaton base (`0x004FB060`/`0x004FB080`);
  - airplane `0xCBEB4F62` and helicopter `0x8BA98EDA`: `u16` 1 + aircraft record + `vec3` home position
    (`0x004FD380`/`0x004FD3C0`, `0x005343C0`/`0x00534400`);
  - watercraft `0x096E75F5`: `u16` 2 + automaton base (`0x00566EE0`/`0x00566F00`).

  automata §6.4.
- ~~**AT5 (Q5):** the generator's two trailing `u32`s, the automaton base's unnamed fields, `CreateAutomaton`'s mode
  and last three arguments.~~ **ANSWERED 2026-10-16 (re-answers16):**
  - the generator `u32`s are the current automata group ID (corpus `0x4122`–`0x4139`) and the count spawned from it;
  - the base fields are named in automata §6.1 (attractor, distance, strength and attraction vector; model
    instance; speed and opacity tweens; priority; group ID);
  - `CreateAutomaton` is (group, const vec3& position, u32 automata flags, const key3& model key,
    cIGZMessageTarget2* callback, u8 priority, const vec3* orientation). It returns a pointer without a reference
    for the caller.

  automata §3.5, §6.1, §7.2.

## From applying the re-answers12 lot developer and zone developer answers (filed by the coordinator from the `apply13` lane's report, 2026-10-16; merged `adf64f4`)

apply13 applied LD2-LD30 (lot-developer.md) and ZG26, ZG30-ZG34 (zones.md, tract-developer.md §4.15.3); ABI 60, taken
as 71 on the lane. Not applied here: ZG22 (slot 19 always writes `*ok`) and ZG24 (the zone tool owns the highlight
region), from re-answers11. The lane's choices are in brackets.

- **LD31:** (lot-developer.md §6.6, LD17) UpdateLotObjectAltitudes: is the building's foundation redone only after the
  object actually moved (beyond the 0.1 tolerance), or for every building visited? [only after a move (inferred)]
- **LD32:** (lot-developer.md §6.5) AddNetworkConnectionNodes: when CreateConnectionNode (network manager slot 20)
  fails and leaves its out-parameter null, the original appends whatever was returned to the list that
  AutoconnectLotNodes (slot 21) reads. Is a null entry appended and passed on? [a null result is not appended; it is
  counted as `nodesFailed` (choice)]

## From applying the traffic glue answers (filed by the coordinator from the `apply11traffic` lane's report, 2026-10-16; merged `85919d3`)

apply11traffic applied the re-answers10 TG DIFFERS entries to the traffic glue (TG1, TG7, TG10, TG15/TG21, TG19, TG2
partly; ABI 61). `enable = traffic` with the 26 other regress replacements is clean to 2032 (run 128, ref_warnings=0).
Three TG2 callbacks stay NULL because the specs leave them open. The current choice is in brackets.

- **TG23:** traffic-simulator §4.9 step 3, §9.1 TG2, spec Q1: what exactly do UpdateRoadsBudget (`0x0070D8A0`) and
  UpdateMassTransitBudget (`0x0070DA40`) call? Which line item numbers get which network-type cell counts (roads:
  Road+OneWayRoad, Street, Highway+GroundHighway, Avenue; mass transit not given), with which method (SetFullExpenses?
  AddToFullExpenses?), what count x tile-cost formula and rounding, and are they also called when the count did not
  change? [`update_network_costs` stays NULL: the transport line items keep the expenses they were loaded with]
- **TG24:** traffic-network §4.4, §7.2, §9.1 TG2: what are the arguments, return values and result containers of path
  info slots 14 (GetPathsFromEntryEdge, 3 args), 3 (GetPath, 2 args) and 7 (GetPathsNearEndPoint, 6 args)? Which field of
  a path object is the exit byte, and which is its first vertex? Also: does GetPathInfo (occupant slot 49) hand over a
  reference, and does the slot-4 turn-around test use all of EAX or only AL? [`path_info_exits_from_entry` and
  `path_info_entries_near` stay NULL, so lane continuity changes nothing; GetPathInfo is treated as borrowed; the whole
  EAX is tested]
- **TG25:** traffic-network §4.8 step 1, §9.1 TG2: at what offset in cSC4NetworkTunnelOccupant is the paired-portal
  member, what type is it, and is it a counted reference? [`tunnel_paired_portal` stays NULL, so no tunnel record is
  made and tunnels carry no trips]
- **TG26:** traffic-simulator §4.1, §9.1 TG19: when does the original release its source iterator's connection snapshot
  on a city close: at Shutdown, or only when the simulator is destroyed? And does a Read that replaces the state release
  it? [released at Shutdown and when Read replaces the state; otherwise at the next Reset]

## From the automata layer core (filed by the coordinator from the `cores14a` lane's report, 2026-10-16; merged `44cf925`)

cores14a implemented automata.md (short form) as a Rust core (`engine/core/src/automata/`), a C ABI (`fc4_automata_*`;
ABI 62, reserved 65 on the lane) and conformance vectors (`am14_01`-`am14_14`); §14 test 12 is the formats save sweep
(`cargo test -p fc4-formats --release --test automata_records -- --nocapture`: 417 saves, every record of components
40, 41 and 43-46 and every referenced record re-encodes byte-identically). There is no plugin glue yet, so none of these
readings has been checked in game. The core predates re-answers16 (AT2-AT5 above), which a later lane applies. AM is the
prefix the code comments use; automata.md's own Q1-Q6 keep their numbers. The current choice is in brackets.

- **AM1:** (versions on re-save) the original's writers always write the current version (traffic manager 3 / base 6,
  automaton base 8, state 3, aircraft 5, watercraft 6), but §14 test 4 expects a version-2 record to round-trip byte
  for byte. Must a re-save upgrade old versions, or keep them? [writes the version it read; raises it only when the data
  needs the newer layout]
- **AM2:** §6.4 gives no read check for the pedestrian (5), vehicle, emergency vehicle, patrol car and trailer (2)
  versions, nor for the incident list's `u16` 1. Do the original readers reject other versions? [any value accepted
  and kept]
- **AM3:** §5.1 field 4 (the ID at the traffic manager's list cursor): what are the IDs of the list nodes, and what
  should a new manager write? [the value read is kept; 0 for a new manager]
- **AM4:** §7.3 SetMaxCount removes the automata "beyond" the new maximum: which ones? Does CreateAutomaton refuse at
  the hard limit? [the newest first; CreateAutomaton returns null at the limit]
- **AM5:** (§3.1) how exactly do the controller manager's two times A and B advance with HourChanged, and when can A
  differ from B?
- **AM6:** (§7.1 slots 5, 6, 9, 10) what does the `extra` argument mean? [ignored]
- **AM7:** (§7.1 slots 11 and 12) does the original track the controllers that an occupant group itself created?
  ["remove the controllers that the group added" is the occupant's controllers whose template is one of the group's
  templates, which also works after a load]
- **AM8:** §6.4 gives the emergency vehicle record as `u16 2, vehicle record, …`, but §12 quirk 3 says the vehicle
  loader "reads the vehicle part and leaves the rest unread". With the §6.4 layout a vehicle reader meets the
  emergency's own `u16` first and misaligns, failing at the occupant base version. Which layout is correct?
- **AM9:** §5.3 gives no upper version check for the watercraft manager. [fails above 6, as §5.2 does for the aircraft
  manager above 5]
- **AM10:** §6.4 names no IID for the trailer's list of vehicle references. [read with the automaton IID
  `0xA9B40F05`]
- **AM11:** (§4, §12 item 4) the original base Read ignores stream errors. Should a truncated manager record load
  partially, as in the original? [a short or cut record is a failed, atomic read, so round trips can be checked]
- **AM12:** (§7.3 slot 31, TrackAutomataGroupInstances) what are the semantics? [adds delta, does not go below 0,
  appends a new group]

## From the session seed (filed by the coordinator from the `seeding` lane's report, 2026-10-16; merged `77b7909`)

seeding implemented foundations §6.3 across every merged core that owns a generator (`engine/core/src/seeding.rs`,
`seeding::keys`, ABI 64) and the plugin's session seed (`session_seed = <u32>`, else the wall clock at game start). The
current choice is in brackets.

- **RN2:** foundations §6.3 and growth-developer.md §7: a decline key whose coordinate seed is `0xFFFFFFFF` makes `Seed`
  read the clock (growth-developer.md §7), but §6.3 says coordinate-seeded temporaries are unchanged. Is that case a
  clock-seeded temporary of the owning class, and is this the intended K? [derive(S, `0x0990C138` XOR n), n counting such
  reads per developer object, not saved; `seeding::keys::growth_decline_clock`, `growth_developer/simulator.rs`]
- **RN3:** foundations §6.2 / §6.3: the budget (RiskyMoney) and pollution-effects generators are file-local statics in the
  original, seeded once per process. In our cores each simulator object owns one, so the stream restarts at every city
  load in the same process (crime and the development cores already keep theirs process-wide). Should budget and
  pollution keep their generator across city loads like the original, or is a restart at each load an acceptable
  *[choice]*? [restart at each load]
