# pollution: answered questions (archive)
<!-- Archive for [pollution.md](../../pollution.md). Lanes do not read this file. Open questions are in ../pollution.md. -->

## From the pollution glue and grid shadow (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/pollution.md` and its `docs/abi` rows. The glue's current answer is in brackets.

<!-- from: From the pollution glue and grid shadow (2026-10-14) -->
- ~~**PG1 — slot 55 `GetRecyclingCenterCurrentCapacity`:** the spec says a bool in AL; `docs/abi` says ST0 (a float).
  Which? [bool; the slot is waived in abi_conformance_test]~~ **ANSWERED 2026-10-14** from the exe (`0x006E3720`)
  and written into the pollution.md slot table. **The bracket is right: bool in AL** (slot 53's result, untouched).
  The percentage is stored through the reference, and the x87 stack is empty on return. `docs/abi`'s ST0 is wrong;
  the abi rows are the coordinator's to fix.

<!-- from: From the pollution glue and grid shadow (2026-10-14) -->
- ~~**PG2 — slot 51 `GarbagePutInLandfill`:** the spec says it returns the new int64 total; `docs/abi` says eax or
  void. [int64; waived]~~ **ANSWERED 2026-10-14** from the exe (`0x006E3250`–`0x006E32CE`) and written into the
  pollution.md slot table. Both descriptions are loose. The original leaves EAX = the new high dword of
  `landfilled` and EDX = the sign of n, which is meaningless as an int64. Its known caller (landfill §4.7.2) ignores
  it. **The bracket is fine:** declare it int64 in EDX:EAX with pop 4 (SDK), since any value conforms. FreeCity4
  returns the new total.

<!-- from: From the pollution glue and grid shadow (2026-10-14) -->
- ~~**PG3 — slot 54:** the spec says AL; the table says x87 or eax. Is it a float? [as the spec]~~ **ANSWERED
  2026-10-14** from the exe (`0x006E2D70`; getter `0x005FDAC0`) and written into the pollution.md slot table.
  **The bracket is right: AL, a bool.** The fraction goes through the reference and the x87 stack is empty on
  return. Also added to the row: the property is read through the shared percentage getter, so it is × 0.01f
  (float32), like slot 55.

<!-- from: From the pollution glue and grid shadow (2026-10-14) -->
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

<!-- from: From the pollution glue and grid shadow (2026-10-14) -->
- ~~**PG5 — §10.2 Q1:** is the prop requester re-added after a load? [open in the spec]~~ **ANSWERED 2026-10-14**
  from the exe (prop developer `Write` `0x006F1970`–`0x006F19B6`, `Read` `0x006F3680`–`0x006F36C8`;
  `AddPropRequester` `0x006F21C0`) and written into pollution.md §4.1 and §10.2 Q1. **No; the prop developer's own
  save restores it.**
  - **How it is restored.** The prop developer writes its requester vector by ref and reads it back with IID
    `0x89C498BE`. A loaded city therefore still gets garbage props.
  - **Do not re-add it.** `AddPropRequester` has no duplicate check, so adding it again would register it twice.
  - **For a replacement.** It must answer `0x89C498BE` and be saved by ref.

<!-- from: From the pollution glue and grid shadow (2026-10-14) -->
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

<!-- from: From the pollution glue and grid shadow (2026-10-14) -->
- ~~**PG7:** `radiationSum` is not saved, so the radiation history at the end of a cycle after a load diverges from a
  run without the load. Intended? [as the spec]~~ **ANSWERED 2026-10-14** from the exe (`Write` skips the field
  between `developedTracts` and `avgGarbage`; `Read` never touches it; ctor `0x006E206A` sets it to 0; new-city
  zeroing `0x006E7288`; end-of-cycle zeroing `0x006E7A34`) and written into pollution.md §6 with a worked example.
  **The bracket matches; reproduce it as a *[quirk]*.** After a load the sum restarts at 0. Only the first end of
  cycle after the load differs, by one entry each in `0xEA110137` and `0xEA110138`. Whether it was intended
  cannot be read from the exe.

<!-- from: From the pollution glue and grid shadow (2026-10-14) -->
- ~~**PG8 — §4.6.1 step 3:** is the unsigned `wteRemaining` converted to float as signed or unsigned? [as written]~~
  **ANSWERED 2026-10-14** from the exe (waste-to-energy `0x006E38EA`–`0x006E3903`, incinerators
  `0x006E39A5`–`0x006E39BE`; constant `4294967296.0f` at `0x00A80AA8`) and written into pollution.md §4.6.1 with a
  table. **Unsigned, so the bracket matches, but it is not rounded to `float32`.**
  - **How it converts.** The value is loaded as signed, and 2³² is added when its top bit is set. It is then
    compared on the x87 stack with the `float32` `g`.
  - **Below 2³¹** the comparison is exact. For example, 2²⁴ + 1 > 16,777,216.0 is true.
  - **From 2³¹ up** the add rounds the value to `float32` under the 24-bit precision.
  - **In practice.** Both differ from a plain `f32` conversion only above 2²⁴. Steps 3 and 4 behave the same.

<!-- from: From the pollution glue and grid shadow (2026-10-14) -->
- ~~**PG9:** can landfill usage change within a day? [the shadow keeps the first value it reads]~~ **ANSWERED
  2026-10-14** from the exe (`GetLandfillUsage` `0x006B9820` reads cached totals; they are written only by the ctor
  `0x006B9780` and `UpdateCapacityValues` `0x006B8F20`; its callers are `0x006B9302` month, `0x006B9CE2` zoning,
  `0x006B9F2B` PostCityInit) and written into pollution.md §4.6.1. **Not within one day pass, so the bracket
  matches if the value is read again each day.** None of those callers runs inside pollution's day pass, which
  finishes in one delivery. The values can change between day passes: after zoning, after the landfill month
  agent, or after a load.

<!-- from: From the pollution glue and grid shadow (2026-10-14) -->
- ~~**PG10:** without an ordinance simulator, `cumulative_factor` returns 1.0. [choice]~~ **ANSWERED 2026-10-14**
  from the exe (52 unchecked calls through the held pointer, `0x006E83DD`–`0x006E8479`) and written into
  pollution.md §4.4.3. **The original has no such path: a null pointer crashes on the first read, and the city
  always creates the simulator.** The *[choice]* of 1.0 stands. It equals `GetCumulativeFactor` with no enacted
  ordinance carrying the property (ordinance.md §6.1).

<!-- from: From the pollution glue and grid shadow (2026-10-14) -->
- ~~**PG11 — slots 29/30:** they return int32 (the applied delta and the argument respectively) where the SDK has
  bool. Confirm. [int32]~~ **ANSWERED 2026-10-14** from the exe (slot 29 `0x006E32E0`, slot 30 `0x006E5050`; every
  call site checked) and written into pollution.md §8.1 with a table. **Confirmed; the bracket matches.**
  - **Neither is a `bool`.** EAX holds the applied delta (taken before the final clamp at 0) and the argument
    respectively. AL is just the low byte, so `AddToGarbageSupply(256)` gives AL 0.
  - **No caller reads either value.** That covers the landfill developer, the landfill zones and the simulator
    itself.

<!-- from: From the pollution glue and grid shadow (2026-10-14) -->
- ~~**PG12 — §5.2 posts:** no void or IGZUnknown is set on them, since the spec names none. Confirm. [none set]~~
  **ANSWERED 2026-10-14** from the exe (post sites `0x006E70EA`, `0x006E7A64`–`0x006E7AB2`, `0x006E7B3D`; helper
  `0x005F98C0` → `0x005F9800`; message ctor `0x009134D6`) and written into pollution.md §5.2. **Confirmed; the
  bracket matches.** The helper sets the IGZUnknown explicitly to the argument pollution passes, which is always
  null. void1 is never set and stays null from the constructor, which zeroes every field. The messages are queued
  with `MessagePost(msg, false)`.

<!-- from: From the pollution glue and grid shadow (2026-10-14) -->
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

<!-- from: From the pollution glue and grid shadow (2026-10-14) -->
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

<!-- from: From applying the protection, demand and pollution answers (2026-10-14) -->
- ~~**PG15 — PG11, slot 29:** with a negative supply and n > 0, is `0x7FFFFFFF − s` computed in int32 (wrapping) or
  exactly? [exactly, in 64 bits, so the change equals n]~~ **ANSWERED 2026-10-14** from the exe (slot 29 `0x006E32E0`,
  test at `0x006E32E9`–`0x006E32FB`) and written into pollution.md §8.1 with a worked table. **Exactly; the bracket
  matches.** The original adds first in int32 and takes `0x7FFFFFFF − s` only when that sum wrapped, which needs s >
  0, where the subtraction is exact. With a negative s and n > 0 nothing wraps and the delta is n. A damaged save with
  a negative s and n < 0 can wrap the other way (noted as a quirk).

<!-- from: From applying the residential, protection and save-reference answers (2026-10-14) -->
- ~~**PG16:** PG15: should the n<0 wrap quirk on a damaged save be reproduced? [no, clamped: delta −s, supply 0]~~
  **ANSWERED 2026-10-14** from the exe (`0x006E3301`–`0x006E332F`) and written into pollution.md §8.1 with a table.
  The original wraps only when a damaged save holds a supply within `|n|` of `−2^31`. It then stores the wrapped
  positive sum. Every other negative supply goes to 0, the same as the clamp. Only a damaged save reaches this, so
  the implementer's choice stands. **The bracket is acceptable**; reproducing the wrap is what a conformance test
  against the original would expect.
