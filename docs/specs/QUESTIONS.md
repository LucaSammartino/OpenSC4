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
