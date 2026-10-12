# Spec: the weather subsystem (`cISC4WeatherSimulator`)

Status: **implementation spec**, clean-room input. This document describes *behaviour*, not code. It was
written by the analysis team from observing the original program and its data files. Implementers must work from
this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose.

Related specs: [foundations.md](foundations.md) (streams §2, random numbers §6, messages §5),
[simulator.md](simulator.md) (agents and calendar messages).

## 1. Role

The weather simulator keeps a coarse climate model of the city:

1. a monthly **ambient temperature** and **ambient moisture** that follow the seasons, and a **wind** that changes
   each month and is interpolated day by day;
2. the **season** (winter, spring, summer, fall), announced when it changes;
3. a per-cell **temperature grid** refreshed weekly (it has no reader in the shipped game; it is only saved);
4. a per-cell **moisture grid** refreshed weekly from terrain shape, water, wind and a random base pattern. Flora
   and the map view read it.

Weather has **no** effect on money, demand, traffic or neighbour deals.

| Class | CLSID | Interfaces (IID) |
|---|---|---|
| weather simulator | `0x2990C1BC` | `cISC4WeatherSimulator` (`0xC6C66245`), `cIGZMessageTarget2` (`0x452294AA`), `cIGZMessageTarget` (`0xE98B2F57`, its `DoMessage` returns true and does nothing), `cIGZSerializable` (`0xE4FDA3D4`), `cIGZUnknown` (`1`) |

The city creates it with `GZCOM()->GetClassObject(0x2990C1BC, 0xC6C66245, &p)` and stores it in reference slot 38
of its save record (foundations §3.6). Other code reaches it through `cISC4City::GetWeatherSimulator()`.

## 2. Grid and coordinates

- One weather **cell** is 16 × 16 world units (metres): one city tile. `cellCountX = trunc(city.SizeX() × 0.0625)`,
  likewise Z. Cities are square (64, 128 or 256 cells).
- Grids are stored **x-major**: `grid[x][z]`, x = 0 .. countX − 1.
- `cityCellsPerCellX = trunc(16 / city.CellWidthX())` and `cityCellsPerCellZ = trunc(16 / city.CellWidthZ())`
  (both 1 in the shipped game). The algorithms below are written for the value 1; §6.5 notes where the original
  supports larger values.
- Terrain samples use **vertex** coordinates equal to the cell indices (`GetAltitudeAtVertex(x, z)`,
  `GetVertexNormal(x, z)`, `CellHasWater(x, z)`).

## 3. State

The order of this table is the save order (§10). "Tuning" fields are overwritten from the tuning exemplar at
every `Init` (§8) even after a load.

| Field | Constructor value | Meaning |
|---|---|---|
| countX, countZ (s32) | 0 | grid size (§2) |
| cellWidthX, cellWidthZ (f32) | 0 | 16 |
| invCellWidthX, invCellWidthZ (f32) | 0 | 0.0625 |
| sizeX, sizeZ (f32) | 0 | city size in world units |
| cityCellsPerCellX/Z (s32) | 1, 1 | §2 |
| subdivX, subdivZ (s32) | 8, 8 | tuning `0xA7607D70/71`; weekly cells = cells / subdivX (§5.2); subdivZ is unused |
| blendRadius (s32) | 1 | tuning `0xA7607D72` (§6.3) |
| lastSeason (s32) | −1 | |
| baseTemp (f32) | 24 | tuning `0xA7607D73` |
| tempAmpA, tempAmpB (f32) | 0.5, 0.5 | tuning `0xA7607D74/75` |
| ambientTemp (f32) | 20 | §6.1 |
| temperature grid (u8) | — | §6.3 |
| seaMask grid (bool) | — | §6.6 |
| waterRects: list of rectangles | empty | §6.6 |
| cell iterator | — | §6.7 |
| cellsPerWeek (s32) | 1 | §5.2 |
| seaRadius (s32) | 6 | tuning `0xA7607D76` |
| seaWeight (f32) | 0.95 | tuning `0xA7607D77` |
| heightFactor (f32) | 0.0035 | tuning `0xA7607D78` |
| windFactor (f32) | 0.25 | tuning `0xA7607D79` |
| moisture grid (u8) | — | §6.5 |
| moistureBase grid (u8) | — | §6.4 |
| ambientMoisture (f32) | — | §6.1 |
| baseMoisture (f32) | 0.5 | tuning `0xA7607D7A` |
| moistAmpA, moistAmpB (f32) | 0.5, 0.4 | tuning `0xA7607D7B/7C` |
| slopeK (f32) | 128 | tuning `0xA7607D7D` × 255 |
| highMoisture (f32) | 0.12 | tuning `0xA7607D7E` |
| curvatureThreshold (f32) | 4.0 | tuning `0xA7607D7F` |
| tuning80 (u32) | 64 | tuning `0xA7607D80`; unused |
| defaultWindSpeed (f32), defaultWindDir (2 × f32) | 3, (0, 1) | tuning `0xA7607D86`, `0xA7607D87/88` |
| windSpeed (f32), windDir (2 × f32) | 3, (1, 0) | the current wind |
| prevWindSpeed, prevWindDir | 3, (1, 0) | the previous month's target |
| targetWindSpeed, targetWindDir | 3, (1, 0) | this month's target |
| rng: `cRZRandom` | seeded from the wall clock (`0xFFFFFFFF`) | foundations §6.1 |
| windMonth (s32) | −1 | the season month of the current target |
| windSpread (f32), not saved | 0.25 | `tan(0xA7607D89 × π / 360) × 0.5` |
| tuning81 (u32), not saved | 15 | `0xA7607D81`; unused |
| initialized, loadedFromSave, postInitDone (bool), not saved | false | §8 |

