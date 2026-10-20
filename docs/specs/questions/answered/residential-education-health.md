# residential-education-health: answered questions (archive)
<!-- Archive for [residential-education-health.md](../../residential-education-health.md). Lanes do not read this file. Open questions are in ../residential-education-health.md. -->

## From the residential education and health core (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/residential-education-health.md` (REH) §2 to §7. RS7 to RS17 are applied. The lane's answer is in
brackets; the module doc of `core/src/residential/education_health.rs` lists 1 to 13.

<!-- from: From the residential education and health core (2026-10-14) -->
- ~~**EH1:** do the efficiency ratings and the base staff efficiency start (a, b) at (0, 0)? [yes]~~ **ANSWERED
  2026-10-14** from the exe (`GetOccupantEfficiencyRatings` `0x006F8749`–`0x006F8759`, `GetBaseStaffEfficiency`
  `0x006F8611`–`0x006F8621`) and written into REH §4.2. **Yes.** Both set a = b = 0.0 and always call slot 20 before
  the staff-item test. With a staff item whose department is missing, the curves are read at 0: shipped school e1 =
  0, e2 = 0.5, base = 0.

<!-- from: From the residential education and health core (2026-10-14) -->
- ~~**EH2:** slot 19 sets both percentages: does it end with the dirty flag set? [yes]~~ **ANSWERED 2026-10-14** from
  the exe (`SetOccupantFundingPercentages` `0x006F8290`; flag store `0x006F834A`) and written into REH §4.2. **Yes.**
  The flag store is shared by both "no item" exits and the normal end; only a null department returns without it.
  The flag is never read.

<!-- from: From the residential education and health core (2026-10-14) -->
- ~~**EH3:** after `ChangeBudgetItemLine`, is the line item's line the building type? [yes]~~ **ANSWERED 2026-10-14**
  from the exe (`DoBuildingChanged` `0x006FB120`: hospital `0x006FB17F`, school `0x006FB39A`) and written into REH
  §4.1. **Yes.** The call stores the building type into its own copy of the item right after `ChangeBudgetItemLine`,
  and every later step of the call uses that line; the item is not read again.

<!-- from: From the residential education and health core (2026-10-14) -->
- ~~**EH4:** on removal with no line item for the line, is the count step skipped? [skipped]~~ **ANSWERED
  2026-10-14** from the exe (`DoBuildingChanged`: removal path `0x006FB677` → count step `0x006FB1F4`; line item
  used unchecked at `0x006FB200`; schools `0x006FB41B`) and written into REH §4.1. **No: the original crashes.** It
  fetches the line item and calls it with no null check. With consistent budget data the line item exists
  whenever a building of that line is removed. *[choice]* skip the count step and `RemoveLineItem` (the bracket).

<!-- from: From the residential education and health core (2026-10-14) -->
- ~~**EH5:** a building added without a capacity: is its effective capacity 0? [yes]~~ **ANSWERED 2026-10-14** from
  the exe (`DoBuildingChanged` capacity reads `0x006FB741` / `0x006FB8A2`, getter `0x005FD480`; grading
  `0x006FDBB7`) and written into REH §4.1 and §4.5 (corrected). **No: the original writes a leftover.** The getter
  leaves its output alone, so the add writes a stack leftover (the health department pointer, the school line
  item pointer, or older garbage), and grading uses the coverage-radius property ID as the capacity (not 0, as
  §4.5 had inferred). No shipped building lacks a capacity (all 18 school and hospital buildings checked).
  *[choice]* 0 in both places (the bracket).

<!-- from: From the residential education and health core (2026-10-14) -->
- ~~**EH6:** do "served", the tract list and the boost all use the lot centre, else `GetPosition`? [yes, all three]~~
  **ANSWERED 2026-10-14** from the exe (`GetCoveredTractList` `0x006F7E00`; `GetPopulationServed` `0x006F8F60`,
  position at `0x006F8FB7`; boost centre in `ComputeOccupantGrade` `0x006FDE22`–`0x006FDE4E`) and written into REH
  §4.5. **No, two of three.** The tract list and the boost use the lot centre, else `GetPosition`. "Served" takes
  the building's own tract from the occupant's `GetPosition` only, with no lot lookup, so the two tracts can
  differ (worked example in §4.5).

