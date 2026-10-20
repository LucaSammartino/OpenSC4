# building-development: answered questions (archive)
<!-- Archive for [building-development.md](../../building-development.md). Lanes do not read this file. Open questions are in ../building-development.md. -->

## From the development glue and shadow (filed by the coordinator from the `devglue` lane's report, 2026-10-14)

The building development simulator and the growth developers are replaced as one unit (`enable = development`;
building-development.md §9, growth-developer.md §12), with a read-only `development_shadow`. Each question is marked
in the code; the implemented answer is in brackets.

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG1:** The return channel of `AddToDemandValue`. [bool in AL; no x87]~~
  **ANSWERED 2026-10-14** from the exe (`cSC4Demand::AddToDemandValue` `0x005D4C70`: load, add, store-and-pop, AL = 1,
  `ret 4`) and written into growth-developer §5.3, §11 and §13.1 (quirk 11 withdrawn). It adds the amount to the
  node's float32 demand value and returns true in AL; the x87 stack is left as found. **The bracket matches**; the spec's
  old "ST0, never popped" was wrong (docs/abi already says `eax-or-void`). Bears on the run 102 record: the same
  section now records that the original's constructor never initialises total capacity (`+0x108`), so the saved
  `capacity=1036996237` (`0x3DCF4E8D`, a float bit pattern) is the original's own garbage, not a misread; zone, purpose,
  wealth and source are tuning set by `Init` after `Read`, and five grid references of six is the record's shape
  (growth-developer §2, §9). *[choice]* total capacity 0 for a new city.

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG2:** The timer that slices state 18. [a real timer, QueryPerformanceCounter]~~
  **ANSWERED 2026-10-14** from the exe (adaptive helper `0x0068CC20`/`0x0068BF50`, growth developer step `0x00B49098`)
  and written into growth-developer §5.2. The original reads the processor's time-stamp counter and converts the
  budget with the simulator's `MillisecsToCycles` (simulator §5.5). **The bracket matches in effect**: any monotonic
  real-time clock gives the same behaviour, since slicing changes only how much of state 18 runs per call.

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG3:** The layout behind `GetLotConfigurationMap`. [lot configuration manager slot 9 `GetLotConfigurationsByFilter`
  with an accept-all filter, into a B hash_set]~~
  **ANSWERED 2026-10-14** from the exe (slot 20 `0x00489720`, map `0x00B216C8`, begin `0x004B0390`; slot 9
  `0x0048B1A0`, null-filter path `0x0048B2E7`) and written into building-development §4.7.4 (Inputs). Slot 20 returns
  the manager's own STLport `hash_map` (allocators.md hash_set shape, 12-byte nodes `{next, key, config*}`), with no
  reference. **The bracket matches**: slot 9 with a null (or accept-all) filter gives the same configurations, each
  AddRef'd into a B `hash_set`; the caller must release each one. Order does not matter (the table keeps maxima).

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG4:** The budget item cost at +0x38. [0]~~
  **ANSWERED 2026-10-14** from the exe (`LoadBuildingProperties` `0x0069BE00`–`0x0069BE3F`; profile readers after
  BDS slots 39/40 and occupant slot 11 scanned) and written into building-development §4.7.1. The original stores the
  sum of the `cost` fields of the exemplar's budget items (budget simulator slot 50), e.g. 120 + 80 = 200. **The
  bracket differs**, but no reader of `+0x38` was found in the exe *[inferred]*, so 0 is not observable by the game;
  *[choice]*: fill the sum when the budget simulator is reachable, else 0.

