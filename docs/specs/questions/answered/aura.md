# aura: answered questions (archive)
<!-- Archive for [aura.md](../../aura.md). Lanes do not read this file. Open questions are in ../aura.md. -->

<!-- from: growth-cities29; answered by re-gc30 -->
- **AU-GC29-1 - first-cycle Lua occupant cones (sections 5.2, 7, 14.3).**
  With the foundations section 18.2 simulation precision restoration active,
  `build-runs/suite/growth-cities29-4/lua-lots-grown.log:3088` still differs in
  482 park-map tracts (max 100); landmark map is exact. The qualified variant
  at :3088 differs in 759 park and 233 landmark tracts (max 300/139).
  State-5 work is exact in both; the first work difference is state 6. Both
  follow states 1-17, all department-cursor moves and posted occupant messages;
  subsequent cycles are exact. Demand, pollution, tract and neighbours are
  now exact in both runs. No tolerance, field or case was excluded.
  Does section 14.3's separate message-server subscriber observe the same
  preview-marker presence, occupant position, lot association and effect
  properties as the original aura recipient during Lua plop/demolish calls?
  Please specify a supported boundary for capturing those inputs at the
  original recipient, and whether any cone path bypasses posted messages.
  Next diagnostic: compare original raw-park (+0x54), department and building
  effect (+0x50) grids against the followed copies before states 2/3/13;
  this will separate message-input drift from map-publication timing.
  Blocks aura in lua-lots-empty, lua-lots-grown and lua-lots-grown-qualified;
  reach29-views is the same class of case and still needs a corrected rerun.

  Follow-up (`growth-cities29-5`): lua-lots-empty and reach29-views now each
  compare 2/2 aura cycles exactly. Both grown Lua variants remain 2/3 exact;
  only their first cycles differ. The remaining block is those two grown
  scenarios; the empty Lua and reach29-views cases no longer reproduce.

  **ANSWERED 2026-10-26 (re-gc30):** DIFFERS — aura.md §14.3.1 corrects the separate-subscriber
  assumption: the message holds a live pointer and the original reads the preview gate, properties,
  position and lot at its own recipient/helper boundaries (0x0068FA60/0x0068ECF0). No immutable
  event snapshot API exists. The PostCityInit landmark rebuild calls remove/add directly, without
  occupant broadcasts or the preview gate; public transient-effect calls are also synchronous.
  Input-record fields, grid checkpoints and three worked cases are specified. The grown first-cycle
  cause still needs recipient-versus-subscriber input traces and pre-consumption raw/department/building
  grid comparisons; exact empty-Lua/reach and later grown cycles remain separate from those failures.

## From the aura glue and grid shadow (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/aura.md` and its `docs/abi` rows. Ini: `enable = aura`; shadow `aura_shadow = 1` (compared at
each cycle-completed message `0x2A09B21A`). The glue's current answer is in brackets.

<!-- from: From the aura glue and grid shadow (2026-10-14) -->
- ~~**AU1 — §11.3:** does Read replace the departments and hotspots, or merge/append? [replace]~~ **ANSWERED
  2026-10-14** from the exe (`Read` `0x0068E340`: map `operator[]` `0x00497A00` at `0x0068E45F`; list insert before
  the end `0x0068E548`–`0x0068E552`) and written into aura.md §11.3 (addresses added; §16 Q12 and test 16 already
  had it). **The bracket differs: the original merges and appends.** An existing department ID gets the new grid
  pointer (the old grid is not released), other IDs stay; hotspots are added after the ones already held. In play
  both give the same result, because the game reads into a fresh object; keep test 16's used-object case.

<!-- from: From the aura glue and grid shadow (2026-10-14) -->
- ~~**AU2 — §10:** the transient tuning is read through properties, not the raw f32 copy. Same values? [properties]~~
  **ANSWERED 2026-10-14** from the exe (copy helper `0x005FFCC0`, called at `0x0068D94E`; reads
  `0x0068D958`–`0x0068D966`) and written into aura.md §10 ("Transient tuning in detail"). **Same values for the
  shipped data** (Float32 arrays of 2). The original copies raw words and returns true even for count 0, then reads
  `v[0]`, `v[1]` unchecked: a scalar or empty array crashes (null read), one value gives a garbage radius, other
  element types are reinterpreted bit for bit. *[choice]* only a Float32 array of at least 2 values is used; anything
  else keeps {0, 0}.

