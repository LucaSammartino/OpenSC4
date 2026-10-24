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
| windSpread (f32), not saved | 0.25 | `tan(0xA7607D89 × π / 360) × 0.5`; re-read by every `Init` before any wind draw, so a loaded city draws with the same value as a new one (§8.1, §10) |
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
| `CellToPosition(cx, cz, &x, &z)` | `x := f32(cx) × cellWidthX`, `z := f32(cz) × cellWidthZ`. **Returns nothing, and leaves nothing on the x87 stack** (slot 11, pop 16; corrected 2026-10-14). |
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
| `GetAmbientWind(&v)` | Returns `windSpeed`. If the pointer is non-null, copies and rescales `windDir` to that length, subject to the small-vector threshold in §5.1.2. |
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

#### 5.1.1 Exact daily rounding (WE1) *[compat]*

The formulas above are mathematical summaries. For byte-identical state use the following evaluation contract.
`R(a)` means round the exact result of **one** operation to a 24-bit significand, nearest-even (foundations §18).
The normal simulation precision is **24 bits**, not the x87 default 53 bits or its full 64-bit significand.
Registers retain the x87 exponent range; for ordinary finite weather values every rounded result is exactly a
binary32 value. A binary64 operand is loaded without first rounding it to binary32. No operation below is fused,
and keeping the whole expression at double or extended precision until its final store changes the contract.
This routine does not change the control word; the rules describe the normal simulation setting, not a session
affected by the unrelated precision leak documented in foundations §18.1.

Let `s₀, D₀` be the previous endpoint and `s₁, D₁` the target endpoint, all binary32. Let `i = dayOfMonth − 1`.
For either component `j ∈ {x,z}`, the successive rounded quantities are:

| Quantity | Required evaluation |
|---|---|
| Fraction `t` | `R(i × C)`, where `C` is binary64 `0x3FA1111111111111` = `0x1.1111111111111p−5`, the nearest binary64 to 1/30 |
| Endpoint vectors | `pⱼ = R(s₀ × D₀ⱼ)`, `qⱼ = R(s₁ × D₁ⱼ)` |
| Displacement | `δⱼ = R(qⱼ − pⱼ)`, then `aⱼ = R(δⱼ × t)` |
| Interpolated vector | `vⱼ = R(pⱼ + aⱼ)` |
| Squared length | `L² = R(R(v.x × v.x) + R(v.z × v.z))` |
| Length | `L = R(sqrt(L²))`, correctly rounded square root |
| Direction if `L > 0` | `r = R(1.0f / L)`, then `windDirⱼ = R(vⱼ × r)` |
| Direction if `L = 0` | `windDir = v`, the newly interpolated zero vector; the old direction is not retained |
| Final scalar speed | `windSpeed = R(s₀ + R(R(s₁ − s₀) × t))` |

In particular, normalization uses **one rounded reciprocal and two products**, which can differ by one ULP
from two component divisions. `t` uses the double constant directly, not its single-precision approximation.
For calendar days 1–31 its result also equals correctly rounded `(day−1)/30`, but multiplication by a
pre-rounded single 1/30 differs on days including 4 and 8. The direction is normalized even on day 1 or 31;
there is no endpoint shortcut. A month mismatch leaves the current direction and speed untouched.

The endpoint products, scaled displacements and interpolated vector components have binary32 stores. `t`, the
endpoint-x target product before subtraction, squared-length arithmetic, square root and reciprocal remain in
registers, but their arithmetic still rounds at **each** operation. `L` is briefly stored as the current speed;
the reciprocal uses the retained rounded `L`. The final scalar interpolation overwrites that temporary speed.
Thus the final speed is the interpolation of endpoint **speeds**, not `L`. Relevant instruction ranges:
fraction `0x0072976E–77` (double constant at `0x00AB3678`); interpolation `0x0072977D–E5`;
squared length/root `0x007297EB–0x00729807`; reciprocal/products `0x0072981A–34`;
final speed `0x0072983E–52`. Stores add no rounding for these ordinary finite 24-bit results.

#### 5.1.2 Returned wind vector and moisture input (WE1) *[compat]*

