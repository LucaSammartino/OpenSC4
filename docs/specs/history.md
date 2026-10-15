# Spec: the history warehouse (`cSC4HistoryWarehouse`)

Status: **implementation spec**, clean-room input. This document describes *behaviour*, not code. It was
written by the analysis team from observing the original program and its data files. Implementers must work from
this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose.

Related specs: [foundations.md](foundations.md) (streams §2, components §3.6, messages §5, the overload order of
this interface §16, x87 precision §18), [simulator.md](simulator.md) (dates are Julian day numbers §2, agents §4),
[demand.md](demand.md) (the tax-rate maxima, §8.1), [advisors.md](advisors.md) (Lua `trend_value`, the safety
advisor), and the producers listed in §6.

## 1. Role

The history warehouse stores **time series** ("history types"), each identified by a 32-bit key. A point is a
signed 32-bit value stamped with the simulation day on which it was recorded. Subsystems record values (§4.1); the
graphs, the advisors and the demand simulator read them back through range queries (§4.7). Each series has a
**queue length**: a positive length keeps only that many most recent points; length 0 keeps everything, but a
yearly pass thins the old points to one per month, half-year, year or five years (§4.3).

| Class | CLSID | Interfaces (IID) | City component | City getter |
|---|---|---|---|---|
| history warehouse | `0x89EFA536` | `cISC4HistoryWarehouse` (`0x69F4A803`) and `cIGZUnknown` (`1`) at +0, `cIGZSerializable` (`0xE4FDA3D4`) at +8 | 1 | `cISC4City` slot 32 (`+0x80`, `GetHistoryWarehouse`) |

The getter at slot 32 (`0x006DC680`) returns the city's field `+0x128`, which `cSC4City::CreateCityComponents`
(`0x0045AA10`) fills with this component. docs/abi names the slot after an identical-code fold
(`cSC4PlumbingSimulator::GetWaterExported`).

The object also has a `cIGZMessageTarget2` sub-object at +4 (used for its subscriptions and agents), but
`QueryInterface` does **not** answer `0x452294AA` *[quirk]*; any IID other than the three above returns false.

The record message type `0x89EFA536` is the same number as the CLSID.

Factory `0x00476BB0` (`GlobalHeapAlloc(0x2C)` + constructor `0x00476A40`). `Release` (`0x00476AA0`) destroys the
object (`0x00476900`, then `GlobalHeapFree`) when the count is 1 or less. It does **not** call `Shutdown` (HW13):
the destructor only frees the series map and its nodes (`0x00476918`–`0x00476952`) and leaves the message
subscriptions alone, so the original relies on the city calling `Shutdown` first. *[choice]* A replacement may
run its `Shutdown` first; with the city's normal order it changes nothing. Vtables: primary `0x00A8CCB4` (14 slots,
§8), message target `0x00A8CCA4` (`DoMessage` `0x00477050`), serializable `0x00A8CC8C` (Write `0x00475D20`, Read
`0x00476BD0`, GetGZCLSID `0x00475650`).

## 2. State

| Offset | Field | Constructor | Saved |
|---|---|---|---|
| `+0x0C`, `+0x10` | `series`: ordered map u32 key → `Series`, and its size | empty | yes |
| `+0x18` | `initialized` (u8) | 0 | no |
| `+0x1C` | reference count | 0 | no |
| `+0x20` | `passState`: 0 idle, 1 armed, 2 running | 0 | no |
| `+0x24` | `cursor` into `series` | — | no |
| `+0x28` | `passCounter` | 0 | no |

`Series` = `{ points: list of {s32 value, u32 date}; u32 maxCount }`. Dates are Julian day numbers
(simulator.md §2: 2000-01-01 is 2451545). Points are kept in ascending date order with **at most one point per
day** (§4.1). `maxCount = 0` means "no limit" (thinned by §4.3); `maxCount = n > 0` keeps at most n points.

## 3. Agents, messages and cadence

`Init()` (`0x004766F0`; it has no "already initialized" test):

1. Subscribe the +4 sub-object to `0x89EFA536` (record) and `0x89EFA537` (set queue length).
2. `AddAgent(+4, level 4 (day), "cSC4HistoryWarehouse", flags 3)` and `AddAgent(+4, level 7 (year), the same name,
   flags 3)`. Flags 3: enabled whether or not the city is established (simulator.md §4).
3. `passState := 0`, `initialized := 1`; return true.

