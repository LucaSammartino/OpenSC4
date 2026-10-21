# traffic-network: answered questions (archive)
<!-- Archive for [traffic-network.md](../../traffic-network.md). Lanes do not read this file. Open questions are in ../traffic-network.md. -->

## From the traffic network core (filed by the coordinator from the lane's report, 2026-10-13; merged with the core)

About `docs/specs/traffic-network.md`. The core implements every section except §4.14 (needs the simulator's safe lot
iterator), §6 persistence (the record is in traffic-simulator.md §6) and the GZCOM glue. Where the spec left a choice
open, the core's current answer is in brackets; "[none]" means the spec must decide.

<!-- from: From the traffic network core (2026-10-13) -->
- ~~**TN1 — §4.7.1:** does the step-1 "stop" also skip the ferry step 6? [yes]~~ **ANSWERED 2026-10-14** from the exe
  and written into traffic-network.md §4.7.1 step 1 and §12, and traffic-simulator.md §4.1.3. **Yes, and more:** the
  stop is a return from DoLotActivated (`0x00723BAC` → `0x00724188`). It also skips the `DoConnectionsChanged`, the
  bucket registration, the `RandomUint32Uniform(100)` draw (`0x00723FBE`), the budget items and the destination
  bits. Also corrected while tracing: the destination bits are **stored** over the lot rectangle, not ORed. The
  "byte size" is a raw byte count: the property is a u8 array in every shipped switch (§4.7.2), so no shipped
  building stops.

<!-- from: From the traffic network core (2026-10-13) -->
- ~~**TN2 — §4.7.1:** does step 6 run for a building without property `0xE90E25A1`? [yes]~~ **ANSWERED 2026-10-14**
  from the exe and written into the §4.7.1 intro and step 6. **Yes:** a missing property jumps straight to step 6
  (`0x00723B7B` → `0x00723ED0`). Ferry activation and its `DoConnectionsChanged` run if the building has
  `0x0BD86FD3`. Otherwise there is no rebuild, because no switch was created. Step 7 (traffic-simulator.md
  §4.1.3) follows in both cases.

<!-- from: From the traffic network core (2026-10-13) -->
- ~~**TN3 — §6 vs §4.7.2 (contradiction):** §6 says the switch record saves the rules; §4.7.2 says the save does not,
  so DoPostCityInit re-reads them. Which is right? [none]~~ **ANSWERED 2026-10-13** from the exe and written into
  §4.7.2 and §6. **§6 is right:** `Write` saves the rules and `Read` reads them for versions above `0x1C`.
  DoPostCityInit still re-reads every switch's rules (`0x00720466`) and **appends** them, so a loaded switch holds
  its rules twice *[quirk]*. This has no effect on the rebuild and shows only in the next save. *[choice]* FreeCity4
  replaces the rules instead.