<!-- from: From the residential education and health core (2026-10-14) -->
- ~~**EH7:** how do slots 28 and 58 pick the service? [like slot 45: hospital capacity first, then school]~~
  **ANSWERED 2026-10-14** from the exe (slot 28 `0x006FD510`, slot 58 `0x006FD680`, slot 45 `0x006FD260`) and
  written into REH §4.10 and §4.11. **The bracket differs (school first in both).** Slot 28: school if the building
  has the school capacity `0x8922041B`, else hospital (unchecked). Slot 58: school if the school query succeeds,
  else hospital if the hospital query succeeds, else AL = 0. Also slot 45's "hospital wins" covers only the radius,
  kind and ordinance list; its school flag stays set. No shipped building has both capacities, so only mods see it.

<!-- from: From the residential education and health core (2026-10-14) -->
- ~~**EH8:** slot 45 with no matching point or no radius? [AL = 1 whenever the building's kind is found, distance 0,
  radius 0]~~ **ANSWERED 2026-10-14** from the exe (`GetOccupantCoverage` `0x006FD260`: radius local zeroed
  `0x006FD308`, dist zeroed `0x006FD435`) and written into REH §4.11 ("Edge cases"). **The bracket matches**, with
  one more case: a distance curve with no points (a property with fewer than two values) skips the step and leaves
  dist unwritten (the caller's value). *[choice]* write 0 there too.

<!-- from: From the residential education and health core (2026-10-14) -->
- ~~**EH9:** does the ideal funding test the served property itself for `0xFFFFFFFF`? [yes, the property]~~
  **ANSWERED 2026-10-14** from the exe (`EstimateIdealFunding` `0x006FD701`–`0x006FD72B`) and written into REH
  §4.10. **Yes, the property**, read into a value preset to `0xFFFFFFFF`, so a missing property also fails. The
  ratio then uses the query data's served (+0x24) and capacity (+0x14), which equal the properties at that point.

<!-- from: From the residential education and health core (2026-10-14) -->
- ~~**EH10:** does the hospital query fail on missing fields, testing the staff item last? [yes]~~ **ANSWERED
  2026-10-14** from the exe (`GetHospitalQueryData` `0x006F7E80`, staff test `0x006F7F37`; school `0x006F91C0`) and
  written into REH §4.11 ("Order of the tests and partial writes"). **Yes**: capacity, effective capacity, served,
  grade, then the staff item. Also: capacity, effective capacity and served are read straight into the record, so
  a failed query leaves them written (served raw) and the rest untouched.

<!-- from: From the residential education and health core (2026-10-14) -->
- ~~**EH11:** what does the ratings query's record start at? [zero]~~ **ANSWERED 2026-10-14** from the exe
  (`GetSchoolSystemRating` `0x006F9F27`–`0x006F9F79`, `GetHealthSystemRating` `0x006F9DD7`–`0x006F9E29`) and
  written into REH §4.11 ("Starting record"). **Partly zero.** Only +0x10, +0x28, +0x2C, +0x34 (and the string)
  are set; the summed +0x18 and the other fields start as stack leftovers. The leftover is reached only when the
  first building's query fails before reading +0x18, which needs a building without a capacity (none ships).
  *[choice]* 0 (the bracket).

<!-- from: From the residential education and health core (2026-10-14) -->
- ~~**EH12:** do the system totals count only successful queries and add onto the caller's record? [yes]~~
  **ANSWERED 2026-10-14** from the exe (`GetSystemTotals` `0x006FA070`: reset `0x006FA0A6`–`0x006FA0BB`, sums
  `0x006FA167`–`0x006FA1CF`) and written into REH §4.11 (Totals, corrected). **No on both.** The helper first zeroes
  the caller's +0x14..+0x28, then counts and adds **every** matching building, ignoring the query result (a failed
  query contributes what it read plus the previous building's other fields). "Matching" is per building:
  `IsOccupantGroup` for any of `groups`, over every group list. Shipped data reaches it: the farmer's market's
  hospital query always fails (no health staff item) but it is still counted.

<!-- from: From the residential education and health core (2026-10-14) -->
- ~~**EH13:** in `AddRadiusEffectWithCurves`, with n = 0 is pop' = pop, and is the same tract index used for all
  grids? [yes]~~ **ANSWERED 2026-10-14** from the exe (`AddRadiusEffectWithCurves` `0x005E48F0`: n test
  `0x005E4B5F`–`0x005E4B72`; reads `0x005E4A41`, `0x005E4B3F`, `0x005E4B4E`, `0x005E4BBA`, `0x005E4AF0`) and written
  into REH §4.6 (step 7, "Which tract each grid is read at", example). **Yes on both.** n = 0 skips k entirely: eff
  and pop are used unscaled. Population, coverage and target are all read at the same (x, z); the age grid is read
  at cell (x × S_t, z × S_t), the same tract when its size equals the target's. Bounds and geometry come from the
  target only; all four grids have tract size 4 in every game-written city. *[choice]* a tract outside a smaller
  source grid (hand-made save only) reads as 0.

<!-- from: From the residential education and health core (2026-10-14) -->
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

## From the residential glue and shadow (filed by the coordinator from the `resglue` lane's report, 2026-10-14)

About `docs/specs/residential-education-health.md` (REH) and the residential glue (`cFC4Residential`,
`FC4ResidentialShadow`). The current answer is in brackets.

<!-- from: From the residential glue and shadow (2026-10-14) -->
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

<!-- from: From the residential glue and shadow (2026-10-14) -->
- ~~**RG2 — REH §4.1 step 4:** building slot 11 is read as `GetBuildingProfile()`, first u32. Right? [yes]~~
  **ANSWERED 2026-10-14** from the exe (call `0x006FB9CC`; building slot 11 `0x00459250` → BDS slot 39) and written
  into REH §4.1 step 4. **Confirmed; the bracket matches.** Slot 11 returns the building's profile (or a static zero
  profile without a building development simulator), and its first u32 is the purpose: the flag is set for purpose 1
  (Residence). An early exit of the call skips the step.

