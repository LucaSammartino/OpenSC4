# Grid ("field") simulators: SimGrid, land value, pollution, crime, aura, power, water, protection, flammability, flora

Target: `SimCity 4.exe` 1.1.641 (GOG), image base `0x00400000`. Read [`../sim-engine.md`](../sim-engine.md) first for the
agent scheduler (levels 0/4/6/10 = SimBegin/NewDay/NewMonth/SimIdle).
Windows addresses for every mapped function are in [`../../re/symbols/grid-sims.tsv`](../../re/symbols/grid-sims.tsv).
Mac addresses in brackets refer to the named Mac Rev A Intel pseudocode.

Status legend: **[confirmed]** read in code (Mac named pseudocode, and where noted, the Windows function too) ·
**[inferred]** consistent with evidence but not proven.

Conventions used below:
- *cell* = one 16 m city cell (city vt `CellWidthX/Z` = `+0x16C/+0x170`, `CellCountX/Z` = `+0x174/+0x178`).
- *tract* = one SimGrid element. A grid with tract size `S` (a power of two) covers `S×S` cells; its *shift* is `log2(S)`.
- Grids are stored and indexed `[x][z]` (outer vector = x).
- All the simulators' tuning lives in the simulator-tuning exemplar group `0xE7E2C2DB` (type `0x6534284A`); each
  simulator reads its own instance in `ReadSimulatorTunableValues`.

---

## 1. cSC4SimGrid<T>

CLSIDs: `0x49B9E600` (generic), Bool8 `…601`, Uint8 `…602`, Sint8 `…603`, Uint16 `…604`, Sint16 `…605`, Uint32 `…606`,
Sint32 `…607`, Uint64 `…608`, Sint64 `…609`, Float32 `…60A`, Float64 `…60B`. The CLSID is stored in the object (`+0x10`),
so `GetGZCLSID` reads a field rather than returning a constant. **[confirmed]** Objects are 0x44 bytes and are usually
constructed inline by the owning simulator (Windows inlines the ctor: it writes the vtables, then the CLSID into `+0x10`).

Windows vtables: Uint8 `cISC4SimGrid` `0x00A878C0` / `cIGZSerializable` `0x00A878A4`; Float32 `0x00A87C20` / `0x00A87C04`.
Type-independent methods are COMDAT-folded, so one body serves every `T` (and sometimes unrelated classes too). **[confirmed]**

### Layout (identical on Mac and Windows) **[confirmed]**

| off | field |
|---|---|
| +0x00 | vptr cISC4SimGrid<T> |
| +0x04 | vptr cIGZSerializable |
| +0x08 | bool (serialized; meaning unknown, initialised false) |
| +0x0C | refcount |
| +0x10 | CLSID |
| +0x14 | instance ID (set by the owner, e.g. `0x89D5BB39`) |
| +0x18 | tract size S (cells per tract side) |
| +0x1C | tract shift = log2(S) |
| +0x20 | tract count X = CellCountX >> shift |
| +0x24 | tract count Z = CellCountZ >> shift |
| +0x28/+0x2C | tract width X/Z in metres = S × CellWidth |
| +0x30/+0x34 | 1/width X/Z |
| +0x38 | `vector<vector<T>>` data, `[x][z]` |

### Behaviour **[confirmed]**

```text
SetTractSize(S):                         // no-op if S unchanged
    size = S; shift = floor(log2(S))
    countX = city.CellCountX >> shift;  countZ = city.CellCountZ >> shift
    widthX = S * city.CellWidthX;       widthZ = S * city.CellWidthZ
    invW*  = 1/width*
    data.resize(countX); each row.resize(countZ, 0)     // existing values are kept where they fit
GetCellValue(cx,cz)            = data[cx>>shift][cz>>shift]
GetTractValue(tx,tz)           = data[tx][tz]           // no bounds check
TractIsInBounds(tx,tz)         = (unsigned)tx < countX && (unsigned)tz < countZ
PositionToTract(px,pz)         = ((int)(px*invWX), (int)(pz*invWZ))
TractCornerToPosition(tx,tz)   = (tx*widthX, tz*widthZ)
TractCenterToPosition(tx,tz)   = ((tx+0.5)*widthX, (tz+0.5)*widthZ)
GetAverageValueInTractRect(x,z,w,h) = (sum over [x,x+w)×[z,z+h) of (int64)value) / (w*h)
GetAverageValueInCellRect(x,z,w,h):
    if the whole rect falls in one tract: return that tract's value
    else average over every cell (each cell reads its tract's value), int64 sum / (w*h)
SetTractValue / SetTractValues(v): write one / all tracts
```

Note: `GetAverageValueIn*Rect` take `(left, top, width, height)` despite the community header calling them
`(TopLeft, BottomRight)`; the loops run `x < left+width`. **[confirmed]** The sum casts each element to int64 even for
float grids, so fractional parts are dropped. **[confirmed, Mac float instantiation]**

### Serialization (`Write`/`Read`) **[confirmed]**

