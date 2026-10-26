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

- **AU-GC29-1 - first-cycle Lua occupant cones (sections 5.2, 7, 14.3).**
  With the foundations section 18.2 simulation precision restoration active,
  `build-runs/suite/growth-cities29-4/lua-lots-grown.log:3088` still differs in
  482 park-map tracts (max 100); landmark map is exact. The qualified variant
  at :3088 differs in 759 park and 233 landmark tracts (max 300/139).
  State-5 work is exact in both; the first work difference is state 6. Both
  follow states 1-17, all department-cursor moves and posted occupant messages;
  subsequent cycles are exact. Demand, pollution, tract and neighbours are
  now exact in both runs. No tolerance, field or case was excluded.
  Does section 14.3's separate message-server subscriber observe the same
  preview-marker presence, occupant position, lot association and effect
  properties as the original aura recipient during Lua plop/demolish calls?
  Please specify a supported boundary for capturing those inputs at the
  original recipient, and whether any cone path bypasses posted messages.
  Next diagnostic: compare original raw-park (+0x54), department and building
  effect (+0x50) grids against the followed copies before states 2/3/13;
  this will separate message-input drift from map-publication timing.
  Blocks aura in lua-lots-empty, lua-lots-grown and lua-lots-grown-qualified;
  reach29-views is the same class of case and still needs a corrected rerun.

  Follow-up (`growth-cities29-5`): lua-lots-empty and reach29-views now each
  compare 2/2 aura cycles exactly. Both grown Lua variants remain 2/3 exact;
  only their first cycles differ. The remaining block is those two grown
  scenarios; the empty Lua and reach29-views cases no longer reproduce.