<!-- from: From the development glue and shadow (2026-10-14) -->
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

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG6:** BDS slots 23, 26 and 29. [false, logged once]~~
  **ANSWERED 2026-10-14** from the exe (slot 23 `0x00695AF0`, slot 26 `0x006982B0`, slot 29 `0x006983C0`, cell helper
  `0x00695610`; callers: the lot developer's foundation code and the wall factory `0x005E3122`) and written into
  building-development §4.9.2. **The bracket differs**: the original implements all three, and the original lot
  developer and wall code depend on them; false gives no foundations and no retaining walls. Slot 26/29 set the
  default exemplar from the foundation (`0x11`) / wall (`0x15`) key maps (26 also reads maximum height `0x88FCC3E3`);
  slot 23 builds the foundation occupant (size from the lot developer's highest/lowest cell altitudes, height ≥ 0.5)
  and inserts it. *[choice]* implement them as specified, or forward to an original BDS; never return false for valid
  input.

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG7:** The static cleanup through app slot 69. [not registered, logged]~~
  **ANSWERED 2026-10-14** from the exe (callback `0x0069B850`, registration in `Init`, app slots 69/70) and written
  into building-development §1.3 step 1. The callback runs only at application shutdown: it clears the static flag,
  unregisters itself and empties the nine catalogue containers. No in-session behaviour depends on it, so **the
  bracket stands** (not registering is unobservable while the process lives).

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG8:** "Declined by stage" in the month line. [counted by wealth only]~~
  **ANSWERED 2026-10-14**: glue-only (a log line). No original behaviour; the implementer's choice stands. (The
  original counts nothing by stage; `ExecuteDeclinePlan`, building-development §4.5, knows each item's developer and
  so its wealth.)

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG9:** When the decline plan line is written. [at BDS state 4 → 0]~~
  **ANSWERED 2026-10-14**: glue-only (a log line). No original behaviour; the implementer's choice stands. State 4 → 0
  is where the original finishes executing a plan (building-development §4.2), so the choice is consistent.

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG10:** The `cSC4LotConfigurationObject` layout. [the SDK header]~~
  **ANSWERED 2026-10-14** from the exe (`0x00477B10` stride `0x34`, type byte at `+0`; reads at `0x0069C43E`,
  `0x0069C6D0`–`0x0069C71E`) and written into building-development §4.7.4 (Inputs). **The bracket matches**: the
  SDK's `cSC4LotConfigurationObject` is the exact `0x34`-byte layout (rotation `+0x02`, x `+0x04`, z `+0x0C`, ID
  vector `+0x28`). Slot 24 returns the first object of type 0 (building), or null.

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG11:** An empty capacity vector against a present-but-empty one. [`n_capacities > 0` gives one map]~~
  **ANSWERED 2026-10-14** from the exe (`0x0069BBB7` property test, pair loop `0x0069BC10`–`0x0069BC44`, push
  `0x0069BC46`–`0x0069BC7E`) and written into building-development §4.7.1. **The bracket differs**: a present
  `0x27812834` always pushes one map, even with no values (an empty map); only a missing property gives no map. The
  difference reaches the lot's copied vector (`cSC4Lot::SetState`) but no capacity sum. *[choice]*: follow the
  original (present → one map); ignore an unpaired last value.

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG12:** A zero profile inserted in advance. [replaced in place]~~
  **ANSWERED 2026-10-14** from the exe (`AddBuildingData` passes true at `0x0069D0F2`; erase only for false,
  `0x0069BF29`) and already in docs/abi/layouts.md (Building profile). **The bracket matches**: a zero profile
  inserted earlier by `GetBuildingProfile(unknown)` is assigned over in place, so pointers to it stay valid.

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG13:** The cheat notification ID. [the cheat ID]~~
  **ANSWERED 2026-10-14** from the exe (`Init` `0x0069D5E2`–`0x0069D649` and the third pair) and written into
  building-development §1.3 step 10. **The bracket matches**: each cheat is registered (slot 3) and subscribed with
  `AddNotification2(target, id)` using its own ID; the delivered message is `0x230E27AC` with data1 = that ID.

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG14:** The name fallback. [GameString, otherwise cRZBaseString]~~
  **ANSWERED 2026-10-14** from the exe (`LoadBuildingProperties` `0x0069BA55`–`0x0069BAD1`; `SC4LoadExemplarString`
  `0x00603220`) and written into building-development §4.7.1. Order: localized LTEXT via `0x8A416A99`, else property
  `0x899AFBAD`, else "". The original stores both names as `cRZString` values in its name maps and returns borrowed
  `cIGZString*`; the string class is not observable through the interface, so **the bracket stands** as long as the
  content follows that order and the object outlives the call.

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG15:** The lot slope read. [slot 19]~~
  **ANSWERED 2026-10-14** from the exe (`0x0069879C` lot `+0x4C`, `0x006987AA` lot developer `+0x78`) and written
  into building-development §4.11. `lot.GetSlope()` is lot slot 19 (ST0); `GetLotSlope(lot, false)` is lot developer
  slot 30 (pop 8, ST0). **The bracket matches.**

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG16:** The advice subject. [not created]~~
  **ANSWERED 2026-10-14** from the exe (`DoBuildingChanged` `0x0069CA80`) and written into building-development §4.10.
  The original creates and fills a subject, then pushes the event with a null subject and drops it. Not creating it is
  unobservable, so **the bracket stands**, provided `highestStage` is updated and the event is still pushed with null.

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG17:** The building type filter's slot 3. [bool(uint32)]~~
  **ANSWERED 2026-10-14** from the exe (`0x00699C8F`–`0x00699C97`: one push, AL tested) and written into
  building-development §4.7.2. **The bracket matches**: `bool(uint32 type)`, pop 4, AL. The exe has no filter class and
  no caller of BDS slot 17 *[inferred]*, so only plug-ins use it.

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG18:** The `GrowthItem` stage width. [masked to uint8]~~
  **ANSWERED 2026-10-14** from the exe (growth developer writes the whole `int32` at `0x006B6FA8`; `Grow` reads the
  low byte at `0x0069B349`) and written into building-development §4.4. **The bracket matches**: the tract developer
  receives the low byte; shipped stages are 1..8.

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG19:** A zone type out of bounds. [0]~~
  **ANSWERED 2026-10-14** from the exe (`GetMaxCapacity` `0x00696C20`: map lookup, 0 when absent; zone loop bound
  `0x0069C975`) and written into building-development §4.7.4. **The bracket matches**: a zone outside 0..15 is never a
  key of the table, so the lookup misses and the result is 0.0; `GrowthStageFitsInZone` / `BuildingFitsInZone` give
  false for unknown zones (§4.7.3). No original path indexes an array by zone.

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG20:** Lot configuration zones: 0..15, where `fc4.h` allows 0..16. [0..15]~~
  **ANSWERED 2026-10-14** from the exe (`0x0069C974`–`0x0069C978`: `z < 16`) and written into building-development
  §4.7.4. **The bracket matches**: the BDS asks `IsCompatibleWithZoneType` for zones 0..15 only. A wider range in
  `fc4.h` is harmless as long as the table loop stops at 15.

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG21:** An off-thread message arriving without the lock. [dropped]~~
  **ANSWERED 2026-10-14**: glue-only. No original behaviour (the original's getters take no lock); the implementer's
  choice stands.

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG22:** ABI numbering after a merge. [renumbered by the coordinator: ABI 35]~~
  **ANSWERED 2026-10-14**: coordinator bookkeeping, not a behaviour question. No original behaviour; the coordinator's
  numbering stands.

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG23:** The `wall_clock_seed` for the `0xFFFFFFFF` key seed. [the development session seed]~~
  **ANSWERED 2026-10-14** from foundations §6.1 (`Seed` `0x00913667`) and growth-developer §7. The original derives
  such a seed from `time()` and `clock()` (non-deterministic, probability 2⁻³² per tract and update). No original
  behaviour can be matched; **the implementer's choice stands** (a session-derived seed keeps runs repeatable).

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG24:** A null property holder. [false]~~
  **ANSWERED 2026-10-14** from the exe (`Init` `0x006B85EF`–`0x006B85F8`: AddRef on the holder without a null test) and
  already in growth-developer §1.2 step 3. The original crashes on a null holder; **the bracket matches the spec's
  *[choice]*** (false).

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG25:** The shadow's `building_purpose`. [slot 40, no insert]~~
  **ANSWERED 2026-10-14** from building-development §4.7.2 (slots 39/40). The original's own readers use slot 39, which
  inserts a zero profile for an unknown type; slot 40 returns null instead, and a null read as purpose 0 equals the
  zero profile's purpose. **The bracket stands**: same value, and the read-only shadow does not grow the catalogue.

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG26:** Growth developer slots 10 and 28. [the full EAX, 0 or 1]~~
  **ANSWERED 2026-10-14** from the exe (`0x006B7530`, `0x006B75D0`: EAX zeroed, then set from the compare) and already in
  growth-developer §10.1 ("EAX 0/1"). **The bracket matches**: slot 10 = `state < 17` (signed), slot 28 = `state = 0`,
  both as the full EAX.

<!-- from: From the development glue and shadow (2026-10-14) -->
- ~~**DG27:** The off-thread fallback for slot 30. [−4 / 20]~~
  **ANSWERED 2026-10-14**: glue-only. No original behaviour; the implementer's choice stands. −4 / 20 are the values the
  original's slot 30 gives for an index outside its vectors (growth-developer §10.1), so the fallback is consistent.

## From the development glue fixes (filed by the coordinator from the `devglue2` lane's report, 2026-10-14)

devglue2 made the development replacement work in game: the catalogue loads 744/744 types and 281 families (a
building's own property list holds only its own properties; most building data sits in parent cohorts), the
shadow is exact (305/305 lines), and the replacement grows like the original at equal pacing without going
bankrupt. QI `0x0AC2B5F7` on new building occupants works (154 created, 0 failures), unlike the landfill tile (LG27).

<!-- from: From the development glue fixes (2026-10-14) -->
- ~~**DG28:** the growth developer's special-building copy is released last, in its destructor; in an unsaved city
  that release is the last reference (the five warnings on class `0xA9BD882D` in run 102). Does the original also
  free those occupants there, after the load's object table has dropped its references? [yes, assumed; the release
  is marked expected (`FC4_RELEASE_AT_DESTRUCTION`), and still warns if the reference was never taken] *Lead from re-answers9 (DG5):* `CreateInstance(0xA9BD882D)` already returns a count of 1, so an extra AddRef after it leaves one reference too many per building; the five warnings may come from that, not from destructor timing. *Tested by apply10a (2026-10-15): the lead is disproved.* Our BDS takes no extra AddRef after `CreateInstance(0xA9BD882D)`, yet with `FC4_RELEASE_AT_DESTRUCTION` removed run `a10a-after-dev` gave the five warnings on class `0xA9BD882D` again, all from the growth developer's destructor releasing the special-building copy (§1.3) at shutdown. Still open: does the original's growth developer destructor drop those occupants' last references in an unsaved city? [assumed yes; the workaround stays, with a comment citing that run]~~ **ANSWERED 2026-10-15 (re-answers10):** **DIFFERS**: the city's `Shutdown` (`0x0045B1D0`) shuts down and releases the BDS (`+0x164`) before the occupant manager (`+0x12C`). The BDS releases each developer (destroying it, so its copy is released) before its own special list. Both happen while the occupant manager still holds every building in the city, so the copy is the last reference only for a special building that left the city since the last state-14 copy. Our last release means our developer outlives the original's order, or a reference is missing. An extra `AddRef` (the DG5 lead) would leak instead, which agrees with apply10a's run: the warnings point to the release order or a missing reference. building-development §1.4.

<!-- from: From the development glue fixes (2026-10-14) -->
- ~~**DG29:** data-formats §4.4 says `EnumProperties` gives the effective property list, but in game it lists only a
  holder's own properties. Is there an effective enumerator, and what does the original's lookup do with cohorts?
  [the glue walks the parent cohort chain itself (`EffectiveProperties`), with cycle detection]~~ **Answered (re-answers9, data-formats §4.4):** `EnumProperties` (slots 19/20) walks only the own list; every lookup binary-searches a sorted vector that already holds the inherited properties; the effective enumerator is slot 4 `GetPropertyList` (ascending ID order, one reference). The glue's own chain walk is acceptable when own properties win.

<!-- from: From the development glue fixes (2026-10-14) -->
- ~~**DG30:** the suitability comparison tests each building type at a cell whose zone fits it, rotating through lots
  in state 1, lots in other states and empty cells. Tooling only. [accepted as tooling]~~ **ANSWERED 2026-10-15 (re-answers11):** Not an RE question (tooling only); accepted as tooling.

<!-- from: From the development glue fixes (2026-10-14) -->
- ~~**DG31:** with the original simulators, a city run at about 85 game days per real second (`sim_turbo`) ends with
  lower income (3540-3647 against 4202-4593 at 8-20 days/s) and negative funds. Which original work is bound to
  frames or real time rather than game days? [unknown; compare runs only at equal pacing. Separately, our
  development unit costs about 3× per game day: 23-29 days/s under turbo against the original's 80-90]~~ **Answered (re-answers9, simulator §5.7):** four kinds of original work are paced by frames or real time: idle cycles with a per-day bound (growth developers, aura, plumbing); idle cycles restarted only when finished (demand census, land value, crime, power, the BDS monthly rebuild: whole months are skipped when they fall behind); real-time budgets inside day work (the BDS 3 ms cut); and posted messages delivered frame by frame. Original runs compare only at equal pacing up to about 20 days/s (cheetah is 50 ms per day); 85 days/s measures the pacing, not the simulation. **Per-day amounts, 2026-10-15 (re-answers10), simulator §5.7.1:** each round of `OnTick` opens at most one day and makes one idle pass, so under a clock that opens a day every round each idle agent gets K ≤ 1 idle delivery per day plus its day delivery. Calls per cycle: growth developer 20 + E (E = special-building effects), crime 6, power 5, land value 21, aura 17 + N (landmark departments), plumbing 2W + 7 (W = networks) plus the time-sliced cell states, demand census 2 plus time-sliced work over every cell. Only the census (time-bound: it skips a month when its CPU time exceeds about D × (5 + K·m) ms) and crime (if it gets fewer than 6 deliveries a month) can lose months. Land value and power defer; aura, plumbing, the growth developers and so the BDS hold days open instead (the BDS's developers end by about day 21).

## From applying the development answers (filed by the coordinator from the `apply10a` lane's report, 2026-10-15)

apply10a applied DG4, DG5, DG6 and DG11 (merged `b398049`, ABI 49): the building loader, the BDS foundation and
retaining wall slots 23/26/29, the capacities-present flag and the budget item cost sum. In game the replacement is
clean (476/476 foundations, 35/35 wall loads, `ref_warnings=0`) and the development shadow exact (927/927). DG28 is
updated above (the AddRef lead is disproved). The lane's choices are in brackets.

<!-- from: From applying the development answers (2026-10-15) -->
- ~~**DG32:** DG5 loader: property `0x27812810` with fewer than three values, or values not stored as float32 (the
  original reads three floats through the raw value pointer). [the missing values are 0; other numeric types are
  converted]~~ **ANSWERED 2026-10-15 (re-answers11):** **DIFFERS** (unreachable with shipped data; the bracket is accepted as a *[choice]*). The loader passes the variant's value field (slot 111, `0x0060CE80`, returns `+0x04` whatever the type) to `SetBoundingBox` as the address of three floats (`0x00698582`–`0x00698592`): a Float32 array of 1 or 2 values is read past its heap block (garbage, not 0), a scalar's value bits or an Empty value's null are used as an address (crash or garbage), and integer arrays give their bits as floats (no conversion). All 743 shipped building exemplars hold a Float32 array of 3, own or inherited. building-development §4.9.1 step 6.

<!-- from: From applying the development answers (2026-10-15) -->
- ~~**DG33:** BDS slot 26: `0x88FCC3E3` is present but has no values. [max height is set to 0]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES. The read is the converting Float32 getter (`0x005FD4F0` → variant slot 37 `0x00917852`); an array of count 0 (or an Empty value) takes the text round trip, gives the text `0`, so 0.0, and the getter returns true, so `SetMaxHeight(0.0)` is called (`0x00698397`–`0x006983A4`). A missing property leaves the constructor's value. Shipped: 46 of 50 foundation exemplars have no `0x88FCC3E3`, 4 have one Float32. building-development §4.9.2.

<!-- from: From applying the development answers (2026-10-15) -->
- ~~**DG34:** BDS slots 26 and 29 with a null occupant (the original does not test for it). [return false]~~ **ANSWERED 2026-10-15 (re-answers11):** the original **crashes**: both slots call `occ.AsOccupant()` through the occupant first, with no test (`0x006982C7`, `0x006983D0`). Returning false is accepted as a *[choice]*. building-development §4.9.2.

<!-- from: From applying the development answers (2026-10-15) -->
- ~~**DG35:** BDS slot 29: its fallback for type 0 needs `GetRetainingWallType` (wall slot 11), but docs/abi gives no
  IID for `cISC4LotRetainingWallOccupant`, so the wall cannot be reached from an occupant token. What is the IID?
  [the plugin keeps the wall the slot 29 call received and answers only for that wall; any other token gives 0]~~ **ANSWERED 2026-10-15 (re-answers11):** **IID `0x295D1162`**. The wall occupant's `QueryInterface` (`0x005E28C0`) answers it with the primary interface (`+0x00`, vtable `0x00AA3080`) and passes every other IID to the `cSC4Occupant` base at `+0x04` (whose slot 0 `0x005E29A0` adjusts back), so the QI works from an occupant token; the game itself reaches walls from occupants this way (`0x005E3766`, `0x005E385A`). Slot 11 (`+0x2C`, `0x0080BC00`, pop 0, EAX) returns the wall type at `+0x38`. The bracket (answer only for the wall the call received) **DIFFERS** for slot 29's type-0 fallback on any other wall; with the IID the plugin can QI the token. building-development §4.9.2; ABI row proposed in the lane report.

<!-- from: From applying the development answers (2026-10-15) -->
- ~~**DG36:** foundation creation: the spec only implies that GZCOM class `0x49C05C8F`, like `0xA9BD882D`, returns with
  the caller's reference already counted. [it is not AddRef'd again; in game `ref_warnings=0` over 476 foundations]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES. Class `0x49C05C8F` is the foundation occupant (`GetGZCLSID` `0x005D6130`, registered at `0x004607F7`); its factory `0x005D64C0` uses slot 23's pool and constructor (count 0) and returns the `+0x04` sub-object without AddRef, and the director's `GetClassObject` (`0x0090DFAC`) then QIs it (`0x0090DFFC`–`0x0090E00B`), which AddRefs: the caller receives count 1, as for `0xA9BD882D`. building-development §4.9.2.

