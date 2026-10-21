# network-manager: answered questions (archive)
<!-- Archive for [network-manager.md](../../network-manager.md). Lanes do not read this file. Open questions are in ../network-manager.md. -->

## From the network manager and network lot manager spec (re-specs13c lane, 2026-10-15)

Open points from network-manager.md §15. The current answer is in brackets; each entry says what would settle it.

<!-- from: From the network manager and network lot manager spec (2026-10-15) -->
- ~~**NM1:** are the globally managed network occupants (bridges, tunnels, prebuilt-model pieces) also saved by the
  occupant manager, or only through component 53? [both hold refs to the same objects; the occupant manager spec
  (re-specs13b) settles it]~~ **ANSWERED 2026-10-15 (re-specs14b):** both. The occupant grid (component 4) and
  component 53 save refs to the same objects; each ref resolves to one object, so `managed` holds each piece once.
  MATCHES. network-manager.md §15.

<!-- from: From the network manager and network lot manager spec (2026-10-15) -->
- ~~**NM2:** does the occupant manager post `0x99EF1142` while a city loads? If it does, the network manager's managed
  list (read first) gets every piece twice. [no, inferred. A turbo run that saves, loads and saves again and compares
  the component 53 counts would settle it]~~ **ANSWERED 2026-10-15 (re-specs14b):** no. Loading posts no Insert. Since
  the base occupant `Read` (`0x005E8D60`) sets bit 1 of the state byte, the base `Init` (`0x005E8C90`) posts no
  Create either. The network occupant `Init`s never insert. MATCHES. network-manager.md §15.

<!-- from: From the network manager and network lot manager spec (2026-10-15) -->
- ~~**NM3:** meanings of network flags `0x40000`, `0x80000`, `0x40000000` and `0x80000000`~~ **ANSWERED 2026-10-15
  (re-specs15b):** `0x40000` bridge-step piece (MATCHES), `0x80000` tunnel portal (MATCHES), `0x40000000` elevated
  with supports (MATCHES), `0x80000000` draft (preview) occupant. network-geometry.md §10.3. History: ~~and of the terrain kinds 3,
  4 and 6 in the network manager's terrain handler. [unknown; logging `data4` of `0x278E6003` for each terraforming
  brush settles the kinds]~~ **PARTLY ANSWERED 2026-10-15 (re-specs14a):** the kinds are settled from every call
  site of `RedisplayTerrain` (terrain.md §8.2): 0 internal (sea level, flat areas, shore cliffs, texture-only),
  1 terraforming in god mode, `ChangeTerrainLevel` and the edge reconciliation, 2 terraforming in mayor mode and
  the `ApplyTerrainBrush` script command,
  3 disasters and water (earthquake, pipe-burst sinkhole, effect brushes such as the volcano, lava solidification,
  the surface water's animation, and the `tBrushParameters` default), 4 the network height solver (road and rail
  grading, `0x00644530`), 5 levelling (demolition `ModifyTerrainHeight`, lot developer, landfill), 6 the tunnel cut
  (`cSC4NetworkTool::InsertTunnelPiece`). Flag `0x40000` is the **bridge** flag *[inferred from use]*: the surface
  water's `IsNavigable` reads a deck height from a piece with that flag and requires 20 m of clearance over the
  water, and its `0x89D3E9F3` handler tests it with "no adjacent piece" (terrain.md §10.3–§10.4). Still open: `0x80000`,
  `0x40000000`, `0x80000000` (a network-tool spec settles them; `0x80000` is the piece kept by a tunnel cut,
  *[inferred]* a tunnel portal).

<!-- from: From the network manager and network lot manager spec (2026-10-15) -->
- ~~**NM4:** the exact visited and rejected set tests of the avenue flood in `UpdateAvenueStyle`. [a cell is visited
  once; styling an avenue loop through an intersection and logging the styled cells settles it]~~ **ANSWERED
  2026-10-16 (re-answers18):** **MATCHES**.
  - `0x0060EF40` keeps three `u16` hash sets (work, accepted, rejected; key `(x << 8) | z`).
  - A neighbour is skipped when it is accepted (`0x0060F228`) or rejected (`0x0060D1B0`), else inserted unique
    into the work set (`0x0060E210`). Each cell is accepted or rejected in the step that takes it, so it is examined
    once.
  - New quirks: the far bounds are inclusive (nx ≤ countX, nz ≤ countZ). The 8-bit x in the key makes a neighbour
    one past the edge of a 256-cell city alias a cell on the opposite edge.

  network-manager §8.7.

<!-- from: From the network manager and network lot manager spec (2026-10-15) -->
- ~~**NM5:** the effect-prop slots 6, 7 and 8 used for potholes. [set the effect name, start, stop; the prop occupant
  spec should confirm]~~ **ANSWERED 2026-10-15 (re-specs14b):** confirmed: `SetEffectType(char const*)`,
  `StartEffect(bool)`, `StopEffect(bool)` (vtable `0x00AA4ED8`, overloads in reverse SDK order). MATCHES.
  network-tools.md §13.2.