`Shutdown()` (`0x004758D0`): unsubscribe both messages; `RemoveAgent(+4)` on the city's simulator (both levels);
`initialized := 0`; return true. The series are **not** cleared.

`DoMessage` (`0x00477050`):

| Message | Handling | Result |
|---|---|---|
| `0x89EFA536` | `Store(data1, (int32) data2)` (§4.1) | true |
| `0x89EFA537` | `SetSize(data1, data2)` (§4.2) | true |
| `0x66956814` SimNewDay | the thinning pass step (§4.3) | the step's result: false asks the simulator to call again this interval |
| `0x66956817` SimNewYear | if `passState = 0`: `passState := 1` | true |
| other | — | true |

No subsystem posts `0x89EFA537` in the executable (HW4: its four bytes occur only in `Init` `0x00476726`,
`Shutdown` `0x00475900` and `DoMessage` `0x004770BE`); queue lengths are set through the interface (slot 13). The win
trend bar (`cSC4WinTrendBar`, UI) also subscribes to `0x89EFA536` and sees every recorded value.

## 4. Behaviour

### 4.1 Recording *[compat]*

`RecordHistoryData(key, value)` (slot 12, `0x00475AA0`) builds a standard message (type `0x89EFA536`, data1 =
key, data2 = value) and **sends** it synchronously (`MessageSend`) through the message server, then releases it.
It does not touch the series directly: the warehouse's own `DoMessage` stores it, and any other subscriber sees it.
Several producers send the same message themselves instead of calling slot 12 (residential `0x006F7D00` and
`PostPopulationChange`, the landfill helper `0x006B8BC0`, `0x006E9070`). The slot's return value is whatever the
message's final `Release` leaves in EAX (HW12). The standard message's `Release` (`0x009132C1`) returns the new count,
or 0 when it destroys the message. The slot creates the message with count 0, takes one reference, sends it and
releases it. Unless a subscriber kept a reference during the send (none in the shipped game does), the release
destroys the message and the slot returns **EAX = 0, so AL = false**. No shipped caller reads the result: the 48
producer call sites found by their keys all discard it. *[compat]* FreeCity4 returns false (0).

`Store(key, value)` (`0x00476EE0`):

1. Find the series, creating an empty one with `maxCount = 0` if the key is new.
2. `day :=` the simulator's current date number (simulator slot 8).
3. If the series has points and `day ≤ last.date` (unsigned): **overwrite the last point's value** and stop. (So a
   second record on the same day replaces the first.) An empty series compares `day` with 0, so a record on day
   number 0 into an empty series is lost (the original writes it into the list header) *[quirk]*; no real date is
   0.
4. Otherwise, if `maxCount ≠ 0` and the point count is `≥ maxCount`, erase the **first** point (one only). Then
   append `{value, day}`. The count is a u32 compared **unsigned** with `maxCount` (`0x00476F8C`, HW8), so a
   `maxCount` of `0xFFFFFFFF` (from `SetSize(key, −1)`, §4.2) never trims, and a series that is already longer than
   its `maxCount` (possible after `Read`, which does not trim) keeps its length: each record drops one point and
   appends one.

### 4.2 Queue length

`SetSize(key, n)` (`0x00476FC0`; slot 13 `SetHistoryDataQueueLength` and message `0x89EFA537`): find or create the
series, set `maxCount := n`. If `n > 0` (signed) and the series has more than `n` points, erase the oldest
`count − n`. Returns true.

### 4.3 The yearly thinning pass *[compat]*

SimNewYear arms the pass (`passState := 1`). From then on, each SimNewDay delivery runs one step (`0x00475E20`):

- `passState = 1`: `cursor :=` the first series, `passCounter := 0`, `passState := 2`, and continue as below.
- `passState = 2`:
  1. Start a timer (milliseconds *[inferred]*). If there are no series: `passState := 0`, return true.
  2. `perPass := n / 365 + 1` (integer division, n = the number of series).
  3. While `passCounter < perPass`: if the cursor is at the end, `passState := 0` and stop. If the cursor's series
     has `maxCount = 0`: `VariableCoalesce(key)` and `passCounter += 1`. Advance the cursor. Stop when the timer
     shows 5 or more.
  4. If `passCounter ≥ perPass`: `passCounter := 0`, return true. Otherwise return **false** (the simulator calls
     again in a later frame with the same message, simulator.md §4).
