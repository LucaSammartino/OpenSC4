# residential: answered questions (archive)
<!-- Archive for [residential.md](../../residential.md). Lanes do not read this file. Open questions are in ../residential.md. -->

## From the residential core (filed by the coordinator from the lane's report, 2026-10-13; branch `residential-core`, tip `cc6e25d`, NOT merged)

All were about `docs/specs/residential.md`. The lane's code compiles clean and its own geometry/tuning tests pass, but
`simulator.rs` has zero tests of its own and no conformance vectors exist, so these questions are unpinned choices in
unexercised code. Preserved here from the lane's branch by hand — the branch's own edit to this file was written
against the older `(No open questions.)` version and cannot be merged as-is.

<!-- from: From the residential core (2026-10-13) -->
~~**RS1 — is the `0.01` in the percentage property getters the float or the double constant? (load-bearing)**~~
**ANSWERED 2026-10-13** from the exe: **float32 `0.01f`** (`0x00A823A4`) at every site, in the getter `0x005FDAC0`,
§5.6 step 4, slots 30/31, the §7.2 tuning and the save reader. Protection was right. Residential §15 test 8 was
wrong (EQ 50 → 39.999996, HQ 60 → 47.999996) and is corrected; test 10 holds, but only because each product is
rounded to float32 before the truncation. Written into residential.md §5.6, §7.1, §15, REH §4 and foundations §18.3.
Kept struck through so RS2–RS6 keep their numbers.

<!-- from: From the residential core (2026-10-13) -->
~~**RS2 — §5.5 step 5: is the `GetSimDate` month read per tract, or latched at pass start?**~~ **ANSWERED
2026-10-13** from the exe and written into residential.md §5.5 step 5: **per tract.** `DoPopulationChange` calls
`GetSimDate` itself (`0x006FAA95`) for every tract with total ≠ 0. Nothing is latched, so the core's literal reading
is right.

<!-- from: From the residential core (2026-10-13) -->
~~**RS3 — §5.5 step 1: the pollution average's accumulator width.**~~ **ANSWERED 2026-10-13** from the exe and
written into residential.md §5.5 step 1 and REH §4.7. The accumulator is a **32-bit unsigned dword that wraps**:
zero-extended 16-bit adds, then an unsigned `div` by the 32-bit product S·S (helper `0x006F7FF0`). The core's
wrapping u32 is right.

<!-- from: From the residential core (2026-10-13) -->
~~**RS4 — §5.6 step 4 / REH §4.8 `GetGlobalEQ`/`GetGlobalHQ`: what is the per-tract traversal order?**~~
**ANSWERED 2026-10-13** from the exe and written into REH §4.8, with a pointer from residential.md §5.7. The order is
**x outer, z inner**, both ascending from 0, over the R$ EQ (HQ) grid's tract counts. Every value is read through
`GetTractValue` (`0x006F7720`/`0x006F7870`), and no raw memory is read. This confirms the core.

<!-- from: From the residential core (2026-10-13) -->
~~**RS5 — §5.3 step 2: does the distance sweep's running `d` widen past a byte?**~~ **ANSWERED 2026-10-13** from the
exe and written into residential.md §5.3 step 2, with a worked example. **Yes.** `d` is a 32-bit register: it starts
at 255, `inc` takes it to 256 without wrapping, and the min uses a signed 32-bit compare (`0x005E4E80`). After the min
it is ≤ 255, so storing its low byte never truncates. The core's i32 plus clamp is equivalent; the clamp never acts.

<!-- from: From the residential core (2026-10-13) -->
~~**RS6 — §6.2 `GetAverageEQGrid`/`GetAverageHQGrid`: are `min`/`max` updated for unpopulated tracts?**~~
**ANSWERED 2026-10-13** from the exe and written into residential.md §6.2, with a worked example. **Yes, and REH §4.8 is
normative.** In `AverageGrids` (`0x006F70E0`) an unpopulated tract stores 0.0 and goes through the same strict min and
max compares, so an empty city returns `min = max = 0.0`.

## From the residential tests and record (filed by the coordinator from the lane's report, 2026-10-14; merged with the core)