## From the network manager and network lot manager cores (filed by the coordinator from the `cores14c` lane's report, 2026-10-16; merged `9dad25e`)

cores14c implemented network-manager.md as a Rust core (`engine/core/src/network_manager/`), a C ABI (`fc4_netmgr`,
`fc4_netlot`, ABI 66) and conformance vectors (`nm16_*`, `nm00_*`). There is no plugin glue yet, so none of these
readings has been checked in game. The current choice is in brackets.

<!-- from: From the network manager and network lot manager cores (2026-10-16) -->
- ~~**NM6:** §4.1, Read of component 53: "any failed ref read returns false at once". Does a null reference in the
  saved list count as a failed read, or is it appended as a null entry? [a null reference is appended as a null entry
  and counts as read; a reference that does not resolve fails the read at once, and the entries appended so far stay.
  Core `manager.rs` read_record, vector `nm16_01`]~~ **ANSWERED 2026-10-16 (re-answers24):** MATCHES
  (network-manager.md §4.1). The ref helper (`0x005D96C0`) returns true with a null pointer for object ID 0
  (`0x005D96F2`–`0x005D9713`), and Read appends it (`0x0061252D`–`0x00612544`). An unresolved reference, or one that
  fails the QI to `0xA821EF94`, makes the helper return false and Read returns false at once (`0x0061252B`), with
  nothing released or removed.

<!-- from: From the network manager and network lot manager cores (2026-10-16) -->
- ~~**NM7:** §7.10, the retrofit at PostCityInit: when the list is emptied (`0x00611A20`), is each entry released? Is
  the list emptied even when no entry still asks for a retrofit? [every entry still asking goes to the host's retrofit
  callback, then the list is always emptied and every entry released once (the reference taken at slot 23)]~~
  **ANSWERED 2026-10-16 (re-answers24):** MATCHES (network-manager.md §7.10). The release is in the walk, not in
  `0x00611A20`: each entry is released once right after its `CheckForRetrofit` test (`0x00612DB3`, also on the false
  branch from `0x006128E0`), and `0x00611A20` then frees the nodes without releasing. An empty list returns at once
  (`0x0061283A`). The net effect is the bracket's.

<!-- from: From the network manager and network lot manager cores (2026-10-16) -->
- ~~**NM8:** §8.2, §8.4, §8.7: zone and wealth values outside the tables (zone outside 0..15, wealth outside 0..3, for
  example from a corrupt zone grid). [a neighbour zone outside 0..15 never wins in GetZoneTypeForNetwork;
  IsLotCompatible fails for such a zone or wealth; the road style tables give such a zone the "every other zone" rows]~~
  **ANSWERED 2026-10-16 (re-answers24):** no defined original behaviour; the choices are **confirmed** as *[choice]*
  (network-manager.md §8.2, §8.4, §8.7). None of the three has a range check. GetZoneTypeForNetwork indexes the
  priority table with the signed neighbour value (`0x0060D0E7`–`0x0060D0F9`) and reads the neighbouring read-only
  tables: most such values never win, but some do (28 reads 0, 92 reads −1, −4 reads 2); an out-of-range own-cell
  value is returned as it is. IsLotCompatible reads bit vectors whose unused bits the constructor never initialises
  (`0x00477E70` grows them bit by bit), and beyond 31 or below 0 reads heap memory. Road style reads past `T` and `V`
  (`0x0060F5AE`, `0x0060F5DA`). Only a corrupt zone grid reaches any of this (wealth comes from §8.3, which clamps).
  For `V`, the "every other zone" row is taken to be the zones 10–13 row, `1 1 1`.

<!-- from: From the network manager and network lot manager cores (2026-10-16) -->
- ~~**NM9:** §7.6, the underground refit: step 2 has no "if any" test, unlike §7.2. When dirty is not empty but holds no
  pipe (or no subway), are tool 4 (or 7) Init, UpdateUndergroundOccupantTerrainHeights and Shutdown still called with
  an empty list? [yes: both kinds always run when dirty is not empty (vector `nm16_10`)]~~ **ANSWERED 2026-10-16
  (re-answers24):** **DIFFERS** (network-manager.md §7.6). Each kind runs only when its occupant list is not empty:
  pipes at `0x00613E53`–`0x00613E59`, subways at `0x00613E9B`–`0x00613EA1`. A kind with no occupant in `dirty` gets no
  `GetNetworkTool`, `Init`, `UpdateUndergroundOccupantTerrainHeights` or `Shutdown`. Also: a failed query (occupant
  manager slot 40 returning false) skips the split and both kinds. Vector `nm16_10` needs updating.