- any other state: return true.

So with fewer than 365 series one unlimited series is thinned per day, in ascending key order, starting on the
first day of each year; series with a positive length are skipped without counting. A call that reaches the end
of the map with `passCounter < perPass` returns false once more, and the next call returns true (state 0).

`VariableCoalesce(key)` (`0x00475950`), with the current date's month `M` and year `Y`:

| Date | Value |
|---|---|
| `D1` | the 1st of the previous month: (M − 1)/1/Y, or 12/1/(Y − 1) when M = 1 |
| `D2`, `D3`, `D4`, `D5` | January 1st of Y − 10, Y − 50, Y − 100, Y − 500 |
| `D0` | day number 0 |

Then, in this order: `Dispose(key, D0, D5)`; `Coalesce(key, D5, D4, 3)`; `Coalesce(key, D4, D3, 2)`;
`Coalesce(key, D3, D2, 1)`; `Coalesce(key, D2, D1, 0)`. Points after `D1` stay daily.

### 4.4 `Coalesce(key, from, to, granularity)` (slot 9) *[compat]*

Return false if the key is missing or the range (§4.6, inclusive, no widening) is empty. Otherwise walk the
points of the range in date order with a group key:

| Granularity | Group key | Period start |
|---|---|---|
| 0 | the **month** number (1–12) only | the 1st of the point's month |
| 1 | `2 × year + (month − 1) / 6` | January 1 or July 1 (odd key) of the point's year |
| 2 | year | January 1 of the point's year |
| 3 | `year / 5` | January 1 of the **first** point's year (not aligned to a multiple of 5) |
| other | −1000 for every point | — |

The first point opens a group (it records the group key and the period start of that point). Each next point
with the same key joins the group. A point with a different key **closes** the group: the group's first point
gets the period start as its date (its value is kept, not averaged), the group's other points are erased, and the
point opens a new group. The **last group of the range is never closed**: its points stay as they are. Return
true.

*[quirk]* Granularity 0 groups by month number only: two points twelve months apart with no point between merge.

### 4.5 Dispose

- `Dispose(key)` (slot 10): erase every point of the series; the series and its `maxCount` stay. False if the key
  is missing.
- `Dispose(key, from, to)` (slot 11): false if the key is missing or the inclusive range is empty. Otherwise erase
  the points of the range **except the last one** *[quirk]*, and return true.

### 4.6 Ranges *[compat]*

A range over a series is built from `from` and `to` (dates passed as `cIGZDate`, read as day numbers) and a
widening flag. Its **base** is `from`'s day number (used by the statistics, §4.7).

- Without widening: the points with `from ≤ date ≤ to`, possibly none.
- With widening (all of §4.7 except `GetMinMaxAverage`), on a non-empty series:
  - `from` after the last point: the last point alone;
  - `to` before the first point: the first point alone;
  - no point in `[from, to]`: the two points around the gap;
  - exactly **one** point P in `[from, to]` and a later point exists: P with its predecessor (if any) and its
    successor;
  - otherwise the points in `[from, to]` (a range that runs to the last point is not widened).

**How the range is found** (HW6; `cVecRange` constructor `0x00475B00`). The series is a linked list; the range is a
pair of positions, a begin and an exclusive end, found by one walk from the first point. All date comparisons are
unsigned.

1. With widening and a non-empty series, two tests come first: `from > last.date` gives the last point alone;
   otherwise `to < first.date` gives the first point alone.
2. Walk the points in order. Until the range has begun, each point P is tested **first** against `to`:
   - `P.date > to`: with widening the range is P's predecessor (if any) and P; without widening it is empty. Stop.
   - otherwise, if `P.date ≥ from`, the range begins at P, and P goes on to the next test.
3. Once the range has begun, the first point P with `P.date > to` ends it (P is excluded). If widening is on and the
   range holds exactly one point B (P comes right after B), it becomes B's predecessor (if any), B and P. Stop.
4. A walk that reaches the end after the range began gives a range that runs to the last point, never widened. A
   walk that never began gives an empty range.

The walk costs one step per point before the range's end (series hold at most a few hundred points, §7.1).