`GetAmbientWind` returns the stored scalar speed, including when its output pointer is null. With a non-null
pointer it copies the stored direction `D`, computes `l = R(sqrt(R(R(D.x²) + R(D.z²))))`, and, **only if**
`l > 0.01f` (`0x3C23D70A`), returns vector components `R(Dⱼ × R(windSpeed / l))`. At equality, below the
threshold, or for zero length it leaves the copied vector unscaled. It does not modify the stored direction or
speed. This rescales to the requested length rather than assuming that stored `D` has length exactly one.
The factor is the rounded **speed/length**, not a product of speed with a separately rounded reciprocal.
`GetWindAtCell` has the same behavior. Evidence: `0x00725A00–36`, helper call `0x00725A28`, and
`ScaleVector2D` `0x005D76D0–0x005D7709` (strict threshold compare `0x005D76E9–F4`).

Moisture uses this accessor once per simulation pass (`0x00727637–4B`), so even a direction just normalized by
the daily update is measured and rescaled again. This is a separate rounding contract from the daily reciprocal.

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

followed by the wind update of §6.2 and `windMonth := m`.

Exact operation order *[compat]* (`0x00729350`; foundations §18, `r24` = one rounding to 24 bits):

```
ambientTemp     := r24( r24(1 + r24(cos(r24(k_T × w)) × r24(tempAmpB × tempAmpA))) × baseTemp )       k_T = m − 8
ambientMoisture := r24( r24(1 + r24(cos(r24(k_M × w)) × r24(moistAmpB × moistAmpA))) × baseMoisture ) k_M = m − 1
```

- `k` is an `int32` (exact); `w` is the `float` constant 0.52359879 (`float32(π/6)`, at `0x00A949E8`); 1 is the
  `float` 1.0. All fields are `float32`.
- The cosine is the x87 `fcos` instruction, which precision control does **not** affect: its argument is the
  24-bit angle above and its result is not rounded before the multiply. FreeCity4 evaluates it as a `double`
  cosine of the rounded angle; this can differ from the original only when the product falls within a `double`
  error of a `float32` rounding boundary *[choice]*.
- Because the angle is rounded, months symmetric about the peak do not give identical values (test 21).

Shipped values: month 8 → 65 × 1.25 = 81.25, month 2 → 65 × 0.75 = 48.75; moisture month 1 → 0.24000001,
month 7 → 0.16000001.

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

Precision *[compat]*: `targetWindSpeed` is `RandomDoubleRangeUniform(2.0, 4.0)` with `double` arguments
(foundations §6.1: `r24(r24(u × 2) + 2)`), stored as `float32`. In the loop `s := r24(g × windSpread)`, accepted
when `|s| ≤ 1.0`. Then `c := r24(sqrt(r24(1 − r24(s × s))))` (1 is a `double` constant; `fsqrt` rounds to 24 bits)
and, with `(bx, bz)` the base direction:
`targetWindDir.x := r24(r24(bx × c) − r24(bz × s))`, `targetWindDir.z := r24(r24(bz × c) + r24(bx × s))`.

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
   unspecified; they are never used by step 4. Order and precision *[compat]*:
   - **Pass 1, along z**, from `H` into a separate buffer `S`: `S[x][z] := (H[x][z−1] + H[x][z+1]) + 2·H[x][z]`,
     not normalised, for every z of the row.
   - **Pass 2, along x**, back into `S`: `S[x][z] := ((S[x−1][z] + S[x+1][z]) + 2·S[x][z]) × scale`, with
     `scale = 0.0625` (= 1/16 ÷ cityCellsPerCell², §6.5 note). Each line of `S` along x is first copied, with its
     mirror cells, into a temporary line buffer, and the filtered values are computed from that copy
     (`0x007273E0`–`0x007275A6`). The right-hand side therefore always reads the **pass-1** values; a value
     already filtered in this pass is never read again *[compat]*.
   - Every operation is a `float32` operation in exactly this order (the game runs the x87 at 24-bit precision,
     foundations §18); doubling and the multiply by 1/16 are exact. No rounding to integers happens here.
4. **Local moisture.** Let `b := roundHalfEven(ambientMoisture × 255)`,
   `high := clamp(roundHalfEven(highMoisture × 255) + b, 0, 255)`, and `w` be the vector returned by
   `GetAmbientWind` (§5.1.2). For every cell
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

#### 6.5.1 Exact local rounding (WE1 moisture continuation) *[compat]*

