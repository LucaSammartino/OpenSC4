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
<!-- from: glue-construction-lr13 real-city run construction-lr13-r1-house-2032-20261024b (2026-10-24) -->
- **LR14 - section 9.1.1, intermittent native field-13 bare Serializable and last-reference lifetime:** the final LR13 implementation retains the literal field QI, releases the reader's temporary Serializable on success/rejection, and keeps an independent observer pin. Native T66-T73 pass, including exact helper acquisition counts after RefAudit cache warmup. Yet the house shadows profile in slot 2 captured construction field 13 at offset 158 as ID 3, class `0xA9BC9AB6`, on 2027-01-01: effect IID `0x2977AA49` was rejected, and strict replay consumed all input but reported `254/250` bytes. The capture pin's release then reached zero at `WriteCapture::ClearReferences:79` (bare Serializable vtable reported as game+0x6A41BC). A foreign write access violation followed on 2027-01-09; the run ended corrupt with foreign_exceptions=1/ref_warnings=1. The preceding build's house run reached 2032 clean with 182/182 construction records exact. Please specify how this observed bare callback arises despite the live-effect derived-Serializable contract, its actual ownership before/after writer slot 26, and any distinction between writing a valid live effect and this rejected input. What additional safe observation distinguishes a native temporary from a stale or misidentified field without guessing interfaces? [Literal LR13: reject to null, count mismatch, release only our acquisitions; do not coerce, normalize four bytes, suppress the warning, or leak the pin. Shadow remains default-off. Evidence: build-runs/game-results/construction-lr13-r1-house-2032-20261024b.log lines 8269-8272 and 8652 onward; normal-speed comparison requested separately.]
