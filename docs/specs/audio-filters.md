# Audio ambience filters: the 83 leaf classes of `cSC4AudioCOMDirector` (behaviour spec)

Scope: the test body (slot 5) of each of the 83 small classes the audio COM director registers (audio.md §11.1–
§11.2), which completes the owner `cSC4AudioCOMDirector` (180 functions, 15,018 bytes) together with audio.md §11, and
the separately named owner `cAmbienceOccupantFilterIsNature` (97 bytes). audio.md §11.1 already gives their shapes,
factories, QueryInterface, AddRef/Release and the always-true slots 3 and 4; §11.3 the director's lifetime.
audio-ambience.md §8.3–§8.4 says what they are for: they are the **ambience layer filters**, created by class ID from
the tuning lists and asked through one of three interfaces.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Each factory's vtable was read in its code and slot 5 in the PE
(build-runs script, not committed); every body was read in the decompile and the irregular ones in the disassembly.
Slot names of foreign interfaces follow the specs cited and, where marked *SDK*, the public gzcom-dll headers.
Written 2026-10-25 (re-audiofilters8).

## 1. The three families *[compat]*

| Family (audio.md §11.1) | Interface (audio-ambience.md §8.4) | Slot 5 | Cache |
|---|---|---|---|
| B8 (8 bytes) | `0xEA40074F`, global | `Test()`, no arguments | none |
| A (16 bytes) | `0xCA400753`, occupant | `Test(occupant)`, pop 4 | `+4` the last occupant pointer, `+8` its result |
| B20 (20 bytes) | `0xCA400757`, cell | `Test(x, z)`, pop 8 | `+4`, `+8` the last cell, `+0x0C` its result |

The caches: a family-A test whose argument equals `+4` returns `+8` without evaluating; any other occupant is
evaluated and stored with its result. The occupant is compared by pointer and not referenced, so a new occupant at
a freed one's address gets the stale answer *[quirk]*. Family-A factories start the cache at (null, false); the
layer never passes null (audio-ambience.md §8.4). Family B20 compares both coordinates; its factories start the
keys at −1, −1, so a query of cell (−1, −1) before any other answers false unevaluated. Results are AL bytes; several
bodies leave other EAX bits from their last call.