```text
uint16 version = 1
bool   flag(+0x08)
uint32 clsid, uint32 instanceID
int32  tractSize, tractShift, countX, countZ
float  widthX, widthZ, invWidthX, invWidthZ
T      value  × countX*countZ   (x outer, z inner)
```
`Read` rejects any version other than 1 and resizes from the stored counts (it does not recompute from the city).

### Common helpers used by the field simulators

- `ProcessTwoGridsWithCallback<A,B>(dst, src, fn, ctx)` **[confirmed]**: walks two grids with different tract shifts.
  Same shift: `fn(dst[x][z], src[x][z], ctx)`. If `dst` is coarser (shift difference `d`): `src` is box-averaged over each
  `2^d × 2^d` block (`sum >> d >> d`) and passed once per `dst` tract. If `dst` is finer: each `src` value is applied to
  every `dst` tract it covers.
- `cSC4MapUtilities::AddDistanceEffect(grid, w, h, mask, maskValue, radius, nearValue, farValue)` **[confirmed]**:
  builds a scratch `uint8` distance map (255 = unreached, 0 where `mask == maskValue`), runs `Expand` `radius` times
  (4-neighbour dilation, so distance is Manhattan), then for every tract with `0 < dist < radius` does
  `grid += nearValue − dist × (nearValue − farValue)/radius`, clamped to `[0,255]`.
- `cSC4ResponseCurve::GetOutputValue(x)` **[confirmed]**: piecewise-linear curve from a float-array property of `(x,y)`
  pairs. Binary search for the segment; below the first point → first y; past the last point → last y; else linear
  interpolation (slope precomputed per segment).

---

## 2. cSC4LandValueSimulator (`0xE990C18E`)

Code: Windows `0x006BD4B0–0x006BF2E0`. Windows vtables: `cISC4LandValueSimulator` `0x00AB1670`,
`cIGZSerializable` `0x00AB1648`, `cIGZMessageTarget2` `0x00AB1660`.

### Grids **[confirmed]**

| member | getter | type | tract size | instance | role |
|---|---|---|---|---|---|
| +0x24 | `GetIntrinsicLandValueMap` | Uint8 | tunable `0x47E2C200` | `0x89D5BB39` | intrinsic = base + water + altitude |
| +0x28 | `GetTotalLandValueMap` | Uint8 | same | `0x89D5BB51` | final land value read by `GetLandValue`, `GetLandValueType` |
| +0x2C | (none) | Uint8 | same | `0x89D5BB60` | scratch/working grid |
| +0x30 | `GetSlopeMap` | Float32 | 8 (fixed) | `0x49D5BB6F` | average slope per 8×8 cells |

`GetLandValue(cx,cz)` = total grid `GetCellValue`. `GetLandValueType(cx,cz)`: `v < boundary[0]` → 1 (low),
`v < boundary[1]` → 2 (medium), else 3 (high), boundaries from `0x47E2C301`. **[confirmed]**

### Schedule and messages **[confirmed, Mac + Windows DoMessage]**

`Init` registers the agent at levels 0, 10, 4 and 6 (flags 2) and subscribes to `InsertOccupant 0x99EF1142`,
`RemoveOccupant 0x99EF1143`, `WaterHeightsChanged 0x678E6E3A`, `TerrainHeightsChanged 0x278E6003`,
`ZoningChange 0x07699455`.

| message | action |
|---|---|
| any of the five subscribed messages | `dirty = true` |
| SimNewDay | `DoIdle(5 ms)` |
| SimIdle | `DoIdle(data3 ms)` |
| SimNewMonth | if `dirty` and `state == 9` (finished): `dirty = false; state = 0` (restart the pipeline) |
| SimBegin | nothing (returns true) |

So land value is recomputed **at most once per month, and only if something changed**; the pipeline itself is
spread over daily/idle slices with a cycle-counter time budget (each loop checks `rdtsc` every 1024/n iterations and
adapts `n`, the "TimerHistory" statics). The state starts at 0, so the first pass runs after load.

### Pipeline (`DoIdle`, a resumable 9-state machine over a `ScanningIterator`) **[confirmed]**

```text
state 0: reset tract iterator; state=1
state 1: for each tract t of the scratch grid:
             scratch[t] = TractHasLand(t) ? intrinsicMin(0x47E2C300[0]) : 0
         state=2
state 2: AddDistanceEffect(scratch, mask=scratch, maskValue=0,
                           radius = water[1], nearValue = water[0], farValue = 0)   // 0x47E2C310 {size, radius}
         minAlt = terrain(+0x60)   ; maxAlt = terrain(+0x5C)   // start values, then narrowed
         reset sum/count, ignoreZoning=false, altitude histogram (bins of 32 m) = 0;  state=3
state 3: for each tract t: alt = terrain height at tract centre
             if TractHasLand(t): minAlt=min(minAlt,alt); maxAlt=max(maxAlt,alt)
             if (ignoreZoning || TractIsZoned(t)) && TractHasLand(t):
                 sum += alt; count++; hist[clamp(alt>>5)]++
         end: if count==0 and !ignoreZoning: ignoreZoning=true; redo state 3
              avgAlt = sum/count (0 if none)
              modeAlt = clamp(bestBin*32 + 16, minAlt, maxAlt)
              SetupAltitudeEffect(); state=5
state 5: for each slope tract (8×8 cells): slope[t] = LotDeveloper.GetAreaSlope(cellRect, true);  state=4
state 4: if altitude curve 0x47E2C320 exists:
             for each tract: intrinsic[t] = min(255, scratch[t] + LookupAltitudeEffect(alt(t)))
         state=6
state 6: scratch = intrinsic (copy);  iterator = first developer in the desirability map; state=7
state 7: one developer per call: ProcessTwoGridsWithCallback(scratch, dev.GetDesirabilityMap() /*int16*/,
                                    AddDesirability, {weight, curve 0x47E2C332})
         after the last developer: total = scratch (copy); reset sum/count; state=8
state 8: for each tract with (ignoreZoning || zoned) && hasLand: sum += total[t]; count++
         averageLandValue = sum/count; HistoryWarehouse.Record(0x6A663AC8, average)
         state=9; DoEndOfSimulationCycle()  -> broadcast message 0x87CC75B3
```