<!-- from: From the traffic network core (2026-10-13) -->
- ~~**TN4 — §4.7.2:** when DoPostCityInit re-reads the rules, are the flags and the "to" mask recomputed, and which
  rectangle is used to refresh the access point? [recomputed; the building's bounding cells]~~ **ANSWERED
  2026-10-14** from the exe and written into §4.7.2 (the TN3 re-read steps, plus "Access points after the re-read").
  The bracket is **partly right.**
  - The mask and flags are recomputed only when the lot at the NW corner exists: the mask from the re-read entries,
    the flags from the first n rules. Otherwise the mask is 0 and the flags are as `Read` left them.
  - The access point is refreshed only for a switch with flag 4 and no saved access point. It is then the **NW
    cell** of the building's bounding cells, with no occupant search. A saved point is kept, and with flag 4 clear
    "has access point" := 0.

<!-- from: From the traffic network core (2026-10-13) -->
- ~~**TN5 — §4.6:** are tunnel cells and cells without car info reached but not extended, or skipped entirely?
  [reached, not extended]~~ **ANSWERED 2026-10-14** from the exe (`UpdateIntersectionData` `0x0071BB1B`–
  `0x0071BC3A`) and written into §4.6. **The bracket is right: reached, not extended.** The intersection test runs
  first, so such a cell can set `best`, and then the search stops there. The order of the stop tests and the absence
  of a visited set are now stated.

<!-- from: From the traffic network core (2026-10-13) -->
- ~~**TN6 — §4.12:** is `GetNeighborPoolPropertyHolder` asked with the off-map cell or the mapped in-city cell?
  [off-map]~~ **ANSWERED 2026-10-14** from the exe (`UpdateSubnetworkInfo` `0x0071674F`–`0x00716755`) and written into
  §4.12. **The bracket is right: off-map.** The mapped cell is used only to find the car record and its `subnet[]`.
  The holder is asked with the connection's own (off-map) cell.

<!-- from: From the traffic network core (2026-10-13) -->
- ~~**TN7 — §11 test 3:** the test lists L1's ring in a different order from BoundaryIterator. Only the order in which
  IDs are found depends on it. Which order is right? [BoundaryIterator]~~ **ANSWERED 2026-10-14** from the exe
  (`BoundaryIterator::GetNext` `0x0060CEE0`, set up with x as the fast coordinate at `0x0071633C`) and written into
  §4.12 step 1 and test 3. **The bracket is right: BoundaryIterator.** The order is first row, then the middle rows
  at x0 and x1, then the last row. So L1's ring is (1, 1), (0, 2), (2, 2), (1, 3), and the test now says so.

<!-- from: From the traffic network core (2026-10-13) -->
- ~~**TN8 — §4.9:** does the DoPostCityInit ferry pass rebuild the neighbour-cell vector and rotate the counters? [no]~~
  **ANSWERED 2026-10-14** from the exe (`0x00720C8A`–`0x00720CAE`, compared with the state-19 entry
  `0x0072134C`–`0x007214CA`) and written into §4.9.3. **The bracket is right: no.** DoPostCityInit only sets the
  re-route flags, points the pass at the first terminal and zeroes its sub-step. The neighbour step therefore walks
  the vector as it stands, which is empty after construction.

<!-- from: From the traffic network core (2026-10-13) -->
- ~~**TN9 — §4.9.5:** do the routes-in-use records report the last or the current counters? [last]~~ **ANSWERED
  2026-10-14** from the exe (`GetFerryRoutesInUse` `0x0071C250`) and written into §4.9.5. **The bracket is right:
  last.** Both the non-zero test and the copied counters read the last fields: `+0x20`/`+0x24` of a terminal link,
  `+0x2C`/`+0x30` of a neighbour link. The field offsets are now listed.

<!-- from: From the traffic network core (2026-10-13) -->
- ~~**TN10 — §4.9.1:** does a water marker with no qualifying side overwrite an earlier one? [no]~~ **ANSWERED
  2026-10-14** from the exe (`DoFerryTerminalActivated` `0x0072384A`–`0x007238AB`) and written into §4.9.1. **The
  bracket is right: no.** With no qualifying side, the loop ends without storing, so an earlier water cell and its
  flag stay. Also clarified: the stored water cell is the under-water **neighbour** N, not the marker's own cell.

<!-- from: From the traffic network core (2026-10-13) -->
- ~~**TN11 — §4.8:** does a re-insert overwrite the tunnel record, and does a portal with no pair get no record?
  [overwrite; no record]~~ **ANSWERED 2026-10-14** from the exe (`DoTunnelChanged` `0x00714154`,
  `0x00714386`–`0x00714396`) and written into §4.8. **The bracket is right on both counts.** A re-insert does
  find-or-insert and copies the whole record over it. A null paired portal (or no matching type) returns before
  that, so no record is created and an existing one is left as it was.

<!-- from: From the traffic network core (2026-10-13) -->
- ~~**TN12 — §4.2:** are edge-density entries created for both evening values? [both]~~ **ANSWERED 2026-10-14**
  from the exe (`DoConnectionsChanged` `0x0071AD20`–`0x0071AE1E`) and written into §4.2 step 3. **The bracket is
  right: both.** For every t on n, the evening-0 key and then the evening-1 key (`| 1 << 24`) are each looked up.
  A missing one is inserted as a copy of the zero record; an existing one is not touched.

<!-- from: From the traffic network core (2026-10-13) -->
- ~~**TN13 — IDs:** are `0x69D5C424` and `0x69D5C42D` grid instance IDs or classes? traffic-simulator.md calls
  `0x69D5C424` the "wormhole byte map". [none]~~ **ANSWERED 2026-10-13** from the exe and written into
  traffic-network.md §1 (Init step 5) and §2.3. Both are **instance IDs**, set by `SetInstanceID` (grid slot 6) at
  `0x0071EF56`/`0x0071EFD2`. The classes are the framework's u8 (`0x49B9E602`) and u32 (`0x49B9E606`) grids.
  `0x69D5C424` is the switch **mode-mask** grid that `DoLotActivated` writes (`0x00723E2B`), not a wormhole map:
  the wormhole bits live in the cell connection record. The name is corrected in traffic-simulator.md (grid table,
  §6.1 row 9) and save-records.md.

<!-- from: From the traffic network core (2026-10-13) -->
- ~~**TN14 — §4.10.2:** is the 0.2 a double or a float32 constant? [double, then 24-bit rounding]~~ **ANSWERED
  2026-10-14** from the exe (`0x00725370` `fmul dword [0x00AB34F4]`) and written into §4.10.2. **The bracket is
  wrong: float32** `0x3E4CCCCD`, as are 40.0, 0.1 and 1.0. The double would change the step cost for 25 depth
  bytes, for example 47 (`0x4081EB85` vs `0x4081EB86`). A worked example is in the spec.

<!-- from: From the traffic network core (2026-10-13) -->
- ~~**TN15 — §4.10.1:** is a water occupant's "cell rectangle" its single cell or its bounding cells? [single cell]~~
  **ANSWERED 2026-10-14** from the exe (`cSC4WaterPathFinder::DoMessage` `0x00724B19`–`0x00724BC8`) and written
  into the §4.10.1 table. **The bracket is wrong: bounding cells.** The rectangle is occupant slot 11
  `GetBoundingCityCells`, and only its NW corner `(x1, z1)` is tested under water. It is added to the dirty
  rectangle as is, not grown or clamped. For an ordinary one-cell network piece the two answers coincide.

<!-- from: From the traffic network core (2026-10-13) -->
- ~~**TN16 — §4.11 Fill step 3:** does a popped state resume after step 3 or redo it? [after; redoing it can loop
  forever]~~ **ANSWERED 2026-10-14** from the exe (`Fill` `0x00717EC0`: `0x00718186`, `0x007181A5`) and written
  into §4.11. **The bracket is right: after.** The state's k runs 0…5. Step 3 runs only at k = 4 and sets k := 5
  before pushing, so a popped tunnel state goes straight to the next pop.

<!-- from: From the traffic network core (2026-10-13) -->
- ~~**TN17:** what does `GetConnectedDestinationCount` return for a negative k? [0]~~ **ANSWERED 2026-10-14** from
  the exe (`0x007106C0`: `cmp k, 3` / `jle`, then index `7·id + 3 + k`) and written into §4.13 under the table. **The
  bracket is wrong:** a negative k passes the signed `k > 3` test and reads before `capacity`. k = −1…−3 read the
  record's population map object (k = −3 is its header pointer, a heap address); lower k read earlier records.
  The spec recommends returning 0, since no known caller passes k < 0.

<!-- from: From the traffic network core (2026-10-13) -->
- ~~**TN18:** what does `GetFerryTerminalCount` return for a type above 2? [0]~~ **ANSWERED 2026-10-14** from the
  exe (`0x0071FFF0`; constructor `0x0071FF7C`/`0x0071FF83`; activation `0x0072368A`) and written into §2.3's
  table, §4.9.1 step 1 and §4.9.5. **The bracket is wrong.** There is no bounds check: it returns the u32 at
  `+0x128 + 4·type`. Only types 0 and 1 have count words. Type 2 reads the potential-accident vector's begin
  pointer, and types 3 and above read its end, its capacity and later members.

  A side effect is that a type-2 ferry terminal can never activate, because its "count" is a heap pointer and so
  always ≥ 16. The spec recommends returning 0 for type ≥ 2.

<!-- from: From the traffic network core (2026-10-13) -->
- ~~**TN19 — §4.7:** are the switch flags taken from all rules or only from kind-0 rules? [all]~~ **ANSWERED
  2026-10-14** from the exe and written into §2.4 and §4.7.2. **All:** the loop at `0x00715EF8`–`0x00715F6D` has no
  kind test and covers the first n rules of the vector. A worked example (shipped station `0x0ADE0000`, flags
  `0x4C`) shows that the subway access point comes from a kind-1 rule.

<!-- from: From the traffic network core (2026-10-13) -->
- ~~**TN20 — §11 test 7:** the Walk-on-Road speed is not stated (3.5 was worked back from the stated Walk cost). Please
  list the shipped speeds of every travel mode.~~ **ANSWERED 2026-10-13** from the shipped data (Traffic Simulator
  exemplar `0xC9133286`, `SimCity_1.dat`). The full 9 × 13 table and the resulting `invMaxSpeed` bits for
  CellWidthX 16 are now in traffic-simulator.md §4.14, and test 7 states its speeds. Walk on Road is **3.5**, as the
  core inferred. Pipe, PowerPole and DirtRoad are 0 for every travel type.

## From applying TN1–TN20 to the traffic network core (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/traffic-network.md`. The core's current answer is in brackets.

<!-- from: From applying TN1–TN20 to the traffic network core (2026-10-14) -->
- ~~**TN21 — §4.6 / TN5:** if the intersection search's start cell is itself a tunnel cell, is it still extended?
  [yes, the start cell is always extended]~~ **ANSWERED 2026-10-14** from the exe (UpdateIntersectionData
  `0x0071B9A0`: the depth-0 entry runs the same tests, tunnel bit test `0x0071BBB8`, empty stack → `0x0071BD98`) and
  written into traffic-network §4.6 (the start-cell bullet and a worked example). **The bracket does not match: no.**
  The start cell goes through the same checks as every other cell. A tunnel start cell is not extended, so its
  search ends at once with `best = N + 1` and it keeps its table value (a Street tunnel next to an intersection
  stays at capacity 100, not 80).

<!-- from: From applying TN1–TN20 to the traffic network core (2026-10-14) -->
- ~~**TN22 — §4.7.2:** what count does an object-reference or empty variant give for `0xE90E25A1`? [0: a switch
  with no rules]~~ **ANSWERED 2026-10-14** from the exe (`GetCount` `0x0040CE60` reads field `+0x10`; constructors
  and `Destroy` `0x00914EA3` zero it; the object-reference setter `0x00915A5F` leaves it; the reader `0x005FFB40`
  copies `count` bytes) and written into traffic-network §4.7.2 ("Empty and object-reference values"). **The bracket
  matches.** Both give count 0, so 0 bytes are copied and DoLotActivated builds a switch with no rules (mode mask 0,
  flags 0, no access point). Only an absent property skips the switch.

<!-- from: From applying TN1–TN20 to the traffic network core (2026-10-14) -->
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

<!-- from: From applying TN1–TN20 to the traffic network core (2026-10-14) -->
- ~~**TN24 — §4.10.1 TN15:** when the unclamped dirty rectangle reaches past the map, are off-map cells asked or
  written during the update? [only in-map cells]~~ **ANSWERED 2026-10-14** from the exe (UpdateDepthMap `0x00724840`:
  no clamp and no bounds check; `IsNavigable` `0x0073EBF0` returns false through `CellIsInBounds` `0x004594D0`; the 0
  is stored at `0x007249A6`–`0x007249B2`) and written into traffic-network §4.10.1 ("The update"). **The bracket
  stands as a *[choice]*.** The original asks about every cell of the rectangle and writes 0 for each off-map cell
  through the raw rows. That write lands outside the grid, so it is an out-of-bounds heap write. In-map cells get
  the same values either way.

<!-- from: From applying TN1–TN20 to the traffic network core (2026-10-14) -->
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

## From applying the pathfinder, zones, landfill and neighbours answers (filed by the coordinator from the `apply4b` lane's report, 2026-10-14)

About PF1–PF15, ZQ4/ZQ7–ZQ10, LF1–LF5 and NB1–NB17 as applied in the core. The current answer is in brackets.

<!-- from: From applying the pathfinder, zones, landfill and neighbours answers (2026-10-14) -->
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

<!-- from: From applying the pathfinder, zones, landfill and neighbours answers (2026-10-14) -->
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

<!-- from: From applying the pathfinder, zones, landfill and neighbours answers (2026-10-14) -->
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

<!-- from: From applying the crime, traffic network and HG answers (2026-10-14) -->
- ~~**TN29:** TN21 worked example: Street (3) is not in the §4.8 tunnel type order, so a Street portal never gets a
  tunnel record. How does a "Street tunnel cell" get the tunnel bit in the game? [any tunnel record keyed by the cell
  sets the bit; the test and vector tn11_04d put a Road portal over a Street piece]~~
  **ANSWERED 2026-10-14** from the exe (`DoConnectionsChanged` step 5, `0x0071B390`–`0x0071B3FF`; tunnel insert
  `0x007140E0`) and written into traffic-network §4.2 step 5 and §4.6. Only through its cell key: step 5 sets wormhole
  bit 2 on every cell whose key is in the tunnel table, with no network or type test. So a cell whose car record is
  typed Street gets the bit when a portal of a listed type is keyed at the same cell; a Street portal itself never gets
  a record. The bracket matches, and tn11_04d is a valid synthetic case. Whether the game's tools can build such a cell
  was not traced.