## 4. `cISC4WeatherSimulator` methods (SDK declaration order)

| Method | Behaviour |
|---|---|
| `Init()` / `Shutdown()` | §8. |
| `DoPostCityInit()` | If `postInitDone` is false: run `DoCompleteSimulation` (§5.5) and set it. Returns true. |
| `CellWidthX()` / `CellWidthZ()` | 16 (the stored widths). |
| `CellCountX()` / `CellCountZ()` | countX / countZ. |
| `PositionToCell(x, z, &cx, &cz)` | `cx := trunc(x / cellWidthX)`, `cz := trunc(z / cellWidthZ)`. The original returns no meaningful value; *[choice]* return 0. |
| `CellToPosition(cx, cz, &x, &z)` | `x := cx × cellWidthX`, `z := cz × cellWidthZ`. |
| `CellToStandardCityCell(cx, cz, &sx, &sz)` | `sx := cx × cityCellsPerCellX`; **`sz := cx × cityCellsPerCellZ`** *[quirk, compat]*: the z output uses the x input. |
| `StandardCityCellToCell(sx, sz, &cx, &cz)` | `cx := sx / cityCellsPerCellX`, `cz := sz / cityCellsPerCellZ` (integer division). |
| `GetCurrentSeason()` | `(GetCurrentSeasonMonth() − 1) / 3`: 0 = Winter (months 1–3), 1 = Spring, 2 = Summer, 3 = Fall. |
| `GetCurrentSeasonMonth()` | `m :=` the simulation date's month. If `city.GetWorldHemisphere() ≠ 1`, `m := (m + 5) mod 12 + 1` (shifts by six months). |
| `UpdateMoisture(rect)` | Only if the terrain is known: convert the world-unit rectangle to cells with `x0' = x0 / 16`, `z0' = z0 / 16`, `x1' = (x1 + 15) / 16`, `z1' = (z1 + 15) / 16` (C integer division, truncation toward zero) and run `SimulateMoisture` on it (§6.5). |
| `GetMoistureValue(x, z)` | 0 if not initialized. Else clamp world `x` to `[0, sizeX)` (values ≥ sizeX become `sizeX − 1`, negatives 0), likewise z; return `moisture[floor(x/16)][floor(z/16)]`. |
| `GetMoistureValueMapPtr(out)` | Writes the **address of the moisture grid object** into the caller's pointer slot *[compat]*: see §11.2. |
| `GetMoistureType(x, z)` | `GetMoistureValue(x, z) >> 4` (0 .. 15). |
| `SetManualMoistureAddition(x0, z0, x1, z1, v)` | §6.4.1. |
| `GetHumidity(x, z)` | `baseMoisture + (1 − baseMoisture) × 0.5`. Position and season are ignored *[compat]*; with shipped data 0.6. |
| `GetAmbientWind(&v)` | Returns `windSpeed`. If the pointer is non-null, `v := windDir × windSpeed`. |
| `GetWindAtCell(x, z, &v)` | `GetAmbientWind(&v)`; x and z are ignored. |

