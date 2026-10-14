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
- **PG4 — §4.10 visual effects:** the effects manager's `CreateVisualEffect` and the `cISC4VisualEffect` /
  `cS3DTransform` layouts are not in `docs/abi`. [visual effects not implemented]
- **PG5 — §10.2 Q1:** is the prop requester re-added after a load? [open in the spec]
- **PG6 — shadow method:** the shadow captures the original's state by calling the original's own `Write`. Is that
  acceptable as read-only? [used]
- **PG7:** `radiationSum` is not saved, so the radiation history at the end of a cycle after a load diverges from a
  run without the load. Intended? [as the spec]
- **PG8 — §4.6.1 step 3:** is the unsigned `wteRemaining` converted to float as signed or unsigned? [as written]
- **PG9:** can landfill usage change within a day? [the shadow keeps the first value it reads]
- **PG10:** without an ordinance simulator, `cumulative_factor` returns 1.0. [choice]
- **PG11 — slots 29/30:** they return int32 (the applied delta and the argument respectively) where the SDK has
  bool. Confirm. [int32]
- **PG12 — §5.2 posts:** no void or IGZUnknown is set on them, since the spec names none. Confirm. [none set]
- **PG13 — slot 5:** what does it return for a type above 3? [null; choice]
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
- **EX10:** the ParentCohort line: field count and spacing. [exactly 3 fields, leading spaces trimmed]
- **EX11:** are PropCount, reps and the type name trimmed? [PropCount and reps trimmed; the type name must match
  exactly]
- **EX12:** is there a cap on reps? [more reps than the record length fails]
- **EX13:** are numeric values cut at `}` too? [no, only strings]
- **EX14:** how is Float32 text parsed? [as f64, then rounded to f32; no hex/inf/nan]
- **EX15:** what counts as a resolvable kind-1 resource key? [an exemplar or cohort that loads, or a key registered
  from the package index]
- **EX16:** shipped data: `Plugin_002_TheLivingMall.dat` writes `Uint8:0:{}` for two LotConfig properties, now read
  as a scalar 0 (§4.3). Intended? [scalar 0]
- **EX17:** slot 5 `GetProperty(id, void*, uint32&)`: what type does the caller request? [not implemented]
- **EX18:** are §10.3 tests 18–22 (resource-manager load order) in scope for `engine/formats`? [out of scope]
- **EX19:** how is depth counted for `MAX_PARENT_DEPTH`? [at most 32 nested loads below the record; a record that
  fails only on depth is retried when reached shallower]
- **EX20 — §5.2:** a raw accessor on a type mismatch. [choice: a missing value]

## From the protection glue and grid shadow (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/protection.md` (fire and police) and its `docs/abi` rows. The glue's current answer is in brackets.

- **PR1 — §5 dispatch record:** "callID" is taken as the record's `+0x08` field (vehicle slot) and "active" as the
  `+0x10` flag (state 5). [inferred]
- **PR2:** the dispatch automaton is handed to the core by a `cISC4Occupant` (`0x26BB767E`) QueryInterface, treated
  as borrowed. Right identity? [as stated]
- **PR3:** `AddControllersFromOccupantGroup(name, occ, 0)` and its Remove: the name as an `SC4String` gives slots 10
  and 12. Right overload? [yes]
- **PR4:** the fire simulator getter is city slot 57, inferred from the SDK order; the shadow depends on it. §14 lists
  only police slot 65. [57]
- **PR5:** is there a read-only way to get the original's unadjusted int16 grid? [no; only the adjusted grid is
  compared]
- **PR6:** city slot 97: interfaces.tsv says ST0, §14 lists only its outputs. [outputs used, ST0 discarded]
- **PR7:** which slots does the game call from its second thread, and are these fallbacks acceptable: lists return 0
  untouched; chance of strike, efficiency and radius return 0; strike length 0xFFFFFFFF with no date, else 0; scalars
  from the month-end snapshot? [as stated]
- **PR8:** slots 21/22/32, dispatch-source slot 6 and slot 42 (non-empty) clear the caller's list without releasing
  its elements, which leaks the caller's references as the original does. Intended? [reproduced]