**`from > to`.** No point can begin the range (it would need `from ≤ date ≤ to`). Without widening the range is
therefore empty. With widening, on a non-empty series, step 1 gives the last point if `from` is after it, or the
first point if `to` is before it; otherwise the walk stops at the **first point after `to`**, and the range is that
point and its predecessor. Example: points on days 10, 20, 30 and 40, `from = 35`, `to = 15`: the range is the
points on days 10 and 20 (not 30 and 40), with base 35. The Lua `trend_count(type, 0)` (advisors.md) asks for
`[now + 1, now]`, which reaches this case; with a point on `now` or earlier, step 1 gives the last point.

### 4.7 Queries *[compat]*

All four return false (and leave their outputs untouched) when the key is missing.

**Statistics** of a range of n points, with `x = date − base` (int32) and `y = value`: `meanX = Σx / n`,
`meanY = Σy / n` (0 when n = 0); `sx = √(Σ(x − meanX)² / n)`, `sy = √(Σ(y − meanY)² / n)` (population standard
deviations, 0 when n = 0); `r = Σ((x − meanX)/sx · (y − meanY)/sy) / n`, with `r = 0` when sx, sy or n is 0. The
**line** of a non-empty range: if `sx = 0`, slope 0 and intercept = the first point's value; otherwise
`slope = (sy / sx) · r` and `intercept = (Σy − slope · Σx) / n`, the value at `x = 0`, i.e. at `from`. An empty
range has no line (false). The sums are held in `double` variables, but the x87 precision of the caller applies
(foundations §18): 24-bit from the simulators, 53-bit inside Lua calls.

**Operation order** (HW5, HW11; verified in the bytes). "Rounded" below means rounded once, at the caller's precision,
from the exact result of that one operation on its exact operands. Integer operands (values, `x`, `n`) are loaded
exactly, never rounded to float first. At 24-bit precision this is a 24-bit significand; the x87's wider exponent
range never matters at these magnitudes. Values stored in the `double` fields are exact copies of the rounded
register values.

1. Sums (`0x004756C0`), for each point of the range in list order: `n += 1` (a 64-bit integer); `Σy := Σy + value`,
   rounded; then `Σx := Σx + x`, rounded, where `x = date − base` is computed in 32 bits and read as a signed
   int32. Then `meanX := Σx / n` and `meanY := Σy / n`, each rounded (0 when `n = 0`).
2. Spreads (`0x00475770`): `Sxx` and `Syy` start at 0. For each point: `dx := x − meanX` (rounded),
   `Sxx := Sxx + dx × dx` (product rounded, then sum rounded), then the same for `dy := value − meanY` and `Syy`. Then
   `sx := √(Sxx / n)` and `sy := √(Syy / n)`, the division and the root each rounded (0 when `n = 0`).
3. Correlation (`0x00475820`): `r := 0` if `sx = 0`, `sy = 0` or `n = 0`. Otherwise compute the reciprocals
   `ix := 1 / sx` and `iy := 1 / sy` once (each rounded), then, starting from `S := 0`, for each point:
   `a := (x − meanX) × ix`, `b := (value − meanY) × iy`, `S := S + a × b`. Each subtraction, product and addition
   is rounded. Finally `r := S / n`, rounded. So the standardised values are **multiplied by the reciprocal**, not
   divided by `sx` and `sy`. The grouping is "product of the two standardised values, summed, then divided by n".
4. The line (`0x00475CC0`): if `sx = 0`, `slope := 0` and `intercept := f32(` the range's first point's value `)`.
   Otherwise `s := (sy / sx) × r` (two roundings), `slope := f32(s)`, and
   `intercept := f32((Σy − s × Σx) / n)` (three roundings). The intercept uses the **unrounded** `s`, which differs
   from the float `slope` only at 53-bit precision.

**Precision values** (HW10). The game's precision helper has modes 24, 53 and 64 bits, but nothing in the game
selects 64 (foundations §18.1), so every call runs at 24 or 53 bits. *[choice]* FreeCity4 may treat any value
other than 24 as 53; there is no original behaviour to match.

| Method | Range | Result |
|---|---|---|
| `GetCount(key, from, to, float& n)` (slot 5) | widened | `n := (float)` the number of points of the widened range; true |
| `GetTrend(key, from, to, float& slope, float& intercept)` (slot 6) | widened | the line, slope per day and value at `from`; false for an empty range |
| `GetPoint(key, date, float& v)` (slot 8) | widened, `from = to = date` | `v :=` the line's value at `date` |
| `GetMinMaxAverage(key, from, to, float& min, float& max, float& avg)` (slot 7) | not widened | below |