Helpers **[confirmed]**:
```text
TractIsZoned(tx,tz): any cell in the S×S block has ZoneManager(+0x30)(cx,cz) != 0
TractHasLand(tx,tz): any cell in the (S+1)×(S+1) block (inclusive bounds) where terrain(+0x90)(cx,cz) is true
                     [inferred: +0x90 = "cell is dry land / above water"]
SetupAltitudeEffect():
    pivot     = min(modeAlt, avgAlt)
    lowRange  = max(1, pivot − minAlt)
    highRange = max(1, maxAlt − pivot)
    topIndex  = 100; if highRange > 100: topIndex = min(150, (highRange−100)/(terrainMax−100)*100 + 100)
LookupAltitudeEffect(alt):
    if alt < minAlt: return 0
    x = alt >= pivot ? 50 + (topIndex−50)*(alt−pivot)/highRange
                     : 50*(alt−minAlt)/lowRange
    return (uint8) curve_0x47E2C320((int)x)          // curve input is 0..~150, 50 = the "typical" altitude
AddDesirability(&v, d, ctx{weight, curve}):
    e = curve ? curve(d) : d
    v = clamp((int)(v + e*weight), 1, 255)            // note lower clamp 1, not 0
```

So: **land value = clamp(base + waterBonus(distance) + altitudeCurve(alt) + Σ_dev weight_dev × curve(desirability_dev))**.
Pollution, crime, parks etc. reach land value only *indirectly*, through each growth developer's desirability map
(computed by the BuildingDevelopment/Residential/Commercial simulators, see `development.md`). **[inferred from data flow]**

### Tuning (`0x47E2C2xx/3xx`) **[confirmed IDs; meanings from code + Maxis names]**

| property | type | use |
|---|---|---|
| `0x47E2C200` Tract Size | uint8 | tract size of the three uint8 grids (+0x7C) |
| `0x47E2C300` Intrinsic Min/Max | uint8[2] | [0] = base value on land tracts; [1] stored (+0x7E), not used by the pipeline |
| `0x47E2C301` Wealth Boundaries | uint8[2] | low/medium and medium/high thresholds for `GetLandValueType` |
| `0x47E2C310` Water Effect | uint8[2] | [0] = bonus next to water, [1] = radius in tracts (linear falloff to 0) |
| `0x47E2C320` Altitude Effect | float curve | maps altitude index 0..150 (50 = pivot) → value added |
| `0x47E2C330` Desirability IDs | uint32[] | growth-developer type IDs (looked up via `BuildingDevelopmentSim.GetDeveloper`) |
| `0x47E2C331` Desirability Factors | float[] | weight per developer, same order |
| `0x47E2C332` Desirability Curve | float curve | maps desirability (int16) → value before weighting |

### Open questions
- Exact semantics of terrain vt `+0x5C/+0x60/+0x90` (used as initial max/min altitude and "is land"). [inferred]
- The slope grid is computed but not used by the land value pipeline itself; it is exported for the developers. [inferred]

---

## 3. cSC4PollutionSimulator (`0x8990C065`): air, water, garbage, radiation

Code: Windows `0x006E2CC0–0x006E8F00`. Vtables: `cISC4PollutionSimulator` `0x00AB2630` (71 slots, matches the community
order exactly), `cIGZSerializable` `0x00AB25E8`, `cIGZMessageTarget2` `0x00AB261C`; `0x00AB2600` is a 7-slot prop-requester
interface (`GetRequestedShare`, `SelectProp`, … used for garbage props). Tuning exemplar: Utility Simulator,
instance `0xC911E35B`.

### Grids (all `cSC4SimGrid<int16>`) **[confirmed]**

Pollution type index: 0 = air, 1 = water, 2 = garbage, 3 = radiation.

