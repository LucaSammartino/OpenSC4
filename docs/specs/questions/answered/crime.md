# crime: answered questions (archive)
<!-- Archive for [crime.md](../../crime.md). Lanes do not read this file. Open questions are in ../crime.md. -->

## From the crime glue and grid shadow (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/crime.md` and its `docs/abi` rows. Ini: `enable = crime` (optional `crime_seed`); shadow
`crime_shadow = 1`. The glue's current answer is in brackets.

<!-- from: From the crime glue and grid shadow (2026-10-14) -->
- ~~**CQ1:** `GetCrimeIncidentArray`: allocators.md does not list its container. [the game's allocator]~~
  **ANSWERED 2026-10-14** from the exe (`0x006A1960`, resize `0x006A1380`, growth `0x0069FE90`; caller
  `0x00553370`) and written into crime.md §11.4 and §14.2. **The bracket is right, and the allocator is B** (the
  STL node allocator, `0x0090CF54`/`0x0090CF63`): `out` is a default-allocator `std::vector`. The only caller
  passes an empty vector and frees the buffer with B. New elements of lot-less incidents are a fill value with a
  zero icon key and 16 uninitialised bytes; *[choice]* zero-fill. allocators.md needs a row (coordinator).

<!-- from: From the crime glue and grid shadow (2026-10-14) -->
- ~~**CQ2:** `PropProfile.crimeIncidentID` is read at offset 24, from the SDK layout; layouts.md does not list
  `PropProfile`. [24]~~ **ANSWERED 2026-10-14** from the exe (`SelectProp` `0x006A016B`–`0x006A016E`: prop
  manager slot 13, then `mov eax, [eax+0x18]`) and written into crime.md §11.6 and §13. **The bracket is right:
  24** (`u32`). Nothing else of the profile is read by crime.

<!-- from: From the crime glue and grid shadow (2026-10-14) -->
- ~~**CQ3:** lot `GetCenter` (slot 15): docs/abi says ST0, §13 says the out parameter with nothing left on the x87
  stack. [both handled; the month line counts which one happens as `center st0=a/b`]~~ **ANSWERED 2026-10-14**
  from the exe (`cSC4Lot::GetCenter` `0x006BF560`, the only implementation) and written into crime.md §13. **§13
  is right; docs/abi is wrong.** The result is only in the out parameter, and EAX = `&out`. Four `fild`s feed two
  `fstp` stores, and the last value is popped (`fstp st(0)`, `0x006BF5B6`), so the **x87 stack is empty** on
  return. Declare it returning the pointer (or void), never `float`: popping ST0 after the call would underflow
  the x87 stack. The glue's ST0 branch should never fire. The abi row is the coordinator's to fix.

<!-- from: From the crime glue and grid shadow (2026-10-14) -->
- ~~**CQ4:** `GetOccupantsByBBox`: §13 names the filter overload (slot 36). [the typed overload, slot 37, last
  argument 0xFFFFFFFF]~~ **ANSWERED 2026-10-14** from the exe (`GetNearestCrimeSim` `0x006A121D`,
  `GetNearestCrimeProp` `0x006A0F7D`; filters `0x0069F4D0`/`0x0069F490`; slot 37 `0x0049DD40`; the manager's
  typed filter `0x004BDF90` → `0x005E9740`) and written into crime.md §11.3 and §13, closing §16 Q5. The original
  calls **slot 36** with a per-call filter object of its own (type plus group, or type plus property) and flags
  `0xFFFFFFFF`. **The bracket is equivalent**, as long as the group or property test is applied to each listed
  occupant before it can become a candidate. Slot 37's filter is `GetType() == type`, and the walk and order are
  the same.

<!-- from: From the crime glue and grid shadow (2026-10-14) -->
- ~~**CQ5:** the session seed "local wall-clock seconds". [`_mkgmtime(localtime(now))`]~~ **ANSWERED 2026-10-14**
  from the exe (static initializer `0x00A74B70`; `cRZTime::cRZTime` `0x00470B50`) and written into crime.md §9.
  **The bracket is not the original value.** The seed is `cRZTime()` XOR `0x6C09C3F6`, and `cRZTime()` is u32
  seconds since **1901-01-01 00:00 UTC**, rebuilt from the `localtime` fields with `_timezone` and the library's
  fixed old-US daylight rule. That is `time() + 2177452800` when the host's daylight rule agrees, one hour off when
  it does not, and 0 in the library's spring-forward hour. The bracket counts local seconds from 1970 instead. No run
  can observe the seed, so this is a *[choice]*: crime.md §9 recommends `(uint32)(time(NULL) + 2177452800) XOR
  0x6C09C3F6`.

<!-- from: From the crime glue and grid shadow (2026-10-14) -->
- ~~**CQ6 — §16.1:** the ordinance crime factor after a load with no ordinance simulator. [left unchanged]~~
  **ANSWERED 2026-10-14** from the exe (crime `Init` step 6; OrdinanceUpdated handler `0x006A22D5`; the only poster
  of `0xC2C09554` is `SetOrdinanceOn`, `0x0080D714`) and written into crime.md §4.1 and §16 Q1. **No original
  behaviour; the implementer's choice stands.** With no ordinance simulator the original crashes in `Init` step 6
  (the AddRef has no null check), before any month. "Left unchanged" means 1.0, the value `Init` step 9 sets. §16 Q1
  is closed: nothing posts OrdinanceUpdated after a load. Even with an ordinance simulator, a loaded city runs at
  1.0 until some ordinance's on flag changes. Then the handler re-reads the factor over all on ordinances.

<!-- from: From the crime glue and grid shadow (2026-10-14) -->
- ~~**CQ7:** the prop requester is never unregistered at Shutdown; the spec is silent. [not unregistered]~~
  **ANSWERED 2026-10-14** from the exe (crime `Shutdown` `0x006A1860`; prop developer `AddPropRequester`
  `0x006F21C0` and its `Shutdown` `0x006F2F70`–`0x006F2FCE`) and written into crime.md §4.2. **The bracket
  matches.** Crime's `Shutdown` makes no prop-developer call. The prop developer AddRefs each requester on
  registration (no duplicate check) and releases each one once at its own `Shutdown`, which empties the list. So
  the crime object lives until then.

<!-- from: From the crime glue and grid shadow (2026-10-14) -->
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

<!-- from: From the crime glue and grid shadow (2026-10-14) -->
- ~~**CQ9:** `Read` on a stream error. [returns false, no state changed]~~ **ANSWERED 2026-10-14** from the exe
  (`cSC4CrimeSimulator::Read` `0x006A05E0`) and written into crime.md §12. **The bracket is not the original but
  matches the §12 *[choice]*, which stands.** The only early exit is the version test. On a stream error the
  original reads on and sets `loadedFromSave`, then returns false. It nulls the grid refs, resizes the iterator
  table to the stream pointer's value, and loops over an uninitialised incident count, so it reads garbage. The
  following `Init` then leaves the simulator unusable.

<!-- from: From the crime glue and grid shadow (2026-10-14) -->
- ~~**CQ10:** crime type list order. [ascending instance]~~ **ANSWERED 2026-10-14** from the exe
  (`LoadAllCrimeTypesFromExemplars` `0x006A1450`, `GetAvailableResourceList` `0x009739CB`, the package hash-table walk
  `0x0096F9AB`) and already written into crime.md §7 ("Order of the list"). **The bracket matches the original for
  the shipped types.** The original uses the resource manager's key-list order. That order is segment order, and
  within each package the index hash-table buckets (`instance mod B`, B ≥ 53). All 14 shipped types are in
  `SimCity_1.dat` with instances below 53, so the order is ascending instance. Plug-in types can land elsewhere;
  ascending instance stays the §7 *[choice]*.

<!-- from: From the crime glue and grid shadow (2026-10-14) -->
- ~~**CQ11:** police slot 5 returns a uint8 grid (the SDK says short) and slot 27 an int32 count, as protection
  declares them. Confirm. [as protection]~~ **ANSWERED 2026-10-14** from the exe (slot 5 `0x006EFA50`, slot 27
  `0x006E0A00`) and written into crime.md §13. **Confirmed; the bracket matches.** Slot 5 returns the adjusted grid
  pointer at `+0x80`, the `uint8` grid, without an AddRef. Slot 27 returns its int32 count in EAX, after a float→int
  conversion.

<!-- from: From the crime glue and grid shadow (2026-10-14) -->
- ~~**CQ12:** arson calls `SetCause` whatever `IgniteOccupant` returns. Right? [yes]~~ **ANSWERED 2026-10-14** from
  the exe (`0x006A1D07`–`0x006A1D10`) and written into crime.md §6.8. **Yes; the bracket matches.** AL from
  `IgniteOccupant` (slot 4) is not tested. `SetCause(2)` (slot 9) follows unconditionally on the same manager.

<!-- from: From the crime glue and grid shadow (2026-10-14) -->
- ~~**CQ13:** property 0xCA5B9306 is read with numeric conversion (`GetU32s`, low bytes). [as stated]~~
  **ANSWERED 2026-10-14** from the exe (`DoOccupantChanged` `0x0069F27B`–`0x0069F2B1`; variant slot 95 `0x0060CE80`)
  and written into crime.md §6.10. **The bracket differs from the original but is equivalent for shipped data.** The
  original takes the variant's raw data pointer whatever its type, and reads bytes 0 and 1 with no conversion and no
  checks. That equals `GetU32s` + low bytes only for byte arrays, and all 6 shipped buildings are `Uint8` ×2. A
  `Uint32` array {100, 8} gives m = 100, r = 0. A scalar is dereferenced as a pointer, which crashes or reads
  garbage. *[choice]*: read raw bytes 0 and 1, and skip scalars and arrays shorter than 2 bytes.

<!-- from: From the crime glue and grid shadow (2026-10-14) -->
- ~~**CQ14:** a failed QueryInterface nulls the out pointer per crime.md §1; flammability, plumbing and pollution leave
  it untouched. Which is the original's rule per class? [crime nulls]~~ **ANSWERED 2026-10-14** from the exe (crime
  `0x0069F1FC`, flammability `0x006AE3D5`, plumbing `0x006DA405`, pollution `0x006E2D1C`; 64 implementations
  checked) and written into crime.md §1, which was wrong. **The bracket is wrong.** Crime returns AL = 0 and leaves
  the out pointer **untouched**, like flammability, plumbing, pollution and every other simulator class. The only
  class seen nulling it is `cSCResExemplarCohort`, on entry (`0x0067C15D`). One rule for all simulators: false with
  the out pointer untouched.

