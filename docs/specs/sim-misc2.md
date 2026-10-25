# Small simulation owners 2: iterators, filters, records, small constructors (behaviour spec)

Scope: 38 small owners as tools/coverage.py assigns them (one to three functions each), listed in the inventory. Most
bodies are named by the spec that uses them; this spec gives each body's behaviour in one line.

Related: zones.md and network-routines.md (cell regions), lot-developer.md and growth-developer.md (scanning iterators),
network-manager.md and pathfinder.md (boundary iterators), occupant-manager.md (occupant filters), city-tools-input.md
(query and placement filters), lot-configuration.md and tract-developer.md (lot-configuration filters), automata.md
(behaviour templates, tweens), traffic-simulator.md (safe list iterators, ferry terminals), prop-occupant.md (prop
placement properties), regional-city-records.md (region thumbnails), tutorial.md (tutorial strings),
modal-dialogs.md (advice message box), terrain.md (brush parameters, waves), framework-misc3.md (`cGZAny` type IDs).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile. Written 2026-10-24 (re-simmisc7b).

## 1. Iterators *[compat]*

- `ScanningIterator` (constructor `0x0060CE90`): step 1, bounds (`INT_MIN`, `INT_MIN`, `INT_MAX`, `INT_MAX`), state 0;
  an empty bound range sets the state to 2 (finished) at once (lot-developer.md).
- `BoundaryIterator::GetNext(&x, &z)` (`0x0060CEE0`): state 2 → false; the first call starts (state 1), later calls
  advance by the step and stop past the bound (network-manager.md, pathfinder.md).
- `cSC4SafeListIterator<cISC4Lot*>`: `GetNext` (`0x00411D70`) releases the entries of its snapshot list and frees the
  nodes through the allocator service; `Release` (`0x0045D5B0`) unregisters from the owner's iterator list when
  registered, then frees the iterator (traffic-simulator.md).
- `ilist<…>`: constructor `0x004124F0` (an empty circular list), list destruction `0x00411870`.

## 2. Filters *[compat]*

| Owner (address) | Accepts |
|---|---|
| `cAutomataOccupantFilter::IsOccupantIncluded` (`0x00562C90`) | occupants of the four automaton types `0xC7CC75F1`, `0x496E5783`, `0x480E6151`, `0x897F7EC2` (occupant-manager.md) |
| `cSC4QueryAutomatonFilter` (`0x004C3AB0`) | types `0x496E5783`, `0xC7CC75F1` |
| `cSC4QueryStoplightFilter` (`0x004C3AD0`) | type `0xA823821E` with property `0x5002` or `0x5003` (city-tools-input.md) |
| `cSC4PlaceLotDemolishFilter` (`0x004BDF70` destructor body, `0x004BDFB0`) | every type except `0x2890D4DE` and `0x0990B2EA` |
| `cSC4BuildingPropFilter::IsOccupantIncluded` (`0x004FA6A0`) | the occupant's property `0x6A95E503` read as a bool |
| `cSC4LotConfigurationFilter` (IID `0x68F0113A`; constructor `0x00489A50`, `QueryInterface` `0x00489670`, `Release` `0x0076A370`) | the base of the lot-configuration filters (lot-configuration.md) |
| `cSC4LotConfigurationFilterByBuildingType(type)` (`0x004BE100`) | holds the type and the lot-configuration manager (`0x00B43D18`) |
| `cSC4TractDeveloperLotConfigurationFilter` (`0x00704950`) | a configuration passing both of its tests (slots 17 and 16 with the filter's two values; tract-developer.md) |

## 3. Records and small constructors *[compat]*

- `SC4CellRegion<long>`: constructor `0x004422A0` (a bounding rectangle of bit rows, 32 cells per word);
  `IncludesCell(x, z)` (`0x00466610`): inside the rectangle and the bit set (zones.md).
- `cSC4Automaton::TweenStruct::GetCurrent` (`0x004FA960`): with a non-positive duration the end value; else the
  linear blend at `elapsed / duration` clamped to [0, 1] (automata.md).
- `tBehaviorTemplate` (constructor `0x0050BD40`, automata.md): weight `+8` = 1.0, `+0xC` = 1, the rest zero.
- `cISTETerrain::tBrushParameters` (constructor `0x004D8B90`, terrain.md): `+0x10` = −1, `+0x14` = 1, two factors 1.0,
  `+0x20` = 3, `+0x34` = 100.0.
- `cS3DRect2D` (`0x00479BE0`): a vector push-back of 52-byte lot-configuration objects (lot-configuration-editor.md).
- `cS3DVector3`: `0x00423260` stores one pointer; `0x00547B50` is the fire crew's target assignment (framework-misc2.md
  §4, the same body); `0x004F20A0` (cited by automata.md).
- `cS3DVector4::operator!=` (`0x007507A0`): any component differing by at least the tolerance `0x00AB4290`.
- `cS3DMesh` (`0x00800EC0`): frees a block of the vertex allocator (`0x00B4E670`, framework-misc2.md §2.1 layout).
- `cSC4PropPlacementProperties` (constructor `0x005ED0C0`: a persistent resource whose twelve values start at the
  global default `0x00B231A0`; deleting destructor `0x005ED150`); its factory's `CreateInstance` (`0x005EDE90`)
  makes one (184 bytes) for type `0xA83479D3` (prop-occupant.md).
