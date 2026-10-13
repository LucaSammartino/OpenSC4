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

**ZQ2 — §7.4 `CreateParcellizableRegion`, the parenthetical "(real mode or the cell is occupied per §8.4)".**
Code 7 of §8.4 is only reachable if codes 0/1/2/3 all pass first, so a cell holding a lot that is *also* unzoned, or under
a network, returns 2 or 3 — not 7. "Occupied per §8.4" is therefore not a clean predicate. The lane implemented it
literally as `can_create_lot(x, z, 1, 1, false, false) == 7`. No test exercises the highlight-mode branch of §7.4 at all
(see ZQ4).

**ZQ3 — §7.2.1 `DetermineLotSize` has no conformance test in §15.**
Tests 19–29 all state "the harness bypasses `DetermineLotSize`", and no other test covers it. It is the most
arithmetic-heavy function in the spec — the x87 chain
`ftol(r24(r24(float(k)/float(sx·sz)) × float((min+max)/2)))`, the `hash_set` iteration order, the developer tie-break on
`EstimateDesirability` then demand, and the stage walk. The lane implemented it from the prose but **cannot verify any of
it**. A test 30 (a scripted BDS + configurations + developer, expected width/depth/minDepth/maxDepth/block) would close
what the lane calls the largest correctness risk in its work.

**ZQ4 — no vector covers highlight mode.**
`HighlightParcels` (§7.1), virtual lots, virtually deleted cells, and the §7.13 draft fallbacks are all untested. The
draft fallbacks need the network tools' `OccupantExists(x, z, flag, &mask)`, which §8.8/§7.13 describe only for highlight
mode. Partly a headless limit (the highlight is a UI preview) and partly a missing spec test.

**ZQ5 — §7.3 `ExamineBoundaries` city-border behaviour is untested.**
Quirk 5 ("examines the region's own edge cells at the city border") and the rule "at the edge the 'outside' coordinate
stays on R's own edge" have no vector: test 13's region is at x 10..12, z 10..12, well inside a 64×64 city. The
implementation follows the prose but nothing pins it.

**ZQ6 — §7.5 step 1 table, three of six cells unpinned.**
`zn15_30`/`zn15_31` pin the x-line `_` arm (Q=3) and the `0|2` arm in its `D = 3` branch, and the z-line `_` and `2`
arms. Not pinned: the x-line `1` arm, the `D != 3` branch of the x-line `0|2` arm, and the z-line `0` and `1` arms.
Documented in `engine/conformance/README.md` rather than left implicit.

## From the residential core (filed by the coordinator from the lane's report, 2026-10-13; branch `residential-core`, tip `cc6e25d`, NOT merged)

All were about `docs/specs/residential.md`. The lane's code compiles clean and its own geometry/tuning tests pass, but
`simulator.rs` has zero tests of its own and no conformance vectors exist, so these questions are unpinned choices in
unexercised code. Preserved here from the lane's branch by hand — the branch's own edit to this file was written
against the older `(No open questions.)` version and cannot be merged as-is.

**RS1 — is the `0.01` in the percentage property getters the float or the double constant? (load-bearing)**
The `SC4Percentage` / "% × 0.01" readers, the `× 0.01` of §5.6 step 4, and §6.4 slots 30/31. Residential's own worked
examples require **double**: §15 test 8 needs `pct(80.0)` to be exactly `0.8f32` (`0x3F4CCCCD`) for EQ 50 → 40 and
HQ 60 → 48 — the float constant gives `0.79999995` (`0x3F4CCCCC`), so `50 × 0.79999995` = `39.999996` — and §15 test
10 needs `1200 × 0.01` to truncate to 12, not 11 (float gives `11.999999`). But
`engine/core/src/protection/algo.rs::pct` multiplies by the **float** `0.01`, and protection's vectors pass with it.
The lane used double `0.01` throughout `engine/core/src/residential/`. Answerable from the getter's bytes
(`0x005FDAC0` and friends): is the constant a `float` or a `double` literal, and does one of the two subsystems have a
wrong worked example or a compensating one-ulp difference? This is a ±1 ulp on every percentage in **both** cores, so
it is worth settling once rather than per subsystem.

**RS2 — §5.5 step 5: is the `GetSimDate` month read per tract, or latched at pass start?**
The tract is aged only when `month = 12`. The core reads it per tract (the literal reading). The two differ only if a
month boundary falls inside one state-4 pass — impossible in the original (one state per day) but reachable if a host
runs a whole pass in one call across a date change.

**RS3 — §5.5 step 1: the pollution average's accumulator width.**
REH §4.7 says "u32 division". The core sums the `S × S` `GetPollutionValue` results (each 16 bits) in a wrapping
`u32`. With `S = 4` the maximum is 1,048,560, so no wrap; a large `S` would. Confirm the original's accumulator is
32-bit and wraps rather than being wider.

**RS4 — §5.6 step 4 / REH §4.8 `GetGlobalEQ`/`GetGlobalHQ`: what is the per-tract traversal order?**
Float addition is not associative, so the accumulated `sum` depends on it. The core walks the grid's tracts x outer,
z inner (the same order as `ProcessTwoGridsWithCallback`, simgrid §7.3). Confirm, or state that the original's order
is the raw-data order of the grid (the same thing for a contiguous row-major grid).

**RS5 — §5.3 step 2: does the distance sweep's running `d` widen past a byte?**
`d` starts at 255 and computes `d := min(d + 1, v)`. If `d` is a byte accumulator, `255 + 1` wraps to 0 and the whole
field collapses to 0 on the first sweep. The core widens `d` to `i32` and clamps the stored value to `0..=255`, which
reproduces the Manhattan distance capped at 255 that §5.3 states and that §15 tests 1–3 require. Confirm the original
widens too (or that its `min` is evaluated in a wider type) — the two differ for every grid with no seeded tract.

**RS6 — §6.2 `GetAverageEQGrid`/`GetAverageHQGrid`: are `min`/`max` updated for unpopulated tracts?**
REH §4.8 says yes ("Each is updated with every tract, populated or not") and the core follows it. With an empty city
that gives `min = max = 0.0` rather than the initial `255.0` / `0.0`, which a caller may observe. residential.md §6.2
only says "track the stored values", so confirm REH is normative here.
