# aura: open questions
<!-- Questions about [aura.md](../aura.md). Append new entries at the END; when one is answered, move it to answered/aura.md. Rules: README.md. -->

<!-- from: verify-regress4 (2026-10-21) -->
- **AU16 - section 14.3 (with sections 5.3 and 6.3):** section 14.3 names state 5 as the only state spread over
  several calls, but state 3 takes one department per call (section 6.3), so with N departments the original reads
  department k's grid at its k-th call. A shadow that runs all of state 3 at the 3 -> 4 move reads later grids: in
  run regress-train19 (cycle 2, 2026-03-11, 11 days) a landfill tile pair inserted between the original's state-3
  calls gave the 208-tract / max 188 landmark-map difference of section 5.3's worked example (orig -91 / ours -96 at
  tract 3,6). Should section 14.3 also say: when the original stays in state 3 but its departmentCursor (+0x7C)
  moves, run one state-3 call of ours? [yes: verify-regress4 does this (the core's 3 -> 3 step); please confirm the
  cursor offset and that a delivery which leaves state and cursor unchanged never consumes a department]

- **AU-GC29-2 — original state-5 lot inputs across timed slices (2026-10-26, growth-cities29):**
  After implementing §14.3.1's native cone-helper/service capture, the permanent park
  and landmark maps are exact in `growth-cities29-12/lua-lots-grown.log:3093`, but
  93 work cells first differ after state 5; eight base/aura tracts then differ
  (maximum 225/97). The run is clean, ref0 and reaches its end date. Please specify
  a native per-lot capture boundary and exact call/argument layouts for the lot
  enumeration and the state-5 population/wealth/building inputs, including skipped
  or removed lots, so the shadow can replay the original read sequence independently
  of its own timed scan. Which live lot/grid results must be retained before the
  original advances its cursor? This blocks the grown unqualified Lua work/base case,
  not the permanent cone maps; both grown variants were exact in tag 11, and the
  qualified variant is exact in tag 12. Evidence root:
  `C:/Users/tlori/FreeCity4/build-runs/suite/`. No output grid is used to hide a difference.