| member | instance | tract | role |
|---|---|---|---|
| +0x40 / +0x44 / +0x48 / +0x4C | `0x89D5BBF0 + i` | 2 | **source** (emission) maps, edited only when occupants appear/disappear |
| +0x50 air value | `0x49D5BC40` | 2 | current air pollution (`GetAirValue`, `GetPollutionGrid(0)`) |
| +0x54 water value | `0x49D5BC41` | 2 | current water pollution |
| +0x58 garbage value | `0x49D5BC42` | **1** | uncollected garbage per cell; bit 15 = "active landfill" flag |
| +0x5C | `0x49D5BC43` | 2 | allocated but `GetPollutionGrid(3)` returns the radiation *source* grid +0x4C instead |

Radiation has no "value" layer: the source grid is the value, and `IsRadioactive(c) = radiation[c] != 0`.
`GetGarbageValue` masks off bit 15. **[confirmed]**

### Messages and schedule **[confirmed, Mac + Windows DoMessage]**

Agent levels 6 and 4 (no level 0/10). Subscribes to InsertOccupant/RemoveOccupant, `PostCityInit 0x26D31EC1`,
`IngredChangesMade 0xCA1B151C` (re-read tunables), `PreViewLevelChange 0xC6CDBF1A`.

| message | action |
|---|---|
| SimBegin | `SimulationBegin`: efficiency = curve `0x6911E2DF`(sanitation funding % × 100) / 100 |
| SimNewDay | `DoDailySimulation` |
| SimNewMonth | `DoMonthlySimulation` |
| Insert/RemoveOccupant | `DoOccupantChanged(occ, inserted)` |
| PostCityInit / view-level change | `UpdatePollutionVisualEffect` |

### Sources: `DoOccupantChanged(occ, add)` **[confirmed]**

Skipped entirely if the occupant has bool property `0x029244DB` set. Otherwise, with `sign = add ? +1 : −1`:
```text
centre[4] = prop 0x27812851 (Pollution at center: air, water, garbage, radiation)
outer[4]  = prop 0xAA5832F3 (default {0,0,0,0} from a static)
radius[4] = prop 0x68EE9764 (float, default {2.5, 1.5, 0.5, 4.0})          // in pollution tracts (2 cells)
// garbage: added to every cell of the lot footprint (rounded through city +0x17C position->cell)
for each cell c in lot: garbageSource[tract(c)] += sign*centre[2]          // so a tract gets centre*cellsInTract
for i in 0..3: if radius[i] > 0:
    AddRadiusEffect(source[i], tract(lotCentre), radius[i], sign*centre[i], sign*outer[i])
// then: register/unregister the occupant in the incinerator (0xC8EBD131), water-treatment (0xA8EBCF0C),
// waste-to-energy (0x48EBD0B1) and recycling (0x88ED1059) lists, reset their used-capacity props, and
// add/remove the sanitation budget line item.
```
`AddRadiusEffect(grid, centre(tx,tz), r, c, o)` **[confirmed]**: for every tract with Euclidean distance
`d < r` (in tracts; table lookup when `r < 16`, else `sqrt`): `v += round(c − d*(c−o)/r)`, clamped to the element range.
So a building's emission is a **linear cone** that is added on insert and subtracted on removal; there is **no
diffusion step** — the "spread" is entirely this precomputed cone. [confirmed]

### Daily update: relaxation toward the sources **[confirmed]**

`tractsPerDay = countX*countZ / (mUpdatePeriod/2)` with `mUpdatePeriod = 30`, so the whole map is visited once every
~15 days. Tracts are visited in a shuffled order (`RandomIterator`), `(x,z)` = top-left cell of a 2×2-cell block.

```text
mTraffic = Ordinance.GetCumulativeFactor(0x8A67E378, 0)      // ordinance multipliers [inferred names]
mAir     = ...(0x08F79B8E, 0); mWater = ...(0xE8F79C8B, 0); mGarb = ...(0xE8F79C90, 0)
zoneMul[z] = {...(0x8A67E373,z), ...(0x8A67E374,z), ...(0x8A67E376,z)} for z in 0..15
repeat tractsPerDay times:
    a = max(0,airSrc[x,z])*mAir;  w = max(0,waterSrc)*mWater;  g = max(0,garbSrc)*mGarb;  r = max(0,radSrc)
    over the 4 cells: average zoneMul of cells whose zone != 14 (landfill); multiply a,w,g by it
    if any of the 4 cells is zoned (zone != 0, != 14): developedBlocks++
    for each of the 4 cells c:
        a += AirPollutingTrafficMap(c) * mTrafficAirFactor(8.0, prop 0x69501944) * mTraffic
        if zone(c) != landfill: garbage disposal for c (below)
    water treatment: treated = min(remaining WTP capacity, w) (pro-rated by last cycle's ratio); w -= treated
    stats += a, w, r
    target air = clamp(a,0,32767):  air[x,z]   moves toward it by at most kMaxAirPollutionDelta   (50, 0x6A406201)
    target water = clamp(w,0,32767): water[x,z] moves toward it by at most kMaxWaterPollutionDelta (50, 0x8A40620B)
        special case: if water would reach 0 but any 4-neighbour block has water >= 2, it is set to 1
    if the iterator is exhausted: Reset() and EndOfCycle()
```