<!-- from: From the crime glue and grid shadow (2026-10-14) -->
- ~~**CQ15:** the return value of the residential `GetPopulationGrids` / `GetEQGrids` is ignored; only the three out
  pointers are used. [ignored]~~ **ANSWERED 2026-10-14** from the exe (`DoCensus` `0x0069FB24`/`0x0069FB3E`) and
  written into crime.md §13. **Confirmed; the bracket matches.** EAX is never read after either call (the next
  instruction clears EBX and the loop reads the out pointers). The order is `GetPopulationGrids` (slot 25) first,
  then `GetEQGrids` (slot 23).
- Not compared by the shadow (stated on every line): the number of crimes to commit (needs
  `ResetUnjailedCriminals`, which has side effects), which attempts the police stopped and each crime's type (the
  original's draws), which lots were tried (the lot manager's generator), the crime-effect grid (no interface).

## From applying the crime answers (filed by the coordinator from the `apply2` lane's report, 2026-10-14)

About `docs/specs/crime.md` (CQ5, CQ8, CQ13 as applied). The glue's current answer is in brackets.

<!-- from: From applying the crime answers (2026-10-14) -->
- ~~**CQ16 — CQ8:** the LTEXT string loads but the game's detokenizer returns false. [The load counts as failed;
  `0xAA12FCB0` replaces the name only if that property exists, otherwise the copied text stays.]~~ **ANSWERED
  2026-10-14** from the exe (`SC4LoadString` `0x00603040`: detokenizer call `0x00603133`, failure exit
  `0x00603202`; crime loader reset `0x006A0C9C`–`0x006A0CA6`) and written into crime.md §7 ("Name", steps 0 and
  "A failure in steps 2 or 3"). **Half right.** The load does fail, and `0xAA12FCB0` replaces the name only if it
  exists. But the copied text does not stay. `SC4LoadString`'s failure exit calls `Resize(0)` on the output, so
  the name is empty. The loader had already emptied it before the load, so every failed load leaves the name
  empty unless `0xAA12FCB0` exists.