About `docs/specs/residential.md` and `residential-education-health.md` (REH). All 24 §15 tests pass. REH tests
1–7, 10 and 18–21 belong to the host's education/health hooks. The core's current answer is in brackets.

<!-- from: From the residential tests and record (2026-10-14) -->
- ~~**RS7 — §15 test 8 vs the shipped LE curve (inconsistency):** the shipped Health Quotient → Life Expectancy curve
  gives 65 at HQ 60, not the 61 the test states. The vector uses a flat custom curve to get round it. Which is
  right: the test's number, or the curve the test should use? [custom curve in the vector]~~ **ANSWERED
  2026-10-14** from the shipped exemplar ("Residential simulator", `0x6941A67F` = (0,50) (20,55) (40,60) (60,65)
  (200,100)) and foundations §4.3. Test 8 is rewritten in residential.md §15. **The curve is right and the test's
  61 was wrong:** at GetCellHQ = 60 the curve is exact at its point, so LE = 65. Since 70 > 65 the reset still
  happens, with the same EQ/HQ results; only leSum changes (65 × 200). The test now uses the shipped curve, with
  EQ/HQ stored as 50.2 / 60.2, which decay to exactly 50 / 60. The custom curve can go. For LE 61, use HQ 44
  (exact).

<!-- from: From the residential tests and record (2026-10-14) -->
- ~~**RS8:** does the "other" list move into the health-affecting list? The spec does not say so. [moved; inferred]~~
  **ANSWERED 2026-10-14** from the exe (state 0 `0x006FF75F`–`0x006FF7E8`; `DoBuildingChanged` `0x006FB5C1`–
  `0x006FB9C1`) and written into the new residential.md §5.8. **The bracket is right: moved.** State 0 appends
  every pending entry, in order and unfiltered, to the end of the health-affecting list, moving the reference,
  and then clears the pending list. Also specified: the add test (both properties > 0), and a removal that tries
  the health-affecting list first and then the pending one.

<!-- from: From the residential tests and record (2026-10-14) -->
- ~~**RS9 — record Read:** what should Read repair after a stream error, and what happens with a version it cannot
  read? [nothing repaired; Read fails]~~ **ANSWERED 2026-10-14** from the exe (`Read` `0x007009A0`, helpers
  `0x005D9730`, `0x005DB3C0`, `0x006F7BA0`) and written into residential.md §9.2 and test 20. The original rejects
  no version (≥ 7 is read as 7, 0–2 as 2), never checks the stream and returns true. After an error, later refs
  are null and the state, list counts and department scalars are stale stack values. Init repairs only the grids,
  the departments and the department scalars.

  **Choice:** return false and leave the simulator **as constructed**, rather than partly read, so that Init
  starts a new city. This is the crime.md §12 rule. Accept every version.

<!-- from: From the residential tests and record (2026-10-14) -->
- ~~**RS10 — record Read:** are list and department entries that a Read replaces released? [released, departments
  with `release_department`]~~ **ANSWERED 2026-10-14** from the exe and written into residential.md §9.2. The
  original releases **nothing**: list clears free nodes only (`0x00413E60`), and ref reads null the destination
  without `Release` (`0x005D9742`). This cannot be observed, because Read only ever runs on a freshly constructed
  object, where every ref is null and every list empty. The core's releasing is fine.

<!-- from: From the residential tests and record (2026-10-14) -->
- ~~**RS11:** how do coverage and list vectors with counts other than 7 / 3 behave? [none]~~ **ANSWERED 2026-10-14**
  from the exe and written into residential.md §9.2. They arise only from a hand-made or damaged record, since
  the game writes 7 and 3. The original accepts them and never checks a bound: the monthly states loop over the
  building-list vector's size, and Init loops over the coverage vector's size. Both index fixed 7- and 3-entry
  tables (`0x00AB2B60`/`7C`/`98`/`A4`) and each other's vectors. A larger count reads the next table, and a
  smaller one drops and leaks pending buildings. **Choice:** treat such a count as a damaged record (RS9 rule).