## 5. Per-message behaviour

The simulator registers one agent at levels **6 (month), 5 (week) and 4 (day)**, name `"cSC4WeatherSimulator"`,
flags 2 (runs only in an established city). Each call finishes in one step. It also subscribes to PostCityInit.

### 5.1 SimNewDay `0x66956814`

Let `d` be the calendar date of data1 (Julian day). If `d.month == windMonth`:

```
t   := (d.dayOfMonth − 1) / 30                       (float)
v   := lerp(prevWindSpeed × prevWindDir, targetWindSpeed × targetWindDir, t)
windDir := v / |v|   if |v| > 0, else unchanged (v itself, which is zero)
windSpeed := prevWindSpeed + (targetWindSpeed − prevWindSpeed) × t
```

*[quirk]* `windMonth` is a **season** month (§5.3). In a city whose hemisphere value is not 1 it never equals the
calendar month, so the daily interpolation never runs there and the wind changes only when the season changes.

### 5.2 SimNewWeek `0x66956815` — `DoWeeklySimulation`

```
n := cellsPerWeek;  cycled := false
while n ≠ 0:
    if iterator.next(x, z): SimulateCellTemperature(x, z); n −= 1
    else: iterator.reset(shuffle = true); post WeatherCycleCompleted 0x6716A4E6 (void1 = IGZUnknown = this); cycled := true
SimulateMoisture(0, 0, countX, countZ)
return cycled
```

With the shipped tuning `subdivX = 1`, `cellsPerWeek` equals the number of cells: every cell is refreshed every
week, and from the second week on the cycle message is posted once per week.

### 5.3 SimNewMonth `0x66956816` — `DoMonthlySimulation(month)` (month = data3)

1. If `month == 1` and the terrain is known: rebuild `waterRects` (§6.6), drop the rectangles that do not touch
   the city edge, and rebuild `seaMask` (§6.6).
2. `GetNewCurrentAmbientValues(GetCurrentSeasonMonth())` (§6.1, §6.2). Note: the **season** month, not `month`.
3. `s := GetCurrentSeason()`. If `s ≠ lastSeason`: `lastSeason := s`; the current wind becomes the defaults
   (`windSpeed := defaultWindSpeed`, `windDir := defaultWindDir`); post SeasonChanged `0x6716A4E5`
   (void1 = IGZUnknown = this, data2 = s). A new city starts with `lastSeason = −1`, so its first month-end
   always posts the message; a loaded city keeps its saved `lastSeason`.

### 5.4 PostCityInit `0x26D31EC1`

Calls `DoPostCityInit()` (§4).

### 5.5 `DoCompleteSimulation`

`DoMonthlySimulation(1)` (so the water list is always rebuilt), then `DoWeeklySimulation` repeatedly until it
reports a completed cycle. With the shipped tuning this is two full temperature passes and two moisture passes.
It runs once per object lifetime, on every city load and every new city, because `postInitDone` is not saved.

## 6. Algorithms

### 6.1 Ambient values (`GetNewCurrentAmbientValues(m)`) *[compat]*

```
ambientTemp     := baseTemp × (1 + cos(2π(m − 8)/12) × tempAmpA × tempAmpB)        (peak in month 8)
ambientMoisture := baseMoisture × (1 + cos(2π(m − 1)/12) × moistAmpA × moistAmpB)  (peak in month 1)
```

followed by the wind update of §6.2 and `windMonth := m`. Shipped values: month 8 → 65 × 1.25 = 81.25,
month 2 → 65 × 0.75 = 48.75; moisture month 1 → 0.24, month 7 → 0.16.

### 6.2 Monthly wind target *[compat]*

```
prevWindSpeed, prevWindDir := targetWindSpeed, targetWindDir
targetWindSpeed := rng.RandomDoubleRangeUniform(2.0, 4.0)
edges := [ (W, ( 1, 0)),    W = number of z in 0..countZ−1 with terrain.CellHasWater(0, z)
           (N, ( 0, 1)),    N = number of x in 0..countX−1 with CellHasWater(x, 0)
           (E, (−1, 0)),    E = number of z with CellHasWater(countX−1, z)
           (S, ( 0,−1)) ]   S = number of x with CellHasWater(x, countZ−1)
base := the direction of the LARGEST element under lexicographic order (count, dirX, dirZ)
repeat: s := windSpread × rng.RandomDoubleGaussian()  until |s| ≤ 1
c := sqrt(1 − s²)
targetWindDir := (c·base.x − s·base.z,  s·base.x + c·base.z)       (rotation by asin(s))
```

