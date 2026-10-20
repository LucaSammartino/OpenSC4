# occupants-props: open questions
<!-- Questions about lot-records.md (§9, §10.3, §10.4, §12), occupant-manager.md (§13.1, §13.2) and prop-occupant.md. Append new entries at the END; when one is answered, move it to answered/occupants-props.md. Rules: README.md. -->

<!-- from: re-occprops (2026-10-20) -->
- **OP1:** text-decal field roles `+0x78`, `+0x84`, `+0x88`, `+0x8C`, `+0xAC` (lot-records.md §10.4): the
  constructor writes `+0x78` = a message-target-derived pointer, `+0x7C` = 0, `+0x84`/`+0x88` = −1, `+0x8C` = 0,
  `+0xAC` = 1 (`0x005F8300`); `SetPosition` writes x→`+0x88`, z→`+0x8C` (`0x005F7AA0`); `UpdateDecal` stores decal
  IDs into `+0x84`/`+0x88` and tests `+0x8C` as the dynamic flag; `SetOrientation` writes `+0xAC` as a u32 but
  `Shutdown` (`0x005F76C0`) treats `+0xAC` as a manager pointer and `+0x7C` as its decal id; `SetAlpha`/`SetTextColor`
  treat `+0x84`/`+0x88` as ids and `+0xA8` as alpha. Settle with a live save: run the CreateTextDecal debug command
  (`0x0BEAF1A2`), save, decode the record, and trace Write `0x005F7DB0`. [the ctor is the truth; the setter
  addresses above are all measured]

<!-- from: re-occprops (2026-10-20) -->
- **OP2:** the `cIGZOStream`/`cIGZIStream` slot `+0x3C` semantic: lot-records.md §13.2 names it "GZStr", but the
  signpost Write (`0x005F5080`) and Read (`0x005F50F0`) pass `&+0x88` (a u32 field) and the decal Write/Read pass
  `&+0x5C` (the prop's lot tag u32). Also settle slot `+0x30` = Float32 vs Uint32 (the decal reads `+0x70` via
  slot 0x24 in v1 with FILD, via slot 0x30 in v2+; SetTextHeight writes f32 to `+0x80` — so 0x30 = Float32 fits,
  and the `+0x84`/`+0x88` pair read as f32 would be NaN bit patterns of the −1 sentinel, which the integer
  compares treat as −1). One live trace of either Write or the SDK header order settles both.
  [0x30 = Float32; 0x3C = ???]

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
- **OP7:** construction-occupant visual/animation flags: `+0xD4`/`+0xD5` (rebuild vs crane flags read by the
  `0x26D31EC1` path), state `+0x13C` (the 2→1→2 toggle around DoAnimationTick), the Init view-slot-0x5C call with
  `0x5007` (occupant view slot 23 `AddOccupantGroup`? measured `CALL [EAX+0x5C]` with 0x5007 — is that
  AddOccupantGroup or a different slot 23?), and the GZCOM creator argument semantics (arg 1 selects the
  per-instance vtable pair). [measured: arg selects +0x3C/+0x40/+0x19C vptrs]

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