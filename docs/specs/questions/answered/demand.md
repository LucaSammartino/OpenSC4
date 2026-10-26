# demand: answered questions (archive)
<!-- Archive for [demand.md](../../demand.md). Lanes do not read this file. Open questions are in ../demand.md. -->

<!-- from: growth-cities29; answered by re-gc30 -->
- **DM-GC29-1 — empty tax-history window (§8.1).** The spec's current choice uses
  `maxRate = neutral * 10` when the history host reports no values. In census
  `sh28-census3/empty-small.log:3433`, the first SimIdle (2000-01-01) reports
  original tax modifiers `0.300000012` for all twelve types, versus core `0`,
  with no history in the window. The shipped curve gives 0.3 at its low end
  and 0 at ratio 1. What value does the original initialize the history maximum
  to, and does a failed/empty history query overwrite it? Please specify both
  the no-series and empty-date-window cases. Blocks the first-day demand
  comparison in `empty-small`; no tolerance or comparison was changed.

  Corrected-precision follow-up (`growth-cities29-5`): Sydenham :2577 and
  reach29-growth :2492 both report original `-3.5`, ours `0`, with all twelve
  history windows empty; lua-lots-empty :2525 reports original `0.300000012`,
  ours `0`. Each of the three empty-size scenarios also has exactly one
  differing first-day modifier sample. Modifiers changed in the original
  delivery (the shadow checks this before comparing); this is not a comparison
  of an untouched loaded modifier. Please also specify the history maximum's
  initial storage and failed-query output contract, rather than assuming the
  same empty-window maximum across loads. These six cases are blocked by this
  question; the corrected grown-Lua demand comparisons are entirely exact.

  **ANSWERED 2026-10-26 (re-gc30):** DIFFERS — demand.md §8.1.1: the maximum slot is
  uninitialized stack storage reused across the twelve types, not neutral×10. Demand ignores
  history's boolean result. Missing keys and wholly empty series return false without writes;
  non-empty series with empty windows include GetPoint endpoint values and return true.
  Controlled cases show preserved 0 → 0.300000012, preserved 512 → −3.5, retention across types,
  and endpoint extension. Initial residue cannot be derived statically per load: capture each
  original query's maximum bits before the demand caller consumes them, even on false, and
  distinguish missing/wholly empty series from an empty date window in the cited first-day runs.

## From the demand shadow differential (filed by the coordinator, 2026-10-13; runs 42/43)

About `docs/specs/demand.md`. Runs 42/43 matched demand bit-for-bit except for one month (2001-10-01). In that month
the tax modifier of 11 of the 12 types was 4 ulp off.

<!-- from: From the demand shadow differential (2026-10-13) -->
~~**DQ1 — §8.1: operation order of the tax ratio `maxRate · 0.1 / neutral`.**~~ **ANSWERED 2026-10-13** from the exe
and written into §8.1, with a bit-level worked example. `UpdateTaxAndEconomyModifiers` (`0x006A6070`, unrolled into
12 copies) computes `recip := r24(1.0f / neutral)` once, in the first copy, and stores it as float32. Every type then
computes `r24(r24(maxRate · 0.1f) · recip)`. `0.1f` is a float32 literal (`0x00A8C950`). This is the order the
implementer found empirically. It reproduces `0x3DAA8124` (2001-10-01), `0xB9794400` (I-R) and `0x3DAA8328`
(2001-09-09). Also answered on the way: the link factor is `r24(float(percent) · 0.01f)` with a float32 `0.01f`
(§4.1).

<!-- from: From the demand shadow differential (2026-10-13) -->
~~**DQ2 — §15 Q5: can the CO/I economy bonuses (`this+0xA0`, `this+0xA4`) be read or written through any interface?**~~
**ANSWERED 2026-10-13** from the exe and written into demand.md §15 Q5 and §8.1. **No.** None of the 24 slots of the
three vtables touches them, nor does any other class. The only code that does is the constructor, the zeroing in
`Init`, `Write`, `Read` and the six `FADD`s. The save record is the only way in or out, so the fields are dead in
play and 0 in every new city.

## From the demand glue: capacity maps and population summary (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/demand.md` (§6.3 capacity, `GetLocalPopulationSummary`) and the STLport map layout in
`docs/abi/allocators.md`. Switches `demand_capacity` and `demand_direct_calls` (both on by default). The glue's
current answer is in brackets.