The base direction points from the edge with the most water into the city (an onshore wind). With no water at any
edge the tie-break picks `(1, 0)`. RNG draws per month: one `RandomDoubleRangeUniform` (double path), then one or
more `RandomDoubleGaussian` (foundations §6.1; the Gaussian caches its second value).

### 6.3 Cell temperature (`SimulateCellTemperature(x, z)`) *[compat]*

```
T := ambientTemp + 273
if seaMask[x][z]: T += (baseTemp − ambientTemp) × seaWeight
T := T − terrain.GetAltitudeAtVertex(x, z) × heightFactor − GetWindAtCell(x, z) × windFactor − 273
T := (T < −15) ? 0 : min(T, 45) + 15                   (0 .. 60)
value := roundHalfEven(T × 4.25)                       (0 .. 255)
WriteTemperature(x, z, value)
```

`WriteTemperature`: if `blendRadius < 1`, `temperature[x][z] := value`. Otherwise, for every cell `(i, j)` with
`x − r ≤ i < x + r` and `z − r ≤ j < z + r` inside the grid: `temperature[i][j] := (temperature[i][j] + value) / 2`
(integer division). With the shipped `r = 1` this blends the 2 × 2 block whose lower-right corner is `(x, z)`.
Because the visit order is random (§6.7), the exact grid is not reproducible without the original's C-runtime
random state (§7).

### 6.4 Moisture base pattern (`GenerateMoistureVariation`, new cities only) *[compat]*

```
(lo, hi) := region.GetWaterPrefs()  (u8, u8; defaults 60 and 100 when there is no region)
v := (lo + hi) >> 1
for x in 0..countX−1:            # v carries over from row to row
  for z in 0..countZ−1:
    p := (x > 0) ? (v + moistureBase[x−1][z]) >> 1 : v
    r := rng.RandomSint32RangeGaussianFast(−10, 10)
    v := (p + r < lo) ? lo : (p + r > hi) ? hi : p + r
    moistureBase[x][z] := v
```

#### 6.4.1 `SetManualMoistureAddition(x0, z0, x1, z1, v)`

```
b  := (v < 0) ? 0 : (v ≤ 255 ? v × 1.275 : 325.125);  byte := roundHalfEven(b) mod 256     [quirk: wraps above 200]
xa := trunc(x0 / 16); xb := trunc(x1 / 16); za, zb likewise (world units → cells)
if xa < 0: xa := 0 (xb is NOT clamped) else xb := min(xb, countX − 1)
if za < 0: za := 0 (zb is NOT clamped) else zb := min(zb, countZ − 1)
moistureBase[i][j] := byte  for xa ≤ i < xb, za ≤ j < zb
```

*[quirk]* Upper bounds are exclusive, so the last row/column is never written when clamped. *[choice]* FreeCity4
also clamps the unclamped bound to the grid (the original would write out of range). The effect appears at the
next `SimulateMoisture`. No caller was found in the shipped game.

### 6.5 Moisture (`SimulateMoisture(rect)`) *[compat]*

The rectangle is half-open `[x0, x1) × [z0, z1)` in cells. Let `A` be it clamped to the grid, `A2` the rectangle
grown by 2 cells on each side and `A4` grown by 4, both clamped. All buffers start at 0.

1. **Heights.** For cells in `A4`: `H[x][z] := terrain altitude at vertex (x, z)`.
2. **Water.** For cells in `A2`: `M[x][z] := 255` if `CellHasWater(x, z)`, else `moistureBase[x][z]`.
3. **Smoothed heights** `S`: a separable `[1 2 1] ⊗ [1 2 1] / 16` filter of `H` over `A4`, with **mirror
   reflection** at the grid edges (the value just outside the edge equals the value one cell inside it, e.g.
   `H[−1] = H[1]`). Values within 2 cells of `A4`'s boundary (when that boundary is not the grid edge) are
   unspecified; they are never used by step 4.