<!-- from: From the residential glue and shadow (2026-10-14) -->
- ~~**RG3:** query-record costs are 32-bit in the game but 64-bit in the core. [zero-extended in, low 32 bits written
  back]~~ **ANSWERED 2026-10-14** from the exe (school `0x006F92B7`, `0x006F92F6`; hospital `0x006F7F76`–`0x006F7FAC`)
  and written into REH §4.11 (cost fields). **The bracket matches.** The record holds the low dword of the item's
  64-bit cost, so writing back the low 32 bits is what the original does. Zero or sign extension on the way in differ
  only for a negative cost, and shipped costs are positive.

<!-- from: From the residential glue and shadow (2026-10-14) -->
- ~~**RG4:** slots 14/15 with a null group vector: the original dereferences it. [ours returns 0]~~ **ANSWERED
  2026-10-14** from the exe (helper `0x006FA070`, dereference at `0x006FA11C`) and written into REH §4.11 (totals).
  **The bracket stands as a *[choice]*.** The original dereferences `groups` per building, so it crashes only when
  some group list holds a building; with no buildings it returns 0 after resetting the totals. Returning 0 for null
  matches the original wherever the original does not crash.

<!-- from: From the residential glue and shadow (2026-10-14) -->
- ~~**RG5:** lot `GetCenter` goes through the FXAM-guarded raw call (crime CQ3). [as crime]~~ **ANSWERED 2026-10-14**
  from the exe (centre helper `0x006F7E00`; lot slot 15 `0x006BF560`) and written into REH §4.4 (GetCoveredTractList).
  **Confirmed; the bracket matches.** The class gets the lot from the lot manager and calls the same `GetCenter` as
  crime: it writes x, y (the lot's altitude) and z, returns the output's address in EAX and leaves the x87 stack
  empty. Without a lot it uses the occupant's `GetPosition`.

<!-- from: From the residential glue and shadow (2026-10-14) -->
- ~~**RG6:** the shadow cannot reproduce the original's clock-seeded strike rolls in state 0. [those fields are not
  compared]~~ **ANSWERED 2026-10-14** from the exe (generator at `+0xB8`, constructor `0x007005AA`) and written into
  REH §2 (the `cRZRandom` row) and §4.9. **No original behaviour; the implementer's choice stands.** Every draw of the
  class is a double draw, which uses only `mul` (`+0xBC`). A shadow that reads that word from the original object
  before state 0 could reproduce the rolls, if it ever wants to compare them.

<!-- from: From the residential glue and shadow (2026-10-14) -->
- ~~**RG7:** does the original re-read ordinance effects outside state 0? [the shadow re-reads them]~~ **ANSWERED
  2026-10-14** from the exe (`GetOrdinanceEffects` `0x006FE0F0`; its only references are the calls at `0x006FED0B` and
  `0x006FF5CA`) and written into REH §4.3. **No; the bracket differs.** The original reads the ordinance effects only
  in `Init` and in state 0. A shadow that re-reads them elsewhere can see an ordinance change earlier than the
  original does; it should keep the lists from state 0 to state 0.