Use `R` from §5.1.1 throughout. The smoothing order in step 3 is unchanged: add the two neighbors, add the
separately doubled center, then (pass 2 only) multiply by the scale; each operation rounds. Both the unrolled
four-cell path and the remaining-cell path use that order (`0x00727250–63`, `0x007274C5–0x00727521`,
`0x00727586–9E`). All filtered heights are binary32. No integer rounding occurs until the moisture additions.

The precise curvature is `cₓ = R(R(Sleft + Sright) − R(Scenter + Scenter))`, likewise `c_z`. The threshold
test uses `R(cₓ + c_z)`; height and sign comparisons use these rounded stored values, with no epsilon
(`0x007277AD–DF`, `0x00727910–45`). Integer `b` is converted exactly for the slope calculation.

For a finite terrain normal `n`, define `d_z = R(w.z × n.z)`, `d_x = R(w.x × n.x)`, then
`d = R(R(d_z + d_x) + R(n.y × 0.0f))`, `e = R(d × slopeK)`, and `u = R(b − e)`.
The original includes the zero vertical product; it is immaterial for finite ordinary normals.
The addend is `clamp(Q(u), 0, 255)`. This is **not** an unrounded dot product followed by one final conversion.
The wind components were already rounded by §5.1.2 before the normal is fetched. Slope calculation evidence:
`0x00727898–D2`; conversion/clamp `0x007278D4–0x00727900`.

Here `Q(v)` describes the original integer conversion: store `R(v + 12582912.0f)` as binary32, reinterpret
its bits as a 32-bit integer, subtract `0x4B400000` modulo 2³², and interpret that difference as signed32.
For `−2²² ≤ v ≤ 2²²` this gives nearest-even integer rounding. Outside that range the bit operation remains
the rule; it is not a general-purpose rounded conversion *[quirk]*. The bias has bits `0x4B400000` at
`0x00AB3518`. Ambient baseline is `b = Q(R(ambientMoisture × 255.0f))`; high addend is
`clamp(b + Q(R(highMoisture × 255.0f)), 0, 255)` (integer sum), verified at `0x007275C2–0x00727635`.
For shipped tuning these agree with the `roundHalfEven` summaries in step 4. The post-addition byte saturation
and the two integer blur passes introduce no floating arithmetic.

These rules identify two independent omissions in the earlier summary: daily reciprocal multiplication and
accessor rescaling before moisture. A daily one-ULP change can propagate into weekly integer rounding and
blurred bytes; it does not establish that every observed later mismatch has that cause. Closing the arithmetic
question supplies a contract for independent replay, not a byte-exact verdict on runs 374–377 or a new replay.

When `cityCellsPerCell > 1` the original sums the sub-vertex heights in step 1, marks water if any sub-cell has
water, averages the sub-cell results in step 4 and divides the smoothed height by the sub-cell count; in that
mode it also writes step 2 into the live moisture grid instead of the work buffer *[quirk]*. This mode is
unreachable with the shipped game; *[choice]* FreeCity4 supports only 1.

*[quirk]* Only step 3's z pass assumes a square grid: its high-z mirror uses countX, so on a non-square grid the
cell past the last z reads 0 (countZ < countX), or an in-grid value is overwritten by the mirror (countZ >
countX). The blur (step 5) mirrors each axis with its own size. All SC4 cities are square, so this never shows.

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
    count := (x1 − x0 + 1)(z1 − z0 + 1);  index := 0;  state := running (1)
    choose start position k in the prime table P (below) by count:
        count < 29 → 0;  < 71 → 10;  < 113 → 20;  < 857 → 30;  < 3259 → 40;  < 12227 → 50;
        otherwise 60 (and count := min(count, 130444))
    advance k while P[k] < count / 4 (integer division)
    then advance k while count mod P[k] == 0 and k < 70
    step := P[k]
    if shuffle: current := rand() mod count             (C runtime rand, §7)
next(&x, &z):
    if state == exhausted (2): return false        (any other value counts as running)
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

The iterator also has a "table" mode that the weather never selects; its fields are still saved (§10). Table
mode *[compat]* (Reset `0x0049D1C0`, GetNext `0x0049CB10`):