4. **Local moisture.** Let `b := roundHalfEven(ambientMoisture × 255)`,
   `high := clamp(roundHalfEven(highMoisture × 255) + b, 0, 255)`, and `w := windDir × windSpeed`. For every cell
   of `A2` with `M ≠ 255`:

   ```
   cx := S[x−1][z] + S[x+1][z] − 2S[x][z]        (an index outside the grid is replaced by x itself)
   cz := S[x][z−1] + S[x][z+1] − 2S[x][z]
   if (cx ≥ 0 and cz ≥ 0) or H[x][z] ≤ S[x][z]:          # hollow or below the smoothed surface
       add := (cx + cz ≤ curvatureThreshold or cx ≤ 0 or cz ≤ 0) ? b : high
   else:                                                 # crest or slope: wind against the slope
       n   := terrain.GetVertexNormal(x, z)
       add := clamp(roundHalfEven(b − (w.x·n.x + w.z·n.z) × slopeK), 0, 255)
   M[x][z] := min(255, M[x][z] + add)
   ```

   Windward slopes (normal opposite to the wind) get more moisture, leeward slopes less.
5. **Blur.** A separable binomial `[1 4 6 4 1] / 16` filter with rounding (`(sum + 8) >> 4`) and the same mirror
   reflection: first along z for rows in `A2`, then along x; only cells in `A` are written to the **moisture
   grid**.
6. Post MoistureMapUpdated `0x09B6D58E` (void1 = IGZUnknown = this).

When `cityCellsPerCell > 1` the original sums the sub-vertex heights in step 1, marks water if any sub-cell has
water, averages the sub-cell results in step 4 and divides the smoothed height by the sub-cell count; in that
mode it also writes step 2 into the live moisture grid instead of the work buffer *[quirk]*. This mode is
unreachable with the shipped game; *[choice]* FreeCity4 supports only 1.

The original's edge reflection uses the grid's x size for both directions; it is correct only for square grids
(all SC4 cities are square).

### 6.6 Water near the city edge (January) *[compat, see note]*

1. **Water rectangles.** Scan the cells row by row (`z` outer, then `x`). A cell is *water* if
   `terrain.GetAltitude(x·16, z·16) ≤ terrain.GetSeaLevel()`. From each unused water cell, take the run of
   consecutive unused water cells along x; if it has at least 3 cells, grow it downwards row by row while the
   run (trimmed at its ends by non-water or used cells) keeps at least 3 cells. Keep the rectangle if its shorter
   side is ≥ 3 and its longer side ≥ 5, mark its cells used and record it as `(x0, z0, x1, z1)` with exclusive
   ends.
2. **Edge filter.** Keep only rectangles with `x0 < 1`, `z0 < 1`, `x1 ≥ countX − 1` or `z1 ≥ countZ − 1`.
3. **Sea mask.** Clear `seaMask`; for each rectangle set `seaMask` true for every cell in
   `[x0 − seaRadius, x1 + seaRadius] × [z0 − seaRadius, z1 + seaRadius]` (inclusive, clamped to the grid). A
   rectangle that would not overlap the grid marks only cell (0, 0) *[quirk]* (never happens with real input).

*Note* *[choice]*: the temperature grid has no reader in the shipped game, and this step only affects it, so
byte-exact reproduction of the rectangle scan is required only to reproduce save files exactly. The original's
row-growing step mishandles runs whose first cells turn to land (its run end can fall before its start); FreeCity4
may implement the intended algorithm above.

### 6.7 Random cell iterator *[compat]*

A shared utility of the original ("prime-step" mode is the only one the weather uses). It visits every cell of
a rectangle `[x0, x1] × [z0, z1]` (inclusive) once in a scrambled order:

```
reset(shuffle):
    count := (x1 − x0 + 1)(z1 − z0 + 1);  index := 0;  state := running
    choose start position k in the prime table P (below) by count:
        count < 29 → 0;  < 71 → 10;  < 113 → 20;  < 857 → 30;  < 3259 → 40;  < 12227 → 50;
        otherwise 60 (and count := min(count, 130444))
    advance k while P[k] < count / 4 (integer division)
    then advance k while count mod P[k] == 0 and k < 70
    step := P[k]
    if shuffle: current := rand() mod count             (C runtime rand, §7)
next(&x, &z):
    if state == exhausted: return false
    index += 1;  if index > count: state := exhausted; return false
    current := (current + step) mod count
    x := x0 + current mod (x1 − x0 + 1);  z := z0 + current div (x1 − x0 + 1)
    if index == count: state := exhausted
    return true
```