`GetPoint` therefore returns the last value after the last point, the first value before the first point, a
linear interpolation between two points, the exact value at the first or last point, and at an **interior point
the value of the regression line through it and its two neighbours** *[quirk]* (§4.8).

`GetMinMaxAverage`: false if the series has no points. Otherwise `min := FLT_MAX`, `max := −FLT_MAX`, `sum := 0`,
`count := 0` (all `float`). For each point of the range: `sum += value`, `min := value` if `value < min`,
`max := value` if `value > max`, `count += 1`; note whether a point lies exactly on `from` and on `to`. If none lies
on `from`, call `GetPoint(key, from)` through the interface (slot 8) and, if it succeeds, treat its value like a
point; the same for `to`. Then `avg := sum / count`; return true. (With a non-empty series `GetPoint` always
succeeds, so `count ≥ 1`.)

Details (`0x00476280`, HW9). The two flags are set by comparing each point's date with `from` and with `to`
separately, and both tests come after the loop, so with `from = to` and no point on that date, `GetPoint(key, from)`
is called **twice** (once as `from`, once as `to`) and its value is added to `sum` twice and counts twice; with a point
on that date, neither call happens. Each addition is `sum := f32(sum + v)` with `v` the exact integer value (or the
float from `GetPoint`), and `count := f32(count + 1)`. A point value is compared exactly with the float `min` / `max`
and stored as `f32(value)`. With `from > to` the range is empty (§4.6), so the result is built from the two
`GetPoint` values alone. Example: one point (10, 4) and `from = to = 15`: `GetPoint(15)` gives 4 twice, so `min = max
= 4`, `avg = 8 / 2 = 4`; with points (10, 4) and (20, 8), `GetPoint(15)` gives 6 twice and `avg = 6`.

### 4.8 Worked examples

**Recording and lengths.** Series `K` with `SetSize(K, 2)`. Days 100, 101, 101, 102 record 5, 6, 7, 8: after day
101 the points are (100, 5), (101, 7) (the second record of day 101 overwrote 6); day 102 erases (100, 5) and
appends (102, 8).

**Coalescing (granularity 0).** Points (2024-03-05, 10), (2024-03-20, 12), (2024-04-02, 15), (2024-04-30, 11),
(2024-05-03, 9), whole range: the March group closes at 04-02 → (2024-03-01, 10); the April group closes at 05-03 →
(2024-04-01, 15); the May group is the last and stays → (2024-05-03, 9). Result: three points.

