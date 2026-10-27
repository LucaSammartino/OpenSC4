# Pollution: open questions

- **PL-GC30-1 — recycling accumulator under live PC53 (2026-10-27, growth-cities30):**
  Pollution §4.6's cell recycling step says `rec := min(gp × recyclingEffect, gp)`
  and `recycled += rec`; §2 gives the monthly counter float64 storage. Please
  specify the division/product/subtraction instruction order and explicit float32
  stores for gp/rec, and the accumulator's arithmetic/store boundary under
  foundations §18.4's live PC53 context. Pollution is absent from its table of
  verified spill boundaries. Is the double accumulator rounded only to live PC,
  and which recycling operands retain register precision before that addition?
  Clean/end/ref0 `build-runs/suite/growth-cities30-3/lua-lots-grown.log:3829`
  first differs at 2197-03-21: month.recycled original/core
  5129.284889832139/5129.23388671875. Pollution is 5/12 samples exact; the first
  mismatch names only that counter. Tick 71 (log:3826) enters CW027F immediately
  before that sample. The core explicitly rounds each accumulated
  sum to float32 before its float64 store; no replacement result was copied.
  Please supply worked PC24/PC53 inputs and outputs, including each explicit spill.
  Evidence root: `C:/Users/tlori/FreeCity4/`. This item stops pending that contract;
  no comparison or tolerance was relaxed.