`P` (71 signed 16-bit values) *[compat]*: 1, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67,
71, 73, 79, 83, 89, 97, 101, 103, 107, 109, 113, 179, 211, 257, 307, 373, 479, 577, 641, 719, 857, 977, 1063, 1237,
1471, 1663, 1993, 2221, 2539, 2909, 3259, 3733, 4507, 5281, 5861, 6481, 6947, 7573, 9013, 10103, 12227, 15683, 17431,
19801, 21569, 23633, 26003, 27529, 29683, 32051, 32611, 20192 (the last entry is table padding).

Steps for SC4 city sizes: 64² = 4096 → 3733; 128² = 16384 → 15683; 256² = 65536 → 17431.

The iterator also has a "table" mode (a shuffled list of packed `(x << 8) | z` 16-bit entries) that the weather
never selects; its fields are still saved (§10).

## 7. Random numbers

| Draw | Generator | Method |
|---|---|---|
| monthly wind speed | the simulator's own `cRZRandom` (saved, §10) | `RandomDoubleRangeUniform(2, 4)` |
| monthly wind angle | same | `RandomDoubleGaussian`, repeated until `|spread × g| ≤ 1` |
| moisture base pattern (new city) | same | `RandomSint32RangeGaussianFast(−10, 10)` per cell |
| iterator start | **C runtime `rand()`** | `rand() mod count` at each reset |

Unlike the subsystems in foundations §6.2, the weather **saves** its generator state, so a loaded city continues
the same double-path and integer-path sequences *[compat]*. The iterator start depends on the process-wide C
runtime generator (foundations §8 question 5) *[choice]*: FreeCity4 uses its own seeded generator for it.

## 8. Lifecycle

### 8.1 `Init()` → true

If not initialized:
1. Read the tuning (§9).
2. Subscribe to PostCityInit `0x26D31EC1`; hold the city.
3. If not `loadedFromSave`: set sizes, counts and widths from the city (§2); allocate the grids: temperature
   filled with 127, moisture with 64, `seaMask` false, `moistureBase` 0; run `GenerateMoistureVariation` (§6.4).
4. Hold the terrain. On the first `Init` of this object, hold the simulator and register the three agents (§5).
5. Iterator: rectangle `(0, 0, countX − 1, countZ − 1)` (lower bounds clamped to 0), prime-step mode,
   `reset(shuffle = true)` — this discards any saved iterator position *[compat]*.
6. `cellsPerWeek := countX × countZ / subdivX` (integer division).
7. `ambientMoisture := baseMoisture`; `ambientTemp := baseTemp` — this also overrides saved values until the next
   month-end *[compat]*.
8. Set initialized.

### 8.2 `Shutdown()` → true

If initialized: release the terrain; remove the agents and release the simulator; release the city; unsubscribe
from PostCityInit; clear initialized.

## 9. Tuning

Exemplar `0x6534284A / 0x7A4A8458 / 0x1A2FDB6B` "Weather Tuning Parameters" (`SimCity_1.dat`), loaded as a
property holder. Missing properties keep the current value.

| Property | Type | Shipped | Code default | Field |
|---|---|---|---|---|
| `0xA7607D70` / `71` | s32 | 1 / 1 | 8 / 8 | subdivX / subdivZ (each clamped ≥ 1) |
| `0xA7607D72` | s32 | 1 | 1 | blendRadius |
| `0xA7607D73` | f32 | 65 | 24 | baseTemp |
| `0xA7607D74` / `75` | f32 | 0.5 / 0.5 | 0.5 / 0.5 | tempAmpA (clamped to [0, 1]) / tempAmpB |
| `0xA7607D76` | s32 | 10 | 6 | seaRadius |
| `0xA7607D77` | f32 | 0.9 | 0.95 | seaWeight (negative → 0) |
| `0xA7607D78` | f32 | 0.082 | 0.0035 | heightFactor |
| `0xA7607D79` | f32 | 0.25 | 0.25 | windFactor |
| `0xA7607D7A` / `7B` / `7C` | f32 | 0.2 / 0.2 / 1 | 0.5 / 0.5 / 0.4 | baseMoisture / moistAmpA (clamped to [0, 1]) / moistAmpB |
| `0xA7607D7D` | f32 | 0.35 | (128 stored) | slopeK := value × 255 |
| `0xA7607D7E` / `7F` | f32 | 1 / 2.5 | 0.12 / 4 | highMoisture / curvatureThreshold |
| `0xA7607D80` / `81` | u32 | 40 / 16 | 64 / 15 | unused |
| `0xA7607D86` / `87` / `88` | f32 | absent | 3, 0, 1 | default wind speed and direction |
| `0xA7607D89` | f32 | 60 | (0.25 stored) | windSpread := `tan(value × π / 360) × 0.5` (60° → ≈ 0.2887) |