- **PR9:** the property setters `0x005FE3C0` / `0x005FE6B0`: scalar u32 or a one-element vector? [open]
- **PR10 — §5.3:** the zone manager's slot-5 grid class is not in docs/abi; `GetZoneType(x, z)` (slot 12) is used
  instead, 0 outside the city. Same values? [slot 12]
- **PR11 — §12:** "void1 = the simulator" posts the primary interface pointer. Right pointer? [primary]
- **PR12 — §4.1:** the simulator, flammability and crime simulators are not held (never called); only reference
  counts differ. Acceptable? [not held]
- **PR13 — §13.2:** the air sources are created through GZCOM and their Init/Shutdown called by address while our
  replacement classes are active. Intended reading? [yes; no air dispatch on an unidentified executable]
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

- **CQ1:** `GetCrimeIncidentArray`: allocators.md does not list its container. [the game's allocator]
- **CQ2:** `PropProfile.crimeIncidentID` is read at offset 24, from the SDK layout; layouts.md does not list
  `PropProfile`. [24]
- **CQ3:** lot `GetCenter` (slot 15): docs/abi says ST0, §13 says the out parameter with nothing left on the x87
  stack. [both handled; the month line counts which one happens as `center st0=a/b`]
- **CQ4:** `GetOccupantsByBBox`: §13 names the filter overload (slot 36). [the typed overload, slot 37, last
  argument 0xFFFFFFFF]
- **CQ5:** the session seed "local wall-clock seconds". [`_mkgmtime(localtime(now))`]
- **CQ6 — §16.1:** the ordinance crime factor after a load with no ordinance simulator. [left unchanged]
- **CQ7:** the prop requester is never unregistered at Shutdown; the spec is silent. [not unregistered]
- **CQ8 — §16.8:** crime type names: which LTEXT key, and the fallback when it is missing. [open]
- **CQ9:** `Read` on a stream error. [returns false, no state changed]
- **CQ10:** crime type list order. [ascending instance]
- **CQ11:** police slot 5 returns a uint8 grid (the SDK says short) and slot 27 an int32 count, as protection
  declares them. Confirm. [as protection]
- **CQ12:** arson calls `SetCause` whatever `IgniteOccupant` returns. Right? [yes]
- **CQ13:** property 0xCA5B9306 is read with numeric conversion (`GetU32s`, low bytes). [as stated]
- **CQ14:** a failed QueryInterface nulls the out pointer per crime.md §1; flammability, plumbing and pollution leave
  it untouched. Which is the original's rule per class? [crime nulls]
- **CQ15:** the return value of the residential `GetPopulationGrids` / `GetEQGrids` is ignored; only the three out
  pointers are used. [ignored]
- Not compared by the shadow (stated on every line): the number of crimes to commit (needs
  `ResetUnjailedCriminals`, which has side effects), which attempts the police stopped and each crime's type (the
  original's draws), which lots were tried (the lot manager's generator), the crime-effect grid (no interface).

## From the demand glue: capacity maps and population summary (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/demand.md` (§6.3 capacity, `GetLocalPopulationSummary`) and the STLport map layout in
`docs/abi/allocators.md`. Switches `demand_capacity` and `demand_direct_calls` (both on by default). The glue's
current answer is in brackets.

- **DQ3:** a NaN or out-of-range summary value: Rust `as i32` saturates and gives 0 for NaN; the game's CRT
  conversion gives 0x80000000. Which does the original produce? [saturating]
- **DQ4:** a map with count 0 but a non-empty tree shape. The spec only says "with a count of 0, the map is left
  untouched". [refused]
- **DQ5:** the census asks for the profile at every cell, so the capacity walk runs there too, although §6.3 needs
  only the purpose. Intended? [walks at every cell]
- **DQ6:** the header colour of a copied capacity map is assumed to be 0. [colour 1 refused]
- **DQ7:** limits of 16 maps, 128 capacity entries and 4,096 summary nodes. Are larger real maps possible? [refused
  above them]
- **DQ8:** which thread calls `GetLocalPopulationSummary`? [unspecified; the first success is logged with its thread]