<!-- from: From the residential tests and record (2026-10-14) -->
- ~~**RS12:** can the lists hold null entries, and what does each pass do with one? [skipped]~~ **ANSWERED
  2026-10-14** from the exe and written into residential.md §9.2. Only `Read` can produce them, through an
  unresolvable reference or a stream error; the game never writes them. The original does **not** skip them:
  distribution (`0x006FF622`), coverage (`0x006F8710`), grading (`0x006FFE61`) and `GetBuildings` (`0x006FAD2C`)
  all dereference the entry and crash. **Choice:** drop nulls in Read, so passes never see one; skipping them in
  each pass is equivalent.

<!-- from: From the residential tests and record (2026-10-14) -->
- ~~**RS13:** do the state-0 grade totals live in the residential simulator or in the host (REH)? [host]~~
  **ANSWERED 2026-10-14** from the exe and written into REH §3 (table), REH open question 3 and residential.md
  §5.1. **The bracket is right: host (REH).** In the original they are two floats of the one object (`+0x314`,
  `+0x318`). They are zeroed at the start of state 0 (`0x006FF567`) and accumulated by grading in states 6 and 8
  (`0x006FFED3`, `0x00700053`). Nothing else in the class touches them, no slot returns them and they are not
  saved. They are therefore write-only, and the host may keep them or drop them without any observable change.

<!-- from: From the residential tests and record (2026-10-14) -->
- ~~**RS14:** what does the state machine do in an unknown state? [waits]~~ **ANSWERED 2026-10-14** from the exe
  (`DoDailySimulation` `0x006FF546` and its jump table; `DoMessage` `0x00700397`) and written into residential.md
  §5.1. **The bracket is right, with a caveat: it waits forever.** A state above 15 does what state 10 does
  (nothing; returns true), but the month message resets only state 10, so the simulator never resumes.
  **Choice:** keep it, but let Read reject a saved state above 15 as damaged (RS9 rule).

<!-- from: From the residential tests and record (2026-10-14) -->
- ~~**RS15:** is the pollution loop's S capped (a hostile S is unbounded)? [no cap]~~ **ANSWERED 2026-10-14** from
  the exe (`0x006F7FF0`) and written into residential.md §5.5 step 1. **The bracket is right: no cap.** S² calls
  are made per tract and type. A wrapped `S·x + S` empties the loop. For S = 0 or S ≡ 0 (mod 65536), `S·S` is 0
  and the `div` at `0x006F8062` faults. **Choice:** Read rejects P = 0 or P > 256 as a damaged record.

<!-- from: From the residential tests and record (2026-10-14) -->
- ~~**RS16:** what does a zero divisor give in the averages? [0]~~ **ANSWERED 2026-10-14** from the exe
  (`PostPopulationChange` `0x006F8070`, `DoDailySimulation` state 9 `0x007000B5`, `DoPopulationChange`
  `0x006FAAC7`, `GetTractQuotient` `0x006F6FC0`, `GetGlobalEQ`/`HQ` `0x006F7720`/`0x006F7870`) and written into
  residential.md §5.6 (new "Zero divisors" table) and §5.5 step 6. **The bracket is right: 0 everywhere, and none
  of them faults.** One correction: the life-expectancy guard is `population ≠ 0`, not `> 0`. A negative population
  (damaged record only) divides as unsigned 64-bit by 2^64 − |population|, which also gives 0.

<!-- from: From the residential tests and record (2026-10-14) -->
- ~~**RS17:** does a version-3 record keep the running population the constructor set? [yes]~~ **ANSWERED
  2026-10-14** from the exe (`Read` `0x00700D5D`–`0x00700D63`; constructor `0x00700510`, store at `0x007005BF`)
  and written into residential.md §9.2, §9.3 and open question 4. **The bracket is right: yes, and the value is
  0.** A version-3 record skips the running population and changes nothing else (versions 0–2 zero it and move
  state 4 to 11). Because `Init` then restarts the pass with leSum 0, a version-3 save made in state 4 does **not**
  get the double count that §9.3 describes for version ≥ 4 (worked example there).