<!-- from: From the aura glue and grid shadow (2026-10-14) -->
- ~~**AU3:** `GetPopulations` (lot slot 54) returns a container not in `docs/abi`; the glue calls `GetPopulation(type)`
  (slot 53) for the 12 developer types and keeps non-zero counts, so a mixed lot whose residential count is 0 reads
  −128 instead of 0. What is the container's layout (allocators.md)? [slot 53 loop]~~ **ANSWERED 2026-10-14** from
  the exe (slot 54 `0x006C21E0`; slot 53 `0x006C04A0`; walk `0x0068CE60`–`0x0068CF23`) and written into aura.md
  §6.5.1 ("The population container"), §13 and test 5. **The bracket differs in play.** Slot 54 returns a pointer to
  the lot's `std::vector<std::map<uint32_t, uint16_t>>` at lot + 0x54 (12-byte STLport maps, 0x18-byte nodes, key
  +0x10, uint16 count +0x14, allocator B; 0 or 1 maps in saves). The original walks every map and entry; a
  residential key with count 0 counts as seen, and keys outside the 12 types count in the total. Saves hold 0
  counts, so read the container through slot 54. docs/abi row in the lane report.

<!-- from: From the aura glue and grid shadow (2026-10-14) -->
- ~~**AU4:** `RefSint32` on a non-Sint32 property: a Uint32 is accepted bitwise, anything else counts as absent.
  Right? [as stated]~~ **ANSWERED 2026-10-14** from the exe (`DoOccupantChanged` `0x0068ECF0`; `RefSint32` slot
  105 body `0x0060CE80`, shared by every `RefX` slot; `GetValSint32` `0x00917721`) and written into aura.md §7 ("How
  the values are read") and test 15. **The bracket is a sound choice, with one addition.** The original checks
  neither type nor count: it uses the word at variant + 4 as an int32 array. Uint32 arrays read bit for bit (as the
  bracket); other element types give meaningless values; a scalar's value is used as a pointer (crash); a
  one-value array reads the radius past the end. *[choice]* also treat fewer than 2 values as absent.

<!-- from: From the aura glue and grid shadow (2026-10-14) -->
- ~~**AU5:** the safe lot iterator's layout is not in `docs/abi`; the glue walks cells with `GetLot(x, z, false)`,
  takes each lot once and re-checks the pointer at the next step. Layout of the iterator? [cell walk]~~ **ANSWERED
  2026-10-14** from the exe (slot 15 `0x00734850` → `0x00734860`; next `0x0045D3F0`; release `0x0045D5B0`) and
  written into aura.md §6.5 ("The safe lot iterator"). **The cell walk stands** *[choice]*: stamping is a maximum,
  so the order does not matter. Layout: an 8-byte global-heap (C) block {current node, lot list object = lot
  manager + 0x5C}, registered in the list object's iterator list (+4) so lot removal moves it; nodes {next, prev,
  `cISC4Lot*`}; the walk ends at the head node; released by a non-virtual function (unregister, free with C).
  docs/abi/allocators.md table 1 already had the block and its allocator.

<!-- from: From the aura glue and grid shadow (2026-10-14) -->
- ~~**AU6:** the game-data registry node "aura" is a stub. What does the original publish there? [stub]~~ **ANSWERED
  2026-10-14** from the exe and the shipped data, written into aura.md §4.3. **The stub is acceptable.** The
  original publishes what §4.3 says: the node `"aura"` and the commands `add_transient_effect(x, z, radius,
  magnitude)` and `end_strike(type)`. No shipped file calls them (every entry of `SimCity_1.dat`–`SimCity_5.dat` and
  `EP1.dat` searched): there are only the empty Lua placeholders and the `END_STRIKE_*` constants. Only mods would
  notice.

<!-- from: From the aura glue and grid shadow (2026-10-14) -->
- ~~**AU7 — shadow timing:** the other inputs are read at the end of the original's cycle, so a mid-cycle change shows
  as a difference; the tax baseline is taken at the first compared cycle. [as stated]~~ **ANSWERED 2026-10-14**
  (with AU13) and written into aura.md §5.3 and §14.3. **Right, and it explains AU13–AU15.** The original reads each
  input in its own state's call (§5.3). Its tax baseline is the rate read in the first state 4 after the object was
  created, so the first cycle after a load; a shadow that starts with that cycle and reads the rates in its state 4
  matches. To remove the timing differences, follow the original's state field step by step (§14.3).