<!-- from: From the residential glue and shadow (2026-10-14) -->
- ~~**RG8:** slots 21/22 called off the main thread create grids through GZCOM. Allowed? [yes]~~ **ANSWERED
  2026-10-14** from the exe (slot 21 `0x006FCF30`, allocation `0x006FCF36`; slot 22 the same) and written into REH
  §4.8 (the slot 21/22 row). **No original behaviour; the implementer's choice stands.** The original does not use
  GZCOM here: it builds the float grid directly (global heap, inlined constructor). Its callers are UI code on the
  main thread *[inferred]*, so nothing in the original constrains a call from another thread.

<!-- from: From the residential glue and shadow (2026-10-14) -->
- ~~**RG9:** setters off the main thread: deferred, and `EndStrike` off the main thread returns the published flag?
  [yes]~~ **ANSWERED 2026-10-14** from the exe (`EndStrike`, slots 8/9) and written into REH §4.9. **No original
  behaviour; the implementer's choice stands.** The original applies every setter at once on the caller's thread, and
  its callers are on the main thread *[inferred]*. `EndSchoolStrike` / `EndHealthStrike` return AL = the city strike
  flag before the call and post the end message when it was set, so returning the published flag matches.

<!-- from: From the residential glue and shadow (2026-10-14) -->
- ~~**RG10:** department funding with no department? [1.0]~~ **ANSWERED 2026-10-14** from the exe (`0x006F8360` via
  slot 20 `0x006F84B0`) and written into REH §4.2. **The bracket differs: not 1.0.** A null department leaves the
  output untouched (both outputs for the staff department; only b for the vehicle department, after a is written). The
  class's own callers preset 0.0, so the curves see 0. The value 1.0 is used only when the budget item itself is
  missing. The host should report a missing department rather than return 1.0.

<!-- from: From the residential glue and shadow (2026-10-14) -->
- ~~**RG11:** slots 19, 20, 21–25, 28, 47 and 49: always true? [true]~~ **ANSWERED 2026-10-14** from the exe (the 14
  call sites listed in REH §7) and written into REH §7. **Confirmed; the bracket matches.** No caller that reaches
  these slots through the residential global tests EAX or AL afterwards, so returning true is safe. Callers that keep
  the simulator pointer in a member of their own were not all traced; for them it stays *[inferred]*.

<!-- from: From the residential glue and shadow (2026-10-14) -->
- ~~**RG12:** slot 45 returning false: are its outputs left untouched? [untouched]~~ **ANSWERED 2026-10-14** from the
  exe (`GetOccupantCoverage` `0x006FD260`; exit `0x006FD502`) and written into REH §4.11 (slot 45). **Confirmed; the
  bracket matches.** AL = 0 happens only when the building has neither capacity, and that path jumps to the epilogue
  before `dist` or `radius` is written.

