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