<!-- from: From applying the crime answers (2026-10-14) -->
- ~~**CQ17 — CQ8:** property `0x8A416B2E` has fewer than 3 values. [No key, no lookup; the fallback is used.]~~
  **ANSWERED 2026-10-14** from the exe (`SC4LoadExemplarString` `0x00603220` → `cSCPropertyHelp::GetPropertyValue`
  `0x005FD5B0`, variant slot 103 `0x0060CE80`) and written into crime.md §7 ("How step 1 reads the key"). **The
  original has no such rule:** it copies three `uint32` from the variant's data pointer with no type or count
  check. With fewer values it reads past the data block, so `G` or `I` is heap garbage, and a scalar's inline
  value is used as a pointer. The lookup then almost always misses, and the fallback applies. **The bracket stands as
  a *[choice]*:** treat fewer than 3 values (or a scalar) as missing, which gives that usual outcome.

<!-- from: From applying the crime answers (2026-10-14) -->
- ~~**CQ18 — CQ8:** there is no language manager. [Language 0, so only (group, instance) is tried.]~~ **ANSWERED
  2026-10-14** from the exe (`SC4LoadString` `0x00603063`–`0x0060306C`; `RZ::LM` `0x00913D7A`, system service
  `0x441E5070` through `cRZSysServPtr` `0x00610800`) and written into crime.md §7 ("The language ID"). **The original
  crashes**: it calls slot 7 through the null language-manager pointer, with no test. The game always has one.
  **The bracket stands as a *[choice]*:** language 0, so only (group, instance) is tried.

