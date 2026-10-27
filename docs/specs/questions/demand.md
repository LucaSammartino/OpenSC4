# Demand: open questions

- **DM-GC29-2 — unordered active-demand min/max operands (2026-10-26, growth-cities29):**
  Section 3.3 defines clamp's comparison order, but not the unordered operand
  selection of its `min`/`max` expressions. With §8.1.1's captured history-output
  bits, `growth-cities29-13/london-sydenham.log:2629` has original active demand
  `-nan` in all twelve total nodes while ours is -6000; raw demand/supply and
  neutral tax rate are exact. The run is clean, ref0 and reaches its end date;
  31 of 103 demand samples differ. The current core's final
  `fmax(activeMin, fmin(target, capLimit))` selects activeMin for NaN target.
  Please specify operand/comparison order on unordered values for both bonus
  and cap min/max, including whether a quiet NaN's payload/sign is preserved.
  This blocks Sydenham's new NaN case; the twelve false history outputs are
  retained, not replaced with a default. Evidence root:
  `C:/Users/tlori/FreeCity4/build-runs/suite/`. No check or tolerance was weakened.

- **DM-GC30-1 — active-demand arithmetic under live PC53 (2026-10-27, growth-cities30):**
  Please supply value-only PC24/PC53 conformance cases for §3.3 under foundations
  §18.4: exact input bits (including history results and control word), expected
  bonus/cap intermediate values and final active-demand bits for each node.
  State the numeric rounding rule needed to reproduce those values; no native
  instructions, registers or implementation excerpts are requested. This is
  separate from DM-GC29-2's unordered operands.
  Clean/end/ref0 `build-runs/suite/growth-cities30-3/lua-lots-grown.log:3755`
  first differs at 2197-03-21 in six active values, with raw demand, supply and
  neutral rate exact: original/core 3328.56226/3328.56201, 60.5309258/60.5308838,
  32.7028008/32.7027588, -741.17218/-741.172241, -176.640945/-176.640991,
  428.150299/428.150391. Demand is 58/65 samples exact.
  Tick 67 (log:3747) enters CW027F before this delivery; tick 69 (:3792) returns
  to CW007F afterwards.
  Please provide a worked input/output case at PC24 and PC53; no formula,
  tolerance or capture was changed.
  Evidence root: `C:/Users/tlori/FreeCity4/`. This item stops pending that contract.
