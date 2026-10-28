# Pollution: open questions

- **PL-SE33-1 — used-capacity property variant types (2026-10-28, services-exact33):**
  Section 4.9.3 specifies unsigned arithmetic but not the variant types stored by
  daily usage distribution, monthly rollover, or insertion. Please specify each
  write's type (including the zero writes and WTE output). In pollution-only
  qcheck ui29-bevopolis, class A9BD882D object 1190 differs only at payload byte
  140: property 88ECF76C has disk tag 00000007 (sint32) in the reference and
  00000003 (uint32) in ours; both values are 000034CB. Paradis-stohess object
  3015 has the same tag difference, equal value 0000B0E0. No delivery or DIGEST
  differs. Evidence: main build-runs/golden-runs/compare-20261028-024202-0883f0b8/result
  and compare-20261028-024202-0eebc2d4/result. The current core uses uint32 for
  these writes. Please provide a value/type contract, without implementation excerpts.
- **PL-SE33-2 — maintenance property on landfill occupants (2026-10-28, services-exact33):**
  Section 4.2.4 says a budget item on any line causes a signed int64 2A58D350
  write on insert and remove. Pollution-only paradis-stohess has two landfill
  occupants (class 8A22BEBE, IDs 15592 and 17792) with empty property holders
  in the reference, but ours adds 2A58D350 values 30 and 20 respectively.
  Does the original filter occupant classes, budget-item lines, or insertion
  notifications before that write, or remove the property later? Please specify
  the gate and ordering. The same run has no delivery/DIGEST difference; its
  other changed record is 8A2482B9/4A2482BB/4 (plus PL-SE33-1's building tag).
  Evidence: main build-runs/golden-runs/compare-20261028-024202-0eebc2d4/result.

- **PL-GC30-1 — recycling accumulator under live PC53 (2026-10-27, growth-cities30):**
  Pollution §4.6's cell recycling step says `rec := min(gp × recyclingEffect, gp)`
  and `recycled += rec`; §2 gives the monthly counter float64 storage. Please
  supply value-only PC24/PC53 cases under foundations §18.4: exact initial gp,
  recyclingEffect and accumulator bits, expected rec and remaining-garbage bits,
  and the final float64 accumulator bits after an ordered sequence of cells.
  State the numeric rounding rule that reproduces those values. No native
  instructions, registers or implementation excerpts are requested.
  Clean/end/ref0 `build-runs/suite/growth-cities30-3/lua-lots-grown.log:3829`
  first differs at 2197-03-21: month.recycled original/core
  5129.284889832139/5129.23388671875. Pollution is 5/12 samples exact; the first
  mismatch names only that counter. Tick 71 (log:3826) enters CW027F immediately
  before that sample. The core explicitly rounds each accumulated
  sum to float32 before its float64 store; no replacement result was copied.
  Please supply worked PC24/PC53 inputs and outputs, including intermediate values.
  Evidence root: `C:/Users/tlori/FreeCity4/`. This item stops pending that contract;
  no comparison or tolerance was relaxed.