<!-- from: From applying the crime answers (2026-10-14) -->
- ~~**CQ19 — CQ8:** is the detokenizer returned by `GetStringDetokenizer` (`cISC4App +0xA8`) borrowed or owned?
  [Borrowed; the glue takes and releases its own reference around the call.]~~ **ANSWERED 2026-10-14** from the
  exe (`cSC4App::GetStringDetokenizer` `0x0044B4D0`; cache `SL::OnAppInit1` `0x00602323`; use in `SC4LoadString`
  `0x00603121`–`0x00603133`) and written into crime.md §7 ("The detokenizer is borrowed"). **The bracket matches.**
  The getter returns the application's member with no `AddRef`, and the game caches it once in a global, also
  with no `AddRef`. `SC4LoadString` uses that global when its caller passes none, and the crime loader passes
  none. Nobody adds or releases a reference around the call. The glue's own `AddRef`/`Release` pair is harmless.

<!-- from: From applying the crime answers (2026-10-14) -->
- ~~**CQ20 — CQ8:** how does the original read the LTEXT: through the resource manager's `GetPrivateResource`, or a
  string loader of its own? [Through `GetPrivateResource`, as the SDK does.]~~ **ANSWERED 2026-10-14** from the exe
  (`SC4LoadString` `0x006030B8`/`0x006030F9`; resource manager slot 3 `0x009732EB`, slot 4 `0x00972D60`) and written
  into crime.md §7 ("How the resources are read"). **The bracket matches.** The LTEXT is requested from slot 4,
  `GetPrivateResource`, as a `cIGZString` (`0x089B7DC8`), with the last two arguments 0 and null. Slot 4 bypasses the
  cache that slot 3 (`GetResource`) uses. Step 1's exemplar is also reloaded through slot 4 (`0x006032AE`), apart
  from the loader's own cached `GetResource` load.

<!-- from: From applying the crime answers (2026-10-14) -->
- ~~**CQ21 — CQ5:** do other generators seeded from `cRZTime` use the same seconds-since-1901 value, so the helper
  belongs to the RNG module? [Crime module only.]~~ **ANSWERED 2026-10-14** from the exe (34 static initialisers
  `0x00A71CB0`–`0x00A75270`, construction occupant `0x005CEDAC`, prop developer `0x006F34A3`, all calling
  `cRZTime::cRZTime` `0x00470B50`) and written into crime.md §9 ("The clock value is shared") and foundations §6.3.
  **The bracket does not match: yes, they all use it.** Every clock-seeded generator calls the same `cRZTime()`
  constructor: 34 file-local generators (each XOR its own constant), the construction occupant's plain reseed, and
  the prop developer (value × 28). The helper belongs with the RNG module.

<!-- from: From applying the crime answers (2026-10-14) -->
- ~~**CQ22 — CQ13:** the crime effect property is an array whose elements are not plain numbers (strings, objects).
  [Treated as absent.]~~ **ANSWERED 2026-10-14** from the exe (`DoOccupantChanged` `0x0069F27B`–`0x0069F2B1`, variant
  raw accessor `0x0060CE80`) and written into crime.md §6.10. **Half right.** The original reads bytes 0 and 1 of the
  variant's data block whatever its type. A **string** is an `RZChar` array, so it gives the codes of its first two
  stored bytes (`"AB"`: m = 65, r = 66), a deterministic result; FreeCity4 should read it like any array, not treat
  it as absent. An **object array** can only be stored by code, never by a data file, and gives the low bytes of
  an object's address. Treating that as absent stays a *[choice]*.

## From applying the crime, traffic network and HG answers (filed by the coordinator from the `apply7b` lane's report, 2026-10-14)

CQ16–CQ22, the crime errata, TN21–TN25 and HG1 are applied (CQ16 and TN21 changed behaviour; the clock-seed helper
moved to `rng.rs`). Not done: the development simulator's own clock seed still XORs by itself (the development glue
lane owns those files); TN22/TN24 have unit tests but no vectors. The current answer is in brackets.

<!-- from: From applying the crime, traffic network and HG answers (2026-10-14) -->
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

<!-- from: From applying the crime, traffic network and HG answers (2026-10-14) -->
- ~~**CQ24:** CQ20 follow-up: the original reads the name key from a second, uncached load of the type's exemplar.
  [FreeCity4 reads it from the same cached load as the other properties; the two differ only if the cached copy was
  changed in memory]~~
  **ANSWERED 2026-10-14** from the exe (loader caller `0x006A1455`–`0x006A14FE`, cached load `0x006A0BC0`, uncached load
  `0x006032AE`) and written into crime.md ("The two exemplar loads agree"). Both loads use the same key (type
  `0x6534284A`, group `0xEA12F32C`), so they read the same record. They differ only if the cached copy was changed in
  memory (the loader only reads it; other writers were not traced) or if the uncached load alone fails, which sends the
  name to `0xAA12FCB0`. The bracket matches; the implementer's choice stands.