**The yearly pass in a save.** `Berlin\City - Konradshohe.sc4` (dated 2026-01-27, 143 series, record v2,
147,442 bytes), series `0xAA11F837` (296 points, `maxCount` 0): January 1 and July 1 of 2000 … 2014, then
2015-01-01, then the 1st of every month from 2015-02 to 2025-01-01, then daily points (2025-01-03, -05, -10 …) up
to 2026-01-27. This is the state after the 2025 pass (half-years in [1975, 2015], months in [2015, 2025-01-01],
each range's last group left open); the 2026 pass had not reached this key yet.

**GetPoint.** Points at days −1, 0, +1 relative to `date` with values 0, 10, 0: the widened range is all three,
`sx = √(2/3)`, `sy = √(200/9)`, `r = 0`, so the slope is 0 and the result is the mean, 3.333…, not 10. With only
the points at −1 and +1 (values 0 and 10) the result is 5.

**GetMinMaxAverage.** Points (10, 4), (20, 8), (30, 6) by day, `from = 15`, `to = 30`: the range holds (20, 8) and
(30, 6); no point on 15, so `GetPoint(15)` adds the interpolated 6; `min = 6`, `max = 8`, `avg = 20 / 3`.

## 5. Random numbers

None.

## 6. Tuning and producers

The warehouse reads no exemplar. Each producer chooses its keys and queue lengths. Informative list (constants
found at the record and set-length call sites; the producers' specs are normative):

| Producer | Keys (length, when set) |
|---|---|
| pollution, end of cycle | `0x0A1100E8`, `0xEA110130`, `0xEA110134`, `0xEA110137`, `0xEA110138` (2, in Init) |
| pollution, monthly | `0xEA110158`, `0x8AB2F258`, `0x6AB2F260`, `0xEA11016F`, `0xEA110197`, `0xEA1101A1`, `0xEA1101AA` |
| plumbing | `0x0A14EBB4`, `0xCA14EBD0`, `0xCAB2F27B`, `0x8AB2F280`, `0xCA14EBFE`, `0xCA14EC19`, `0x8A78AB06`, `0xCA78A955` |
| neighbours (commodity statistics) | `0x0A14EBB5`, `0xAA11F837`, `0xAA11F83A`, `0xCA14EBC6`, `0xEA11017B`, `0xEA11017E`, `0x4B54F333`, `0xEB54F32E` |
| demand | 18 idle-time keys (`0x0A4E2056` …), 12 daily keys `0x2A67A399` … `0x8A67A3CC` (1, in Init), export demand `0xAA772F1D`, `0x6A774F64`, `0xCA774F69` |
| budget | tax rates `0xAABAAA9F` … `0xAABAAAAA` (365, recorded daily; read by demand.md §8.1); `0x6A51178B`, `0xCA5BA3FB`, `0x4A5BA3D0` (monthly) |
| aura | `0x0A5CBF37` (mayor rating) |
| police | `0x0A5CC155`, `0x0A5CC156` (monthly), `0x4A11D354`, `0x4A11D355` (end of cycle) |
| fire protection | `0x2A11D203`, `0x2A11D204`, `0xAAAD4D87` |
| flammability | `0x2A11CB48` |
| land value | `0x6A663AC8` (1) |
| landfill | `0x2A8C4696`, `0x2A8C4697` |
| power | `0xAA11F7CE`, `0xAA11F7D2`, `0xAA11F7D5`, `0xAA11F7D8`, `0xAA11F7DC`, `0xAA11FB59`, `0x0AB2F224`, `0x4AB2F236`, nine keys `0xAA120D5E` … `0xAA120D6D` (2, in Init) |
| residential | `0xCA545CAB + g`, `0xCA545CCA + g` for g = 0…8 (2), `0x4A8A3FE2`, `0x4A8A3FE5` (2), `0xAA1A2864`, `0xAA1A2866`, `0xAA1A2CCA` (daily), `0x2A5E7009` |
| traffic, monthly | `0xEA5BA01D`, `0x8A5D4116`, nine keys `0xABB8FAAC + i`, `0xEC0BDBF8` |
| safety advisor | `0xAA662693` (GA_CRIME) |
| fire disaster, riot | `0x49D15B74`, `0xAA8C87C7` |

## 7. Persistence (`cIGZSerializable`, `GetGZCLSID() = 0x89EFA536`) *[compat]*

`Write` always writes version 2 (`0x00475D2C`), so the original also converts a version-1 record on its first save
(HW17).

```
u16  version = 2
u32  seriesCount
seriesCount × {                     (ascending key)
    u32  key
    u32  maxCount
    u32  pointCount
    pointCount × { u32 date; s32 value }   (list order, ascending date)
}
```

`Read` (`0x00476BD0`):

1. Fail if the version is above 2 (versions 0–2 are accepted). The map is **not** cleared first.
2. For each series: read key and `maxCount`; in a version-1 record `maxCount = 100` becomes 0. Then the points,
   with `prev := 0` before the first:
   - a date of `0xCDCDCDCD` becomes `prev + 1`;
   - a date `≤ prev` (unsigned) overwrites the **last** point's value (with no point yet it writes into the list
     header, *[quirk]*; FreeCity4 drops it *[choice]*); otherwise the point is appended and `prev := date`.
     Since `prev` starts at 0 for every series, the only date dropped this way is 0 (HW7).
3. Insert the series under its key. If the key already exists (the record lists a key twice, or the map already
   held it), the new series is discarded and **`Read` returns false at once** (`0x00476D99` → `0x00476DFE`): the
   rest of the record is not read, and the series inserted before stay. (`Write` never writes a key twice, so this
   needs a hand-made or corrupt record.)
4. After the last series, return true unless the stream reports an error.

**Stream failures** (HW7). `Read` does not test individual reads. A failed read leaves its destination unchanged and
fails every later read (foundations §2.1), and the loops run on with the values left from earlier reads:

- a failure among the points of a series stops it from growing. The later iterations repeat the last date read and
  only rewrite the last point with the value it has. The series is then inserted with the points read before the
  failure. One exception: if only a value read failed, a point with the previous point's value and the new date is
  appended;
- a failure on the next series' key repeats the previous key, so step 3 stops the read with false;
- a failure on `maxCount` or `pointCount`, after a good key, inserts a series under that key with the previous
  series' `maxCount` and usually one stale point (the previous series' last date and value), then the next key
  stops the read. For the first series of a record the stale values are whatever the stack held;
