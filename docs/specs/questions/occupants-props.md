# occupants-props: open questions
<!-- Questions about lot-records.md (§9, §10.3, §10.4, §12), occupant-manager.md (§13.1, §13.2) and prop-occupant.md. Append new entries at the END; when one is answered, move it to answered/occupants-props.md. Rules: README.md. -->

<!-- from: re-occprops (2026-10-20) -->
- **OP2:** remaining signpost stream/receiver semantic: lot-records.md §13.2 calls stream
  +0x3C GZStr, but signpost Write (`0x005F5080`) / Read (`0x005F50F0`) were interpreted
  as passing a u32 field. Reconcile that class's receiver and field independently before
  changing its record. TextDecal scope is answered in text-decal-occupant.md §7 and archive
  OP2-TD: +0x3C is GZStr, +0x30 Float32; its receiver-relative +0x5C is text, not lot tag,
  and its repeated pair is position, not IDs. No signpost source analysis was done in that lane.

<!-- from: re-occprops (2026-10-20) -->
- **OP3:** signpost `SetDynamic` (`0x005F4BA0`) colour tables: it copies 4 bytes from `0x00AA5204 + 4·(arg!=0)`
  into the four corner rows at `+0x4D4…+0x4D7`, `+0x4E4…+0x4E7`, `+0x4F4…+0x4F7`, `+0x504…+0x507` (both byte
  orders measured: the load is `[EAX*4 + 0xaa5204]` style with per-corner offsets shuffled). What are the two
  rows (static vs edit-time colours) and the corner order? [dynamic = row 1]

<!-- from: re-occprops (2026-10-20) -->
- **OP4:** `cSC4RealisticOccupantFilter` flag bytes `+0xC`/`+0xD`/`+0xE` (creator `0x005EA240` zeroes them; ctor
  `0x005EB850`?), the meaning of property `0x5001`, the `0x097F6D98` QI + lot slot 0x84==1 exclusion, and who sets
  the flags (callers of the creator). [include iff byte set for base textures and props]

<!-- from: re-occprops (2026-10-20) -->
- **OP5:** `cSC4BlockingLotOccupantFilter` slot 3 (`0x004BDF90`, 26 bytes) body and users. [untraced]

<!-- from: re-occprops (2026-10-20) -->
- **OP6:** bodies of `cSC4OccupantImmovabilityFilter::IsOccupantIncluded`, `cSC4DummyAutoOccupant` (1 fn, 65
  bytes) and `cSC4ZoomRotationModelChangeOccupantFilter::IsOccupantIncluded`. [untraced in this lane]


<!-- from: re-occprops (2026-10-20) -->
- **OP8:** the `0xCA0E3852` filter (creator `0x007A0420`, vtable `0x00AB7E44`, byte +0x2E = 1, registered at
  `0x004608AB`), the `0xE9C75EE8` types-set filter body, and the hashed-variant filter bodies (vptrs
  `0x00A8EE38`/`0x00A8EE20`). [identity untraced]

<!-- from: re-occprops (2026-10-20) -->
- **OP9:** `cAutomataOccupantFilter` type identities {`0xC7CC75F1`, `0x496E5783`, `0x480E6151`, `0x897F7EC2`}
  (`0x00562C90`): which occupant classes are these? [`0xC7CC75F1` is the subway-adjacent type per the subway
  filter; the rest unknown]

<!-- from: re-occprops (2026-10-20) -->
- **OP10:** `GetOccupantGroups` overload `0x005E9500`: it resolves group IDs through the persist-resource manager
  `0x00B43CA8` slot 0xC with IID `0xA52160F5` and then reads property `0xAA1DD396` off the resolved object
  (slot 0x24 → `0x14` → `+0x19C`/`+0x10` walk). What interface is `0xA52160F5` here (the same IID is the
  persist-resource manager per audio.md §...1145) and what does the walk return — group names or templates?
  [unresolved; both overloads fill the caller's set]

<!-- from: core-occprops4 (2026-10-21) -->
- **OP11:** lot-records.md §3.2 (u8 map counts *[quirk]*): when a lot population map (#21) or the travel-population
  map (#22) has more than 255 entries, the u8 count is truncated. Does `Write` (`0x006C0080`) then still write
  every entry (the loop runs over the map, so the record desyncs on read), or only the `count & 0xFF` first
  entries? Same for the u8 P (#20) over the vector of maps. The core writes every entry (literal reading).
- **OP12:** lot-records.md §6 base-texture `Read` resizes the record vector to n "no bound": on a short read, are
  the records beyond the failure left as constructor defaults (five `0xFF` bytes, the other fields?) and what are
  the defaults of the texture instance, cell, orientation, configuration byte and frame? The core pads with
  0 / `0xFF`×5 / 0 up to 65,536 records and leaves larger counts unpadded.

<!-- from: core-construction-contractfix4 (2026-10-22) -->
- **OP17:** lot-records.md §9.1 / §9.2: which code writes the world box (record #6, object `+0xA8…+0xBC`) that
  `GetPosition` / `GetBoundingBox` read — `RebuildModel` (from the building's box, as `bbox` of ld §6.7.3), `SetLot`,
  or only `Read`? And is it the building's box unchanged or a box derived from the scaffold? The core sets it to the
  building's bounding box in `RebuildModel` (literal reading of "the building's bounding box" being the only box
  `RebuildModel` reads), also on the refresh after a load.
