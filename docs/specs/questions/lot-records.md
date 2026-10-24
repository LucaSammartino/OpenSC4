# lot-records: open questions
<!-- Questions about [lot-records.md](../lot-records.md). Append new entries at the END; when one is answered, move it to answered/lot-records.md. Rules: README.md. -->

<!-- from: core-lotrecords-final4 (2026-10-22) -->
- **LR6 - section 7.1 GetLotWallEdgeFlags step 4:** when lot manager slot 11 `GetVerticesAtEdgeOfLot(R, 1 << i, V)`
  returns false (its return is not tested), which vertex range does the loop walk: what V holds before the call
  (initialised by the caller, or left from the previous edge), or what slot 11 writes on its false path?
  [the host always fills V and the loop walks whatever it filled]

<!-- from: core-lotrecords-final4 (2026-10-22) -->
- **LR7 - section 7.2 step 3:** when occupant manager slot 33 returns true with `w` null, the update path calls
  `QueryInterface` through null. Is that reachable (does slot 33 ever return true without an occupant)? [treated as a
  host contract violation: the core panics, no null guard is synthesised]

<!-- from: glue-simgrid-lots-shadow5 round 2 (2026-10-23) -->
- **LR12 - sections 1, 3.1, 14: lotrec_shadow observation inventory:** please define the exhaustive
  record-only load/month shadow scope and discovery contract. Is the denominator the lot manager's
  master-list cSC4Lot records (27 fields, including inline trips/properties), or every record class in
  section 1, including business units, construction, signposts, decals, wild flora and the crime-only
  lot mentioned in section 14? The master lot list alone cannot establish the latter inventory;
  section 3.1 explicitly says the lot does not save its occupants. For the intended scope, specify
  public owner/getter or record enumeration, exact native signatures/reference ownership, treatment
  of objects absent from the master list, and required outcomes for unsupported/unspecified prop
  subclasses. [lotrec_shadow deferred; implement and land the independently specified SG3
  simgrid_shadow first, without claiming lot-record coverage]