- in every case the result is false.

*[choice]* FreeCity4 stops at the first failed read and returns false, keeping the series completed before it and
dropping the partly read one. This differs from the original only in that partial series (whose content depends on
stale values), which no shipped save produces: every corpus record parses to its end (§7.1).

### 7.1 Corpus validation

All 421 distinct saves parse exactly:

| Version | Saves | Series per record | Size |
|---|---|---|---|
| 1 | 405 | 62 | from 874 bytes |
| 2 | 16 | 63, 65, 132, 141, 142, 143 | up to 147,442 bytes |

- `maxCount` values: 0, 1, 2 and 365 only (v1: 2,835 / 5,265 / 12,150 / 4,860 series; v2: 950 / 208 / 480 / 192).
  No v1 record has 100, so the conversion of step 2 never applies to the corpus.
- No date is `0xCDCDCDCD`. Every v1 record has repeated dates: 6 series start (2451545, 2451545, 2451545, d) and a
  reader keeps one point for the three 2000-01-01 entries. A v1 record therefore cannot be re-encoded byte for byte
  after a read; v2 records round-trip.
- The largest series has 365 points (a length-365 series); the longest unlimited one has 296 (§4.8).

## 8. Interface slot table *[compat]*

`cISC4HistoryWarehouse` (vtable `0x00A8CCB4`). The two `Dispose` overloads are in the reverse of the SDK's order
and `Coalesce` has two dates (foundations §16).

| Slot | Offset | Method | Original | Args (dwords) | Callee pops | Return |
|---|---|---|---|---|---|---|
| 0 | `+0x00` | `QueryInterface(iid, void**)` | `0x00475660` | 2 | 8 | AL |
| 1 | `+0x04` | `AddRef()` | `0x004F0240` (shared) | 0 | 0 | EAX |
| 2 | `+0x08` | `Release()` | `0x00476AA0` | 0 | 0 | EAX |
| 3 | `+0x0C` | `Init()` | `0x004766F0` | 0 | 0 | AL (1) |
| 4 | `+0x10` | `Shutdown()` | `0x004758D0` | 0 | 0 | AL (1) |
| 5 | `+0x14` | `GetCount(key, const cIGZDate& from, const cIGZDate& to, float&)` | `0x00476160` | 4 | 16 | AL |
| 6 | `+0x18` | `GetTrend(key, from, to, float& slope, float& intercept)` | `0x004761F0` | 5 | 20 | AL |
| 7 | `+0x1C` | `GetMinMaxAverage(key, from, to, float& min, float& max, float& avg)` | `0x00476280` | 6 | 24 | AL |
| 8 | `+0x20` | `GetPoint(key, const cIGZDate& date, float&)` | `0x00476490` | 3 | 12 | AL |
| 9 | `+0x24` | `Coalesce(key, from, to, int32 granularity)` | `0x00476520` | 4 | 16 | AL |
| 10 | `+0x28` | `Dispose(key)` | `0x00476650` | 1 | 4 | AL |
| 11 | `+0x2C` | `Dispose(key, from, to)` | `0x004765C0` | 3 | 12 | AL |
| 12 | `+0x30` | `RecordHistoryData(key, int32 value)` | `0x00475AA0` | 2 | 8 | not meaningful |
| 13 | `+0x34` | `SetHistoryDataQueueLength(key, uint32 n)` | `0x00477040` (→ `0x00476FC0`) | 2 | 8 | AL (1) |

## 9. Who reaches in

| Caller | Uses |
|---|---|
| almost every simulator (§6) | slot 12, or the `0x89EFA536` message directly; slot 13 at `Init` |
| demand simulator | slot 7 on the tax-rate series (demand.md §8.1) |
| advisors and Lua (`trend_value`, safety advisor) | slots 5, 6, 8, 12 (advisors.md) |
| win trend bar (UI) | subscribes to `0x89EFA536` |
| the city (component 1) | creates it, `Init`, `Shutdown`, save and load |

Answer to demand.md §15 item 7: when the tax series is missing or empty, `GetMinMaxAverage` returns false and
leaves `min`, `max` and `avg` untouched, so the caller keeps whatever its variable held.

## 10. Original quirks (reproduced)