"Lot at the cell" below is lot manager (`0x00B43D08`) slot 6 `GetLot(x, z, false)`, borrowed; a null lot fails the
test. "The occupant's lot" is lot manager slot 14 `GetOccupantLot(occupant)`, AddRef'd for the test and released.
Lot slots (*SDK* names, the order matching lot-developer.md's slots 9, 14, 21, 48): 9 `GetLotConfiguration`, 31
`GetZoneType`, 33 `GetState`, 35 `GetCondition`, 41 `GetOccupantWealth`, 46 `GetBuildingType`, 48 `GetBuilding`.

## 2. Global tests (family B8)

| Class ID | True when | Body |
|---|---|---|
| `0x8A401E0F` | it is night: lighting manager (`0x00B43DDC`) slot 9 `IsNight` (tail call) | `0x004E2AA0` |
| `0xCA426982` | it is not night | `0x004E2AB0` |
| `0x6A426995`, `0x0A426991`, `0x6A42698E`, `0x8A42698A`, `0x2A426986` | the zoom (`GetZoomAndRotation`'s first output) is 0, 1, 2, 3, 4 respectively | `0x004E2B90`, `0x004E2B60`, `0x004E2B30`, `0x004E2B00`, `0x004E2AD0` |
| `0x0A426998` | always (the shared stub `0x00648F00`) | – |
| `0x0A54E719`, `0xAA54E715`, `0xAA54E710` | the 3D view (`0x00B43DC4`) slot 22 returns 1, 2, 3 respectively | `0x004E2C00`, `0x004E2BE0`, `0x004E2BC0` |
| `0x0B817C41` | the simulator (`0x00B43D5C`) is paused (slot 15 `IsPaused`) but not emergency-paused (slot 17, *SDK* `IsEmergencyPaused`) | `0x004E2C20` |
| `0xAB817C0B`, `0xCB817C1B`, `0x2B817C27` | simulator slot 27 `GetSimSpeed` is 0, 1, 2 | `0x004E2C50`, `0x004E2C70`, `0x004E2C90` |
| `0x8B817C32` | U-Drive-It is active: `0x00B43D44` slot 13 `IsDrivingMode` (tail call) | `0x004E2CB0` |

None of them tests its global for null.

## 3. Occupant tests (family A)

"Group g" is occupant slot 22 `IsOccupantGroup(g)` (occupant-groups.md). "Profile" is the building occupant's slot 11
record (QueryInterface `0x87DDA3A9` on the occupant; the record is copied with `0x004E41F0` and destroyed with
`0x00459220`; the query reference is released); its first dword is compared *[inferred: the purpose type,
1 residential … 8 high tech]*.

| Class ID | True when | Body |
|---|---|---|
| `0xCA5A6A5F` | group `0x4002` | `0x004E2570` |
| `0xCA5A6A81` | group `0x4005` | `0x004E2840` |
| `0x6A401BBA` | group `0x4002` and not group `0x4005` | `0x004E29A0` |
| `0x2A5A6A84` | group `0x4200` or `0x420F` (in that order) | `0x004E2880` |
| `0x0A5A6A64` | group `0x4202`, `0x4228` or `0x4211` (in that order) | `0x004E25B0` |
| `0x2A872395` | group `0x4224` or `0x4225` | `0x004E2620` |
| `0xEA5A6A8B`, `0x4A5A6A88`, `0x6A5A6A8F`, `0xAA5A6A7A`, `0x6A5A6A6B`, `0x4A5A6A6F`, `0x0A5A6A72`, `0x0A5A6A76`, `0xAA5A6A7D`, `0x2A5A6A68` | group `0x4205`, `0x4206`, `0x4209`, `0x420A`, `0x420B`, `0x420C`, `0x420D`, `0x420E`, `0x4218`, `0x4219` respectively | `0x004E2920`, `0x004E28E0`, `0x004E2960`, `0x004E27C0`, `0x004E26C0`, `0x004E2700`, `0x004E2740`, `0x004E2780`, `0x004E2800`, `0x004E2680` |
| `0x0A42B520` | occupant slot 7 (its type) is `0x6A11BD2B` | `0x004E2A00` |
| `0x8A5F603E` | the bool property `0x69DBEBE1` of the occupant's property holder (slot 6), read with `GetPropertyValue` (`0x005FD390`) into the argument's own stack slot; when the property is absent the test reads the low byte of the occupant pointer instead *[quirk]* | `0x004E2A40` |
| `0x6A401BB3` | QueryInterface `0xC83DD057` succeeds (`cAmbienceOccupantFilterIsNature::IsTrueOccupant`) | `0x004E2E10` |
| `0x6A43C75D` | QueryInterface `0x87DDA3A9` (building occupant) succeeds | `0x004E2E80` |
| `0x4A5D0EB0`, `0xAA5D0EB5`, `0xEA5F4954`, `0xCA401BAE` | a building whose profile value is 1, 2, 5, 8 respectively | `0x004E48A0`, `0x004E4260`, `0x004E45A0`, `0x004E4490` |
| `0x4A401BA7` | a building whose profile value is 6 or 7 | `0x004E4370` |
| `0x0A401BA0` | occupant slot 11 `GetBoundingCityCells` gives a minimum corner inside the city (0 ≤ x < city slot 93, 0 ≤ z < city slot 94), the lot there exists and its slot 35 is 2, 3, 5 or 6 (jump table `0x004E2D7C`) | `0x004E2CC0` |
| `0x8A60F033` | the same lot lookup **without** the bounds test, slot 35 = 4 | `0x004E2D90` |
| `0xCA9D68A1`, `0xCA9D68A5`, `0xAA9D68A9` | the occupant's lot exists and its slot 41 is 1, 2, 3 | `0x004E2EF0`, `0x004E2F60`, `0x004E2FD0` |
| `0x2AA17957` | the occupant's lot exists and its slot 35 is not 1 | `0x004E3040` |

The ambience manager's "tracked-occupant set" (audio-ambience.md §2, `+0x118`) is an instance of `0x4A401BA7`, this
table's profile 6/7 test.

## 4. Cell tests (family B20)

"Zone" is zone manager (`0x00B43D14`) slot 12 `GetZoneType(x, z)` (zone numbers as network-manager.md's zone table:
1–3 residential, 4–6 commercial, 7–9 industrial, 14 landfill).

| Class ID | True when | Body |
|---|---|---|
| `0xEA5F495C` | zone 0 | `0x004E30B0` |
| `0x8A53E23E`, `0xCA53E243`, `0x0A53E247` | zone 1–3, 4–6, 7–9 | `0x004E33B0`, `0x004E3420`, `0x004E3490` |
| `0x4A53E24B`, `0x2A53E24F`, `0xCA53E252`, `0x0A53E290` | zone 11, 12, 13, 14 | `0x004E3500`, `0x004E3560`, `0x004E35C0`, `0x004E3620` |
| `0x8A427F7C` | the cell has no occupant: occupant manager (`0x00B43D0C`) slot 33 `GetFirstOccupantByStandardCityCell(&o, x, z, 0)` is false (a found occupant is released) | `0x004E3100` |
| `0xEA4298FC`, `0x2A429900`, `0x8A60D873`, `0x2A429907`, `0x4A60D870`, `0x4A429903` | zone developer (`0x00B43DB0`) slot 8 (*SDK* `ExistsNetworkOfType`) `(x, z, flags)` with flags 1, 2, 4, `0x10`, `0x48`, `0x80` | `0x004E3170`, `0x004E3260`, `0x004E3210`, `0x004E3310`, `0x004E31C0`, `0x004E32B0` |
| `0xCA42990A` | zone developer slot 9 (*SDK* `IsCellWater`) `(x, z)` | `0x004E3360` |
| `0x2A53F0C3` | the lot at the cell has slot 33 (`GetState`) = 2 | `0x004E3680` |
| `0x2A9D6690`, `0xAA9D6694`, `0x2A9D6697` | the lot's slot 41 is 1, 2, 3 | `0x004E36F0`, `0x004E3760`, `0x004E37D0` |
| `0xEA80835E` | the lot's slot 35 is 2, 3, 5 or 6 | `0x004E3D90` |
| `0x4A808375` | the lot's slot 35 is 4 | `0x004E3E90` |
| `0x2A808371` | the lot has a building (slot 48 non-null) | `0x004E3E20` |
| `0x4A8084F4` | the lot's building, as an occupant (building slot 3), is in group `0x1702` | `0x004E3F00` |
| `0x0A808367`, `0xCA80836C` | the lot's building's profile (building slot 11, copied as in §3) has value 6 or 7, value 8 | `0x004E46B0`, `0x004E47C0` |
| `0x6A60F166` | the lot's slot 46 `GetBuildingType(true)` is non-zero and the building development simulator (`0x00B43D68`, null → false) slot 39 `GetBuildingProfile(type)` has value 5 | `0x004E49B0` |
| `0x8A53F0E4` | with s = the growth stage (configuration slot 11, *SDK* `GetGrowthStage`) of the lot's configuration (lot slot 9, AddRef'd and released): (zone of the lot (slot 31) 7–9 and s < 1) or s < 3 | `0x004E3840` |
| `0x4A53F0E8` | (lot zone 7–9 and s = 1) or 3 ≤ s ≤ 6 | `0x004E3920` |
| `0x8A53F584` | (lot zone 7–9 and s ≥ 2) or s ≥ 7 | `0x004E3A10` |
| `0xEA5A6FCE`, `0x8A5A6FD1`, `0xEA5A6FD4` | the crime simulator (`0x00B43D70`, null → false) slot 5 map's byte at (x, z) (map slot 7) is ≥ 200, 120–199, < 120 | `0x004E3AF0`, `0x004E3B60`, `0x004E3BD0` |
| `0x6A5D0682`, `0x0A5D068D`, `0x2A5D0691` | the land value simulator (`0x00B43D88`, null → false) slot 8 map's byte at (x, z) is < 140, 140–199, ≥ 200 | `0x004E3C40`, `0x004E3CB0`, `0x004E3D20` |

The six network tests and the water test call the zone developer **twice**: the first result is cached, the second
returned *[quirk]* (the two agree unless the network changes between them). A lot or configuration without the
growth-stage object fails the three stage tests.

## 5. Quirks

1. Family-A caches compare occupant pointers without holding a reference (§1).
2. `0x8A5F603E` reads the occupant pointer's low byte when the property is absent (§3).
3. `0x8A60F033` skips the city-bounds test that `0x0A401BA0` makes (§3).
4. The zone-developer tests evaluate twice (§4).

## 6. Conformance tests

1. For each class ID, create the object through GZCOM with IID `0x4A40074C` and require it to answer exactly its
   family's interface from §1.
2. Family A: two calls with the same occupant evaluate once (count the foreign calls); a different occupant
   evaluates again.
3. `0x6A401BBA` on an occupant in groups `0x4002` and `0x4005` is false; in `0x4002` only, true.
4. Crime byte 199 → `0x8A5A6FD1` true, `0xEA5A6FCE` false; land value 140 → `0x0A5D068D` true.
5. Growth stage 1 on a zone-8 lot: `0x8A53F0E4` true (s < 3), `0x4A53F0E8` true (industrial, s = 1),
   `0x8A53F584` false.
6. `0xEA4298FC` makes two `ExistsNetworkOfType(x, z, 1)` calls on a cache miss and none on a hit.

## 7. Function inventory

Slot-5 bodies, by address: `0x004E2570`, `0x004E25B0`, `0x004E2620`, `0x004E2680`, `0x004E26C0`, `0x004E2700`,
`0x004E2740`, `0x004E2780`, `0x004E27C0`, `0x004E2800`, `0x004E2840`, `0x004E2880`, `0x004E28E0`, `0x004E2920`,
`0x004E2960`, `0x004E29A0`, `0x004E2A00`, `0x004E2A40`, `0x004E2AA0`, `0x004E2AB0`, `0x004E2AD0`, `0x004E2B00`,
`0x004E2B30`, `0x004E2B60`, `0x004E2B90`, `0x004E2BC0`, `0x004E2BE0`, `0x004E2C00`, `0x004E2C20`, `0x004E2C50`,
`0x004E2C70`, `0x004E2C90`, `0x004E2CB0`, `0x004E2CC0`, `0x004E2D90`, `0x004E2E10`, `0x004E2E80`, `0x004E2EF0`,
`0x004E2F60`, `0x004E2FD0`, `0x004E3040`, `0x004E30B0`, `0x004E3100`, `0x004E3170`, `0x004E31C0`, `0x004E3210`,
`0x004E3260`, `0x004E32B0`, `0x004E3310`, `0x004E3360`, `0x004E33B0`, `0x004E3420`, `0x004E3490`, `0x004E3500`,
`0x004E3560`, `0x004E35C0`, `0x004E3620`, `0x004E3680`, `0x004E36F0`, `0x004E3760`, `0x004E37D0`, `0x004E3840`,
`0x004E3920`, `0x004E3A10`, `0x004E3AF0`, `0x004E3B60`, `0x004E3BD0`, `0x004E3C40`, `0x004E3CB0`, `0x004E3D20`,
`0x004E3D90`, `0x004E3E20`, `0x004E3E90`, `0x004E3F00`, `0x004E4260`, `0x004E4370`, `0x004E4490`, `0x004E45A0`,
`0x004E46B0`, `0x004E47C0`, `0x004E48A0`, `0x004E49B0`; helper `0x004E41F0` (profile copy).
