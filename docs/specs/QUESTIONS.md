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

**ZQ4 — no vector covers highlight mode.**
`HighlightParcels` (§7.1), virtual lots, virtually deleted cells, and the §7.13 draft fallbacks are all untested. The
draft fallbacks need the network tools' `OccupantExists(x, z, flag, &mask)`, which §8.8/§7.13 describe only for highlight
mode. Partly a headless limit (the highlight is a UI preview) and partly a missing spec test.

~~**ZQ5 — §7.3 `ExamineBoundaries` city-border behaviour is untested.**~~ **ANSWERED 2026-10-13** from the exe:
the clamp was checked on the bytes (`0x00731C8D`) and zones.md §15 now has **test 31**, with both low edges, both
high edges and the placeStreets-off case. §7.3 now states the consequence: at the border the examined cell is R's
own zoned, empty cell. It scores like an empty outside cell (+1 per cell with `placeStreets`), so a border side is
neither skipped nor scored as an obstacle.

~~**ZQ6 — §7.5 step 1 table, three of six cells unpinned.**~~ **ANSWERED 2026-10-13** from the exe. The table was
checked on the bytes and is correct as written: Q is `+0xCC`, D is `+0xD0`, and the jump table is at `0x0072EC44`.
zones.md §15 now has **test 32**, which gives the street positions for all eight perpendicular (Q, D) pairs on a
16 × 16 rect (n = 6), plus the edge rule. That pins every arm, including the four listed here.

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
- **TN4 — §4.7.2:** when DoPostCityInit re-reads the rules, are the flags and the "to" mask recomputed, and which
  rectangle is used to refresh the access point? [recomputed; the building's bounding cells]
- **TN5 — §4.6:** are tunnel cells and cells without car info reached but not extended, or skipped entirely?
  [reached, not extended]
- **TN6 — §4.12:** is `GetNeighborPoolPropertyHolder` asked with the off-map cell or the mapped in-city cell?
  [off-map]
- **TN7 — §11 test 3:** the test lists L1's ring in a different order from BoundaryIterator. Only the order in which
  IDs are found depends on it. Which order is right? [BoundaryIterator]
- **TN8 — §4.9:** does the DoPostCityInit ferry pass rebuild the neighbour-cell vector and rotate the counters? [no]
- **TN9 — §4.9.5:** do the routes-in-use records report the last or the current counters? [last]
- **TN10 — §4.9.1:** does a water marker with no qualifying side overwrite an earlier one? [no]
- **TN11 — §4.8:** does a re-insert overwrite the tunnel record, and does a portal with no pair get no record?
  [overwrite; no record]
- **TN12 — §4.2:** are edge-density entries created for both evening values? [both]
- ~~**TN13 — IDs:** are `0x69D5C424` and `0x69D5C42D` grid instance IDs or classes? traffic-simulator.md calls
  `0x69D5C424` the "wormhole byte map". [none]~~ **ANSWERED 2026-10-13** from the exe and written into
  traffic-network.md §1 (Init step 5) and §2.3. Both are **instance IDs**, set by `SetInstanceID` (grid slot 6) at
  `0x0071EF56`/`0x0071EFD2`. The classes are the framework's u8 (`0x49B9E602`) and u32 (`0x49B9E606`) grids.
  `0x69D5C424` is the switch **mode-mask** grid that `DoLotActivated` writes (`0x00723E2B`), not a wormhole map:
  the wormhole bits live in the cell connection record. The name is corrected in traffic-simulator.md (grid table,
  §6.1 row 9) and save-records.md.
- **TN14 — §4.10.2:** is the 0.2 a double or a float32 constant? [double, then 24-bit rounding]
- **TN15 — §4.10.1:** is a water occupant's "cell rectangle" its single cell or its bounding cells? [single cell]
- **TN16 — §4.11 Fill step 3:** does a popped state resume after step 3 or redo it? [after; redoing it can loop
  forever]
- **TN17:** what does `GetConnectedDestinationCount` return for a negative k? [0]
- **TN18:** what does `GetFerryTerminalCount` return for a type above 2? [0]
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

## From the residential tests and record (filed by the coordinator from the lane's report, 2026-10-14; merged with the core)

About `docs/specs/residential.md` and `residential-education-health.md` (REH). All 24 §15 tests pass. REH tests
1–7, 10 and 18–21 belong to the host's education/health hooks. The core's current answer is in brackets.

- **RS7 — §15 test 8 vs the shipped LE curve (inconsistency):** the shipped Health Quotient → Life Expectancy curve
  gives 65 at HQ 60, not the 61 the test states. The vector uses a flat custom curve to get round it. Which is
  right: the test's number, or the curve the test should use? [custom curve in the vector]
- **RS8:** does the "other" list move into the health-affecting list? The spec does not say so. [moved; inferred]
- **RS9 — record Read:** what should Read repair after a stream error, and what happens with a version it cannot
  read? [nothing repaired; Read fails]
- **RS10 — record Read:** are list and department entries that a Read replaces released? [released, departments
  with `release_department`]
- **RS11:** how do coverage and list vectors with counts other than 7 / 3 behave? [none]
- **RS12:** can the lists hold null entries, and what does each pass do with one? [skipped]
- **RS13:** do the state-0 grade totals live in the residential simulator or in the host (REH)? [host]
- **RS14:** what does the state machine do in an unknown state? [waits]
- **RS15:** is the pollution loop's S capped (a hostile S is unbounded)? [no cap]
- **RS16:** what does a zero divisor give in the averages? [0]
- **RS17:** does a version-3 record keep the running population the constructor set? [yes]