## 10. Persistence (`cIGZSerializable`, `GetGZCLSID() = 0x2990C1BC`) *[compat]*

```
u16  version = 2
s32  countX, countZ
f32  cellWidthX, cellWidthZ, invCellWidthX, invCellWidthZ, sizeX, sizeZ
s32  cityCellsPerCellX, cityCellsPerCellZ, subdivX, subdivZ, blendRadius, lastSeason
f32  baseTemp, tempAmpA, tempAmpB, ambientTemp
grid temperature                      (byte grid, below)
grid seaMask                          (byte grid, bytes 0/1)
u32  rectCount;  rectCount × (s32 x0, s32 z0, s32 x1, s32 z1)
iterator                              (below)
s32  cellsPerWeek, seaRadius
f32  seaWeight, heightFactor, windFactor
grid moisture
grid moistureBase
f32  ambientMoisture, baseMoisture, moistAmpA, moistAmpB, slopeK, highMoisture, curvatureThreshold
u32  tuning80
f32  defaultWindSpeed;  f32 × 2 defaultWindDir
f32  windSpeed;         f32 × 2 windDir
f32  prevWindSpeed;     f32 × 2 prevWindDir
f32  targetWindSpeed;   f32 × 2 targetWindDir
rng                                   (u32 integer-path state, u32 double-path state, s32 hasCachedGaussian, f64 cachedGaussian)
s32  windMonth
```

**Byte grid**: `u32 rows, u32 cols` (cols = length of row 0, or 0), then the cells row by row. *[quirk]* The
original writer emits `rows × rows` bytes (it uses the row count for both loops); this is identical for the
square grids of SC4. The reader reads `rows × cols` bytes and skips the grid entirely if either count is
≥ 1,000,000. *[choice]* FreeCity4 writes `rows × cols` and rejects non-square grids on write.

**Iterator**: `s8 state, s8 mode, u8 dirty, s32 x0, z0, x1, z1, s32 count, u32 n, n × u16 table, s32 index,
s32 current, s32 step`. For the weather, mode = 1 (prime step) and `n = 0`.

**Read**: set `loadedFromSave` first; fail for `version ≥ 3`. Versions 0 and 1 have, after `targetWindDir`, a
legacy block `u32 n, n × (f32 × 2, f32 × 2)` and two byte grids; read and discard them, then continue with the
generator and `windMonth`. `windSpread`, `tuning81` and the three flags are not saved.

## 11. Interactions and replaceability

### 11.1 Consumers

| Consumer | Uses |
|---|---|
| Flora simulator | `GetMoistureType(x, z)` when choosing and propagating flora types. |
| Flammability simulator | `GetCurrentSeason()` (summer multiplier) and the SeasonChanged message `0x6716A4E5`. |
| Fire disaster (`ChooseSpreadCell`) | `GetAmbientWind(&v)`. |
| Lighting manager (`SetTimeOfDay`) | `GetCurrentSeasonMonth() − 1`. |
| Map view (moisture layer) | `GetMoistureValueMapPtr` (§11.2). |
| Effects manager | holds a reference *[usage not confirmed]*. |
| Terrain | subscribes to WeatherCycleCompleted `0x6716A4E6`, but ignores it. |
| City | creates, `Init`/`Shutdown`, save/load. |

No consumer of `MoistureMapUpdated` `0x09B6D58E`, `GetHumidity`, `GetMoistureValue`, `UpdateMoisture` or
`SetManualMoistureAddition` was found in the shipped game *[inferred]*.

### 11.2 Replaceability findings

