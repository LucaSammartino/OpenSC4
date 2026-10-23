# occupants-props: answered questions (archive)

- **OP1:** text-decal field roles `+0x78`, `+0x84`, `+0x88`, `+0x8C`, `+0xAC` (lot-records.md §10.4): the
  constructor writes `+0x78` = a message-target-derived pointer, `+0x7C` = 0, `+0x84`/`+0x88` = −1, `+0x8C` = 0,
  `+0xAC` = 1 (`0x005F8300`); `SetPosition` writes x→`+0x88`, z→`+0x8C` (`0x005F7AA0`); `UpdateDecal` stores decal
  IDs into `+0x84`/`+0x88` and tests `+0x8C` as the dynamic flag; `SetOrientation` writes `+0xAC` as a u32 but
  `Shutdown` (`0x005F76C0`) treats `+0xAC` as a manager pointer and `+0x7C` as its decal id; `SetAlpha`/`SetTextColor`
  treat `+0x84`/`+0x88` as ids and `+0xA8` as alpha. Settle with a live save: run the CreateTextDecal debug command
  (`0x0BEAF1A2`), save, decode the record, and trace Write `0x005F7DB0`. [the ctor is the truth; the setter
  addresses above are all measured]
  **ANSWERED 2026-10-23 (re-text-decal-owner4):** DIFFERS — independently traced receiver
  transformations settle the aliases statically. Complete O: +78 string bookkeeping, +84/+88
  IDs, +8C dynamic byte, +AC orientation. SetPosition and Shutdown receive O+12; serializable
  Read/Write receive O+16. Their raw displacements must be converted before assigning roles.
  Rule and instruction citations: text-decal-occupant.md §§1–2 and lot-records.md §10.4.

- **OP2-TD:** TextDecal portion of OP2, slot +3C and +30 semantics and supposed ID/height repeats.
  **ANSWERED 2026-10-23 (re-text-decal-owner4):** DIFFERS — compiled stream interface facts
  identify +3C GZStr and +30 four-byte Float32. O+16 record receiver maps +5C to O+6C text,
  repeated pair +84/+88 to O+94/+98 position, +70 to height, +80 to size. No saved IDs here.
  Version1 height is numeric Uint32-to-f32; version2+ Float32. Canonical rule:
  text-decal-occupant.md §7. OP2 remains open only for its signpost scope.
<!-- Archive for [occupants-props.md](../occupants-props.md). Lanes do not read this file. Open questions are in ../occupants-props.md. -->

