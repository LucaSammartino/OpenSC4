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

- **HG1 — foundations (load order):** are all grid records read before any simulator's `Init`, so that the first
  non-empty geometry is the final one? Which order does the class store read records and resolve refs in, relative
  to `Read` and `Init`? [assumed yes]

## From applying TN1–TN20 to the traffic network core (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/traffic-network.md`. The core's current answer is in brackets.

- **TN21 — §4.6 / TN5:** if the intersection search's start cell is itself a tunnel cell, is it still extended?
  [yes, the start cell is always extended]
- **TN22 — §4.7.2:** what count does an object-reference or empty variant give for `0xE90E25A1`? [0: a switch
  with no rules]
- **TN23 — §4.7.2 TN3/TN4:** how do the DoPostCityInit walks treat a switch with building 0 (a default inserted by
  a §4.7.3/§4.7.4 lookup), where the original would use a null building? [not re-read; flag 4 clear still clears the
  access point; flag 4 set gets no NW cell]
- **TN24 — §4.10.1 TN15:** when the unclamped dirty rectangle reaches past the map, are off-map cells asked or
  written during the update? [only in-map cells]
- **TN25 — §4.7.2 TN3:** FreeCity4 replaces the rules on the re-read, so the flags come from the property's rules;
  the original takes them from the first n rules of the vector (the saved copy). They differ only when the saved
  rules differ from the exemplar. Acceptable? [flags from the re-read rules]

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