- Created only through GZCOM (`GetClassObject(0x2990C1BC, 0xC6C66245)` in the city's `Init`; the class store on
  load). No original code calls a weather method directly; the only outside reference is the COM director's
  class table. **Replaceable** through the class registry.
- **`GetMoistureValueMapPtr`** *[compat]*: the SDK declares the parameter as `SC4Vector<SC4Vector<uint8_t>>&`, but
  the original treats it as `SC4Vector<SC4Vector<uint8_t>>*&`: it stores the address of its own grid object into
  the caller's variable, and the map view then walks the grid's memory directly. A replacement must keep its
  moisture grid as an `SC4Vector<SC4Vector<uint8_t>>`-compatible object (three pointers per vector: start, end,
  end of storage; rows indexed by x) that stays valid while the city is loaded, and write its address through the
  parameter.

## 12. Original quirks (reproduced)

`CellToStandardCityCell` z output (§4); southern-hemisphere wind interpolation never runs (§5.1); humidity is a
constant (§4); `SetManualMoistureAddition` byte wrap and exclusive clamped bounds (§6.4.1); saved iterator and
ambient values are overridden by `Init` (§8.1); square-only grid writer (§10).

## 13. Open questions

1. The value of `GetWorldHemisphere()` for the northern hemisphere: the code treats 1 as "no shift". Which value
   do new cities get, and does the region map ever set the other one?
2. The two size parameters of the original's water-rectangle helper (3 and 5, §6.6) are read as "shorter side
   ≥ 3, longer side ≥ 5"; confirm with a test map containing a 4 × 4 lake at the edge.
3. Who, if anyone, calls `UpdateMoisture` and `SetManualMoistureAddition` (terraforming tools, god mode?).
4. The effects manager's use of the weather simulator.
5. C runtime `rand()` seeding (foundations question 5) decides the weekly visit order.

## 14. Conformance tests (must exist in `engine/conformance`)

1. **Ambient values**: shipped tuning, season month 8 → temp 81.25; month 2 → 48.75; moisture month 1 → 0.24,
   month 7 → 0.16; code defaults when the exemplar is missing.
2. **Season**: months 1–3 → 0 … 10–12 → 3; hemisphere ≠ 1 shifts January to season month 7 (summer);
   SeasonChanged is posted only on change, with data2 = season, and resets the current wind to the defaults.
3. **Wind**: with a stub RNG returning uniform 0.5 and Gaussian 0, the target speed is 3 and the direction is the
   onshore edge's; ties without water give (1, 0); with spread 0.2887 and Gaussian 4 the draw is repeated; the
   daily interpolation on day 16 is halfway; it does not run when `windMonth` differs from the calendar month.
4. **Temperature** (blendRadius 0): ambient 48.75, height 400, wind 3, no sea → byte
   `roundHalfEven((48.75 − 32.8 − 0.75 + 15) × 4.25) = 128`; T > 45 → 255; T < −15 → 0; sea cells add `(65 − 48.75) × 0.9`; blend radius 1 halves into the 2×2
   block toward (x−1, z−1).
5. **Iterator**: prime steps 3733 / 15683 / 17431 for 64² / 128² / 256²; a full pass visits every cell exactly
   once; `next` after the last cell returns false; the weekly pass posts WeatherCycleCompleted once per wrap.
6. **Moisture**: on a flat dry 16×16 test terrain with base 80 and ambient moisture 0.2, every cell becomes
   131 (`80 + roundHalfEven(51.0)`, unchanged by the blur); a water cell counts as 255 before the blur; a windward slope cell exceeds a
   leeward one; only cells inside the requested rectangle change; `0x09B6D58E` is posted.
7. **Moisture variation**: with a fixed integer-path seed the pattern stays within `[lo, hi]` and follows the
   recurrence of §6.4 exactly.
8. **Accessors**: `GetMoistureType` = value >> 4; `GetMoistureValue` clamps positions; `GetHumidity` = 0.6 with
   shipped data; `CellToStandardCityCell(3, 7)` = (3, 3) with factors 1; `GetMoistureValueMapPtr` yields an object
   whose rows read back the grid.
9. **Save**: write → read → write is byte-identical (v2); version 3 is rejected; a version-1 record with a legacy
   block loads; after `Init` on a loaded city, ambient values equal the base values and the iterator is reset.