<!-- from: re-occprops (2026-10-20) -->
- **OP7:** construction-occupant visual/animation flags: `+0xD4`/`+0xD5` (rebuild vs crane flags read by the
  `0x26D31EC1` path), state `+0x13C` (the 2→1→2 toggle around DoAnimationTick), the Init view-slot-0x5C call with
  `0x5007` (occupant view slot 23 `AddOccupantGroup`? measured `CALL [EAX+0x5C]` with 0x5007 — is that
  AddOccupantGroup or a different slot 23?), and the GZCOM creator argument semantics (arg 1 selects the
  per-instance vtable pair). [measured: arg selects +0x3C/+0x40/+0x19C vptrs]
  **ANSWERED 2026-10-21 (re-construction-answers4):** DIFFERS — lot-records §9.2 Init/DoMessage/refresh: DoMessage runs at object+4, so its +0xD4/+0xD5/+0x13C are object +0xD8 (load-only has-crane, from record #8), +0xD9 (refresh pending, set only by Read) and +0x140 (state, toggled 2→1→2 around DoAnimationTick(0)). The Init call is occupant vtable 0x00AA21B8 slot 23 = 0x005E9350 on the occupant sub-object, one argument 0x5007, callee pops 4; its body appends 0x5007 to property 0xAA1DD396 when absent and then posts 0xEBFF18A3. Creator argument: as already in §9.1.

<!-- from: re-occprops / core-construction4 -->
- **OP13:** lot-records.md §9.1 / §9.2, construction occupant: where do the life-cycle values of lot-developer.md
  §6.7.4 / §6.7.5 (state, counter, `T0`, `nextDay`, `msAcc`, N, A, B, C, E, scaffold height) sit in the saved
  scalars (§9 #14), and with which type? §9.1 gives `+0x14C`/`+0x150` as 2 × f32 and `+0x154…+0x164` as 5 × u32,
  while §9.2 calls `+0x150` a u32 segment count and `+0x158`/`+0x15C` f32 heights; `+0x148` "segment end day" vs
  `nextDay`. The core keeps the scalars as raw data (bit-exact round trip) and the life cycle apart, so a loaded
  construction does not resume its schedule until this is answered.
  **ANSWERED 2026-10-21 (re-construction-answers4):** DIFFERS — lot-records §9.1 table (object frame): Write/Read run at object+0xC, so the old §9.1 offsets were 12 low. #14a state s8 (+0x140); #14b T0, unused, nextDay, N, counter (u32); #14c span N·segH, startY (f32); #14d A, B, C, E, msAcc (u32); #14e crane state s8; #14f swing min/max f32; #14g unused bool; #14h crane acc/duration u32; #14i target/previous angle f32; #14j crane type u8. A load resumes from these with no recomputation.

<!-- from: re-occprops / core-construction4 -->
- **OP14:** lot-records.md §9.1, construction occupant: which code fills the **saved** lot / building references
  (`+0x5C`/`+0x60`, record #3/#4) from the live ones (`+0x68`/`+0x6C`) that `SetLot` holds? And record #8 is "written
  as `(+0xF8 != 0)`" while #14g `+0x174` is the "crane exists flag (drives `+0xCC` on Write)", yet `SetLot` sets
  `+0x174 = 2` (a crane state, ld §6.7.4): which field is written for #8 and #14g? The core writes the saved refs as
  stored and #8 from its own "crane model exists" result of `AttemptAddCrane`.
  **ANSWERED 2026-10-21 (re-construction-answers4):** DIFFERS — lot-records §9.1 rows 3/4/8/14g: #3/#4 are the live lot/building fields +0x68/+0x6C themselves (no saved copy). #8 is written as (crane model pointer +0x104 != 0) and read into the load-only flag +0xD8; the SetLot "=2" field is the crane state +0x174 (#14e), and #14g (+0x180) is an unused bool.

<!-- from: re-occprops / core-construction4 -->
- **OP15:** construction occupant `Init` / `DoMessage`: lot-records §9.2 lists five message-server subscriptions
  including `0x86AD10EE`; lot-developer §6.7.1 lists four (the tick is delivered directly by the tick manager,
  §6.7.8). Which is right? The tick's `ms` is `data2 / 1000` in ld §6.7.1 and "data3 ÷ 1000" in lr §9.2 (data3 holds
  float bits, ld §6.7.8). Does `Init` (`0x005D1300`) also run the occupant base `Init` (§2.4, CreateOccupant
  message)? The core subscribes lr's five, uses data2, and does not run §2.4.
  **ANSWERED 2026-10-21 (re-construction-answers4):** DIFFERS — lot-records §9.2 Init/DoMessage: four subscriptions (0x66956814, 0xA6B79602, 0x06CDB65B, 0x26D31EC1); 0x86AD10EE comes by direct DoMessage from the tick manager. ms = message dword +0x14 (ld data2, µs) / 1000 unsigned. Init runs the occupant base Init (0x005E8C90) unconditionally after the IsInitialized check.

<!-- from: re-occprops / core-construction4 -->
- **OP16:** construction visuals with simulation-visible draws (ld §6.7.6 / §6.7.7): (a) `CreateSparks`' "max box
  extent": which box (building / scaffold) and which axes? The core uses the scaffold box over x, y, z. (b)
  `AnimateCrane` 2 → 0: the swing limits min / max, the constant `k` at `0x00AA2098`, the target before the first
  swing, whether "6000" is sfCraneTimeOfFullRotation, and the duration kept by 1 → 2. The core takes min / max / k
  from the caller, starts the target at 0, uses the literal 6000 and keeps the duration.
  **ANSWERED 2026-10-21 (re-construction-answers4):** DIFFERS — lot-developer §6.7.6/§6.7.7: (a) sparks use max(x span, z span) of the scaffold box (record #7, object +0xC0), y excluded; ties 20 → 2, 30 → 3. (b) limits c ± h from AttemptAddOuterCrane (h = π/2 or asin(d/R)); k = 1/(2π) f32 at 0x00AA2098; the multiplier is the tunable sfCraneTimeOfFullRotation global 0x00B0A7EC, not a literal; SetLot starts target = min, previous = max, acc 1, duration 0, state 2; 1 → 2 keeps the duration.

<!-- from: re-occprops (2026-10-20) -->
- ~~**OP8:** the `0xCA0E3852` filter (creator `0x007A0420`, vtable `0x00AB7E44`, byte +0x2E = 1, registered at
  `0x004608AB`), the `0xE9C75EE8` types-set filter body, and the hashed-variant filter bodies (vptrs
  `0x00A8EE38`/`0x00A8EE20`). [identity untraced]~~
  **ANSWERED 2026-10-21 (re-occfilters):** occupant-manager.md §13.2: the `0xE9C75EE8` filter is `cSC4OccupantTypesFilter`, a type list filled only by `Read` (§13.2.3); the `0xAA25CB90` one is `cSC4OccupantTypesFilterHashed`, a hash set the model maker fills (§13.2.3); the `0xCA0E3852` filter is the data map view's filter for the 3D view (§13.2.8).

<!-- from: re-occprops (2026-10-20) -->
- ~~**OP9:** `cAutomataOccupantFilter` type identities {`0xC7CC75F1`, `0x496E5783`, `0x480E6151`, `0x897F7EC2`}
  (`0x00562C90`): which occupant classes are these? [`0xC7CC75F1` is the subway-adjacent type per the subway
  filter; the rest unknown]~~
  **ANSWERED 2026-10-21 (re-occfilters):** occupant-manager.md §13.2.7: they are the vehicle, pedestrian, aircraft and watercraft occupant types, the names the string specification filter's parser gives them (§13.2.5: `Vehicle` `0xC7CC75F1`, `Pedestrian` `0x496E5783`, `Aircraft` `0x480E6151`, `Watercraft` `0x897F7EC2`).

<!-- from: re-occprops (2026-10-20) -->
- ~~**OP4:** `cSC4RealisticOccupantFilter` flag bytes `+0xC`/`+0xD`/`+0xE` (creator `0x005EA240` zeroes them; ctor
  `0x005EB850`?), the meaning of property `0x5001`, the `0x097F6D98` QI + lot slot 0x84==1 exclusion, and who sets
  the flags (callers of the creator). [include iff byte set for base textures and props]~~
  **ANSWERED 2026-10-21 (re-occfilters):** DIFFERS - occupant-manager.md §13.2.3: `+0xC`, `+0xD` and `+0xE` keep, when set, empty lots' base textures, the occupants of group `0x5001` and the signposts and text decals; `0x5001` is an occupant group (occupant slot 22), not a property; a base texture is dropped only when `+0xC` is clear and the lot under it is in state 1 (ready). The 3D view sets all three (`0x007F42C5`), the cheats `Zoneria` and `TastyZots` install copies with one flag flipped (OM10), the inside-lot filter and the region thumbnail writer use them clear. The group's meaning stays open (OM9).

<!-- from: re-occprops (2026-10-20) -->
- ~~**OP5:** `cSC4BlockingLotOccupantFilter` slot 3 (`0x004BDF90`, 26 bytes) body and users. [untraced]~~
  **ANSWERED 2026-10-21 (re-occfilters):** occupant-manager.md §13.2.6: slot 3 `0x004BDF90` is the base's `IsOccupantTypeIncluded(GetType())`; the blocking-lot filter's own slot 4 (`0x006C3C20`) is true only for flora `0x74758926`; the lot developer uses it.

<!-- from: re-occprops (2026-10-20) -->
- **OP2:** remaining signpost stream/receiver semantic: lot-records.md §13.2 calls stream
  +0x3C GZStr, but signpost Write (`0x005F5080`) / Read (`0x005F50F0`) were interpreted
  as passing a u32 field. Reconcile that class's receiver and field independently before
  changing its record. TextDecal scope is answered in text-decal-occupant.md §7 and archive
  OP2-TD: +0x3C is GZStr, +0x30 Float32; its receiver-relative +0x5C is text, not lot tag,
  and its repeated pair is position, not IDs. No signpost source analysis was done in that lane.

  **ANSWERED 2026-10-23 (re-pr-harvest5):** DIFFERS — PR10's re-signposts evidence
  identifies the signpost serializable receiver as O+0x10, so its +0x88 is O+0x98
  text. Stream slot +0x3C (15) is GZStr, and +0x30 (12) is Float32. Rule: lot-records.md
  §§10.3–10.4; original contributor evidence: Write 0x005F5080, Read 0x005F50F0.
  Main's independent OP1 and OP2-TD answers above remain authoritative for TextDecal.

<!-- from: re-occprops (2026-10-20) -->
- ~~**OP3:** signpost `SetDynamic` (`0x005F4BA0`) colour tables: it copies 4 bytes from `0x00AA5204 + 4·(arg!=0)`
  into the four corner rows at `+0x4D4…+0x4D7`, `+0x4E4…+0x4E7`, `+0x4F4…+0x4F7`, `+0x504…+0x507` (both byte
  orders measured: the load is `[EAX*4 + 0xaa5204]` style with per-corner offsets shuffled). What are the two
  rows (static vs edit-time colours) and the corner order? [dynamic = row 1]~~
  **ANSWERED 2026-10-22 (re-signposts):** row 0 is the static colour (144, 172, 190, 255), row 1 the dynamic
  (edit-time) colour (255, 255, 255, 204), bytes in memory order (`0x00AA5204` = `90 AC BE FF`, `FF FF FF CC`;
  *[inferred]* R, G, B, A); `SetDynamic(d)` copies row `d != 0` into the four corners.