```
reset(shuffle):                       (table mode)
    count := (x1 − x0 + 1)(z1 − z0 + 1);  index := 0;  state := running (1)
    if table length ≠ count: resize the table to count entries; dirty := true
    if dirty: for x in x0..x1 (outer), for z in z0..z1 (inner): append (x mod 256) << 8 | (z mod 256)
    if shuffle: random_shuffle(table)          (C runtime rand, foundations §12.3)
    current and step are left unchanged
next(&x, &z):                         (table mode)
    if state == exhausted (2): return false
    if index ≥ table length: state := exhausted; return false
    e := table[index];  index += 1
    x := e >> 8;  z := e & 0xFF              (absolute coordinates, not relative to x0, z0)
    if index == table length: state := exhausted
    return true
```

- The entries hold **absolute** coordinates, each truncated to its low 8 bits. City grids are at most 256 cells
  wide, so with in-grid rectangles the values are exact; a rectangle with negative coordinates or coordinates above
  255 would wrap *[quirk]*.
- `dirty` is set by `SetClipRect` (`0x0049C820`) and by the resize above and is never cleared, so once a table
  has been sized it is refilled in natural order at **every** table-mode reset before the shuffle. A reset without shuffling
  therefore yields natural order (x outer, z inner).

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
5. Iterator: rectangle `x0 = z0 = 0`, `x1 = countX − 1`, `z1 = countZ − 1`, each upper bound then raised to at
   least its lower bound; prime-step mode; `reset(shuffle = true)` — this discards any saved iterator position
   *[compat]*.
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
s32 current, s32 step`. For the weather, mode = 1 (prime step) and `n = 0`. **State values** *[compat]*: the
original has only two, **1 = running** (also a freshly constructed, never-reset iterator) and **2 = exhausted**;
it never writes 0. `Read` stores the byte unchanged, and `next` treats any value other than 2 as running. With the
shipped tuning the weekly pass consumes exactly `count` cells, so a save normally holds 2 (with `index = count`).
On a stream error the shared iterator reader (`0x005DB280`) reads the table length into the variable that held the
stream pointer, so the table is resized to the stream object's address (a huge allocation) *[quirk]*; *[choice]*
FreeCity4 fails the `Read` instead and leaves the iterator unchanged.

**Generator**: `hasCachedGaussian` and `cachedGaussian` are written as they are. The cached word is **not**
cleared when the cached value is consumed: with the flag at 0 it holds the second value of the most recent
Gaussian pair (already returned). Only a generator that never drew a Gaussian holds uninitialised memory there;
the weather always draws one at PostCityInit. *[compat]* FreeCity4 keeps the value on consume and writes it
regardless of the flag; a never-drawn generator writes 0.

**Read**: set `loadedFromSave` first; fail for `version ≥ 3`. Versions 0 and 1 have, after `targetWindDir`, a
legacy block `u32 n, n × (f32 × 2, f32 × 2)` and two byte grids; read and discard them, then continue with the
generator and `windMonth`. `windSpread`, `tuning81` and the three flags are not saved. `windSpread` needs no
saving: `Init` (which always runs on a loaded city, because `initialized` is not saved) re-reads the tuning first,
and every wind draw (month message, PostCityInit) comes after `Init`.

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

1. **Ambient values**: shipped tuning, season month 8 → temp 81.25; month 2 → 48.75; moisture month 1 →
   0.24000000953674316, month 7 → 0.1600000113248825; code defaults when the exemplar is missing.
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
10. **Ambient precision** (§6.1, shipped tuning 65 / 0.5 / 0.5 and 0.2 / 0.2 / 1): temperature month 1 →
    50.92708969116211 but month 3 → 50.927085876464844 (the rounded angles differ); month 7 → 79.07291412353516;
    month 4 → 56.875. Moisture month 2 → 0.23464103043079376, month 3 → 0.2200000137090683, month 5 →
    0.17999999225139618.
11. **Table-mode iterator** (§6.7): rectangle `[2, 3] × [5, 6]`, reset without shuffling → entries `0x0205`,
    `0x0206`, `0x0305`, `0x0306` in that order, `next` yields (2, 5), (2, 6), (3, 5), (3, 6), then false; the
    prime-step `current` and `step` are unchanged by the reset. With shuffling, the reset makes 3 `rand()` calls.
12. **Pass-2 smoothing reads pass-1 values** (§6.5 step 3): a row of pass-1 values `S = [0, 16, 0, 0]` along x
    (interior cells, scale 1/16) → `[·, 2, 1, ·]` for the two interior cells (from the copy), not `[·, 2, 0.125, ·]`.
13. **WE1 daily rounding:** all hex words below are binary32 bit patterns, not decimal numbers or serialized
    byte order. Input endpoints are `s₀=41200000` (10), `D₀=(3F800000,00000000)` (1,0),
    `s₁=41A00000` (20), `D₁=(00000000,3F800000)` (0,1); calendar month equals `windMonth`.
    Each row is an independent daily delivery with those endpoints. Endpoint vectors are `(10,0)` and `(0,20)`.

    | Day | `t` | `v.x`, `v.z` | `L²` | `L` | Reciprocal `r` | Final `windDir.x`, `windDir.z` | Final speed |
    |---|---|---|---|---|---|---|---|
    | 2 | `3D088889` | `411AAAAB`, `3F2AAAAB` | `42BBC71D` | `411B08B7` | `3DD35C2B` | `3F7F64B5`, `3D8CE81D` | `41255555` |
    | 8 | `3E6EEEEF` | `40F55556`, `40955555` | `42A11C72` | `410F9ABA` | `3DE42EB3` | `3F5AACC1`, `3F051B3D` | `41455555` |
    | 31 | `3F800000` | `00000000`, `41A00000` | `43C80000` | `41A00000` | `3D4CCCCD` | `00000000`, `3F800000` | `41A00000` |

    Day 2 component division would give x `3F7F64B4`, one ULP below the required product. Day 8 component
    divisions would give `(3F5AACC2,3F051B3E)`; multiplication by single 1/30 would first give `t=3E6EEEF0`.
    With equal endpoint speeds 10 and opposite directions `(1,0)` and `(−1,0)`, day 16 instead gives
    `t=3F000000`, `v=(00000000,00000000)`, `L²=L=00000000`, zero direction, and final speed `41200000`.
    With a mismatching month, an arbitrary current direction/speed must remain bit-identical.
14. **WE1 returned vector:** scalar speed is `41200000` (10) for every row. Calls leave stored fields unchanged.

    | Stored direction bits | Measured length | Scale factor if applied | Returned vector bits |
    |---|---|---|---|
    | `(3F800000,3F800000)` | `3FB504F3` | `40E24630` | `(40E24630,40E24630)` |
    | `(3C23D70A,00000000)` | `3C23D70A` | none (threshold equality) | `(3C23D70A,00000000)` |
    | `(3CA3D70A,00000000)` | `3CA3D70A` | `43FA0000` | `(41200000,00000000)` |

    For day 2 of vector 13, with its final speed `41255555`, the accessor measures direction length
    `3F800000`, uses scale `41255555`, and returns `(4124F10A,3F360125)`. A null output pointer returns the
    same scalar without calculating a vector. Zero direction stays zero even with nonzero speed.
15. **WE1 moisture ties:** supply a crest/slope cell with `H > S`, at least one negative curvature, baseline
    `b=51` (`ambientMoisture=3E4CCCCD`), `slopeK=3F800000`, and accessor vector `(1,0)`. Finite normals have
    `n.z=0`; these are local additions **before** byte saturation and blur. The expected intermediate `u`
    and integer addend are:

    | `n.x` bits | `u` bits | Biased stored float bits | Integer addend |
    |---|---|---|---|
    | `3F000000` (+0.5) | `424A0000` (50.5) | `4B400032` | 50 |
    | `BF000000` (−0.5) | `424E0000` (51.5) | `4B400034` | 52 |
    | `3EFFFF80` | `424A0001` (next float above 50.5) | `4B400033` | 51 |

    To distinguish accessor rescaling in the moisture path, set stored direction `(1,1)`, speed 10,
    `n=(0.5,sqrt(0.75),0)` with binary32 components, and retain the other cell conditions above. The local
    addend is 47 using returned x `40E24630`; assuming `w=direction×speed` would incorrectly give 46.
    These are instruction-derived arithmetic vectors, not observations from a new game run.

## Compiled slot tables (round 2) *[compat]* (see foundations §16, §17)

`cISC4WeatherSimulator`, original vtable `0x00AB3610`, 24 interface slots (3 + 21). It has no overloads, and
every slot's argument size matches the SDK. Slot 14 `GetCurrentSeason()` returns an enum in a register (no hidden
pointer). The slot after the interface is the class's own destructor. The SDK order can be used as is.
