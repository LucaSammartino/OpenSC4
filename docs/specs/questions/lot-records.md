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

<!-- from: glue-lotrecords4 (2026-10-22) -->
- **LR8 - sections 3.1, 4, 7.1/7.2 and 15, complete-owner runtime interception boundary:** what published
  registration or interception contract lets an independently loaded plugin replace these three owners while
  retaining the native zone developer and lot developer? The class-object store uses GZCOM on load
  (foundations sections 2.7/3.6), but new lots are constructed directly by the zone developer
  (`zones.md` section 12, `0x006BF430` + `0x006C1E50`); `PickUnitToJoin` constructs business units directly,
  without the COM directory (`lot-developer.md` section 5.7, constructor `0x006C0800`); and native
  `AddLotFoundation`/`ClearLot` call the nonvirtual retaining-wall helpers (sections 7.1/7.2/15,
  `0x005E3070`, `0x005E37F0`, `0x005E3960`), including direct wall construction (`0x005E2F40`).
  Please specify the supported redirection boundary for those creation/helper paths, its calling convention,
  installation/restoration lifetime and ownership obligations, or explicitly require replacing the calling
  developers together. Factory registration and method addresses alone do not specify an interception contract.
  [blocked before runtime wiring: no `enable=lotrecords` registration, no partial factory or same-owner forwarding;
  existing Rust owner implementations unchanged; LR6/LR7 remain open]

<!-- from: glue-lotrecords4 (2026-10-22) -->
- **LR9 - section 3.1 versus published cISC4Lot slot 4, property-holder view:** section 3.1 places the
  plain property-holder view at lot +8 and the traffic-source view at +0x1C, but `docs/abi/interfaces.tsv`
  (`cISC4Lot`, vtable `00ab19b0`, slot 4, target `006c2050`) describes `AsISCPropertyHolder` as returning
  this +0x1C. Which offset and view does that slot actually return, and are the section 3.1 QI view offsets
  correct? [no runtime view/cast implemented; preserve both published sources until reconciled]