- `QueryInterface` does not answer `cIGZMessageTarget2` (§1).
- A second record on the same day overwrites the first (§4.1); a full queue drops one point per record.
- The yearly pass leaves the last group of each range open and groups granularity 0 by month number only (§4.4).
- `Dispose(key, from, to)` keeps the last point of the range (§4.5).
- `GetPoint` at an interior point is the regression value through three points (§4.7); `GetCount` counts the
  widened range.
- `Read` does not clear the map and drops a duplicate key's later series (§7).

## 11. Open questions

See QUESTIONS.md, section "From the commercial, industrial, civic, park, history and clock specs" (HW1–HW4). HW5–HW12
(section "From the history warehouse and 24-hour clock cores") are answered in §4.1 (HW8, HW12), §4.6 (HW6), §4.7
(HW5, HW9, HW10, HW11) and §7 (HW7).

## 12. Conformance tests (must exist in `engine/conformance`)

1. **Init**: subscribes `0x89EFA536` and `0x89EFA537`; registers day and year agents with flags 3; `QueryInterface`
   answers 1, `0x69F4A803`, `0xE4FDA3D4` and nothing else.
2. **Store**: the recording example of §4.8 (overwrite on the same day, one front erase per record); a new key
   gets `maxCount` 0.
3. **SetSize**: 10 points, `SetSize(K, 3)` keeps the newest 3; `SetSize(K, 0)` keeps all; `SetSize(K, −1)` changes
   only `maxCount`.
4. **Coalesce**: the granularity-0 example; granularity 1 on points in March, May, August and November 2010 gives
   (2010-01-01, March value), then August and November untouched (last group); granularity 3 on 2011 … 2019 yearly
   points gives (2011-01-01, 2011 value) for 2011–2014 and leaves 2015–2019; month-only grouping: points
   2010-03-01, 2011-03-01 and 2011-05-01 give (2010-03-01, first value) and 2011-05-01.
5. **Thinning pass**: 3 unlimited series and 2 limited ones, SimNewYear then SimNewDay ×4: each day thins the next
   unlimited series in key order; the fourth day ends the pass (state 0). With 400 series `perPass` is 2.
6. **Dispose**: `Dispose(K, a, b)` over 4 in-range points erases 3; `Dispose(K)` empties the series but keeps it.
7. **Ranges and GetPoint**: the GetPoint example of §4.8 (3.333… and 5); before the first point → first value;
   after the last → last value; exactly at the last point → its value.
8. **GetTrend**: points (0, 0), (10, 10), (20, 20) relative to `from` → slope 1, intercept 0; a single point →
   slope 0, intercept = its value; a missing key → false.
9. **GetMinMaxAverage**: the example of §4.8; an empty series → false and outputs untouched; points exactly on
   both ends → no extra `GetPoint`.
10. **Save**: write → read → write is byte-identical for v2; a v1 record with the triple 2000-01-01 date reads as
    one point with the last value; version 3 is rejected; `0xCDCDCDCD` dates become `prev + 1`; all 421 corpus
    records parse.
11. **Inverted ranges** (HW6): points on days 10, 20, 30, 40 (values 1, 2, 3, 4), `from = 35`, `to = 15`:
    `GetCount` gives 2 (days 10 and 20); `from = 50`, `to = 45` gives 1 (the last point); `from = 8`, `to = 5`
    gives 1 (the first point); `GetMinMaxAverage(35, 15)` reads no point and adds `GetPoint(35)` and `GetPoint(15)`.
12. **Statistics order** (HW5, HW11): at 24-bit precision, points at `from`, `from + 1` and `from + 3` with values
    0, 1 and 1: `GetTrend` gives slope `0x3E924922` (0.28571421) and intercept `0x3E924928` (0.28571439). (Dividing
    by `sx` and `sy` instead of multiplying by the reciprocals would give `0x3E924924` and `0x3E924925`.) With values
    0, 3 and 8 it gives slope `0x40292492` and intercept `0x3E12492B`. These were computed by applying the rounding
    rules of §4.7 step by step.
13. **GetMinMaxAverage with `from = to`** (HW9): the two examples of §4.7 (avg 4 and 6, count 2).
14. **Read stops on a duplicate key** (HW7): a v2 record listing keys A, A, B reads A, returns false, and B is
    absent.
15. **RecordHistoryData** (HW12) returns false; **Store** with `maxCount = 0xFFFFFFFF` never trims (HW8).