- `cSC4GameDataRegistry::cEntry` (constructor `0x004744E0`): an empty name string, the given values, `+0x2C` = 1.
- `cSC4ConfigRule` (constructor `0x007D0500`): name `unknown`, all fields zero (graphics-rules.md).
- `cSC4TrafficSimulator::FerryTerminal` (constructor `0x00718A80`): two empty lists.
- `cPortLot::GetLot` (`0x006E91C0`): resolves the lot at its stored cell through the lot manager (`0x00B43D08`, slot 6)
  the first time, keeps it (counted) and returns it.
- `cSC4Blimp`: `GetModelResourceInstance` (`0x00523F50`) = `0x0818F120`; `0x00523F20`, `0x00523FD0` (the `Init`
  thunk) are the aircraft code of aircraft.md.
- `cSC24BitRGBColorCustomizer`: `QueryInterface` (`0x006625F0`, IIDs 1, `0x273B784C`, `0x873B785A`);
  `GetPropertyValueString` (`0x00662660`) writes `Color: R:%d G:%d B:%d` from the three colour bytes.
- `cTutorialString::init(text)` (`0x00440370`): reads a string key from forms like `text 0x%08x`, `text %08x`,
  `text %u`, `text %i` (type `0x2026960B`, group `0xCA554B03`) or keeps the text (tutorial.md).
- `nsSC4TutorialSystem::FilterUICommandMessage(cmd)` (`0x0043F530`): with the city (`0x00B43CEC`) and its tutorial
  object (city slot 82) and that object active (its slot 8), the object's filter (slot 13) decides; otherwise true.
- `SC4QuadGridQuadrant::RemoveAllElements` (`0x005F3040`): decrements each element's 16-bit use count, deleting it at
  zero, and empties the list.
- `AnyString`: constructor `0x009407F4` (a counted string copy), destructor `0x0094002A`.
- `cSTEWaterWave::GetMeanWaveOrientation(&o)` (`0x007623B0`): false with no wave, else the first wave's orientation.

## 4. Small free functions and scoped helpers *[compat]*

- `gAdviceMessageBox(a, b)` (`0x004179A0`): the window manager's message box, slot 44 with `(b, a, 0, 1, 0)`
  (modal-dialogs.md).
- `SC4ReadCityRegionViewThumbnailBitmapFile(path, buffer)` (`0x005DC910`): a Gimex factory load as colour type 9;
  `SC4WriteCityRegionViewThumbnailBitmapFile` (`0x005DCA10`) is regional-city-records.md's.
- `cSC4AutoFlushInputMessageQueue(ms)` (`0x005F8630`): starts a timer; the destructor (`0x005F90D0`) flushes the input
  queue (window manager slot 19) when at least `ms` passed.
- `cSC4AutoDisableAppSwitching` destructor (`0x005F88C0`): `0x005F87D0(0)`, which re-enables application switching
  *[inferred from the name]*.

## 5. Function inventory

`ScanningIterator` `0x0060CE90`; `BoundaryIterator` `0x0060CEE0`; `cSC4SafeListIterator` `0x00411D70`, `0x0045D5B0`;
`ilist` `0x00411870`, `0x004124F0`; `cAutomataOccupantFilter` `0x00562C90`; `cSC4QueryAutomatonFilter` `0x004C3AB0`;
`cSC4QueryStoplightFilter` `0x004C3AD0`; `cSC4PlaceLotDemolishFilter` `0x004BDF70`, `0x004BDFB0`;
`cSC4BuildingPropFilter` `0x004FA6A0`; `cSC4LotConfigurationFilter` `0x00489670`, `0x00489A50`, `0x0076A370`;
`cSC4LotConfigurationFilterByBuildingType` `0x004BE100`; `cSC4TractDeveloperLotConfigurationFilter` `0x00704950`;
`SC4CellRegion` `0x004422A0`, `0x00466610`; `cSC4Automaton::TweenStruct` `0x004FA960`; `tBehaviorTemplate`
`0x0050BD40`; `cISTETerrain::tBrushParameters` `0x004D8B90`; `cS3DRect2D` `0x00479BE0`; `cS3DVector3` `0x00423260`,
`0x004F20A0`, `0x00547B50`; `cS3DVector4` `0x007507A0`; `cS3DMesh` `0x00800EC0`; `cSC4PropPlacementProperties`
`0x005ED0C0`, `0x005ED150`; `cSC4PropPlacementPropertiesFactory` `0x005EDE90`; `cSC4GameDataRegistry::cEntry`
`0x004744E0`; `cSC4ConfigRule` `0x007D0500`; `cSC4TrafficSimulator::FerryTerminal` `0x00718A80`; `cPortLot`
`0x006E91C0`; `cSC4Blimp` `0x00523F20`, `0x00523F50`, `0x00523FD0`; `cSC24BitRGBColorCustomizer` `0x006625F0`,
`0x00662660`; `cTutorialString` `0x00440370`; `nsSC4TutorialSystem` `0x0043F530`; `SC4QuadGridQuadrant` `0x005F3040`;
`AnyString` `0x0094002A`, `0x009407F4`; `cSTEWaterWave` `0x007623B0`; `fn:gAdviceMessageBox` `0x004179A0`;
`fn:SC4ReadCityRegionViewThumbnailBitmapFile` `0x005DC910`; `fn:SC4WriteCityRegionViewThumbnailBitmapFile`
`0x005DCA10`; `cSC4AutoFlushInputMessageQueue` `0x005F8630`, `0x005F90D0`; `cSC4AutoDisableAppSwitching` `0x005F88C0`.