<!-- from: From applying the development answers (2026-10-15) -->
- ~~**DG37:** `SetDefaultExemplar` on an occupant with no `cISCExemplarPropertyHolder` (QI `0x0AC2B5F7`), which the
  original does not test for. [false, which the building loader treats as success with an empty box]~~ **ANSWERED 2026-10-15 (re-answers11):** **DIFFERS** (unreachable with the game's classes; the bracket is accepted as a *[choice]*). The holder's `SetDefaultExemplar` is called without a null test in all three loaders (building `0x0069853A`–`0x00698545`, foundation `0x0069836A`–`0x00698375`, wall `0x00698452`–`0x0069845D`), so such an occupant crashes the original. Buildings, foundations and walls all answer `0x0AC2B5F7` through the property-holder base. building-development §4.9.2.

<!-- from: From applying the development answers (2026-10-15) -->
- ~~**DG38:** DG4: `GetBudgetItemInfo` needs the building exemplar as a property holder. Does the original pass the
  resource exemplar or the occupant? [the type's exemplar is loaded from the catalogue key and passed; 0 when it does
  not load or the budget simulator is not reachable]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES for the holder: the original passes the building **exemplar's** property holder, loaded once per type at catalogue build (`AddBuildingData` `0x0069D0A0`: `GetResource` then exemplar slot 7, `0x0069D0CF`–`0x0069D0FF`), the same holder every other profile field is read from; never an occupant. The budget simulator (`0x00B43D64`) is called with no null test (`0x0069BDCF`–`0x0069BE00`), so "0 when the budget simulator is not reachable" is a *[choice]* (the original crashes). building-development §4.7.1 table.

<!-- from: From applying the re-answers10/11 DIFFERS entries (2026-10-16) -->
- ~~**DG39:** (building-development §1.4, DG28) after loading a saved city (Konradshohe, run `apply12-dev-1`), our
  growth developers are destroyed only after the simulator's Shutdown, not inside the BDS Shutdown, although the
  core shuts each one down and releases it there. Does the original's load-time object table (or anything else)
  hold a reference to a growth developer it resolved for the BDS record, and when is that reference released?
  [the destructor's audited release stays until this is answered]~~ **ANSWERED 2026-10-16 (re-answers18):**
  **MATCHES**: yes, and the observed late destruction is the original's.
  - The BDS `Read` reads each developer by reference (`0x0069AD53`). The class store creates it (factory
    `0x006B7870`), and the object table of the city segment's COM serializer keeps one reference (resolver
    `0x005D2DD0`) on top of the map's.
  - That reference goes only when the loaded segment closes. On city close, `PartialCloseCity` (`0x00454A60`)
    unregisters the segment at `0x00454AF3`. That is after the city's `Shutdown` (`0x00454AC9`), in which the
    simulator (`+0x158`, `0x0045B912`) comes after the BDS and the occupant manager. It is also after the
    `0x26D31EC3` post and the city's release (`0x00454AE7`).
  - So in a loaded, unsaved city the original's developers die at that segment close. After a save, the table is
    empty (`0x00456449`) and the BDS `Shutdown`'s `Release` destroys them, as in a new city.
  - The reset (`0x005D3250`/`0x005D2FE0`) goes in descending class ID. It drops lots (`0xC9BD5D4A`) and buildings
    (`0xA9BD882D`) before developers (`0x0990C138`), so each developer's copy then holds the **last** reference to
    its special buildings.
  - This corrects re-answers10's DG28 answer, which counted no table reference. For a loaded, unsaved city the DG28
    bracket ("the copy's release in the destructor is the last one") holds. `FC4_RELEASE_AT_DESTRUCTION` stays.

  building-development §1.4, growth-developer §1.1.