<!-- from: From the residential glue and shadow (2026-10-14) -->
- ~~**RG13:** does `SetLocalFundingPercent` set the budget item's local funding? [the shadow assumes so]~~ **ANSWERED
  2026-10-14** from the exe (`ChangeBudgetItemLocalFunding` store `0x00692DBD`; `GetBudgetItemInfo` read `0x006939E6`)
  and written into REH §4.2 with a table. **Yes, but not exactly: the bracket needs a correction.** The item's local
  funding is stored as a percentage, float32(q × 100.0f), and read back as float32(pct × 0.01f), so q = 0.43 reads
  back as 0.42999998 and 0.79999995 as 0.7999999 (0.5 and 1.2 survive). A shadow must compare the read-back value.
  budget.md §2/§7 writes the read as "pct/100"; it is a multiplication by the float 0.01f (reported for the budget
  spec's owner).

<!-- from: From the residential glue and shadow (2026-10-14) -->
- ~~**RG14:** the dirty flag cannot be compared by the shadow. [not compared]~~ **ANSWERED 2026-10-14** from the exe
  (byte `+0xA0`; constructor `0x007005B2`; the setters listed in REH §2) and written into REH §2 (the dirty-flag row).
  **No original behaviour; the implementer's choice stands.** No interface slot returns the flag and nothing in the
  class reads it; it shows only in the save, as `dirty ∨ state ≠ 10`.

<!-- from: From the residential glue and shadow (2026-10-14) -->
- ~~**RG15:** a building's identity is its `AsOccupant()` pointer, used for the lists, the strike message's void1 and
  IGZUnknown, and record references. Right? [yes]~~ **ANSWERED 2026-10-14** from the exe (`DoBuildingChanged` list
  inserts and removals; `BeginStrike` `0x006F84C0`, void1 `+0x30` and IGZUnknown `+0x84`; `EndStrike` `0x006F8570`)
  and written into REH §4.1 (building identity) and §5. **Confirmed; the bracket matches.** The lists hold
  `AsOccupant()` pointers and removal compares `AsOccupant()`. The strike-begin message carries that list pointer as
  both void1 and IGZUnknown, with data2 = 1; the strike-end message sets only data2 = 0 and data3 = the length.

<!-- from: From the residential glue and shadow (2026-10-14) -->
- ~~**RG16:** per-building properties: scalars of their type (bool, u32, f32), or arrays? [scalars]~~ **ANSWERED
  2026-10-14** from the exe (helpers `0x005FE3C0`, `0x005FE4F0`, `0x005FE210`; `SetValUint32` `0x0091587D`) and
  written into REH §2 (per-building state). **Scalars; the bracket matches.** Each helper creates the property if it
  is missing and calls the variant's scalar setter, which replaces any other content (an array too) with one value of
  the property's type.

<!-- from: From the residential glue and shadow (2026-10-14) -->
- ~~**RG17:** slots 47, 49 and 61 append to the caller's list with the game allocator. Is that so for slot 61?
  [assumed]~~ **ANSWERED 2026-10-14** from the exe (slot 61 `0x006FAEC0`: allocations at `0x006FAF30` and
  `0x006FAFBA`; slots 47/49 through `GetBuildings` `0x006FACE0`, allocation at `0x006FADA3`) and written into
  residential.md §10.1 (slot 61 row). **The bracket matches: yes.** All three take each 12-byte node from the framework
  allocator service (allocator A, the cached pointer at `0x00B207E4`, slot 3), link it at the tail of the caller's
  list and `AddRef` the occupant. This is the allocator of the SDK's `SC4List`, so the caller frees the nodes the
  usual way.

<!-- from: From the residential glue and shadow (2026-10-14) -->
- ~~**RG18:** the simulator taken at Init step 4 is released after `RemoveAgent`; REH §4.2 has no matching release.
  [released]~~ **ANSWERED 2026-10-14** from the exe (`Init` `0x006FEC0E`; `Shutdown` `0x006F980D`) and written into
  residential.md §4.1 step 4 and §4.2. **The original holds no reference: the simulator is borrowed.** `Init` reads
  the simulator global without `AddRef` and keeps no member; `Shutdown` asks the city for the simulator again for
  `RemoveAgent` and releases nothing. A glue that takes a reference at Init and releases it after `RemoveAgent` is
  equivalent, since the city outlives both calls.

<!-- from: From the residential glue and shadow (2026-10-14) -->
- ~~**RG19:** what goes in a strike message's data1? [0]~~ **ANSWERED 2026-10-14** from the exe (message constructor
  `0x009134D6`; `BeginStrike` `0x006F84C0`, `EndStrike` `0x006F8570`) and written into REH §5. **0; the bracket
  matches.** The constructor zeroes every field and neither strike call sets data1.

<!-- from: From the residential glue and shadow (2026-10-14) -->
- ~~**RG20:** `GetBuildingMapKey` returns 32 bits; only the low 8 are used. Right? [yes]~~ **ANSWERED 2026-10-14**
  from the exe (calls `0x006FF863`, `0x006FF9E8`, `0x006FFB56`; seed helper `0x005E4FA0`, byte loads at `0x005E5049`,
  `0x005E5078`, `0x005E50CC`) and written into residential.md §5.3 step 1. **Confirmed; the bracket matches.** Slot 42
  sets only AL; the class passes EAX on, and the seed helper reads and compares the key as a byte.

