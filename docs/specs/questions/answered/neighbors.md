# neighbors: answered questions (archive)
<!-- Archive for [neighbors.md](../../neighbors.md). Lanes do not read this file. Open questions are in ../neighbors.md. -->

## From the landfill tests and the neighbours core (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/landfill.md` (tests §11 1–23, vectors `lf11_*`) and `docs/specs/neighbors.md` (tests §18 1–12,
vectors `nb18_*`). The core's current answer is in brackets.

<!-- from: From the landfill tests and the neighbours core (2026-10-14) -->
- ~~**NB1:** the Julian day of `DEFAULT_DATE`. [0]~~ **ANSWERED 2026-10-14** from the exe: there is no fixed default.
  The deal's date objects are default-constructed from the **wall clock** (`0x0091307C`: `time`, `localtime`, then
  `Jday`), so a deal built on 2026-10-14 starts at 2461328. The bracket's 0 is kept as a *[choice]*, because the
  original is clock-dependent. The date survives only in fields nothing reads before overwriting them (neighbors.md
  §5.1).

<!-- from: From the landfill tests and the neighbours core (2026-10-14) -->
- ~~**NB2:** does `Activate` change the flags directly, or through `SetFlags` with its status-0 reports? [directly]~~
  **ANSWERED 2026-10-14** from the exe (`0x006CE640`): **the bracket does not match.** `Activate` calls its own
  virtual `SetFlags` three times: (Active, true), (Proposed, false), (Cancelled, false). Each call that changes a
  non-Silent deal posts status 0, so activating a proposal posts 0, 0, then `simulate_deals`'s 1 (neighbors.md
  §5.4, §7.2).

<!-- from: From the landfill tests and the neighbours core (2026-10-14) -->
- ~~**NB3 — §6.3:** does the marker altitude `GetAltitude(x, z)` use the connection cell as given or the inner cell?
  [as given]~~ **ANSWERED 2026-10-14** from the exe (`0x006D494A`–`0x006D4A4F`): **the bracket does not match.** The
  height is `GetAltitude(px, pz)` at the **inner** cell's centre, the same point the prop is placed at. The
  connection cell is not used (neighbors.md §6.3, with an example).

