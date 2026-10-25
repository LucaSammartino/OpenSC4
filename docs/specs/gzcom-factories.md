# GZCOM class factories and class-ID getters (behaviour spec)

Scope: 46 free-function owners `fn:GZCOM_Create…` and `fn:GZCOM_…CLSID` that tools/coverage.py leaves to the
`fn:GZCOM_*` catch-all: the creation functions the COM directors register for their classes, and the getters that
return a class ID. Each table row names the class's own spec. Not covered here: the factories of the Lua console and
script views, the effects parser and manager, the HTML window, the font renderer, the network layout custom view and
the resource-script file parser (their areas are out of this lane's scope).

Related: framework-gz-services.md and application.md (the COM directors and `GetClassObject`), framework-misc2.md §2
(allocators), audio.md §3 (`0x0090CF9A`, the fixed-size pool's grow).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the disassembly. Written 2026-10-24 (re-gzcomfactories7).

## 1. How a creation function works *[compat]*

A director's `AddCls(classID, create)` stores the function; `GetClassObject` calls it with no argument, then queries
the returned object. Three forms exist:

- **Heap**: `GlobalHeapAlloc(size)`; null → null; else the class constructor runs on it.
- **Pool**: the class keeps a fixed-size block pool (pool object `P`, its state at `P + 4`: free list `+8`, live count
  `+0xC`). An empty free list is refilled (`0x0090CF9A`, which adds a page); the first block is unlinked, the count
  incremented and the constructor runs on it. The pool form does not test for failure.
- **Inline**: small occupant filters are built in the function: the two vtables (`+0`, `+4`), a reference count
  `+8` of 0 and the fields listed.

The result is the object's `cIGZUnknown` face: the object itself, or the object plus the offset given.

## 2. Creation functions *[compat]*

| Function (address) | Form, size or pool | Constructor (arguments) | Returns | Class spec |
|---|---|---|---|---|
| `CreatecGZMessageServer2` (`0x0092EBB7`) | heap `0xB0` | `0x0092EB23` | `+4` | framework-gz-services.md |
| `CreatecSC4AnimationTickManager` (`0x00448FF0`) | heap `0x128` | `0x00448E30` | object | animation-tick.md |
| `CreatecSC4BusinessUnit` (`0x006C0EB0`) | heap `0x20` | `0x006C0800` | object | lot-records.md |
| `CreatecSC4Demand` (`0x005D52E0`) | heap `0x54` | `0x005D5280` | object | demand.md |
| `CreatecSC4DisasterInstanceFire` (`0x0057BB60`) | heap `0x100` | `0x0057AD90` | `+8` | disasters.md |
| `CreatecSC4DisasterInstancePipeBurst` (`0x0057D000`) | heap `0x38` | `0x0057CED0` | `+8` | disasters.md |
| `CreatecSC4DisasterInstanceVolcano` (`0x0057FF00`) | heap `0x54` | `0x0057FAF0` | object | disasters.md |
| `CreatecSC4MySimDispatch` (`0x00434620`) | heap `0x18C` | `0x00433A60` (0) | `+0xC` | mysims.md |
| `CreatecSC4NeighborConnection` (`0x006D4B90`) | heap `0x44` | `0x006D4680` | object | neighbors.md |
| `CreatecSC4NeighborDeal` (`0x006CF940`) | heap `0x80` | `0x006CF270` | object | neighbors.md |
| `CreatecSC4StringSpecificationOccupantFilter` (`0x005EBBD0`) | heap `0x48` | `0x005EBB10` | object | occupant-manager.md |
| `CreatecSTETerrainView3D` (`0x00757470`) | heap `0xE8` | `0x007567C0` | `+0xC` | terrain-view.md |
| `CreatecSC4FloraOccupant` (`0x00472C30`) | pool `0x00B07FC8` | `0x00472B30` | object | flora.md |
| `CreatecSC4LotBaseTextureOccupant` (`0x005E0820`) | pool `0x00B0A86C` | `0x005E05B0` | object | lot-records.md |
| `CreatecSC4PropOccupant` (`0x005EEF50`) | pool `0x00B0A8E4` | `0x005EE050` | object | prop-occupant.md |
| `CreatecSC4PropOccupantTerrainDecal` (`0x005EF180`) | pool `0x00B0A8F0` | `0x005EE360` | `+4` | prop-occupant.md |
| `CreatecSC4PropOccupantBurntBuilding` (`0x005EFC80`) | pool `0x00B0A908` | `0x005EF810` (1) | object | prop-occupant.md |
| `CreatecSC4PropOccupantAutomataController` (`0x005F0490`) | pool `0x00B0A914` | `0x005EFDB0` | `+4` | automata-support.md |
| `CreatecSC4NetworkTunnelOccupant` (`0x006477F0`) | pool `0x00B0B570` | `0x006476F0` (1) | `+4` | network-occupants.md |
| `CreatecSC4OccupantFilter` (`0x005EA130`) | inline `0x0C`, vtable `0x00AA42C8` | — | object | occupant-manager.md |
| `CreatecSC4DefaultOccupantFilter` (`0x005EA160`) | inline `0x0C`, vtable `0x00A87FC0` | — | object | occupant-manager.md |
| `CreatecSC4OccupantTypeFilter` (`0x005EA190`) | inline `0x10`, vtable `0x00A8F79C`; type `+0xC` = 0 | — | object | occupant-manager.md |
| `CreatecSC4NetworkOccupantFilter` (`0x005EA1C0`) | inline `0x10`, vtable `0x00AA42FC`; network mask `+0xC` = `0x1FFF` (all 13 types) | — | object | network-occupants.md |
| `CreatecSC4ViewLevelOccupantFilter` (`0x005EA200`) | inline `0x10`, vtable `0x00A8FAD4`; level `+0xC` = 1 (the surface) | — | object | occupant-manager.md |
| `CreatecSC4RealisticOccupantFilter` (`0x005EA240`) | inline `0x10`, vtable `0x00A88084`; bytes `+0xC`…`+0xE` = 0 | — | object | occupant-manager.md |
| `CreatecSC4OccupantTypesFilter` (`0x005EA330`) | inline `0x18`, vtable `0x00A8EDBC`; an empty type list `+0xC`…`+0x14` | — | object | occupant-manager.md |
| `CreatecSC4DataViewOccupantFilter` (`0x007A0420`) | heap `0x30`, `cSC4OccupantFilter` constructor `0x005EA080`, vtable `0x00AB7E44`; `+0xC`…`+0x28` zero, bytes `+0x2C`, `+0x2D` 0, `+0x2E` 1 | — | object | occupant-manager.md |

## 3. Class-ID getters *[compat]*

Each returns its constant in EAX (`mov eax, id; ret`).

| Function (address) | Class ID | Class spec |
|---|---|---|
| `cGZBufferCLSID` (`0x00825C80`) | `0xC470D325` | persistence.md §6 |
| `cGZDrawContextAsBufferCLSID` (`0x0082ABD0`) | `0xAB300B20` | span-render-4444.md §1.3 |
| `cGZDrawContextCLSID` (`0x0082BE70`) | `0x0AE6320E` | gz-graphics.md |
| `cGZDrawSystemCLSID` (`0x0082B840`) | `0xAB2EA3AA` | draw-system.md |
| `cGZMessageQueueManager2CLSID` (`0x0092D02D`) | `0x452294DF` | framework-gz-services.md |
| `cGZMessageServer2CLSID` (`0x0092D055`) | `0x252294E9` | framework-gz-services.md |
| `cRZMessage2StandardCLSID` (`0x0091328C`) | `0x46DF45E7` | foundations.md §5.1 |
| `cS3DFSHTextureFactoryCLSID` (`0x008005E0`) | `0x3AB50E2A` | graphics-formats.md |
| `cS3DResModelFactoryCLSID` (`0x00804080`) | `0xFAD0F0B6` | graphics-formats.md |
| `cSC4AdvisorBaseCLSID` (`0x0040D4B0`) | `0xCA09F543` | advisors.md |
| `cSC4DisasterLayerCLSID` (`0x005773A0`) | `0x61F6ABF5` | disasters.md |
| `cSC4DisasterManagerFireCLSID` (`0x005790B0`) | `0x69C1718E` | disasters.md |
| `cSC4DisasterInstanceFireCLSID` (`0x00579180`) | `0x69C1718F` | disasters.md |
| `cSC4DisasterManagerPipeBurstCLSID` (`0x0057C010`) | `0x4A132FDF` | disasters.md |
| `cSC4DisasterInstancePipeBurstCLSID` (`0x0057C030`) | `0x0A133012` | disasters.md |
| `cSC4DisasterManagerVolcanoCLSID` (`0x0057F2F0`) | `0xA9B5AB9F` | disasters.md |
| `cSC4DisasterInstanceVolcanoCLSID` (`0x0057F350`) | `0x29B5AB89` | disasters.md |
| `cSC4PathInfoFactoryCLSID` (`0x0053CF90`) | `0x496678FE` | traffic-network.md |
| `cSC4PropPlacementPropertiesFactoryCLSID` (`0x005EC930`) | `0xA83479EA` | prop-occupant.md |