- **LG1 — §4.4.5, §4.8.1:** what arguments does RemoveOccupant take for flora and base-texture removal? [(true, 3)]
- **LG2:** how is the base-texture occupant created and given the landfill texture (SetEmptyCellTexture)? [not done]
- **LG3:** how is the tile occupant created; is the SDK's `SetTileID` (slot 6) enough to attach the tile exemplar?
  [tiles live in FreeCity4 only; pollution's active-landfill flag is still set, which is what the simulation reads]
- **LG4:** who owns the reference in occupant manager slot 33's out-parameter? [slot 41 lists used instead]
- **LG5 — open question 6:** where does the zot orientation come from? [0]
- **LG6 — §3.3, foundations §13.4:** must the month run after pollution and budget? [not enforced]
- **LG7:** does FindNetworkTile's return carry a reference? [borrowed]
- **LG8:** tiles are not saved; they are rebuilt at PostCityInit (interior orientation 0, no random draw). [choice]
- **LG9:** is a null `GetLandfillZoneFromCell` acceptable to the city situation manager? [null]
- **LG10:** city slot 97 returns a float in ST0 (`docs/abi`) where the spec says "—". Which is right? [discarded]
- **LG11:** should the tile item cost be read from the tile exemplar? [no]
- **LG12:** should the shadow clamp GetGarbageSupply? [live plus delta, clamped]
- **LG13:** on Shutdown, does a zone release its zot or also remove the prop? [release only]
- **LG14:** the shadow's supply is live plus delta, clamped. [choice]
- **LG15 — §1.1:** only the city and the message server are held; terrain, pollution and budget are fetched per
  call. [as stated]
- **LG16:** the 3D view level is not reachable through a documented interface. [not done]
- **LG17:** the traffic network map comes through city slot 40 (`GetTrafficNetwork`, `intptr_t` in the SDK). Right?
- **LG18 — §1.1/§1.2:** which simulator does the original use for AddAgent before services are acquired? [the app's
  city's simulator, held until RemoveAgent]
- **LG19 — foundations §6/§12:** the core uses its own CRT-style generator per object (state 1), not the game's
  unseeded per-thread `rand()` sequence. Acceptable?
- **LG20:** a message from another thread that cannot get the lock is dropped. Acceptable?
- Not done (each logged once): tile models, `SetEmptyCellTexture`, zone effects (trucks, seagulls), zot
  orientation, the 3D view level, `GetLandfillZoneFromCell`. The shadow cannot compare the zots, trucks and seagulls
  the original drew, the tile occupants it changed, the expenses it booked or the history it sent.

## From the traffic simulator core (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/traffic-simulator.md`. All §11 tests 1 to 15 are unit tests and `ts11_*` vectors (29 vectors).
TN1, TN3, TN14, TN15 and TN18 hold against the merged network core. The core's answer is in brackets.

- **TS1 — §3.3:** does the pacing reset keep `rerouteBucket` (pass counter 2)? [kept; the other counters are zeroed]
- **TS2 — §3.2 state 11:** what does UpdateNeighborConnections (`0x00714BE0`) do? [nothing]
- **TS3 — §4.8 state 3:** what does ReduceTransitSwitches do? [the switch load restarts from 0; CreateMaps keeps the
  previous value in counter 1]
- **TS4 — §3.1:** what do DoRoadChanged / DoRailChanged do on occupant messages? [on insert the cell joins the damage
  block's intact set unless present; on remove it leaves]
- **TS5 — §3.1:** message `0x66956810` "marks the network as changed": which flag, and who rebuilds? [sets
  `network_changed`, consumed by the trip phases]
- **TS6 — §3.1:** what does the network part do on neighbour added / removed? [nothing]
- **TS7:** DoPostCityInit steps 4 (neighbour connections) and 5 (destination-map boundary pass)? [skipped; the
  network rebuild runs, then the transit multiplier]
- **TS8 — §4.9 step 2:** what is the commercial traffic window? [the 4 × 4 block at (x, z); travel types car 1,
  el-train 7, monorail 8; maximum morning usage plus maximum evening usage]
- **TS9 — §4.9:** which records are "not excluded" from the statistics? [flag bit 0 clear]
- **TS10 — slot 7 SetupPathFinderForLot:** what else is configured? [the lot rectangle as start, plus SetupTrip
  steps 3–4 only]
- **TS11 — slot 35 GetTransitSwitchQueryData:** the data fields? [returns the network core's query target only]
- **TS12 — §4.12:** what does RecalculateNeighborConnectionCapacities do? [nothing]
- **TS13 — §4.5:** a route with an off-map root is encoded as the in-city cell while its unit steps are relative to
  the off-map cell. [decoded literally: shifted by one cell]
- **TS14 — §4.3:** what does ConfirmRoute check? [during a strike any route using types 2, 3, 6, 7 or 8 fails; each
  unit step leaving an in-city cell needs an exit on that side for its type, or the cell's switch for a type change;
  jumps and off-map steps are not checked]
- **TS15 — §4.5:** is the switch fee the switch's `0x0BFC0A4C` value read as f32? [yes]
- **TS16 — §4.5:** the ferry-leg counters? [round(Q) to counter 1 (mode 0), 2 (mode 1) or both (mode 2), on the
  departure terminal's link to that cell, else its neighbour link]
- **TS17 — §4.4:** a highway-return adjustment for an off-map destination? [none]
- **TS18 — §4.2:** when is the capacity property promoted from u16 to u32? [u16 until a value reaches 0xFFFF]
- **TS19:** does the pathfinder adapter's GetLot match any cell of a lot? [yes (`false`)]
- **TS20:** is the traffic-source list a snapshot? [yes: lots, then connections, when the phase is entered]
- **TS21:** when are `toReRoute` / `toSearch` zeroed? [on entering state 16]
- **TS22:** does ReduceEdgeDensities also clear ferry terminal cells? [yes, departure and arrival; missing records
  are not created]
- **TS23:** what does DoLotDeactivated do? [destination bytes over the lot := 0, its bucket entries removed,
  `freeDest` unchanged]
- **TS24:** is the direction towards a non-adjacent rectangle defined? [yes, always]
- **TS25:** when is "the last switch seen" on a route updated? [at every in-city cell, so leaving and re-entering a
  station counts again]
- **TS26:** FinishTripMap donor search order? [distance first, then −x, −z, +x, +z]
- **TS27:** default `trafficUnitsPerSlice`? [16 until calibrated]
- **TS28 — tuning reads with short or missing data:** fewer vector entries [the rest keep their value]; the
  strategy re-read with fewer than 3 values [missing ones unchanged]; missing ordinance offsets [0]; wealth above 3
  [zeros].
- **TS29:** max funding conversion from percent? [`r24(v / 100)`]
- **TS30:** does Read commit only on success? [yes: parsed first; a failed Read leaves the simulator unchanged]
- **TS31:** an unknown iterator-state byte in the record? [Done]
- **TS32:** record versions above 0x29? [read with the 0x29 layout]
- **TS33:** Read's own switch flags: is 0x40 set for a subway / el-train rule? [yes, as the re-read does; TN25]
- **TS34 (fixtures only):** every cell reports a network occupant; department funding defaults to 1.0; department
  tokens are 900+.
- Not covered by vectors: path searches over a real network (the vector world has no network occupants; test 10
  drives ApplyRoute with the spec's route bytes); switches, tunnels and ferries in the save record round-trip only
  as empty lists (Read's switch mask and flags have a unit test). No plugin or capi integration yet (M5 glue).

## From the power grid shadow (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/power.md`. Ini: `power_shadow = 1` (needs our simulator and the ORIGINAL power simulator: do not
enable `power`); one line per recompute cycle at state 3, plus month, bookkeeping and month-summary lines. The
shadow's current answer is in brackets.

- **PW1:** `FindUnvisitedConnection` is not in the spec. The shadow takes the first connection from
  `GetConnectionsOfTypeToCity` whose visited flag is false. The original's search order? [first unvisited]
- **PW2:** the log date for SimNewDay, SimIdle and SimNewMonth comes from the message's `data1` as a julian date.
  Does SimIdle's `data1` carry the date? [assumed]
- **PW3:** the pass import, export and demand are not saved. What does the original hold after a load, before its
  first state 0? [counted as `not_exact` until then]
- **PW4:** `brokenFraction` is not saved; the shadow starts it at 0. The original's value after a load? [0]
- **PW5 — §5.3:** the dirty rectangle is not saved; the shadow uses the bounding box of the changed wire bits. Can a
  larger rectangle give different wire bits?
- **PW6:** the shortfall block probe tries starts 0 to min(count, 32768), assuming one `rand()` of at most 0x7FFF.
  True for networks with more than 32768 blocks?
- **PW7 — §7.5:** the ordinance factor is read live when the shadow starts, then each month. Does the original read
  it anywhere else (at load, on an ordinance change)?
- Not compared (cannot be read without changing the game): posted messages, history values, budget line items,
  demolitions and lot conditions, which poles the month's shuffle breaks (the broken count is compared), deal
  flags, the dirty rectangle.
- **PW8 — first in-game shadow (run 80, coordinator):** all 213 cycles and 94 month lines are exact, but every
  bookkeeping line differs a little: the original's running counts against our from-scratch insertion of the live
  occupants, e.g. consumers 725/722, requirement 6481/6480, 4 consumption-map cells (max |d| 2). Which occupants does
  the original count that a rebuild from the live occupants does not (or the other way round)?

## From the plumbing grid shadow (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/plumbing.md`. Ini: `plumbing_shadow = 1` (needs our simulator and the ORIGINAL plumbing simulator:
do not enable `plumbing`); one line per pass at the original's swap and after its state 8, one per month. The
shadow's current answer is in brackets.

- **PS1:** the order in which unvisited water connections are listed is inferred; the read-only host cannot find
  them, so the shadow reproduces it over its own visited flags. The original's order? [inferred]
- **PS2:** pipe condition is a distress fraction that drops to 0 at funding of 50% or more. Intended? [as stated]
- **PS3:** the original's pipe condition is not saved and not observable; the shadow starts its own at 0, so the
  first month after a load assumes the original also restarts at 0. Right?
- **PS4 — §5.3:** demand is a running sum the original builds as things are added; the shadow takes it from the
  original's record instead of recomputing it. Acceptable? [taken from the record]
- **PS5:** the monthly funding comes from department `0xAA4014B4`, a choice the spec does not state. Which
  department? [`0xAA4014B4`]
- **PS6:** a pass's start is seen only when state 2 changes a probe cell; if the next working grid already matches
  what state 2 would write, the start is missed and the pass is marked not exact. [as stated]
- **PS7:** when a state 8 may have run unobserved, ageing is logged as `unknown` and producer condition is not
  compared. [as stated]
- Not compared: the pipes' watered states (only reachable through a non-virtual call), redisplay, posts, history,
  line-item expenses, lot conditions and deal flags; the original's pipe condition and pass state (not saved, not
  exposed). New bursts are range-checked only (random draw).
- **PS8 — first in-game shadow (run 81, coordinator):** of 95 replayed passes, 4 differ only in `demand`, ours
  lower by 8 to 51 (e.g. 25315/25287 in 2026-03, 35967/35916 in 2030-11); ours equals the pass's consumed total.
  The original's demand is a running sum kept as things are added and removed. Which additions does it count that
  the pass's consumption does not (lots whose water use changed since insertion, removals not subtracted)? Compare PW8.
- **PS9 — first in-game shadow (run 81, coordinator):** in one pass (2028-03-04, pass 357) producer 0's production
  and efficiency read 0 in ours (orig 19944 and 0.997), although the pass's produced total matches (39687) and the
  grid is identical. This looks like a shadow-side read of the wrong producer record (not an RE question); the next
  plumbing lane should look at how the shadow pairs our producers with the original's.

## From applying the crime answers (filed by the coordinator from the `apply2` lane's report, 2026-10-14)

About `docs/specs/crime.md` (CQ5, CQ8, CQ13 as applied). The glue's current answer is in brackets.

- **CQ16 — CQ8:** the LTEXT string loads but the game's detokenizer returns false. [The load counts as failed;
  `0xAA12FCB0` replaces the name only if that property exists, otherwise the copied text stays.]
- **CQ17 — CQ8:** property `0x8A416B2E` has fewer than 3 values. [No key, no lookup; the fallback is used.]
- **CQ18 — CQ8:** there is no language manager. [Language 0, so only (group, instance) is tried.]
- **CQ19 — CQ8:** is the detokenizer returned by `GetStringDetokenizer` (`cISC4App +0xA8`) borrowed or owned?
  [Borrowed; the glue takes and releases its own reference around the call.]
- **CQ20 — CQ8:** how does the original read the LTEXT: through the resource manager's `GetPrivateResource`, or a
  string loader of its own? [Through `GetPrivateResource`, as the SDK does.]
- **CQ21 — CQ5:** do other generators seeded from `cRZTime` use the same seconds-since-1901 value, so the helper
  belongs to the RNG module? [Crime module only.]
- **CQ22 — CQ13:** the crime effect property is an array whose elements are not plain numbers (strings, objects).
  [Treated as absent.]

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
- **LV3:** is `GetAreaSlope` on the lot developer free of side effects? [assumed read-only]
- **LV4:** `GetLandValueType` is sampled at each tract's first cell. Is the type uniform over a tract, or which
  cell does the original use? [first cell]
- **FL1:** is 2 × tractsPerDay the right bound for tracts left unvisited at `SimCycleComplete`? It assumes the
  message is delivered queued, after the day agent. [2 × tractsPerDay]
- **FL2:** the multiplier g is 0 until the first season or ordinance message; the mirror takes the original's g at
  the baseline. Right? [taken at the baseline]
- **FL3:** a tract-size difference between the original and ours is reported as `INCOMPLETE`. [INCOMPLETE]
- **FL4:** the message server's delivery order to the original's target and to the mirror is unspecified; both read
  the same game state. Is that enough? [assumed so]
- **FL5 — first in-game shadow (run 82, coordinator):** 64 of 65 compared cycles exact; in cycle 66 (2033-03-15)
  a 2×2 block of adjusted tracts, (7..8, 53..54), is 81 in the original and 65 in ours (max |d| 21; raw grid
  identical). One lot's flammability contribution changed in a way the mirror missed or counted differently: which
  event sets an adjusted tract outside the day visit (a lot condition or occupant change mid-cycle)?
- **FL6 — second occurrence (run 83, coordinator):** 69 of 70 compared cycles exact; in cycle 15 (2027-08-11)
  595 adjusted tracts differ (raw grid identical; max |d| 21), starting with the column (0, 42..46): 100/80,
  106/85 original/ours, the city average 43 against 40..41. The original is about 1.25 times ours, the same ratio
  as FL5's 81/65, so a ×1.25 factor reached a whole region mid-cycle. Which event applies it (a fire, a disaster or
  an ordinance), and does it go through the day visit?

## From applying the data-format answers (filed by the coordinator from the `apply4a` lane's report, 2026-10-14)

About `docs/specs/data-formats.md` (EX1–EX20 as applied in `engine/formats` and the core variant). The current
answer is in brackets.

- **EX21 — §4.3:** "multiply the scale by 0.1": the double constant 0.1 or the float `0.1f`? [double 0.1: it
  reproduces every worked example, while `0.1f` turns `0.05` into `0x3D4CCCCE`]
- **EX22 — §4.3:** is `value × 10 + digit` rounded once or twice (after the multiply and after the add)? [twice,
  each to 24 bits]
- **EX23 — §4.3:** does the exponent accept a `+`, and is it read when no mantissa digit came first? [yes to both]
- **EX24 — EX8:** how is −0.0 printed? [`0`]
- **EX25:** signed decimal text read into a Float32: one rounding from the exact value, unlike a Sint64 scalar,
  which goes through double? [one rounding, so `-16777217` gives −16777216.0]
- **EX26:** a Bool scalar's "leading spaces trimmed": spaces only, or tabs too? [spaces only]
- **EX27:** a String value is "trimmed" of what? [spaces and tabs]
- **EX28 — EX8:** a Float32 signalling NaN as text? [the x87 load quiets it: `1.#QNAN`; a Float64 sNaN gives
  `1.#SNAN`]
- **EX29 — EX8:** does the 63-character cut also apply to Float32 text (a subnormal reaches it)? [yes]
- **EX30 — EX17:** `get_void` with a buffer shorter than `length`? [copies what fits; the original would overrun]
- **EX31 — EX15:** when do factories run? [eagerly, inside `load_package`; a factory registered later applies from
  the next `load_package`]
- **EX32:** a Float32 that no text reproduces under the 24-bit parser: what does the writer do? [`to_text` fails
  with `Unsupported`]
- **EX33 — §5.1:** the core property readers apply the scalar conversion steps to every array element (e.g. a
  Float32 array read as `u32`). Right for readers that use raw access plus their own conversion? [every element]
- Not applied: the scalar RZUnicodeChar text rule (one byte when ≤ 0xFF): the reader cannot produce that scalar
  (tag `0x0D` is not read).

## From the x87 fix: demand's corrected return channels (filed by the coordinator from the `x87fix` lane's report, 2026-10-14)

About `docs/specs/demand.md` §4.1 and §5 after the ABI correction (slot 8 `CalculateJobsPerUnitOfDemand` returns
void and writes jobs[1..3] of a four-float record). The current answer is in brackets.

- **DQ10 — §5:** `GetJobsBySensus` still reads `j0 + j1 + j2`, and ExtrapolateBusinessCensus reads `j[w]`,
  w = 0..2, but the outputs are now jobs[1..3]. Which elements are summed, and which one does wealth w read? [the
  three outputs, summed in order: f32(f32(out1 + out2) + out3); extrapolation reads output w for wealth w + 1]
- **DQ11 — §5:** in the original `GetJobsBySensus`, what is in the local record's jobs[0] when slot 8 returns, and
  is it really summed with jobs[3] left out? [not modelled; the sum of the three outputs]
- **DQ12 — §5 against §4.1:** §5 says `f32(percent) × 0.01f`, while §4.1 describes an exact FILD followed by a
  single rounding. They agree for every percent below 2^24; which is the game's? [the §4.1 link factor]
- Not a spec question: whether the WOW64 `CONTEXT` TagWord is the full or the abridged tag. The crash report logs
  both; the lane's host test shows the full tag (0xFFFF empty, 0x3FFF with one value) next to FXSAVE's 0x00/0x80.

## From the residential glue and shadow (filed by the coordinator from the `resglue` lane's report, 2026-10-14)

About `docs/specs/residential-education-health.md` (REH) and the residential glue (`cFC4Residential`,
`FC4ResidentialShadow`). The current answer is in brackets.

- **RG1:** buzzards (the visual effects): what creates them? [not created: the effect callback returns 0, logged
  once]
- **RG2 — REH §4.1 step 4:** building slot 11 is read as `GetBuildingProfile()`, first u32. Right? [yes]
- **RG3:** query-record costs are 32-bit in the game but 64-bit in the core. [zero-extended in, low 32 bits written
  back]
- **RG4:** slots 14/15 with a null group vector: the original dereferences it. [ours returns 0]
- **RG5:** lot `GetCenter` goes through the FXAM-guarded raw call (crime CQ3). [as crime]
- **RG6:** the shadow cannot reproduce the original's clock-seeded strike rolls in state 0. [those fields are not
  compared]
- **RG7:** does the original re-read ordinance effects outside state 0? [the shadow re-reads them]
- **RG8:** slots 21/22 called off the main thread create grids through GZCOM. Allowed? [yes]
- **RG9:** setters off the main thread: deferred, and `EndStrike` off the main thread returns the published flag?
  [yes]
- **RG10:** department funding with no department? [1.0]
- **RG11:** slots 19, 20, 21–25, 28, 47 and 49: always true? [true]
- **RG12:** slot 45 returning false: are its outputs left untouched? [untouched]
- **RG13:** does `SetLocalFundingPercent` set the budget item's local funding? [the shadow assumes so]
- **RG14:** the dirty flag cannot be compared by the shadow. [not compared]
- **RG15:** a building's identity is its `AsOccupant()` pointer, used for the lists, the strike message's void1
  and IGZUnknown, and record references. Right? [yes]
- **RG16:** per-building properties: scalars of their type (bool, u32, f32), or arrays? [scalars]
- **RG17:** slots 47, 49 and 61 append to the caller's list with the game allocator. Is that so for slot 61?
  [assumed]
- **RG18:** the simulator taken at Init step 4 is released after `RemoveAgent`; REH §4.2 has no matching release.
  [released]
- **RG19:** what goes in a strike message's data1? [0]
- **RG20:** `GetBuildingMapKey` returns 32 bits; only the low 8 are used. Right? [yes]
- **RG21:** the strike clock seed is `time() + 2177452800` (the cRZTime epoch), as in crime. [yes]
- **RG22 — `docs/abi`:** residential slots 37/38 (`GetCellEQByWealth`/`GetCellHQByWealth`) are `via-call`, low
  confidence, in `interfaces.tsv`, while residential.md §10.1 says ST0 (they end by calling the float grid's
  `GetTractValue`, x87 high). Should the rows say x87? [declared float, on the spec's word]
- **RG23 — first in-game residential shadow (run 86, coordinator):** every cycle, 12 of 15 daily states match
  exactly (all 25 grids, the saved fields, the per-building properties, funding and the scalars: population, EQ,
  HQ, life expectancy). States 1, 2 and 3 (the proximity grids R$, R$$, R$$$) differ in about 16,000 of 16,384
  cells: at the corner the original's grid reads 1, 0, 0, 0 along t(0,0..3) and ours 104, 103, 102, 101 (R$; R$$
  106.., R$$$ 116..). Ours looks like a distance ramp counting down from the corner; the original's is 0 or 1
  there. What do states 1–3 write into the proximity grid (a distance, a count within a radius, a flag), from
  which seed (the building map's keys `4p + w + 1`?), and how far does one state's call get? The shadow re-seeds
  this temporary grid from the building map as it stands. [ours as residential.md reads today]

## From applying the pathfinder, zones, landfill and neighbours answers (filed by the coordinator from the `apply4b` lane's report, 2026-10-14)

About PF1–PF15, ZQ4/ZQ7–ZQ10, LF1–LF5 and NB1–NB17 as applied in the core. The current answer is in brackets.

- **TN26 — traffic-network §2.7:** the sweep and the edge pass when Z < X: where do they stop? [at the array end]
- **TN27 — §2.7:** `FindNeighborConnection` and the pool-ID lookup for travel types above 8. [no types match]
- **TN28 — §2.7:** `GetTrafficNeighborConnection` for a corner cell or a cell inside the city. [an empty record]
- **ZQ11 — zones test 33:** a second draft `Place` on the same cell. [the mask bytes are merged (OR); inferred]
- **ZQ12 — zones test 33:** a draft `Place` over a cell that already has a network. [no draft, as a real `Place`]
- **LF6 — LF3:** should the core add the message-server release callback (an ABI bump) to match the original's
  shutdown order? [no: one `release_services` call after the zones shut down; LF3 says the difference cannot be
  observed]
- **NB18 — NB12:** the shim supplies the game's global `cISC4City` pointer through the new host method
  `current_city_object`. Is that the object `UpdateCommodityStats` compares against? [yes; the test hosts use a
  token distinct from every regional city ID]
- Not applied: PF6 beyond the record level (pools, capacities and the roll stay traffic-simulator host methods, as
  the answer says); NB12/NB17 glue (there is no neighbours glue yet).

## From applying the protection, demand and pollution answers (filed by the coordinator from the `apply3` lane's report, 2026-10-14)

About PR1–PR13, DQ3–DQ8 and PG4–PG13 as applied. The current answer is in brackets.

- **PR14 — protection §5:** the dispatch active/inactive handler: which occupant does `GetDispatchEffect` read?
  [`info.station`, as UpdateUnadjusted; inferred]
- **PG15 — PG11, slot 29:** with a negative supply and n > 0, is `0x7FFFFFFF − s` computed in int32 (wrapping) or
  exactly? [exactly, in 64 bits, so the change equals n]
- **PR15 — PR12:** how long does the protection simulator hold the budget simulator and the zone manager? [taken in
  base init only if not already held, released at the end of Shutdown; the read-only shadow host asks the city each
  time]
- **PR16 — PR5:** how does a shadow recognise the original class before reading +0x84, and are unadjusted
  mismatches judged like the adjusted grid's? [by the primary vtable against the §15.1/§15.2 addresses (compat
  table, identified executable only; otherwise not read); logged, not judged]
- **DQ13 — DQ5:** does the default `building_purpose` go through `building_profile`, and may the host's visitor be
  NULL? [yes to both]

## From the budget shadow (coordinator, run 86)

- **BQ3 — budget.md, the monthly settlement total:** in run 86 (`regress rbc7f`), 30 of 31 months match. In
  2028-08 every one of the twelve tax lines is equal (1762, 767, 123, 98, 101, 30, 83, 34, 0, 1547, 290, 0), but the
  total income is 4839 in the original and 4840 in ours (net 792/793). The lines sum to 4835, so the total is not
  their integer sum: it comes from unrounded values. Which values does the original add for the total (float
  per-lot sums, or per-line floats before display rounding), in which order and width, and how is the total
  rounded? First seen in this run; runs 80–83 had none in 93–95 months. [ours as budget.md reads today]