<!-- from: From the residential glue and shadow (2026-10-14) -->
- ~~**RG21:** the strike clock seed is `time() + 2177452800` (the cRZTime epoch), as in crime. [yes]~~ **ANSWERED
  2026-10-14** from the exe (constructor `0x007005AA` → `cRZRandom(−1)` `0x00913A5C` → `Seed` `0x00913667`; `time`
  `0x009EFB59`, `clock` `0x009F361F`) and written into REH §2 (the `cRZRandom` row) and foundations §6.1 (`Seed`).
  **The bracket differs, but it cannot be observed.** The seed is `time(NULL) XOR ((clock() << 16) | clock())`, not
  the `cRZTime` epoch value. No save or output exposes it. *[choice]* FreeCity4 uses `derive(0x4990C013)` from the
  session seed, as foundations §6.3 prescribes for member generators.

<!-- from: From the residential glue and shadow (2026-10-14) -->
- ~~**RG22 — `docs/abi`:** residential slots 37/38 (`GetCellEQByWealth`/`GetCellHQByWealth`) are `via-call`, low
  confidence, in `interfaces.tsv`, while residential.md §10.1 says ST0 (they end by calling the float grid's
  `GetTractValue`, x87 high). Should the rows say x87? [declared float, on the spec's word]~~ **ANSWERED 2026-10-14**
  from the exe (`0x006F7A60`, `0x006F7AB0`; float grid `GetTractValue` `0x0045F210`) and noted in REH §7 (slot 37).
  **Yes: the rows should say x87, high confidence.** Each method divides x and z by the EQ grid's tract size, indexes
  `EQ[w − 1]` (HQ for slot 38) and returns through the float grid's `GetTractValue`, whose `fld` leaves the result in
  ST0; the method's `ret 0xC` does not touch it. The corrected `docs/abi/interfaces.tsv` rows are in the lane's report
  (the spec does not edit `docs/abi`).

<!-- from: From the residential glue and shadow (2026-10-14) -->
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

<!-- from: From applying the aura, education/health and flora answers (2026-10-14) -->
- ~~**AQ1:** REH: the totals record's leftover fields +0x14..+0x24. [they start at 0]~~
  **ANSWERED 2026-10-14** from the exe (totals helper `0x006FA070`, record set up once at `0x006FA0D5`–`0x006FA0E9`) and
  written into residential-education-health §4.11. The original does not start them at 0: only +0x10, +0x28, +0x2C and
  +0x34 are set, so +0x14..+0x24 hold stack leftovers until a query writes them. Shipped data reaches it: a farmer's
  market counted first adds two leftover words at +0x1C and +0x20. The bracket (0) is a *[choice]*, recommended.

<!-- from: From applying the aura, education/health and flora answers (2026-10-14) -->
- ~~**AQ2:** REH §4.8: S_EQ against member E. [E is kept]~~
  **ANSWERED 2026-10-14** from the exe (`GetCellEQ` `0x006F79C0`, `GetCellEQByWealth` `0x006F7A60`,
  `GetEQMinAndMaxCellCoords` `0x006F7380`, `GetAverageEQGrid` `0x006FCF30`) and written into
  residential-education-health §4.8 and residential §2 and §9.2. Only `GetAverageEQGrid` reads the member E, as the new
  grid's tract size. Slots 35, 37 and 43 divide by the R$ EQ grid's own `GetTractSize()`, asked on every call. Both are
  4 in every game-written city. The bracket (E) matches slot 21 only; elsewhere it is a *[choice]* that differs only in
  a hand-made save, and asking the grid is recommended.

<!-- from: From applying the aura, education/health and flora answers (2026-10-14) -->
- ~~**AQ3:** S_EQ for the HQ variants. [E]~~
  **ANSWERED 2026-10-14** from the exe (`GetCellHQ` `0x006F7A10`, `GetCellHQByWealth` `0x006F7AB0`,
  `GetHQMinAndMaxCellCoords` `0x006F7550`, `GetAverageHQGrid` `0x006FCFD0`) and written into
  residential-education-health §4.8. Slot 22 uses E like slot 21. Slots 36, 38 and 44 divide by the R$ **HQ** grid's own
  tract size, not an EQ grid's and not E. The bracket (E) matches slot 22 only; as in AQ2, the difference shows only in
  a hand-made save.

<!-- from: From applying the residential, budget, demand, land value and flammability answers (2026-10-14) -->
- ~~**RG24:** RG23: what does the proximity seed do when there is no building map? [writes FAR to `(x, 0)` only and
  keeps every other cell; a choice]~~
  **ANSWERED 2026-10-14** from the exe (seed helper `0x005E4FA0`: null map → return at `0x005E4FA8`, null grid at
  `0x005E4FB5`; state 13 call `0x006FF87C`) and written into residential §5.3 step 1. It writes nothing, not even the
  edge. A fresh grid stays all 0 and a reused one keeps the previous wealth's result, so with no map every proximity
  grid is 0. The bracket (255 on the edge, giving 1 there after the passes) differs; *[choice]* write nothing, as the
  original does (recommended).

## From applying the residential, protection and save-reference answers (filed by the coordinator from the `apply8a` lane's report, 2026-10-14)

RG1–RG22, RG13 in the budget core, PR14–PR16, PG15, DQ13, SV1–SV2, LF6 and NB18 are applied or cited. RG1 creates
the buzzards through the effects manager (new switch `residential_effects`, 160 switches). The current answer is in
brackets.

<!-- from: From applying the residential, protection and save-reference answers (2026-10-14) -->
- ~~**RG25:** RG1: the buzzard transform's bytes +2 and +3 are not stated. [0]~~
  **ANSWERED 2026-10-14** from the exe (`0x006FA814`–`0x006FA819`; effect `SetTransform` `0x005BE880` →
  `0x005BEC20`) and written into residential-education-health.md §4.7 step 3. The original never writes +2 and
  +3: they are uninitialised stack bytes, the alignment gap after the two flag bytes. The effect copies only byte
  +1 and the dwords +0x04 to +0x34, so it reads neither +0, +2 nor +3. **The bracket (0) is equivalent.**

<!-- from: From applying the residential, protection and save-reference answers (2026-10-14) -->
- ~~**RG26:** RG10: how does a host report "cannot be read"? [NaN or a NULL callback means missing; the outputs are left
  alone]~~
  **ANSWERED 2026-10-14**: glue-only, no original behaviour; the implementer's choice stands. Written into
  residential-education-health.md §4.2. The only original "missing" is a null `GetDepartmentBudget` for an
  existing item. A missing item gives 1.0 and must not be reported as missing. A NaN is a safe sentinel, because
  a real funding fraction is never NaN.

<!-- from: From applying the residential, protection and save-reference answers (2026-10-14) -->
- ~~**RG27:** RG1: does "cannot show" (a NULL callback or `FC4_EFFECT_NOT_SHOWN`) count as created? [yes: the second
  draw is made and the flag set, but there is no altitude read, start or release]~~
  **ANSWERED 2026-10-14** from the exe (`0x006FA6AA`–`0x006FA83E`) and written into
  residential-education-health.md §4.7 step 3. **Yes.** After a successful create, only the second draw and the
  flag change simulation state. The altitude read is a pure query, and `Start` and `Release` act only on the
  effect. **The bracket matches**, as long as the second draw is never skipped.

<!-- from: From applying the residential, protection and save-reference answers (2026-10-14) -->
- ~~**RG28:** RG7: which ordinance effects does the shadow use before its first state 0? [it reads them as they stand]~~
  **ANSWERED 2026-10-14** from the exe (callers `0x006FED0B` and `0x006FF5CA`, RG7) and written into
  residential-education-health.md §4.3 with an example. Before its first state 0, the original uses the lists that
  `Init` built: the ordinances on in the save, or none in a new city. The shadow has no original counterpart.
  **The bracket is acceptable** while no ordinance has changed since `Init`. *[choice]*: to match always, read the
  effects once at `Init` and then only at the original's state 0.

<!-- from: From applying the REH, flora, residential and flammability answers (2026-10-14) -->
- ~~**AQ11:** AQ2/AQ3: when there is no R$ EQ or HQ grid, which tract size do the R$$ and R$$$ variants and the
  minimum and maximum coordinates use? [member E]~~ **ANSWERED 2026-10-15 (re-answers10):** no original behaviour: the original never tests the R$ grid pointers (`+0x7C`, `+0x88`). Slots 35–38 call the R$ grid's `GetTractSize` at once (`0x006F79C4`, `0x006F7A63`); the wealth variants never ask the R$$ or R$$$ grid. Slots 43/44 return the six zeros when the population is 0 (`0x006F7389`), and otherwise take the walk's tract counts and the size from the R$ grid. The quotient helper (`0x006F6FC0`) reads all six grids untested, so a missing grid crashes. Member E (with E's tract counts) is accepted. residential-education-health §4.8.