Garbage disposal per cell **[confirmed, simplified]**: `produced = g/zonedCells`; a share is recycled (recycling-centre
effect %), then `produced + garbage already lying on the cell` is offered to `eatGarbage` (incinerators, waste-to-energy
plants, landfill/export via neighbour deals `0x4773FFE4/5`); what is not eaten is written back to the cell's garbage
value (capped at 32767), or pushed onto the city "garbage supply" valve (capacity `kMaxGarbageOnValve` 655340,
prop `0x4A652679`).

### End of cycle (every ~15 days) and monthly **[confirmed]**

- `EndOfCycle`: recycling effect recomputed; averages over developed blocks → history (`0x0A1100E8` air,
  `0xEA110130` water, `0xEA110134` garbage, `0xEA110137` radiation, `0xEA110138` combined); posts map-changed
  messages for each layer that changed (`0xDA6DD35D` radiation, `0xFA6DD0BB` air, `0x5A6DD1AA` water,
  `0xDA6DD2CB` garbage), and `0xEA418D8F`.
- `DoMonthlySimulation`: settles neighbour garbage deals, charges `garbage × costPerUnit (0xAA1F8AB7)` to the
  sanitation budget, rolls monthly garbage statistics into "previous month", ages incinerators/WTE/water plants
  (`AgeOccupantList`), refreshes the visual effects.

### Tuning (Utility Simulator exemplar `0xC911E35B`) **[confirmed IDs]**

| property | default (code) | meaning |
|---|---|---|
| `0x69501944` | 8.0 | traffic → air pollution factor |
| `0x49134BD4` / `0x49134BDD` / `0x09134BE2` / `0x89135044` | 27000 / 27000 / 32000 / 32000 | max garbage/air/water/radiation level for UI |
| `0x6A406201` / `0x8A40620B` | 50 / 50 | max air / water change per visit |
| `0x0913519B` / `0xA91351A6` / `0x091351AE` / `0x291351B5` | 16000/16000/16000/1 | "polluted" thresholds garbage/air/water/radiation (`IsAirPolluted` etc. = value >= threshold) |
| `0x09135479` | 5000 | garbage abduction amount (UFO event) |
| `0x09135482` | 0.125 | garbage scaling factor |
| `0xAA1F8AB7` | — | garbage service cost per unit |
| `0x4A652679` / `0x4A65267A` | 655340 / 32767 | max garbage on valve / landfill tile capacity |
| curves `0x69A437F6`, `0x0A10C772..775`, `0x8C19C3E9`, `0x0911E2E8/9`, `0x6911E2DF` | — | prop request, visual effect strengths, garbage transport vs landfill tiles, funding→decay, funding→efficiency |

Building-side properties: `0x27812851` pollution at centre (int[4]), `0xAA5832F3` outer values, `0x68EE9764` radii (float[4]).