<!-- from: From the landfill tests and the neighbours core (2026-10-14) -->
- ~~**NB4 — §12 vs §7.1:** ProposeDeals reads `0xCA469346` (f32) in §12 but `0xCA46931F` in §7.1. [`0xCA46931F` for
  proposals; `0xCA469346` as GetDealCost's f64 maximum markup]~~ **ANSWERED 2026-10-14** from the exe: the bracket
  matches. `ProposeDeals` reads both. The `0xCA469346` read (`0x006D22B4`) becomes `1 + v` in a local that is never
  read; the budget test uses `0xCA46931F` (`0x006D24EE`). Both sections were right, and neighbors.md §12 now marks
  the read as dead.

<!-- from: From the landfill tests and the neighbours core (2026-10-14) -->
- ~~**NB5:** what does `UpdateCost` do with no current simulator? [nothing]~~ **ANSWERED 2026-10-14** from the exe:
  the original **crashes**. `UpdateCost` (`0x006CF9C0`) checks neither the global city nor `GetNeighborsSimulator()`
  (`0x006CF9E8`); only `SetAmount` guards both. The bracket is kept as a *[choice]*: no cost change and no report
  (neighbors.md §5.4).

<!-- from: From the landfill tests and the neighbours core (2026-10-14) -->
- ~~**NB6 — §2.2:** `neighbour_connection_cell` for an interior cell has no row. [`None`]~~ **ANSWERED 2026-10-14**
  from the exe (`0x006CE920`): **the bracket does not match.** The outputs start as null, 0, 0. An interior cell
  keeps our own region position, finds our own regional city there, and returns **true** with `(0, 0)`.
  `FindConnectionOnOtherSide` still ends with null, so only the helper's result differs. neighbors.md §2.2 has the
  row and an example.

<!-- from: From the landfill tests and the neighbours core (2026-10-14) -->
- ~~**NB7 — §13:** null or host references in the connection list are skipped on read. Correct? [skipped]~~
  **ANSWERED 2026-10-14** from the exe (`0x006CFE10`–`0x006CFEDD`). Failing references (wrong interface, unknown
  class) are skipped: the bracket matches. A **null** reference succeeds: the original appends the null and then
  releases it unchecked (`0x006CFEC5`), so it crashes inside `Read`. Skipping it is kept as a *[choice]*
  (neighbors.md §13 table).

<!-- from: From the landfill tests and the neighbours core (2026-10-14) -->
- ~~**NB8 — §8.1:** does `existing` include the deal being priced? [yes: its own `UpdateCost` counts it; 262 in
  `nb18_03`]~~ **ANSWERED 2026-10-14** from the exe (`0x006D33A5`–`0x006D33E1`): the bracket matches when the deal
  is in this simulator's list and is the first of its type with that city. `existing` is the first `FindDeals(type,
  C)` match, with no flag filter. An earlier deal of the same type and city takes its place, and an unlisted deal
  never counts itself. The 262 is reproduced in neighbors.md §8.1.

<!-- from: From the landfill tests and the neighbours core (2026-10-14) -->
- ~~**NB9:** §5.2 says pricing ignores `0x3D` deals, but §8.1's `FindDeals` has no filter. [as §8.1]~~ **ANSWERED
  2026-10-14** from the exe: the bracket matches. Both `FindDeals` overloads (`0x006D0760`, `0x006D0840`) match on
  type and city only, and `GetDealCost`/`GetDealMaxAmount` check no flags. §5.2 was wrong and now limits the `0x3D`
  rule to the totals (neighbors.md §5.2).

<!-- from: From the landfill tests and the neighbours core (2026-10-14) -->
- ~~**NB10:** SetAmount's synchronous report re-enters `simulate_deals` during `ProposeDeals(add = true)` and
  activates the proposal. Intended? [as written]~~ **ANSWERED 2026-10-14** from the exe: the bracket matches, and
  it must be kept. In `0x006D0520` the order is: un-Silence, append, then `SetAmount(0)`, which sends status 0
  synchronously. The handler runs a full `simulate_deals`, since `ProposeDeals` sets no busy flag, and that run
  activates the proposal. `Propose` writes the dates afterwards (neighbors.md §7.1, step by step).

<!-- from: From the landfill tests and the neighbours core (2026-10-14) -->
- ~~**NB11:** the store ignores a false `Read` (foundations §3.5), so a version-3 simulator record loads "clean" with 0
  connections. Confirm. [yes]~~ **ANSWERED 2026-10-14** from the exe: confirmed. `Read` returns false right after
  the `u16` (`0x006CFDDB`), with the stream still good. The store ignores the result, and the freshly created
  simulator keeps its empty list (neighbors.md §13).

<!-- from: From the landfill tests and the neighbours core (2026-10-14) -->
- ~~**NB12:** `UpdateCommodityStats` takes a regional-city token. Is that the right subject? [yes]~~ **ANSWERED
  2026-10-14** from the exe (`0x006D2A2C`–`0x006D2A46`): **no.** The argument is the `cISC4City*`, compared by
  identity with the current city. Null or any other pointer, including our own regional-city object, does nothing.
  The core should take a city handle. Also, `R` is not null-checked (`0x006D2A6C`); returning early there is a
  *[choice]* (neighbors.md §8.2).

<!-- from: From the landfill tests and the neighbours core (2026-10-14) -->
- ~~**NB13:** a present `0xEA46934F` of 0 is divided by literally (inf or NaN); a missing one gives 0. [as stated]~~
  **ANSWERED 2026-10-14** from the exe. Present 0: the bracket matches; the x87 divide at `0x006D309C` gives +∞, or
  NaN when free = 0. Missing: the original divides by an **uninitialized** stack float (no default is written before
  the read, `0x006D2AE3`). Its result is undefined, so "missing gives 0" is kept as a *[choice]* (neighbors.md §8.2,
  §12).

<!-- from: From the landfill tests and the neighbours core (2026-10-14) -->
- ~~**NB14:** the order of `GetAdjacentCities`. [choice: by region position, x then z]~~ **ANSWERED 2026-10-14** from
  the exe (`0x006D01E0`): the original collects into a `std::set<cISC4RegionalCity*>` with `std::less`, so the order
  is ascending heap address, which cannot be reproduced. The *[choice]* stands. The order mainly affects list and
  message order; the one effect on values is in `ProposeDeals(…, true)` (neighbors.md §9.1).

<!-- from: From the landfill tests and the neighbours core (2026-10-14) -->
- ~~**NB15:** without a current regional city, the UpdateState budget test is skipped. [skipped]~~ **ANSWERED
  2026-10-14** from the exe (`0x006CF360`): for a buy deal the original dereferences the missing regional city and
  **crashes** (`0x006CF386`); sell deals never use it. Skipping is kept as a *[choice]*. The test is `cost > budget`,
  and an affordable buy deal then goes on to the connection test (neighbors.md §7.3).

<!-- from: From the landfill tests and the neighbours core (2026-10-14) -->
- ~~**NB16:** a missing payer at renewal counts as "cannot afford". [yes]~~ **ANSWERED 2026-10-14** from the exe
  (`0x006D13A5`): the original does not null-check the payer, but cannot reach the case. A sell deal with no
  neighbour gets NoConnection earlier in the same pass; a buy deal with no `R` crashes in `UpdateState` first
  (NB15). The bracket is kept as a *[choice]*, needed only because of the NB15 choice (neighbors.md §7.3).

<!-- from: From the landfill tests and the neighbours core (2026-10-14) -->
- ~~**NB17:** the return types of `RequestCreateConnection` and `CreateConnection`. [choice: the request returns
  `ConnectionRequest`, `CreateConnection` the new connection or null]~~ **ANSWERED 2026-10-14** from the exe. The
  internal types are glue: there is no original behaviour, so the implementer's choice stands. At the interface,
  `RequestCreateConnection` returns a real `bool` (confirmed: true; exists or declined: false), and the shim must
  match it. `CreateConnection` leaves a leftover `EAX` that callers ignore (neighbors.md §6.2).
- Lane choices worth knowing: the proposal generator is per simulator, seeded `derive_seed(session, 0xC990C0A4)`
  (the original's is process-wide and clock-seeded); save reads are capped at 65,536 zones.

<!-- from: From applying the pathfinder, zones, landfill and neighbours answers (2026-10-14) -->
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

## From the neighbours glue and shadow (filed by the coordinator from the `neighglue` lane's report, 2026-10-14)

About `docs/specs/neighbors.md` and the neighbours glue (`cFC4NeighborsSimulator`, `FC4NeighboursShadow`). The current
answer is in brackets.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG1:** Property holder semantics for our own objects (connections, deals). [sorted by property id, values copied;
  `GetPropertyList` returns false; data as a Uint8 array]~~ **ANSWERED 2026-10-14** (coordinator, run r50e2): copies are wrong. The
  holder keeps the caller's object (AddRef) and returns it borrowed; callers write its variant in place
  (foundations §10.2, "Ownership"). With copies, the traffic simulator's trip-capacity store used a freed
  property and the game crashed.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG2:** Object-valued properties on our connection objects. [dropped from the core sync]~~
  **ANSWERED 2026-10-14** from the exe (holder slot 11 `0x005FF950`; the connection properties seen in samples and
  code) and written into neighbors §15.4. The original holder keeps an object-valued property (a reference), returns
  it through slots 6 and 9, writes it as a reference on save and drops it on load. No shipped code puts one on a
  connection. The bracket is fine for the core sync and the save, as long as the game-facing holder keeps the object.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG3:** Does the region reach our connection objects non-virtually (by field offset)? [unknown; the risk is noted]~~
  **ANSWERED 2026-10-14** from the exe (image-wide scan for direct calls and pointers to the class's methods; vtable
  constants only in `0x006D4680`/`0x006D4BB0`; field-read scan of the 66 connection users and the 55 readers of
  `0x00B43D8C`; current-city record filled at `0x004AD34E`) and written into neighbors §15.3. **No; there is no such
  site.** Every access is a virtual call through the connection, traffic-source, holder or serializable interface. The
  only original code that touches the concrete class constructs its own objects (`0x006D489B`, `0x004AC994`). The
  current city's regional record holds our objects after `UpdateCityCache` and writes them through
  `QueryInterface(0xE4FDA3D4)` → `Write`. The one layout contract is `GetTripDataList`'s STL list (already met).

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG4:** `AddDeal` with a foreign (original) deal object. [copied into a deal of ours]~~
  **ANSWERED 2026-10-14** from the exe (`AddDeal` `0x006D2870`; no caller through slot 26) and written into neighbors
  §15.4. The original keeps the caller's object (AddRef), clears its Silent flag and simulates that object. The bracket
  differs (pointer identity, later changes lost), but no shipped code calls `AddDeal`, so only mods can see it.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG5:** The lifetime of the deal wrapper objects. [created lazily, parked at zero references]~~
  **ANSWERED 2026-10-14** from the exe (`FindDeals` `0x006D0760`, `AddDeal`, the status-report subscribers) and written
  into neighbors §15.4. An original deal lives until its last Release. The holders are the deal list, the regional
  cities' local lists, queued status reports, `FindDeals`/`ProposeDeals` lists and the utility advisor's advice. Lazy
  creation and parking are invisible, so the bracket stands, provided a removed deal keeps answering while referenced.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG6:** Getters called from other threads. [the lock is tried for 20 ms, then the mirror or a neutral value is
  returned and counted as a fallback]~~
  **ANSWERED 2026-10-14**: no original behaviour (the original simulator has no lock); the implementer's choice stands.
  Noted in neighbors §15.4.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG7:** The shadow's state it cannot read from the original. [ready = 1, uiDisabled = 0]~~
  **ANSWERED 2026-10-14** from the exe (field writes in the simulator class; budget window `0x0077E6C5`,
  `0x0077E735`, `0x0077A216`) and written into neighbors §15.4. The fields are bytes in the original object: ready
  `+0x0C`, initialized `+0x0D`, dealsDirty `+0x3C`, statsDirty `+0x3D`, uiDisabled `+0x3E`, busy `+0x3F`. The bracket
  is wrong before ConnectionsReady (ready 0) and while the budget window's neighbour tab is open (uiDisabled 1). Read
  the bytes.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG8:** The volume-dirty flag. [true once after creation]~~
  **ANSWERED 2026-10-14** from the exe (`0x006D4C10`, flag at connection `+0x40`) and written into neighbors §15.4. Set
  only by the constructor; cleared at the start of the first `UpdateTrafficVolume`, even when no other side is found.
  The bracket matches.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG9:** `SetType` and similar setters on foreign connections. [ignored]~~
  **ANSWERED 2026-10-14** from the exe (connection vtable `0x00AB2034`; the NG3 field scan; writes in `CreateConnection`,
  `Read` and `Repair`) and written into neighbors §15.4. Type and cell never change after creation, so ignoring is
  right. `Repair`, `SetVisitedThisCycle`, `SetIsActive` and holder writes must go through the interface.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG10:** The return of `CreateConnection`. [our pointer, borrowed; null if the connection already exists]~~
  **ANSWERED 2026-10-14** from the exe (`0x006D4860`, NB17) and noted in neighbors §15.4. The original's value is
  garbage and every shipped caller ignores it, so the bracket stands.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG11:** Slots 32/33 (64-bit values). [the low 32 bits]~~
  **ANSWERED 2026-10-14** from the exe (`0x006CF860`, `0x006CF8D0` and their eleven call sites) and written into
  neighbors §15.4. Both return an int64 in EDX:EAX, and every caller (finance advisor, budget simulator, budget
  windows) reads EDX. The bracket is wrong: return the full 64 bits. `docs/abi` rows 32/33 need `edx:eax`.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG12:** The return channel of the regional city getters 50–64. [double (x87)]~~
  **ANSWERED 2026-10-14** from the exe (regional-city vtable `0x00A8FB20`, slots 50–64) and noted in neighbors §15.4.
  `double` on x87; the bracket matches and `docs/abi` is already right.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG13:** The return of deal `Copy`. [void]~~
  **ANSWERED 2026-10-14** from the exe (`Copy` `0x006CE2E0`, its only caller `Clone` `0x006CF3E0`) and noted in
  neighbors §15.4. Nothing meaningful is returned and nothing reads it; the bracket stands.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG14:** `RequestCreateConnection`: the yes/no dialog. [confirmed without the dialog, logged once]~~
  **ANSWERED 2026-10-14** from the exe (`0x006D15E0` → `GenericConfirmationDialog` `0x0078DFF0`) and written into
  neighbors §15.4. The original asks with a modal yes/no dialog and returns the answer; with popups disabled the dialog
  returns false, so the request is refused. The bracket differs; recommend the dialog, and "no" when popups are off.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG15:** `GetCity` with out-of-range cells. [passed on as uint32]~~
  **ANSWERED 2026-10-14** from the exe (`cSC4Region::GetCity` `0x004A4930`) and noted in neighbors §15.4. A hash lookup
  on the 32-bit pair; the bit pattern is what counts, so passing it as uint32 is fine and an out-of-range cell gives
  null.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG16:** Creating the connection marker. [`CreateProp` + `AddCityProp`, then our reference released]~~
  **ANSWERED 2026-10-14** from the exe (`0x006D4A53`–`0x006D4B15`; prop manager slots 15 and 19) and written into
  neighbors §15.4. `CreateProp`, position and orientation, the bool properties, `AddCityProp`, then `Release`. The
  bracket matches; a failed `CreateProp` just skips the marker.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG17:** The occupant cell of a connection. [the network's `GetOccupiedCell`, else the pole's `GetCell`]~~
  **ANSWERED 2026-10-14** from the exe (`0x006CEF62` `GetPosition`, `0x006CEF7E` `PositionToCell`) and written into
  neighbors §6.5 and §15.4. The original converts the occupant's position to a cell for networks and poles alike. The
  bracket agrees whenever the occupant lies in its own cell; recommend the original's rule.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG18:** A deal holder not reachable by QueryInterface. [left open]~~
  **ANSWERED 2026-10-14** from the exe (deal QI `0x006CE260`; holder vtable `0x00AB1D58` only in the constructor and
  destructor) and written into neighbors §15.4. The original deal's holder is not reachable either; it only travels with
  the record. Nothing to expose: round-trip the record's properties.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG19:** No city on the other side of a connection. [targets 0]~~
  **ANSWERED 2026-10-14** from the exe (`0x006D4C3B` lookup, `0x006D5030` list clear) and written into neighbors §15.4
  and traffic-simulator §4.1.4. The original clears the whole trip list. "Targets 0" agrees except for trips of a type
  above 2, which the sync never creates; recommend clearing the list.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG20:** The `GetSourceCells` rectangle. [(x, z, x, z)]~~
  **ANSWERED 2026-10-14** from the exe (`0x006D4820`) and noted in neighbors §15.4. `(x, z, x, z)` with the connection
  cell (outside the city); the bracket matches.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG21:** The return of `UpdateTrafficVolume`. [true]~~
  **ANSWERED 2026-10-14** from the exe (`0x006D4C10`; caller `0x0071E28E`) and written into neighbors §15.4. `AL` is 0,
  3 or undefined, and the only caller on a connection ignores it. "True" is safe; the bracket stands. `docs/abi`: slot
  4 of `cISC4TrafficSource` on connections is not a clean bool.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG22:** `cISCProperty` Write/Read. [through the segment stream]~~
  **ANSWERED 2026-10-14** from the exe (`cSCProperty` `0x005FD870`/`0x005FD8B0`; holder writer `0x005FD990`) and
  written into neighbors §15.4. Write and Read put foundations §10.1 on the given stream and return "not in error". The
  original holder calls each stored property's own `Write`, so ours must write those exact bytes. The bracket matches.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG23:** The size query of `GetPropertyData`. [left open]~~
  **ANSWERED 2026-10-14** from the exe (holder slot 5 `0x005FD290`, variant slot 57 `0x00915714`) and written into
  neighbors §15.4. No size query: only a raw-byte (`0x8000`) value is copied, `length := min(length, count)`, else false.
  A null buffer crashes the original; recommend false with `length` unchanged.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG24:** Regional cities held across calls. [AddRef until the outermost exit]~~
  **ANSWERED 2026-10-14**: no original behaviour (the original holds regional cities only within one call); the
  implementer's choice stands. Noted in neighbors §15.4.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG25:** The marker query type. [`0xA823821E`]~~
  **ANSWERED 2026-10-14** from the exe (prop manager slot 18 `0x004A1DB0`, filter `0x004A2210`; calls at `0x006D189A`,
  `0x006D18D6` and `0x006D0D65`) and written into neighbors §15.4. The original asks for city props (type `0xA823821E` and usage 1) in the
  cell, then tests `0xCAA46595`. The type matches; add the usage test or use `GetCityProps`.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG26:** Traffic source `QueryInterface`. [forwarded to the connection]~~
  **ANSWERED 2026-10-14** from the exe (thunk `0x006D47F0` → `0x006CE000`) and written into neighbors §15.4. Forwarded
  to the connection, which answers only `1`, `0x0773FEF2` and `0xE4FDA3D4`. The bracket matches.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG27:** `GetPropertyU32` on array properties. [the first element]~~
  **ANSWERED 2026-10-14** from the exe (holder slot 8 `0x005FD230`, variant `0x009176C8`, conversion `0x00917350`
  via text `0x00916EAA`) and written into neighbors §15.4. Non-scalars convert through text, and an array's text is its
  first element ("0" if empty). The bracket matches for unsigned integer arrays; other element types were not traced.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG28:** The shadow's statistics tolerance. [relative 1e-6]~~
  **ANSWERED 2026-10-14**: no original behaviour; the implementer's choice stands. Noted in neighbors §15.4.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG29:** Network flag bits used as type numbers. [left open]~~
  **ANSWERED 2026-10-14** from the exe (network occupant slot 22 `0x00619520`, table `0x00AA7CE8`, slot 8 `0x00604D10`)
  and written into neighbors §15.4. §6.5 passes type numbers; the occupant maps each to `1 << type` and tests it
  against its flags.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG30:** `ReloadCity` through the SDK `cISC4Region`, which is not in `docs/abi`. [gated, with an identity check]~~
  **ANSWERED 2026-10-14** from the exe (region vtable `0x00A8FA40`; `Init` `0x006D3B0C`–`0x006D3BEC`; `ReloadCity`
  `0x004A5030`) and written into neighbors §15.4. The SDK interface matches slot for slot. The original reloads every
  adjacent city with no gate and no identity check (the check always passes, so it is harmless; a closed gate is not).
  `ReloadCity`'s result is garbage, and after a failed read the city is gone and the holder pointer dangles. `docs/abi`
  rows for `cISC4Region` are in the re-answers8 report.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG31:** Status reports from deals with no host object. [left open]~~
  **ANSWERED 2026-10-14** from the exe (subscribers of `0xEA418D8F`: power, plumbing, utility advisor; status-8 posters)
  and written into neighbors §15.4. Statuses 0–7 always carry the deal; power and plumbing read it on 0, the advisor on
  3–6. Recommend creating the wrapper when posting; only status 8 is posted without a deal.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG32:** The shadow's connection-state comparison. [the original's after-state against our before-state]~~
  **ANSWERED 2026-10-14**: no original behaviour; the implementer's choice stands. Noted in neighbors §15.4, with the
  callers that change connection state outside the simulator.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG33:** A message from another thread that cannot get the lock. [dropped, returns true]~~
  **ANSWERED 2026-10-14**: no original behaviour (no lock; every message is handled and `DoMessage` returns true); the
  implementer's choice stands. Noted in neighbors §15.4.

<!-- from: From the neighbours glue and shadow (2026-10-14) -->
- ~~**NG34:** A null connection reference in a save. [skipped]~~
  **ANSWERED 2026-10-14** from the exe (`0x006CFEC5`, NB7) and noted in neighbors §15.4. The original crashes; skipping
  is the recorded choice of §13, and the bracket matches it.

<!-- from: From applying the residential, protection and save-reference answers (2026-10-14) -->
- ~~**NB19:** NB18: does `cISC4App::GetCity` return the same primary cISC4City address the game's city global holds?
  [assumed yes]~~
  **ANSWERED 2026-10-14** from the exe (`cISC4App` slot 37 `+0x94` `0x0081F910`; `cSC4App::NewCity` `0x00454908`;
  `PartialCloseCity` `0x00454AEC`; the global is cleared at `0x00602100` from the city destructor `0x0045A468`) and
  written into neighbors.md §8.2. **Yes.** `GetCity` returns the application's city member, borrowed. NewCity
  stores in it the address the constructor gave to the global `0x00B43CEC`: the primary `cISC4City`. The two can
  differ only after the city's `Shutdown`. **The bracket matches.** The ABI row is in the re-answers9 report.

## From applying the neighbours answers (filed by the coordinator from the `apply9a` lane's report, 2026-10-14)

apply9a applied NG4–NG27 (merged `bf0e4b1`, ABI 45). The main in-game change: with our neighbours simulator active
the original budget now reads slots 32/33 as int64 in EDX:EAX (NG11). Before, it read garbage
(`income orig/ours=844424930136029/4061`); now 25/25 budget lines are exact. Konradshohe has no deals, connection
requests or marker changes, so NG4, NG14, NG16, NG17, NG19, NG23 and NG25 are tested only outside the game.

<!-- from: From applying the neighbours answers (2026-10-14) -->
- ~~**NG35:** NG14: there is no interface for `GenericConfirmationDialog`. What should happen when popups are on?
  [confirm without asking; refuse when popups are off]~~ **ANSWERED 2026-10-15 (re-answers10):** **DIFFERS**. `GenericConfirmationDialog` (`0x0078DFF0`) is a free cdecl function `(title, text, yesLabel, noLabel)` (labels null for Yes/No), true only when closed with Yes (`0x8A8DFD4E`). It returns false at once when popups are off, otherwise it pauses and asks the player. With popups on, the original asks; confirming without asking lets the player never decline. Recommended: call it at its build-gated address and return its answer; refusing when popups are off already matches. neighbors §15.4 NG35.

<!-- from: From applying the neighbours answers (2026-10-14) -->
- ~~**NG36:** NG14: with no app reachable, is the popup flag taken as enabled? [enabled]~~ **ANSWERED 2026-10-15 (re-answers10):** **DIFFERS**. `SC4GetPopupModalDialogsEnabled` (`0x005F9090`) returns false when the app global `0x00B43C94` is null, so the original behaves as popups off (request refused). It cannot happen in the game; only tests without an app see it. Treat a missing app as off. neighbors §15.4 NG36.

<!-- from: From applying the neighbours answers (2026-10-14) -->
- ~~**NG37:** NG23: what does the original's `AddPropertyData` (holder slot 10) store? [ours stores a Uint8 array, so
  `GetPropertyData` does not read it back]~~ **ANSWERED 2026-10-15 (re-answers10):** **DIFFERS**. Holder slot 10 (`0x005FFA40`) stores raw bytes of memory type `0x8000`, with count = length, through variant slot 58 (`0x00915743`). It reuses the buffer when the type and count already match; otherwise it allocates with allocator C. It skips the copy for null data. `GetPropertyData` reads exactly that type back. A `Uint8` array is refused by the getter and is saved with a different tag. Store type `0x8000`. neighbors §15.4 NG37.

<!-- from: From applying the neighbours answers (2026-10-14) -->
- ~~**NG38:** NG17: should a failing `GetPosition` be checked? [unchecked: a zero vector, so cell (0, 0)]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES. Occupant slot 8 cannot fail in the original: every implementation writes the vector, the base class (`0x005E8A20`) writes (0, 0, 0), and the caller ignores `AL` (`0x006CEF62`). A zero vector, and so cell (0, 0), is the original's result for an occupant with no position of its own. neighbors §15.4 NG38.

<!-- from: From applying the neighbours answers (2026-10-14) -->
- ~~**NG39:** NG25: what is the rectangle form of `GetCityProps` (prop manager slot 18)? [not called; the usage test is
  added to the cell query]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES in practice. Prop manager slot 18 (`0x004A1DB0`): false without an occupant manager. Otherwise it sets the filter's usage to 1 and calls the occupant manager's cell-rectangle query (slot 40, `0x0049E250`) with the two inclusive corners. That query clamps the corners to the city, converts them to world coordinates (16 m cells, the far corner +1), shrinks the box by 0.1 m per side, opens the height range and runs the world-box query (slot 36). Ours differs only for a prop reaching less than 0.1 m into the cell or for a cell outside the city. neighbors §15.4 NG39.

<!-- from: From applying the neighbours answers (2026-10-14) -->
- ~~**NG40:** NG7: what if the original's state bytes cannot be read (an unidentified vtable)? [ready = 1, everything
  else 0, logged once]~~ **ANSWERED 2026-10-15 (re-answers10):** glue choice, accepted. The original's bytes are always readable, and an unidentified vtable means a mod replaced the simulator. Its usual values between Init and Shutdown are ready 1 and initialized 1, with uiDisabled and busy 0 outside the budget tab and `simulate_deals`. The bracket's initialized 0 is unusual but unread (NG41). neighbors §15.4 NG40.

<!-- from: From applying the neighbours answers (2026-10-14) -->
- ~~**NG41:** NG7: the `initialized` byte has no shadow input. Should it be compared? [read and logged only]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES. Only `Init` (`0x006D38FA`) and `Shutdown` (`0x006CFBC4`) read the byte, each returning true at once when already done; nothing else looks at it. Reading and logging it is enough. neighbors §15.4 NG41.

<!-- from: From applying the neighbours answers (2026-10-14) -->
- ~~**NG42:** NG4: is the foreign deal's reference released exactly when it leaves the list, or when nothing in the
  core still holds it? [when the core object goes, which is list removal unless something else in the core holds it]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES. The original's list releases its reference exactly when the deal is erased (`0x006D1588` → `0x004216F9`). Every other holder (regional cities, queued reports, caller lists) has its own reference on the same object. The bracket is equivalent as long as each core holder that outlives removal stands for one of those. neighbors §15.4 NG42.

<!-- from: From applying the neighbours answers (2026-10-14) -->
- ~~**NG43:** NG4: what should `AddDeal` return when the core cannot list the deal? [true, and the reference is given
  back]~~ **ANSWERED 2026-10-15 (re-answers10):** partly **DIFFERS**. `AddDeal` (`0x006D2870`) cannot fail and returns true. It clears Silent, appends without a duplicate check, pushes deals and statistics to the region regardless of the dirty flags, then clears both flags. A null deal crashes. "True" matches; not listing the deal differs (the original lists it and lets the simulation judge it). It is accepted as a logged, mod-only deviation. neighbors §15.4 NG43.

<!-- from: From applying the neighbours answers (2026-10-14) -->
- ~~**NG44:** NG27: how do signed, 64-bit and float values convert, and what about an empty character array? [as C
  converts them; the empty character array stays false]~~ **ANSWERED 2026-10-15 (re-answers10):** partly **DIFFERS**. Scalars convert numerically (`0x00915AA5`): signed values sign-extend, 64-bit values keep their low 32 bits, and floats truncate toward zero through `_ftol` (NaN, ±∞ and values of 2^63 or more give 0). Arrays go through their first element's text: signed and 64-bit elements give the same bits. A float element of 2^63 or more wraps (Float32 {1e30} gives `0x19058000`), and a NaN element parses as ±1. An empty character array gives the empty text, which parses as 0, so the getter returns **true** with 0, not false. "As C" matches only for in-range values. neighbors §15.4 NG44; data-formats §5.1.

## From applying the re-answers10/11 DIFFERS entries (filed by the coordinator from the `apply12` lane's report, 2026-10-16; merged `463cca7`)

apply12 applied ZG18, ZG21, CM5, NG35-NG37, NG43, NG44 and DG35 (ABI 59, taken as 66 on the lane; FC4TestSeam op
66 installs a fake confirmation dialog). DG28 is not applied: the audited release in the growth developer's
destructor (`FC4_RELEASE_AT_DESTRUCTION`) stays. The lane's choices are in brackets.

<!-- from: From applying the re-answers10/11 DIFFERS entries (2026-10-16) -->
- ~~**NG45:** (neighbors.md §6.2, NG35) RequestCreateConnection's title and text are LTEXT keys, but the spec does not
  say which loader reads them or which group the two instance-only keys (`0x0A8E152D` text, `0x0A8E15F9` fallback
  name) belong to. [SC4LoadString's rule (group + language ID, then the group alone, U+2022 becomes U+200B), with
  both keys in the title's group `0x6A231EAA`; the sprintf template takes the name at the first `%s` and `%%` gives
  `%`; a missing LTEXT gives empty text (choice)]~~ **ANSWERED 2026-10-16 (re-answers18):** **MATCHES**.
  - All three keys are constructed with group `0x6A231EAA` (`0x006D1612`, `0x006D1627`, `0x006D163C`).
  - The loader is the localized-string object `cSLString` (`0x00603580`). Its `Load` (`0x00603300`) uses
    `SC4LoadString`'s lookup and U+2022 → U+200B. A missing LTEXT gives an empty string (`0x006034E4`), so the
    choice is the original's behaviour.
  - One detail differs from `SC4LoadString`: a failed detokenize keeps the text.
  - `cRZString::Sprintf` (`0x0090F574`) uses the C `printf` conversions (first `%s`, `%%`).

  neighbors §6.2.
