# network-occupants: open questions
<!-- Questions about [network-occupants.md](../network-occupants.md). Append new entries at the END; when one is answered, move it to answered/network-occupants.md. Rules: README.md. -->

## From the network occupants spec (re-netocc lane, 2026-10-20)

Spec: [network-occupants.md](../network-occupants.md) §§1–10. RE-side follow-ups; nothing here blocks an
implementer of the class tables and records.

<!-- from: the network occupants spec (2026-10-20) -->
- **NO1 - network flag 0x40 vs the intersection bit:** the meaning of network flag `0x40` tested by
  `IsIntersection` (primary slot 55, `0x0061BD80`) versus the `0x100000` intersection bit used elsewhere
  (network-manager.md §5). [inferred: `0x40` is a styling/presentation bit; the slot checks it before the
  highway-intersection piece-id lookup at `0x00B4528C`]

<!-- from: the network occupants spec (2026-10-20) -->
- **NO2 - the power pole's delegate interface:** the identity of the delegate interface `0x26A68448` answered by
  the power pole via `+0xBC` (jump-table dispatch; likely a UI/drag helper). Probe: dump the `[+0xBC]` vtables on
  a live pole. [unknown]

<!-- from: the network occupants spec (2026-10-20) -->
- **NO3 - the underground occupant's type:** does `cSC4UndergroundOccupant` ever carry a distinct occupant type?
  Its base ctor leaves `0x74758925`; the demolition dispatcher never names an underground type. [inferred:
  underground pieces are dispatched as `0xC772BF98` through their `cISC4NetworkOccupant` interface]

<!-- from: the network occupants spec (2026-10-20) -->
- **NO4 - the prebuilt record's first five fields:** the semantics of the prebuilt `Write` fields
  `+0x170`/`+0x174`/`+0x178`/`+0x17C`/`+0x180` (network-occupants.md §2.8): model refs? shear inputs?
  [not specified]

<!-- from: the network occupants spec (2026-10-20) -->
- **NO5 - the 0x4835C6A7-AC property family:** SDK names of the property-ID family `0x4835C6A7` (edge types),
  `0x4835C6A8` (passage ceiling), `0x4835C6AA` (one-way), `0x4835C6AB` (anchor), `0x4835C6AC` (retrofit) stored
  in the occupant's own property holder. [unknown]

<!-- from: the network occupants spec (2026-10-20) -->
- **NO6 - the pipe's +0x268 field:** the meaning of pipe `Read`'s extra u32 → `+0x268` and `+0x258 = 1`
  (network-occupants.md §3.3): funding state? watered state? [unverified]

<!-- from: the network occupants spec (2026-10-20) -->
- **NO7 - GetSortValue's stored field:** which field holds the sort value of an underground occupant when flag
  `0x6000000` is set (§3.2); the negative displacement (`−0x178`/`−0x274`) in the decomp is ambiguous.
  [not specified]

<!-- from: the network occupants spec (2026-10-20) -->
- **NO8 - the connection registration strings:** the strings `0x00B0B6D0`/`0x00B0C388` passed with the class IDs
  to helper `0x00654710` by the pipe/subway-connection `Init` (§3.6): GZCOM class names? [unknown]