<!-- from: From the demand glue: capacity maps and population summary (2026-10-14) -->
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

<!-- from: From the demand glue: capacity maps and population summary (2026-10-14) -->
- ~~**DQ4:** a map with count 0 but a non-empty tree shape. The spec only says "with a count of 0, the map is left
  untouched". [refused]~~ **ANSWERED 2026-10-14** from the exe (clear `0x006A5F95`–`0x006A5FB7`; the writers of
  regional city `+0x3C`: constructor `0x004AC09E` and the reader's `operator[]` at `0x004AC841`/`0x004AC8A1`) and
  written into demand.md §5. **No original behaviour; the implementer's choice stands.**
  - **Unreachable in the game.** Every writer keeps the count equal to the number of nodes.
  - **What the original would do.** It skips the clear, keeps the stale nodes and inserts into the stale tree,
    overwriting matching keys in place. The count ends as the number of new nodes only.
  - **Verdict.** That is a corrupt map, so refusing it is fine.

<!-- from: From the demand glue: capacity maps and population summary (2026-10-14) -->
- ~~**DQ5:** the census asks for the profile at every cell, so the capacity walk runs there too, although §6.3 needs
  only the purpose. Intended? [walks at every cell]~~ **ANSWERED 2026-10-14** from the exe (`PerformCensus`: BDS
  slot 39 at `0x006A5A53`, `purpose` read at `0x006A5A5D`) and written into demand.md §6.3. **Not intended: the
  original does no walk.**
  - **What the original does.** At each cell that passes the lot tests, slot 39 returns a reference into the
    profile table, and only `purpose` (+0x00) is read. Nothing is copied or walked.
  - **The glue's walk.** Walking the capacity maps per cell has no original counterpart. It is harmless while it is
    read-only.
  - *[choice]* Read `purpose` only, and walk the maps only in `UpdateCapacity` (§7.2).

<!-- from: From the demand glue: capacity maps and population summary (2026-10-14) -->
- ~~**DQ6:** the header colour of a copied capacity map is assumed to be 0. [colour 1 refused]~~ **ANSWERED
  2026-10-14** from the exe (map copy constructor `0x004C9470`, colour write `0x004C9499`; called from the vector
  copy `0x004CDF5F` under the profile copy `0x004D1230`) and written into demand.md §7.2. **Confirmed: 0.** The copy
  constructor writes colour 0 into its new header before copying the nodes, as every other map constructor does,
  and nothing changes it later. Colour 1 never occurs in the game, so refusing it is the implementer's choice, and
  that choice stands.

<!-- from: From the demand glue: capacity maps and population summary (2026-10-14) -->
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

<!-- from: From the demand glue: capacity maps and population summary (2026-10-14) -->
- ~~**DQ8:** which thread calls `GetLocalPopulationSummary`? [unspecified; the first success is logged with its thread]~~
  **ANSWERED 2026-10-14** from the exe (regional city `DoMessage` `0x004ABD40`, calls `0x004ABE76`/`0x004ABE9B`;
  senders `cSC4App::NewCity` `0x00454963`, `LoadCity` `0x004558C5`, `SaveCity` `0x00456102`) and written into
  demand.md §5. **The main thread** *[inferred: static; no thread trace]*.
  - **When it runs.** Its only caller, the regional city's `UpdateCityCache`, runs on `PostCityInit`
    (`0x26D31EC1`) and `PreSave` (`0x26C63343`) for its own city.
  - **Where those messages come from.** Only the app's new-city, load-city and save-city paths send them, and the
    game has no message dispatch thread.
  - **Settling it at run time.** The glue's log of its first call would confirm the thread.

<!-- from: From the demand glue: capacity maps and population summary (2026-10-14) -->
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

## From the x87 fix: demand's corrected return channels (filed by the coordinator from the `x87fix` lane's report, 2026-10-14)

About `docs/specs/demand.md` §4.1 and §5 after the ABI correction (slot 8 `CalculateJobsPerUnitOfDemand` returns
void and writes jobs[1..3] of a four-float record). The current answer is in brackets.