### Interactions
- Reads traffic `GetAirPollutingTrafficMap` (uint8), zone types, ordinance factors, neighbour deals, budget.
- Feeds land value only indirectly (developers' desirability maps read the pollution grids). [inferred]
- Plumbing reads the water-pollution grid to shut down polluted water pumps (`GetNumPollutedWaterSources`). [inferred]

---

## 4. cSC4PowerSimulator (`0x0990C005`)

Code: Windows `0x006EABB0–0x006F0900`. Vtables: `cISC4PowerSimulator` `0x00AB28B8` (52 slots; MSVC reverses the
`GetMonthlyConsumption` and `GetPowerPlantCount` overload pairs, and folds the two `GetPoweredGrid` overloads),
`cIGZSerializable` `0x00AB288C`, `cIGZMessageTarget2` `0x00AB28A4`. Tuning: Utility Simulator exemplar `0xC911E35B`.

### Data **[confirmed]**

Power is computed **per cell** with four 1-bit-per-cell bitmaps (`(CellCountX*CellCountZ+31)/32` words each):

| member | bitmap | meaning |
|---|---|---|
| +0x2CC | wire | cell conducts power |
| +0x2C8 | mark | scratch: the connected component being flood-filled |
| +0x2D4 | powered (next) | built during the current pass |
| +0x2D8 | powered (current) | what `IsPowered(x,z)` reads; swapped with "next" at the end of a pass |

Plus `+0x84` `vector<vector<int16>>` **consumption per cell** (`GetMonthlyConsumption(x,z)`), and the published
`cSC4SimGrid<uint8>` power map at `+0x80` (tract 1, instance `0x49D5BC86`, `GetPoweredGrid`): `0` = not wired,
`1` = wired but unpowered, `2` = powered.

**Wire bits** (`UpdateWireBits`, `DoOccupantInserted`) **[confirmed]**: a cell is wired if it lies within
`R = 0xA911E2F1` (default **2** cells, "Power radiation radius") of
- any building occupant (`0x278128A0`) footprint,
- a power pole (`0x2890D4DE`) or the `0x0990B2EA` occupant (power line segment [inferred]),
- a network occupant (`0xC772BF98`) whose network flags include `0x40000` (power-carrying network) — its own cell only,
- any **zoned** cell (ZoneManager zone != 0).

**Consumption/production** (`DoOccupantInserted`) **[confirmed]**: for a building, `0x27812854` (power consumed) is
added to the consumption cell at the building's centre; if `0x27812852` (power produced) > consumed, the building is
appended to the plant list (`0x27812853` = plant type, counted per type).

### Schedule **[confirmed]**

Agent levels 6, 4, 10. Subscribes to Insert/RemoveOccupant, ZoningChange, IngredChangesMade, TerrainHeightsChanged,
`OrdinanceUpdated 0xC2C09554`, `StateChanged 0xE8BDA2E7`, `0xEA418D8F`, `ConnectionsReady 0x6AC284F3`, PostCityInit.

- `ConnectionsReady` sets the "enabled" flag; nothing runs before it.
- SimNewDay / SimIdle → `DoIdle`; SimNewMonth → `DoMonthlySimulation` (sets `recompute` and `agePlants`, rolls the
  12-month consumption ring buffer, refreshes the conservation factor, applies pole funding).
- Occupant insert/remove, zoning and terrain changes set `recompute` (and patch wire bits for the changed rect).

### Power-flow pass (`DoIdle`, one state per call) **[confirmed]**

```text
state 4 (idle): if recompute: recompute=false; state=0
state 0: roll monthly ring slot; reset neighbour import/export deals; zero counters; mark all plants unvisited;
         clear powered(next); state=1
state 1: for each plant p not yet visited: UpdatePowerGrid(cell(p))      // one call per connected component
         compute generation cost per plant (funding curve 0x0911E2E7); if agePlants: AgePowerPlant(each)
         state=2
state 2: for each neighbour power *import* deal still owed: UpdatePowerGrid(connection cell)   // islands fed only by neighbours
         swap(powered current, powered next); UpdatePoweredLots(); if anything changed post 0x6A07C2EE; state=3
state 3: history (0xAA11F7CE produced, 0xAA11F7D2.., per-type 0xAA120D5E..6D), deficit %, update neighbour connections;
         post 0xEA418D8F; state=4

UpdatePowerGrid(seed):
    mark = {seed}; Bit1_SelectionGrow(mark, wire, maxPasses=600)          // 4-connected flood fill through wired cells
    supply  = Σ CurrentPowerProduced(p) for unvisited plants whose footprint touches `mark`; mark them visited
              (CurrentPowerProduced = 0x27812852 × efficiency(funding, age, strike))
    imports = Σ (deal.capacity − deal.used) for import deals (0x4773FFE0) connected to this component
    avail   = (supply*1.1 + imports) / conservationFactor            // factor = Ordinance.GetCumulativeFactor(0x0911E117)
    exports (0x4773FFE1) connected to this component take what they are owed first, up to avail
    for each 8×8-cell block intersecting `mark`, in RANDOM order:
        need = Σ consumption[c] over marked cells c in the block (only > 0)
        if avail > 0 and need <= avail: powered(next) |= mark ∩ block;  avail -= need;  poweredCells += n
        else: shortfall = true                                         // the whole block browns out
    used = (supply*1.1 + imports) − (shortfall ? 0 : avail*factor)
    imports are charged for `used` first; the rest is attributed to plants: each plant's 0x08D9763D (power used)
    = produced × used/max(used,supply)  (MarkPowerPlantsAsVisited)
```

Consequences: power is **all-or-nothing per 8×8-cell block**, blocks are served in random order, so in a deficit a
random subset of blocks goes dark; and plants can deliver 110% of rated output. The ring buffer at `+0x90` keeps
12 months × 10 plant types of production for the graphs. [confirmed]

`UpdatePoweredLots` XORs the new and old powered bitmaps; for every 8×8 block with a change it rewrites the uint8
power map for the changed cells and posts `kMessageTractPowerChanged 0x8A07AE00(x1,z1,x2,z2)`. Buildings learn their
powered state from these messages / `IsOccupantPowered`. [confirmed / inferred for the consumer side]

### Tuning (Utility Simulator exemplar `0xC911E35B`) **[confirmed IDs and defaults]**

| property | default | meaning |
|---|---|---|
| `0xA911E2F1` | 2 | power radiation radius (cells) |
| `0xE911E2FC` | 110 (%) → 1.1 | power plant max production ratio (stored /100 at +0x2B8) |
| `0x29121E8B` | 60 → 0.6 | min acceptable funding % |
| `0xE911E2D1` | 120 → 1.2 | max allowed funding % |
| `0x09122027` | 50 → 0.5 | efficiency during strike |
| `0x2912202E` | 10 | strike duration (months) |
| `0x49122036` | 90 → 0.9 | funding needed to end strike |
| `0x6911E2DF` | {(0,0),(100,100),(120,110)} | funding → efficiency curve |
| `0x0911E2E6` | {(0,20),(100,100)} | strike response curve |
| `0x0911E2E8` / `0x0911E2E9` | {(0,1),(1,1)} | funding → decay rate multipliers |
| `0xAA713782` | {(0,−0.02),(120,0.01)} | funding → pole decay/repair |

Note: the decompiled `UpdatePowerGrid` uses the literal 1.1 (Windows: `fmul [0x00AB2888]` = −1.1, then subtracts), not
the `0xE911E2FC` field, so that property may only affect UI/efficiency. [confirmed literal; effect of the property inferred]

Building properties: `0x27812852` power produced, `0x27812854` power consumed, `0x27812853` plant type,
`0x08D9763D` current power used (written by the sim).

---

## 5. cSC4PlumbingSimulator (`0x0990C075`)

Code: Windows `0x006DA3B0–0x006DF600`. Vtables: `cISC4PlumbingSimulator` `0x00AB2228`, `cIGZSerializable` `0x00AB21FC`,
`cIGZMessageTarget2` `0x00AB2214`. Tuning: Utility Simulator exemplar `0xC911E35B`.

### Data **[confirmed]**

| member | type | role |
|---|---|---|
| +0x88 (stride +0x8C) | raw `uint8[cellsZ][cellsX]` | per-cell flags: bits 0–3 = pipe connections to the 4 neighbours, `0x10` = pipe, `0x20` = water producer cell, `0x40` = consuming building cell |
| +0x94 | 1-bit bitmap | "visited" during a BFS |
| +0x30 | SimGrid<uint8>, tract 1, `0x49D5BB8C` | working mark map (`8` = pipe, bit 0 = watered) |
| +0x34 | SimGrid<uint8>, tract 1, `0x49D5BBA1` | published watered map (`GetWateredGrid`, `GetSupplyValueMapPtr`; `IsCellWatered = v & 1`) |
| +0x38 | `deque<QueuePoint>` | BFS queue; a QueuePoint packs `x | z<<8` (byte coordinates) |

Water producers (pumps/towers/treatment) are kept in a list (+0xBC), each with capacity, "visited" and "assigned" flags.

### Schedule **[confirmed]**

Agent levels 6, 4, 10. SimNewDay → `DoIdle(5)`, SimIdle → `DoIdle(data3)`, SimNewMonth → `DoMonthlySimulation`
(sets the recompute flags and applies pipe funding/decay). `ConnectionsReady 0x6AC284F3` enables the sim;
`0xEA1F0DCD` clears a cell's pipe-connection bits (`flags &= 0xE0`) and requests a recompute; neighbour water deals
(`0x4773FFE2` import / `0x4773FFE3` export) changing also force a restart (state 9). Occupant insert/remove update
the flag map (skipped when the occupant has `0x029244DB`).

### Algorithm (`DoIdle`, 9-state pipeline) **[confirmed structure; details partly inferred]**

```text
state 0: read ordinance consumption factor (0xA8F4EB0C) -> +0xEC; reset neighbour deals; state=1
state 1: UpdateWaterProduction(): each producer's output = capacity × funding/age efficiency; pumps are disabled
         if the water-pollution value at their cell exceeds 0xAA161F81, or reduced (0x68F5436D %) when farther
         than 0x68F5436B/0x68F5436C cells from fresh/salt water [inferred from tunable names + NearestWater]
state 2: markMap[c] = (flags[c] & PIPE) ? 8 : 0 for all cells; clear producer "visited"; state=3
state 3: take the next unvisited producer (or a neighbour import connection): BFS through pipe cells using the
         4 connection bits; every producer reached adds its capacity to `supply`; state=4
state 4: supply -= exports owed to connected neighbours;
         ring-BFS outward from the network: each pass of the queue is one ring; cells with the BUILDING flag consume
         BuildingConsumption(c) × consumptionFactor in BFS order (closest to the source first) until supply is
         exhausted; every reached cell gets markMap |= 1; neighbours (8-connected for non-pipe cells) are queued;
         stop after `pipeEffectRadius` rings (0x68F5436A, ctor default 7)
         set each producer's 0x08D9763D (water used) = capacity × used/supply; back to state 3 for the next network
         (when none left: state=5)
state 5: pipes' watered state and neighbour-connection states are set from markMap; state=6
state 6/7: time-sliced (cycle-budgeted) passes: buildings whose cells are watered get FillGridAtOccupant on the
         mark map; then the mark map is published to the +0x34 grid
state 8: AgeWaterProducer for each producer (funding curves), statistics/history (0x8A78AB06 ...), post 0x6946DC34
         and 0xEA418D8F; idle until the next recompute request
```

Key difference from power: water is distributed **nearest-first along the pipe network** (BFS order) rather than in
random 8×8 blocks, and pipes only water cells within `pipeEffectRadius` rings of a pipe. [confirmed for the ordering
and the ring limit; the exact neighbour rule for buildings is inferred]

### Tuning **[confirmed IDs]**

| property | meaning |
|---|---|
| `0x68F5436A` | water pipe effect radius (cells/rings; ctor default 7) |
| `0x68F5436B` / `0x68F5436C` | fresh / salt water pump reach |
| `0x68F5436D` | fresh water distance loss % |
| `0xAA161F81` | max acceptable pumped water pollution |
| `0x8A235C56` | pipe burst probability per month |
| `0xEA4025A9` | water department max funding |
| `0x6A410BF2` | water funding → efficiency curve |
| `0xCA7138C4` | funding → pipe decay/repair |
| `0x8A84778F` | monthly cost per pipe |
| `0x0911E2E8/9` | funding → decay rate multipliers |

---

## 6. cSC4CrimeSimulator (`0x8990C09A`)

Code: Windows `0x0069F1A0–0x006A2300`. Vtables: `cISC4CrimeSimulator` `0x00AB0DC8`, `cIGZSerializable` `0x00AB0D84`,
`cIGZMessageTarget2` `0x00AB0DB8` (`0x00AB0D9C` is the prop-requester interface for crime props).
Tuning: Crime Simulator exemplar instance `0xC8E8F6C5`.

### Grids **[confirmed]**

| member | getter | type | tract | instance | role |
|---|---|---|---|---|---|
| +0x5C | `GetCrimeGrid` | Uint8 | `0x47E2C200` (crime exemplar) | `0x89D5B895` | accumulated crime incidents |
| +0x60 | — | Uint8 | same | `0x2A5BA1B3` | building "crime effect" field from prop `0xCA5B9306` |

`DoOccupantChanged(occ, add)`: if the occupant has `0xCA5B9306 = {magnitude, radiusCells}`,
`AddRadiusEffect(effectGrid, tract(occ), radiusCells/tractSize, ±magnitude, 0)` (linear cone, clamped 0..255).
Note the radius *is* divided by the tract size here, unlike pollution. **[confirmed]**

### Schedule **[confirmed]**

Agent levels 4, 6, 10. SimNewDay/SimIdle → `DoIdle` (one state per call); SimNewMonth → if `state == 6` (done):
`state = 0`. `OrdinanceUpdated` re-reads the ordinance crime factor `0x28ED0380`; `0xC53D10AA` (exemplar reload) re-reads
tunables or crime types.

### Monthly pipeline (`DoIdle`) **[confirmed]**

```text
state 0: wait for city-ready flag; state=1
state 1: DoCensus: for each wealth w in {R$, R$$, R$$$}:
             for each tract: pop[w][ (int)(EQ/50) ] += residentialPopulation_w   // EQ buckets 0-49,50-99,100-149,150+
         CalculateEconomicFactors: for each wealth, over a static list of demand IDs:
             jobless[w] = 2*(cur−min)/(max−min) − 1        // in [−1, 1], from cISC4Demand values
state 2: criminalBase = Σ_w [ Σ_b pop[w][b] × EQcurve_w(b*25)   (0x8A0CBAA4+w)
                             + Σ_b pop[w][b] × joblessCurve_w(jobless[w]) ]   (0x8A0CBAA7+w)
state 3: free = criminalBase − police.jailed (vt+0x74)
         crimes = (free > 0 ? crimesCurve(free) (0x8A0CBAAA) : 0) + police.unjailed (vt+0x6C) × criminalDecay (0x8A0CBAAD)
         crimes = round(crimes × Ordinance.GetCumulativeFactor(0x28ED0380))
state 4: crimeGrid[t] = (uint8)(crimeGrid[t] × gridDecay)          // 0x8A0CBAAC, default 0.5 per month
         clear last month's incidents
         AddCrimes(crimes): repeat `crimes` times:
             lot = LotManager.RandomLot()
             p = attenuationCurve(avg(police grid over lot) − avg(crimeEffect over lot))   // 0x8A0CBAAB, default (0,0)-(255,1)
             if random() >= p and AddCrimeEvent(lot) = s > 0:     // s = crime type's severity byte
                 crimeGrid[t] = min(255, crimeGrid[t] + s) for every tract under the lot; track hotspot
         Police.ProcessMonthlyCrimeStats(); ComputeAverageCrime()
state 5: post 0x88EA4DFF (crime map changed); state=6
```

`AddCrimeEvent` picks a crime type (loaded from crime-type exemplars) whose 4 bitmasks match the lot's bits
(`GetLotBits`: zone/wealth/purpose/density [inferred]) with probability ∝ the type's weight, records a `tCrimeEvent`,
optionally starts a fire (arson types, via the fire disaster manager) and posts `0x0A159D95`. **[confirmed]**

So crime is **not a diffused field**: it is an event count stamped onto the lots where crimes happened, halved each
month, and the probability of a crime at a lot depends on police coverage minus building crime effects there.

### Tuning (Crime exemplar `0xC8E8F6C5`) **[confirmed IDs]**

| property | default | meaning |
|---|---|---|
| `0x47E2C200` | — | crime grid tract size |
| `0x8A0CBAA4/5/6` | — | R$/R$$/R$$$ EQ → base criminality |
| `0x8A0CBAA7/8/9` | — | R$/R$$/R$$$ jobless ratio → criminality |
| `0x8A0CBAAA` | (0,0)–(1e6,50000) | criminality → crimes per month |
| `0x8A0CBAAB` | (0,0)–(255,1) | (police − crime effect) → probability that a crime attempt is stopped |
| `0x8A0CBAAC` | 0.5 | crime grid decay per month |
| `0x8A0CBAAD` | 1.0 | fraction of unjailed criminals who reoffend |
| `0x49A34301` | — | crime prop request curve |
Building property: `0xCA5B9306` crime effect `{magnitude, radius}`.
