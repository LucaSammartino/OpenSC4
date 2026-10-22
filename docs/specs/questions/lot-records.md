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

<!-- from: glue-lotrecords-bind4 (2026-10-22) -->
- **LR10 - sections 3.2/13.2, concrete lot slot 55:** what is the complete mutation contract of
  `cISC4Lot::SetPopulation(devType, population)` (`0x006C1940`, pop 8)? The save record specifies a vector
  of population maps, not which map(s) this setter selects/creates, whether it clamps to capacity, what
  zero/absent/duplicate developer-type entries do, or which notifications/caches it updates. Native tract
  callers invoke slot 55 (tract-developer.md section 4, population adjustment), and lot-developer.md
  section 6.4 ReoccupyBuilding passes its AL through. The ABI row says `eax-or-void`, not a defined
  boolean contract. Please specify mutations, observable calls/order and return definedness with vectors
  for empty maps, the same developer type in two maps, and setting an existing population to zero.
  [concrete binding withheld; do not infer a BTreeMap insert, successful bool, or call the native method
  on a foreign object whose full native layout/construction contract is not published]

<!-- from: glue-lotrecords-bind4 (2026-10-22) -->
- **LR11 - sections 3.2/13.2, concrete lot slot 34:** please publish the complete `SetState` owner
  behavior needed for a replacement lot, including `UpdatePrimaryOccupantWealth` (`0x006C1430`).
  Lot-developer.md section 3.2 specifies message data4 and expressly leaves that wealth computation
  unspecified; the lot-records slot-34 row specifies pop 12 and no defined return, while the record
  describes saved fields. Required discriminators: no building type, unknown profile, entering/leaving
  constructing/occupied with wealth 0 versus nonzero, population/capacity-container updates and resource
  releases, state-day update, and queued copy lifetime. Which operations occur before the queued copy
  is taken? Native SetState uses a native lot copy constructor (`0x006C2450`) with embedded containers;
  forwarding to it is not a supported operation on a portable record or arbitrary replacement layout.
  [withhold complete-owner activation rather than synthesize successful transitions or manufacture a
  native-layout facade; the LR8/LR9 construction and view answers remain accepted]