<!-- from: From the x87 fix: demand's corrected return channels (2026-10-14) -->
- ~~**DQ10 — §5:** `GetJobsBySensus` still reads `j0 + j1 + j2`, and ExtrapolateBusinessCensus reads `j[w]`,
  w = 0..2, but the outputs are now jobs[1..3]. Which elements are summed, and which one does wealth w read? [the
  three outputs, summed in order: f32(f32(out1 + out2) + out3); extrapolation reads output w for wealth w + 1]~~
  **ANSWERED 2026-10-14** from the exe (`GetJobsBySensus` `0x006A3E60`, sum `0x006A3E8C`–`0x006A3E9C`;
  ExtrapolateBusinessCensus `0x006A6BD8`–`0x006A6C83`) and written into demand.md §5 (slot 9 row and
  "`GetJobsBySensus` arithmetic") and §9.2. **The elements are right, the order is not.** The three outputs are
  summed **f32(f32(jobs[3] + jobs[2]) + jobs[1])**, last output first. Then `x := f32(supply × sum)` goes to `_ftol`
  with no `≤ 0` clamp, and the function returns EAX. The order matters for whole percents: 1, 3, 96 sum to
  0.99999994 in the original and 1.0 in the bracket's order, so supply 1000 gives 999, not 1000. The two orders
  give different sums for 1198 of the 5151 splits of 100 %. **Extrapolation:** each wealth reads its own output. $
  (`0x12010`, `0x2010`) reads jobs[1], $$ reads jobs[2] and $$$ reads jobs[3], each `f32(jobs[k] × n)`.

<!-- from: From the x87 fix: demand's corrected return channels (2026-10-14) -->
- ~~**DQ11 — §5:** in the original `GetJobsBySensus`, what is in the local record's jobs[0] when slot 8 returns, and
  is it really summed with jobs[3] left out? [not modelled; the sum of the three outputs]~~ **ANSWERED 2026-10-14**
  from the exe (`0x006A3E82` passes the address of a 16-byte local; the `FLD`/`FADD`s at `0x006A3E8C`–`0x006A3E9C`
  read its offsets `+0xC`, `+8`, `+4`) and written into demand.md §5. **The bracket matches.** jobs[0] is
  uninitialised stack that nothing writes or reads, and jobs[3] is summed. "j0 + j1 + j2" was a misreading from
  before the slot-8 correction. Nothing to model.

<!-- from: From the x87 fix: demand's corrected return channels (2026-10-14) -->
- ~~**DQ12 — §5 against §4.1:** §5 says `f32(percent) × 0.01f`, while §4.1 describes an exact FILD followed by a
  single rounding. They agree for every percent below 2^24; which is the game's? [the §4.1 link factor]~~
  **ANSWERED 2026-10-14** from the exe (slot 8 `0x006A2469`–`0x006A24CD`, the graph builder's `0x006A5514`,
  `0x006A5604`, `0x006A56F4`; the 2^32 constant at `0x00A80AA8`) and written into demand.md §4.1 and the §5 slot 8
  row. **The bracket matches; both use the same instructions.** One correction for very large percents: below 2^31 it
  is one rounding, `r24(percent × 0.01f)`. From 2^31 up, the `FADD` of 2^32 rounds the integer first, so it is
  `r24(r24(percent) × 0.01f)`. Example: 3242996796 gives `0x4BF76BC7`, not `0x4BF76BC8`. The shipped percents (at most
  1500) cannot tell any of these apart.
- Not a spec question: whether the WOW64 `CONTEXT` TagWord is the full or the abridged tag. The crash report logs
  both; the lane's host test shows the full tag (0xFFFF empty, 0x3FFF with one value) next to FXSAVE's 0x00/0x80.

<!-- from: From applying the protection, demand and pollution answers (2026-10-14) -->
- ~~**DQ13 — DQ5:** does the default `building_purpose` go through `building_profile`, and may the host's visitor be
  NULL? [yes to both]~~ **ANSWERED 2026-10-14** from the exe (`PerformCensus` `0x006A5A00`: BDS slot 39 at
  `0x006A5A53`, only `purpose` read at `0x006A5A5D`) and written into demand.md §6.3 (host shape). **Yes to both; the
  bracket matches, with one condition.** The original reads only the purpose and walks nothing, so a null visitor is
  the exact match. Routing the default `building_purpose` through `building_profile` is faithful as long as an unknown
  type yields purpose 0, as the original's inserted zero profile does.