<!-- from: From the aura glue and grid shadow (2026-10-14) -->
- ~~**AU8:** `GetCumulativeOffset` (ordinance slot 24): the SDK says int64, the spec and table int32 in EAX. [int32,
  declared as `GetCumulativeOffset32` in `cFC4ISC4OrdinanceSimulator`]~~ **ANSWERED 2026-10-14** from the exe
  (`0x0080DD70`, epilogue `0x0080DEA1`–`0x0080DEAC`) and written into aura.md §13 (SDK errors). **The bracket is
  right.** The sum is an int32 in EAX, the callee pops 8 bytes, and EDX is not set for the caller (a leftover), so an
  int64 declaration reads a garbage high half. docs/abi/interfaces.tsv already has pops 8 and EAX.

<!-- from: From the aura glue and grid shadow (2026-10-14) -->
- ~~**AU9:** a cycle in an empty city keeps the lot iterator open until the next state 4. Intended? [as the spec]~~
  **ANSWERED 2026-10-14** from the exe (state 4 `0x0068F325`–`0x0068F379`, iterator `0x0068F32F`–`0x0068F353`) and
  written into aura.md §6.4 (corrected). **The spec was wrong about "fresh", and nothing observable depends on it.**
  The original gets one iterator at the first state 4 of the object's life, rewinds it at every later state 4, and
  releases it only in `Shutdown`. So it stays open between all cycles, not only after an empty-city cycle. The
  glue's choice stands (with the cell walk of AU5 it needs no iterator at all).

<!-- from: From the aura glue and grid shadow (2026-10-14) -->
- ~~**AU10:** with no land value simulator the land value type is 0; the spec is silent. [0]~~ **ANSWERED 2026-10-14**
  from the exe (`LotBaseAura` `0x0068CE00`, unchecked global call at `0x0068CFB7`) and written into aura.md §6.5.1.
  **The original crashes**: after the curve test it calls the land value simulator with no null check. The shipped
  game always creates one. *[choice]* skip the value/wealth term (result = the lot average), as if the curve were
  absent. The bracket's type 0 gives `d = −wealth` and adds +3 to +5 to every residential lot with the shipped
  curve.

<!-- from: From the aura glue and grid shadow (2026-10-14) -->
- ~~**AU11 — §16 Q7:** the base-aura process table is not written. [not written]~~ **ANSWERED 2026-10-14** from the
  exe (table `0x00B0D150`; every instruction naming it: tuning `0x0068D8E3`/`0x0068D8F1`, state 4 `0x0068C002`–
  `0x0068C098`, `LotBaseAura` `0x0068CE7E`–`0x0068CEB2`) and written into aura.md §10 ("The base-aura table"), §3
  and §16 Q7 (resolved). **Not writing it is fine.** It is the aura's own storage: three entries {developer type,
  tax index, wealth, tuned base, effective base} for R$, R$$, R$$$, and no other code refers to it. State 4 rewrites
  the effective values before state 5 reads them, so keeping them in the object changes nothing.

<!-- from: From the aura glue and grid shadow (2026-10-14) -->
- ~~**AU12:** the CPU-cycle limit timer runs on `QueryPerformanceCounter` instead of the TSC. Acceptable? [QPC]~~
  **ANSWERED 2026-10-14**, written into aura.md §6.5 (time slicing). **Acceptable; no original behaviour depends on
  it.** The original reads the time-stamp counter, but the clock only decides how many lots one state-5 call stamps.
  Stamping takes the maximum, so the work grid is the same for any clock. The implementer's choice stands.

First in-game aura shadow (coordinator, `regress -Tag r7c3f`, Spandau, 98 cycles 2026-01 to 2034-02; the replace run
with aura was clean, 0 problems). The park, landmark and riot fields aside, **6 of 98 cycles match exactly**:

<!-- from: From the aura glue and grid shadow (2026-10-14) -->
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

<!-- from: From the aura glue and grid shadow (2026-10-14) -->
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

<!-- from: From the aura glue and grid shadow (2026-10-14) -->
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
