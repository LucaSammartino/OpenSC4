# Spec: the data map view (`cSC4WinMapView`)

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from the Windows 1.1.641 executable (read in the instruction bytes and in a Ghidra 12.1.4 project of
the Steam build, whose code equals the GOG build's; evidence in `docs/lanes/re-dates.md`) and from the shipped
DataView exemplars. Implementers must work from this spec, the public LGPL gzcom-dll SDK headers and general
knowledge only. They must **not** consult decompiled or disassembled code (see
[CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose; *[compat-limit]* depends on something FreeCity4 cannot reproduce, so only the rule is matched.

Related specs: [ui-windows.md](ui-windows.md) (the city view slots), [simgrid.md](simgrid.md) (the grids),
[aura.md](aura.md), [landvalue.md](landvalue.md), [protection.md](protection.md), [crime.md](crime.md),
[flammability.md](flammability.md), [power.md](power.md), [pollution.md](pollution.md), [plumbing.md](plumbing.md),
[traffic-simulator.md](traffic-simulator.md), [residential-education-health.md](residential-education-health.md),
[growth-developer.md](growth-developer.md), [building-development.md](building-development.md).

## 0. Summary

The map view is the small city map with its data views (mayor rating, land value, crime, pollution, desirability,
traffic and so on). A data view is an **exemplar**: it names a data **source** (a simulator grid), a colour ramp, a
scale, the buildings whose coverage to draw, a legend, and how the 3D city view should be filtered. Choosing a view
loads the exemplar; the map is redrawn when the source's simulator reports new data, at most once a second.

## 1. The window *[compat]*

- **`Init`** (`0x0079E400`): the window generator's `Init`; subscriptions to `0x26D31EC1`, `0x26D31EC2` and
  `0xA6B79602`; a marked-cell view (`SC4MarkedCellView`, 0x50 bytes) made, initialised, told to ignore the depth
  buffer and to use the water altitude.
- **Activation**: `0x26D31EC1` registers the view for animation ticks and marks it active (`+0x142`);
  `0x26D31EC2` unregisters it and clears the flag. While inactive, `SetDataView` and `Update` do nothing.
- **`ShowWindow`** (`0x007A6290`) / **`HideWindow`** (`0x007A64D0`) / **`ShowLargeWindow`** (`0x0079DE70`) show and hide
  the panels `0x8A2871B1`, `0x8A2871B2`, `0xAA32BCD4`, `0xAA32BCD5`, `0x8A2871C3` and `0x8A2871C4` (the small map,
  the large map, the legend) *[summary]*; showing also redraws the legend and resets the data view (§3.3).

`Init` does not propagate the window generator's return value or the three subscription results. If it needs
the message server and that server is absent, it returns false, after setting its initialization latch; it
does not undo window initialization. Otherwise it initializes the marked-cell view and returns true. Allocation
of that view is assumed to succeed: a null allocation is subsequently dereferenced, rather than producing a
recoverable false result. These are native failure boundaries, not a transactional initialization guarantee
(`0x0079E43F`–`0x0079E489`, `0x0079E49C`–`0x0079E53E`).

## 2. The data view exemplar *[compat]*

Data view `n` is the exemplar with key (type `0x6534284A`, group `0x690F693F`, instance `0x4A0B6800 + n`), loaded
through the resource manager (`0x00B43CA8`, slot 3, IID `0xA52160F5`). Its properties:

| Property | Type | Use |
|---|---|---|
| `0x4A0B47E5` | u32 | the **source** (§4.1) |
| `0x4A0B47E4` | u32 pairs | the colour ramp: (value, ARGB) pairs, interpolated into a 256-entry palette |
| `0x4A0B47EC` | u32 | the **scale** `s` (§4.2); used when `1 < s < 2^24`, else 256 |
| `0x4A0B47E7` | bool | **interpolate**: a coarse grid is resampled to cells before drawing |
| `0x4A0B47E8` | u32 | **block by lot**: present → every lot is drawn as one value, with this argument (§4.3) |
| `0x4A0B47ED` | u32 | the desirability layer for the per-wealth sources (default 1, §4.1) |
| `0x4A0B47E9` | u32 | the **highlight** kind (§5) |
| `0x4A0B47EA` | string | named visual effect for highlights (`"PlopMode_Police_Existing"` …), §5.1 |
| `0x4A0B47EE`, `0x4A0B47EF` | u32 | coverage-ring and building-footprint colours, respectively (§5.2) |
| `0x4A0B47F2`, `0x4A0B47F3` | u32 lists | the legend colours and string ids (§6) |
| `0x4A0B47F4` | u32 scalar | the legend title string id (§6); not a singleton list |
| `0x4A0B47E0` | u32 list | occupant types for the 3D view's filter (the non-zero ones; occupant-manager.md §13.2.8) |
| `0x4A0B47E1` | bool | the filter shows every type but the listed ones; without it, a list shows only its types and no list shows every type |
| `0x4A0B47E2` | bool | the filter hides the base textures of empty lots, and the signposts and text decals the type test leaves out |
| `0x4A0B47F0` | bool | passed (false when absent) to the lot developer's `SetForceEmptyLotBaseTextures` (building development simulator slot 36, then lot developer slot 47) |
| `0x4A0B47E3`, `0x4A0B47F1` | u32 0–2 | terrain view mode 0 → 1, 1 → 3, 2 → 2 (normal, pipes, subway *[inferred]*) |
| `0x6B7C5CDD` | u32 list | up to eight bytes kept for the source (the travel types of the traffic volume views) |
| `0x4A0B47E6` | u8 | a zone display option *[summary]* |
| `0x4A0B47EB` | u32 | a GZCOM class made with IID `0x8A175D6B` (the pipe, subway and regional transport tools' views) |
| `0x4A0B47F5` | bool | sets a flag of the map's child `0x4203` (IID `0xCA318385`) |

The shipped views (number `n`, name, source): 1 Mayor rating (1), 2 Land value (2), 3 Police (4), 4 Crime (4),
5 Fire (6), 6 Flammability (6), 7 Power (7), 8 Air pollution (8), 9 Traffic (9), 10 Water (10), `0x14`–`0x17`
residential, commercial service, commercial office, industrial desirability (20–23), `0x18`–`0x1A` water, garbage,
radiation pollution (24–26), `0x1B` EQ (27), `0x1C` HQ (28), `0x1D` Moisture (29), `0x1E` Zones (11), `0x20` Age
(30), `0x28`–`0x2A` R$ to R$$$ (64–66), `0x32`, `0x33` CO$$, CO$$$ (70, 71), `0x34`–`0x36` CS$ to CS$$$ (67–69),
`0x3C`–`0x3F` IR, ID, IM, IHT (72–75), `0x4C`–`0x55` traffic volume (76; road, pedestrian, car, bus, subway and
elevated train, passenger train, freight truck, freight train, ferry, monorail, by the travel types of
`0x6B7C5CDD`), and `0xF0`–`0xF3` the pipe, subway, subway building and regional transport tools (no source).

## 3. Choosing a view *[compat]*

### 3.1 `SetDataView(n)` (`0x007A49B0`)

While inactive, returns false without changing the selection. When `n` is the current view, returns true
without refreshing it. Otherwise the previous custom view (`+0x9D0`) is
shut down and released, the update subscriptions dropped (`UnhookUpdateMessages`, `0x0079FB20`), and:

- **`n` = 0** (none): a default occupant filter; the city view's level (slot 23) and filter (slot 19) reset; the
  terrain's view modes (terrain slots 86 and 84) and the zone manager's display (slots 17, 18) reset; the marked
  cells cleared; source, block and highlight state cleared; `UpdateHighlights`, `Update`.
- **otherwise**: the exemplar is loaded (none → return false) and its properties stored as in §2; the palette is
  built (`InterpolateRampToPalette`, 256 entries) together with a copy whose colour channels are multiplied by
  their alpha (`floor((c × a + 128) / 256)`); the city view gets the filter and the terrain the view mode; then
  `UpdateHighlights`, `Update`, the child `0x4203` flag, the current view := `n`, `HookUpdateMessages` (§7).

#### Selection completion and foreign failures *[compat]*

Selection is not atomic. In particular, a failed exemplar lookup returns false **after** the old custom view
has been shut down/released and its update hooks removed. The old current-view identity and stored source
properties remain; no previous custom view or subscriptions are restored. Asking for that old identity again
therefore takes the same-view success shortcut, rather than repairing its subscriptions
(`0x007A49B7`–`0x007A4A21`, `0x007A4BB5`–`0x007A4BEB`, `0x007A54B5`–`0x007A54CB`).

The owner tests particular results, rather than providing a general foreign-error/rollback protocol:

| Operation/outcome | Owner behavior |
|---|---|
| Old custom view's shutdown returns normally | Ignore its return; clear the owned handle and release it before unhooking. |
| Exemplar lookup returns false | Return false as above; no further selection work. |
| Custom class creation returns false | Continue the selection without calling its initialization slot. No rollback. The factory is expected to leave no handle on failure. |
| Custom class creation succeeds | Retain its returned IID `0x8A175D6B` interface and call slot 3 (`+0x0C`) **with no arguments**. Ignore the result. There is no intervening owner/window attachment call. |
| Child `0x4203` does not provide IID `0xCA318385` | Skip the child flag/refresh work, then commit the current view and install update hooks normally. |
| Filter/terrain/zone/lot setting, custom initialization, highlight update, bitmap update or subscription call returns normally | No selection failure is synthesized from its return value. Continue to the later selection work. |

For a loaded nonzero view, custom creation/initialization precedes the highlight and bitmap updates. The
current-view identity is committed **after** those updates and the optional child work, immediately before
installing the new hooks. The method then returns true. For view zero the current identity is committed to
zero after the reset/update/child work and no new source hooks are installed. A false custom-initialization
result thus does not discard the created handle: the next different selection still shuts it down and releases
it. During `Update`, an owned custom view receives slot 5 (`+0x14`), again with no arguments
(`0x007A538B`–`0x007A53D7`, `0x007A540F`–`0x007A54B2`, `0x007A4B83`–`0x007A4BAB`, `0x007A4855`–`0x007A4864`).

This describes normally returning primitives. It does not make a null required service, an invalid success
handle, a fault or an exception into a native false result. Several required interfaces are dereferenced
without null checks. A portable adapter may diagnose those invalid inputs *[choice]*, but an opaque adapter
error and immediate stop is not the native rule for the ordinary false results listed above. Property/ramp
domains and the meanings of reset primitives remain the separate questions QV1 and QV6;
palette construction and building reconciliation are specified below (QV7–QV8).

### 3.2 Buttons (`DoMessage`, `0x007A56E0`; tutorial-filtered commands `0x287259F6`)

`OnViewModeButtonSelected(button, n)` (`0x007A0F30`) makes the button the only one down
(`SC4DoButtonExclusion`), sets view `n`, shows its legend and the large map when needed:

| Buttons | View |
|---|---|
| `0x5001` | 0 (none) |
| `0x5002`, `0x5003`, `0x5004` | 5 fire, 3 police, `0x1B` EQ |
| `0x5006`, `0x5007` | 10 water, 7 power |
| `0x5009`, `0x500A`, `0x500B`, `0x500C`, `0x500D` | `0x20` age, 2 land value, 1 mayor rating, `0x1E` zones, `0x1C` HQ |
| `0x500E`, `0x500F`, `0x5010`, `0x5011` | 8 air, `0x18` water, `0x19` garbage, `0x1A` radiation pollution |
| `0x5005`, `0x5008` | a group's last view (`0x5008`: 9 traffic when the toggle `0x8001` is down) *[summary]* |
| `0x6001`–`0x600C` | `0x28`–`0x2A`, `0x32`–`0x36`, `0x3C`–`0x3F` (desirability by wealth) |
| `0x8001` | 9 traffic; `0x8002` the last traffic volume view; `0x8003`–`0x800C` views `0x4C`–`0x55` |
| `0x8100`, `0x8101` | a two-way toggle (`+0x9AC` := 0 / 1) and `ResetDataView` *[summary]* |

The buttons `0x51xx` and `0x61xx` act as `0x50xx` and `0x60xx`. `0xCA2871F0`, `0xCA2871F1` redraw and show the
large map; `0x8A2871D5` posts the command `0x6A935CF1` (`0x00B43CB0` slot 20); `0xA32CACD4` and `0x0A32CAC3` return
to the small map (`0x0079E050`).

`OnViewModeButtonSelected` ignores the result of setting the view: after button exclusion it always requests
the **requested** view's legend, checks panel `0xAA32BCD5` and shows the large map when that panel is visible,
then remembers the requested view for the mode button. This also happens after an inactive or missing-resource
selection failure. It does not substitute the still-current view in the legend request
(`0x007A0F54`–`0x007A0F91`).

### 3.3 `ResetDataView` (`0x007A5610`)

Sets the view of the current mode button again and redraws the legend when the legend panel `0x8A2871C3` is visible
*[summary]*.

The selection is an ordinary `SetDataView` call; it does **not** bypass the same-view guard. Its result does not
gate the remaining panel updates or the visible legend refresh (`0x007A5644`–`0x007A56D1`). Thus resetting an
unchanged active view may refresh its legend without reloading its exemplar, recreating its custom view,
updating its bitmap or replacing subscriptions.

## 4. Drawing (`Update`, `0x007A2F60`) *[compat]*

### 4.1 Sources

With source 0 the map is cleared. Otherwise the map bitmap (GZCOM `0x20732180` / `0xC470D325`, 32 bits) is filled
from the source's grid:

| Source | Grid | Draw |
|---|---|---|
| 1 | aura simulator slot 12 (the mayor rating map) | signed 8-bit |
| 2 | land value slot 8 | unsigned 8-bit |
| 3, 5 | police / fire simulator slot 5 | unsigned 8-bit |
| 4 | crime slot 5, plus a marker per crime incident (crime slot 11 `GetCrimeIncidentArray`) | unsigned 8-bit |
| 6 | flammability slot 5 | unsigned 8-bit |
| 7 | power slot 35 | unsigned 8-bit |
| 8, 24, 25, 26 | pollution slot 5 `GetPollutionGrid(0, 1, 2, 3)` | signed 16-bit |
| 9 | traffic simulator slot 10 `GetCongestionMap`; the 3D overlay also marks damaged roads (slot 16) | unsigned 8-bit |
| 10 | plumbing slot 7 | unsigned 8-bit |
| 12 | aura simulator slot 14 | signed 16-bit |
| 27, 28 | the residential simulator's EQ / HQ maps (slots 21, 22) | float |
| 29 | the city slot 70 object (`0x00B43DAC`, the weather simulator *[inferred]*), slot 18 | unsigned 8-bit, own cells |
| 30 | residential slot 41 (the age map) | unsigned 8-bit |
| 64–75 | the growth developer of the source's type (below; building development slot 32) | by layer (below) |
| 76 | the traffic volume of the kept travel types *[summary; QV2]* | — |

The developer types of sources 64–75 are `0x1010`, `0x1020`, `0x1030` (R), `0x3110`, `0x3120`, `0x3130` (CS),
`0x3320`, `0x3330` (CO), `0x4100`, `0x4200`, `0x4300`, `0x4400` (I), in that order.

Sources 11, 13–23 and 31–63 draw nothing *[quirk: the shipped zones and the four generic desirability views
(sources 11, 20–23) leave the map empty]*. For sources 64–75 the layer (`0x4A0B47ED`, or 6 when the debug flag
`+0x9E0` is set) picks the developer's grid: 2 → slot 14 (unsigned 16-bit), 3 and 4 → slot 13 (signed 16-bit),
5 → a grid built by `0x007A2080` (signed 16-bit), 6 → a debug grid of slot 11's entries × 12 (signed 16-bit).

### 4.2 Values to palette indices

| Grid | Index |
|---|---|
| unsigned 8-bit | the value |
| signed 8-bit | value + 128 |
| unsigned 16-bit | `min((v × ⌊0xFFFF00 / s⌋) >> 16, 255)` |
| signed 16-bit | `128 + ((v × ⌊0x7FFF00 / s⌋) >> 16)` (64-bit, arithmetic shift), clamped to 0–255 |
| float | low byte of the signed-64-bit truncation; see §4.2.1 |

With the default scale 256 the factors are 256 (so an unsigned 16-bit value is divided by 256). Each index selects
the alpha-multiplied palette entry (`MakeMapColorBuffer`).

### 4.2.1 Float conversion outside the byte domain (QV10) *[compat]*

The float path loads each binary32 sample, calls `_ftol2`, and stores **AL**, with no range test or clamp
(`0x007A2ED9`–`0x007A2EE4`). For finite samples whose truncation is representable as signed 64-bit, truncate
toward zero and take the result modulo 256. Thus −1.75 gives 255, 256.75 gives 0 and 511.75 gives 255.
Under the native masked x87 invalid-operation exception, NaN, either infinity and samples outside the
signed-64-bit conversion domain give the integer-indefinite result and hence index **0**. This is the
qword conversion in `_ftol2` (`0x009EEF04`–`0x009EEF78`), not a saturating float-to-u8 operation. With
unmasked invalid exceptions the conversion can trap; the map owner provides no exception recovery.

### 4.3 Interpolation, lots and the 3D overlay

- With **interpolate** set and a grid coarser than the city's cells (city slots 93, 94), the grid is first
  resampled to one value per cell (`SC4InterpolateGrid`) and drawn at the cell size.
- With **block by lot**, a copy of the grid is made in which every lot's cells carry one value (`BlockMapByLot`,
  with the property's argument) *[summary]*.
- While the 3D overlay is on (`+0x140`), the marked-cell view is added once to the renderer (slot 32, layer 5,
  priority 1000) and every coloured cell is also marked in the city at the grid's cell size.

#### Owned palettes and cell resampling (QV8) *[compat]*

The ramp is a flat list of unsigned `(index, ARGB)` pairs in **supplied order**, not a sorted map
(`0x0078D520`). Fill entries before the first stop with its colour. At each palette index, advance past
every next stop whose index is less than or equal to that index; duplicate stops therefore use the last
such supplied stop, without dividing by their zero separation. After the final stop, fill with the last
colour. Between successive stops a,b, use integer weight `w=floor(256×(i−a.index)/(b.index−a.index))`.
Each of the four straight-colour channels is `a.channel + floor(((b.channel−a.channel)×w+128)/256)`;
halfway channel results round toward positive infinity, including descending channels. There is no
floating-point interpolation (`0x0078D59A`–`0x0078D62F`, integer multiply/divide, packed channel arithmetic).
No sorting, stop clamping, even-list validation or zero-length-list validation is performed. Descending
stops already passed by the monotonically increasing palette cursor are skipped; an odd list uses its
final scalar as the tail colour and can read beyond its logical end when attempting an incomplete pair.
Empty/inaccessible lists have no defined successful palette. Do not turn these unsafe inputs into a
native validation failure. For unusual unsigned indices, numerator arithmetic wraps at 32 bits before
unsigned division. The premultiplied copy retains alpha and replaces each RGB byte c with
`floor((c×alpha+128)/256)` (`0x007A4F45`–`0x007A50EC`); even opaque white becomes `0xFFFEFEFE`.

Resampling is centre-aligned, separable fixed-point bilinear interpolation, **before palette lookup**.
For a positive source axis length S and destination length D, the step is integer
`q=trunc(S×65536/D)` and destination coordinate j samples fixed coordinate
`q/2−32768+j×q`. Clamp beyond either source end to that end sample. Otherwise use the lower source
index and fractional low 16 bits f. Interpolate two values with
`L(a,b,f)=a+floor(((b−a)×f+32768)/65536)`. Interpolate the inner (z) axis on both adjacent x rows first,
then interpolate those **rounded integer** results on x. This is not one final rounding of a four-corner
weighted sum. The u8 helper emits that byte; the signed-byte helper sign-extends source bytes, performs
the same calculation, then adds 128 to make an unsigned palette index (`0x0079EB40`, `0x0079EFD0`;
rounding/width instructions `0x0079ECA7`–`0x0079ED31`, `0x0079F137`–`0x0079F18E`). No extrapolation,
wraparound neighbour or averaging of palette colours is used. Empty/jagged grids and size products that
overflow the native 32-bit arithmetic are unsafe inputs, not a new boundary sampling convention.
The owner upsamples to the city's x/z cell counts if interpolation is enabled and **either** source
dimension is smaller (`0x007A23EC`–`0x007A23FE`); destination cell sizes then become the city's world
width/height divided by its respective cell counts (16 m for an ordinary city).
Block-property presence forces interpolation on, even when its mask is zero.

#### Marked-cell inclusion and geometry (QV10) *[compat]*

For the ordinary u8 path, scan the final grid in x-then-z order. Include a coordinate iff the
**straight** palette entry's alpha byte is nonzero; RGB zero with nonzero alpha still counts, and
nonzero RGB with zero alpha does not. Each included coordinate gets that straight ARGB entry, not the
premultiplied bitmap colour (`0x007A1020`). The signed path first adds 128 modulo 256 to the signed
sample before looking up the same straight palette (`0x007A1120`); both owner calls pass zero as the
helper's optional minimum-index threshold, so negative samples are not excluded. The owner replaces the marked-cell
coordinate and colour arrays (including replacing them with empty arrays) and supplies the grid's
two world cell sizes; after resampling these are 16,16 (`0x007A2680`–`0x007A26DD`).

Coordinates are grid coordinates with origin (0,0), independent of minimap pixel scaling/rotation.
A cell (x,z) spans world `[x×w,(x+1)×w] × [z×h,(z+1)×h]`. Height, terrain clipping, corner order,
diagonal, water-max and depth offset are the marked-cell view's rules in
[overlay-views.md §2](overlay-views.md#2-the-marked-cell-view-sc4markedcellview-compat), including its
exact §2.2 corrections. This owner initializes ignoreDepth=true and useWater=true and leaves useLots
at the marked-cell constructor's false default. There is no owner-supplied vertical scale or origin
offset. Overlay registration occurs before child lookup; it does not imply that new cells were produced.

## 5. Highlights (`UpdateHighlights`, `0x007A1A00`) *[compat]*

By the highlight kind, each building of a kind is collected with its coverage radius:

| Kind | Buildings | Radius |
|---|---|---|
| 1 | schools (residential slot 47) | `GetOccupantCoverage` (residential slot 45) |
| 2 | hospitals (slot 49) | the same |
| 3 | fire stations (fire slot 22) | fire slot 21 at level 0.5 |
| 4 | police stations (police slot 21) | police slot 20 at level 0.5 |
| 5 | crime hotspot (crime slot 8 `GetCrimeHotspot`), §5.1 | 100 world units |
| 6 | garbage buildings (pollution slot 46), plus §5.0 zone occupants | zero |
| 7 | power plants (power slot 25) | zero |
| 8 | water producers (plumbing slot 28) | zero |
| 9 | transit switches (traffic simulator slot 28) | zero |

The result is compared with the previous collection (`0x007A1390`); a change marks the map dirty.

### 5.0 Building collection and reconciliation (QV7) *[compat]*

The two collections are ordered **multisets**, keyed only by the occupant interface pointer, compared as
signed 32-bit addresses. Insertion retains duplicates; enumeration order does not determine comparison order
(`0x007A0AE0`, `0x0079FBD0`). An equal non-null key preserves the old record, effect, position and radius:
**radius and position are not compared or refreshed**. Thus moving a still-listed building or changing its
coverage alone does not dirty this view. Adding or removing a key occurrence does dirty it, even if all the
other occurrences have the same pointer. A removed record has its highlight flag cleared and its effect
removed; a newly retained record gets the normal effect creation from §5.1. After reconciliation the temporary
collection is emptied. Evidence: pointer-only comparisons at `0x007A13DA`–`0x007A13E8`, `0x007A149D`,
`0x007A1505`–`0x007A1621`; there is no float/radius comparison in `0x007A1390`.

Kinds 6, 7, 8 and 9 all pass **binary32 zero radius**, rather than an unknown radius. Kind 6 includes both
pollution slot 46's garbage buildings and occupants found in cells whose zone byte is `0x0E`: it scans the
zone grid in x-then-z order, asks occupant-manager slot 32 for the matching occupant, and includes each
non-null result with radius zero. It does not deduplicate a building found through both routes or through
multiple cells (`0x007A1DE6`–`0x007A1F2A`). Kind 0 and unrecognized kinds collect no new building records;
comparison therefore removes any previous building highlights. The separate hotspot-name behavior is §5.1.
The collection/coverage callbacks' boolean results are not used to abort reconciliation: the entries actually
returned are consumed, so an empty returned list removes old entries even if its producer reports false.
The temporary occupant list is released with occupant-manager slot 48. Null services or allocation failure
have no owner-level recovery/rollback contract; unchecked dereferences and container failures are native
failure behavior, not a successful empty collection. The special null-occupant record follows the hotspot
replacement rule in §5.1, rather than this equal-non-null retention rule.

### 5.1 Named effects and the crime hotspot *[compat]*

`0x4A0B47EA` supplies the **visual-effect resource name**, not a command to enter a plop tool. A successful
nonzero-view load clears the previous name, then copies this property when present. The two colours are
also reset to zero before their properties are read (`0x007A52BD`–`0x007A5363`). An empty name suppresses
effect creation; it does not suppress highlight records or their bitmap decorations.

The effects manager's slot 7 (`+0x1C`, `CreateVisualEffect(name, out interface)`) creates each named effect.
On success the owner calls visual-effect slot 8 (`+0x20`, `SetTransform`) and slot 3 (`+0x0C`, `Start(false)`).
The common building-highlight effect has identity rotation, the collected world position and uniform scale
**twice the coverage radius** (stored as binary32). Its transform flag bytes are 2, then 1 when the scale
equals 1, otherwise 3. These are raw transform flags, not a choice of effect resource. Creation
failure leaves the record without a newly created effect; it does not abort highlight reconciliation.
Removing an old highlight stops its retained effect with slot 4 (`+0x10`, `Stop(false)`) and releases the
record's references; building highlights are cleared with occupant slot 17 (`+0x44`, arguments false, 0).
Adding a building marks it highlighted with that slot's arguments true, 0. The owner does not attach the
visual effect to the occupant through visual-effect slot 9
(`0x0079F620`–`0x0079F74E`, `0x007A1266`–`0x007A12A7`, `0x007A13EE`–`0x007A1408`,
`0x007A1650`–`0x007A166B`). General building comparison is specified in §5.0.

For kind 5, the crime query returns a success flag and three optional outputs: a byte of hotspot strength
and two signed 32-bit **cell coordinates**, x and z. The simulator reports false when its stored strength
is not positive; on false it writes none of these outputs. On true it writes the low byte of strength and
its stored grid coordinates multiplied by the crime grid's cell-size factor. The map uses the returned
coordinates directly and does not apply that factor a second time; strength is not used in the decoration
(`GetCrimeHotspot`, `0x0069F430`–`0x0069F48A`).

A successful hotspot poll contributes one highlight with **no occupant**, radius 100 and world position
`((x + 0.5) × 16, terrainAltitude, (z + 0.5) × 16)`. The x and z computations convert the signed integers,
add the double constant 0.5, multiply by double 16 and store binary32; terrain slot 13 (`+0x34`) supplies
altitude at that world x/z, also stored binary32. There is no building lookup or footprint for this record.
A false query contributes no hotspot record, so an old one is removed during reconciliation
(`0x007A1C21`–`0x007A1C8C`, `0x007A1DAE`–`0x007A1DC6`).

Hotspot effect lifetime has two observable stages, even on an unchanged position:

| Stage | Named-effect requests and ownership |
|---|---|
| Successful poll, nonempty name | Attempt a temporary effect with identity rotation, hotspot position, scale 100 and transform flag bytes 2, 3; start it with `false` on success. Put it in the candidate record. |
| Reconcile a newly present hotspot | The common highlight-effect operation attempts another effect at the same position, scale 200, flag bytes 2, 3. On success this replaces the candidate's temporary effect reference; that temporary reference is released without an owner `Stop` call. Retain the resulting record. |
| Reconcile an already present hotspot | Stop the previously retained effect with `false`, repeat the common scale-200 creation attempt and replace the old record with the candidate. Release the replaced references. This happens on **every** successful poll, even with identical coordinates. |
| Hotspot disappears or highlight kind becomes zero | Stop the retained effect with `false` and release the record; do not create another hotspot effect. |

Each successful hotspot poll therefore marks highlights changed, as does removal of an existing hotspot.
The map's ordinary redraw throttle still applies; effects are processed on the poll, not deferred until a
bitmap redraw. With an empty name there are no creation/start calls, but the null-occupant record still
changes on every successful poll. If the common creation fails, any successfully created temporary effect
remains in the candidate and can become the retained effect. A failed initial creation does not prevent
the common creation attempt. Normally returning `SetTransform`, `Start` and `Stop` results are ignored
(`0x007A1C90`–`0x007A1DDC`, `0x007A149D`–`0x007A1528`, `0x007A15A2`, `0x007A1707`–`0x007A1723`).

### 5.2 Bitmap highlight colours *[compat]*

These colours decorate the bitmap **after** its data grid is drawn; they do not tint or parameterize the
named 3D effects. Both properties default to zero on a new nonzero selection. A zero colour disables its
entire decoration pass (`0x007A5303`–`0x007A535D`, `0x007A45F6`–`0x007A481E`).

- **EF (`0x4A0B47EF`): building footprints.** For each retained nonnull occupant, obtain its inclusive
  cell rectangle from occupant slot 11 (`+0x2C`). With bitmap width/height `W,H` and city cell dimensions
  `Cx,Cz`, the painted half-open rectangle is
  `[trunc(left×W/Cx), trunc((right+1)×W/Cx)+1) ×
  [trunc(top×H/Cz), trunc((bottom+1)×H/Cz)+1)`.
  Products and divisions are signed 32-bit integer operations, division toward zero. Intersect this with
  the buffer clip rectangle before painting. The alpha byte of EF is the blending weight. Acquire the
  buffer with flags `0x8040` for the pass; if acquisition fails skip the pass; release that acquisition
  afterwards. Hotspots have no occupant and receive no footprint
  (`0x007A4608`–`0x007A474B`, rectangle helper `0x0079E0C0`–`0x0079E12A`).
- **EE (`0x4A0B47EE`): coverage rings.** For every retained record with a positive radius, including a
  hotspot, draw a ring at its world x/z and radius, each multiplied by
  `q = binary32(binary32(W) / cityWorldWidth)`. Store each scaled argument as binary32. Both axes use
  **width**, even on a rectangular bitmap. The ring helper obtains/releases the buffer with `0x8040`
  independently for each ring; a failed acquisition skips only that ring. EE's RGB channels provide
  the colour; its alpha byte is **ignored** by the ring rasterizer
  (`0x007A474E`–`0x007A47D4`, helper `0x0079E130`–`0x0079E3F4`).

For finite ordinary bitmap coordinates, the ring helper offsets the supplied centre by +0.5 on both axes.
For supplied centre `(u,v)` and radius `r`, candidate rows run inclusively from `ceil(v−r−0.5)` to
`floor(v+r+0.5)`, clipped to the buffer. At row `j`, store `dy² = binary32((j−binary32(v+0.5))²)` and
`R² = binary32((r+1)²)`. If `R²−dy²` is negative, draw no pixels in that row. Otherwise candidates run
from `ceil(binary32(u+0.5)−sqrt(R²−dy²))` to the corresponding `floor` with a plus sign, clipped to the
buffer. A candidate pixel `(i,j)` has distance error
`e = abs(sqrt((i−binary32(u+0.5))²+dy²)−r)`.
Only `e < 2` is blended, with weight `b = 255−floor(128×e)`. This is a soft ring, not a filled disk.
The native square roots and intermediate arithmetic use x87 precision between the stated binary32 stores
(`0x0079E156`–`0x0079E3B3`; constants 0.5, 1, 2, 128 at `0x00A84D2C`, `0x00A81228`,
`0x00A8825C`, `0x00AB7E10`). Ring-helper invalid geometric inputs are unsafe numeric inputs;
the float-grid conversion rule is separately specified in §4.2.1. Exact ring precision remains QV14.

Both passes share the pixel blend (`0x0079DC00`–`0x0079DCD8`). For source RGB `S`, destination ARGB `D`
and weight `b`, each output RGB byte is `floor((Dchannel×(256−b)+Schannel×b+128)/256)`;
output alpha is `255−floor(((255−Dalpha)×(256−b)+128)/256)`. Footprints use EF's alpha for `b`;
rings use the distance weight above. Thus a nonzero EE with alpha zero can still draw a visible ring.

## 6. The legend (`DisplayLegend`, `0x007A04F0`) *[compat]*

The legend is generated from its requested view number, independently of whether that number was successfully
selected. Before loading anything it hides the nine label controls `0x8A909E00`–`0x8A909E08` and nine swatches
`0x8A909E10`–`0x8A909E18`, and invalidates the first label. The title control is `0x8A909D6A`, queried for
IID `0x212CDC1F`. View zero sets title string `0x2A5E854F` in group `0x6A231EAA` and hides panel `0x8A2871C4`.
A nonzero view loads its exemplar afresh. A failed lookup leaves the rows hidden and the previous title
unchanged; it does not roll the hiding back (`0x007A0500`–`0x007A065B`).

`0x4A0B47F4` is read with the **scalar u32** property helper (`0x005FD480`), with initial value `0x2A5E854F`.
The two row properties use the list helper (`0x005FF510`). There is no title-list cardinality requirement:
the earlier singleton-list interpretation was incorrect. The title is localized in group `0x6A231EAA`; a
failed title-interface query skips setting it without stopping the row work (`0x007A0675`–`0x007A073D`).

Rows are driven by the **colour-list length**, retaining their original indices. A zero colour omits both
swatch and label at that index. A nonzero colour shows its swatch (IID `0xC2AFA76F`) with the colour's RGB
channels; its alpha is not passed to the swatch. The corresponding localized label uses the same index in
the string-id list, when allowed by the following native test:

| Zero-based colour index `i`, string count `m` | Label behavior |
|---|---|
| `i < m` | Use string id `i`. |
| `i == m` | Also attempts to read string id `i`: an unchecked one-past-end access. There is no defined fallback text. |
| `i > m` | Skip the label; the swatch may still be shown. |

Extra string ids do not create rows. There is no equality check, shortest-list zip or explicit nine-row cap:
nine is the number of preexisting controls hidden at entry. Longer colour lists attempt controls with larger
ids and rely on those controls existing. Missing required windows may be dereferenced; unsuccessful swatch
or label **interface queries** merely skip that control's work. A safe portable implementation may reject the
one-past-end/resource-layout cases explicitly *[choice]*; it must not claim a native empty-label convention
(`0x007A0750`–`0x007A0783`, `0x007A07C8`, `0x007A0832`–`0x007A0892`, `0x007A08C7`–`0x007A092A`,
`0x007A09FC`–`0x007A0A0C`).

## 7. Updates *[compat]*

`HookUpdateMessages` (`0x007A17B0`) registers for animation ticks (priority `0xC0` for source 76, else 0) and
subscribes to the source's message:

| Source | Message |
|---|---|
| 1 | `0x2A09B21A` |
| 2 | `0x87CC75B3` |
| 4 | `0x88EA4DFF` |
| 7 | `0x6A07C2EE` |
| 8, 24, 25, 26 | `0xFA6DD0BB`, `0x5A6DD1AA`, `0xDA6DD2CB`, `0xDA6DD35D` |
| 9, 76 | `0x69247DC7` |
| 10 | `0x6946DC34` |
| 29 | `0x09B6D58E` |
| others | the animation tick `0x86AD10EE` itself |

`DoMessage` (`0x007A54D0`): a subscribed message marks the map dirty. On each animation tick the highlights are
updated (a change marks it dirty) and a dirty map is redrawn once at least **1000 ms** have passed since the last
redraw (**250 ms**, and always dirty, for source 76). `0xA6B79602` with a changed value marks it dirty.

There is no bitmap-update success code for this tick handler to propagate: `Update` returns void. In
particular, a normally returning custom-view slot-5 result is ignored; when the common update tail is reached,
the last-redraw clock is stored after that call (`0x007A4855`–`0x007A486F`). No foreign error causes an owner
rollback of the selection. The clock/dirty rules for individual early exits remain QV9; the ordinary false
results and local skip behavior specified in §§1, 3, 5 and 6 must not be replaced by a universal stop-on-error
policy.

## 8. Quirks

1. The zones view and the four generic desirability views draw nothing (§4.1).
2. A view with a scale of 1 or less, or of `2^24` or more, keeps the default factors (§2).
3. The highlights are polled on every animation tick, not on a message (§7).

## 9. Open questions

See [questions/map-view.md](questions/map-view.md): QV1 (the 3D view filter options and the terrain and zone display
calls), QV2 (the traffic volume source 76 and its travel types), QV3 (the panels, the group buttons and the toggle
`0x8100`/`0x8101`), QV4 (`BlockMapByLot` and the crime and traffic markers), QV5 (16-bit conversions),
QV6 (raw properties/resources), QV9 (construction and update commits), QV11 (developer layers), and
QV14 (ring precision). QV7, QV8 and QV10 are answered in §§4.2.1, 4.3 and 5.0.

## 10. Conformance tests

1. **Index:** with scale 1024, an unsigned 16-bit value of 512 gives index 127; a signed 16-bit value of −512
   gives index 65.
2. **Mayor rating:** a signed 8-bit value of −128 gives index 0, +127 gives 255.
3. **Throttle:** two pollution messages 200 ms apart cause one redraw at the first tick after 1000 ms.
4. **Empty view:** choosing view `0x14` (residential desirability) leaves the map empty.
5. **Selection guards:** inactive + any request → false, no side effects; active + current request → true,
   no reload, shutdown, update or hook calls. `ResetDataView` targeting that same view still refreshes its
   legend when visible, without bypassing the guard.
6. **Failed replacement:** select A with custom handle H; request missing exemplar B → shutdown/release H,
   remove A's hooks, false, current identity and source still A. Request A again → true without reinstalling
   hooks. Through a mode button, the failed request for B still invokes legend B and remembers B.
7. **Custom initialization:** a valid exemplar supplies class K. Factory success → one no-argument slot-3
   call before highlights/update, identity commit and hooks, even if slot 3 returns false. Next different
   selection shuts down/releases that handle. Factory false with null output → no slot-3 call, remaining
   selection completes true. Child-interface query false likewise skips only child work.
8. **Initialization failure policy:** make base window Init and subscription primitives return false while
   providing valid required objects → their results alone do not make map Init false. With the needed
   message server absent → false, with the initialization latch already set; no rollback calls.
9. **Legend scalar/default:** F4 scalar `0x12345678` → title localized from `(0x6A231EAA,0x12345678)`;
   absent F4 → `0x2A5E854F`. No list length is demanded. Failed exemplar load hides all nine row pairs
   but preserves the old title. Failed title QI does not prevent valid rows from rendering.
10. **Legend unequal rows:** colours `[0xFF112233,0]`, strings `[S]` → swatch/label 0 only, safe despite
    unequal lengths. Colours `[0xFF112233]`, strings `[S,T]` → only S. Colours `[0,0xFF112233]`, strings
    `[S]` → swatch 1 and an attempted out-of-range string read at index 1; no deterministic label expected.
    A portable diagnostic is acceptable; returning a fabricated native empty label is not.
11. **Hotspot position:** successful crime output `(strength=7,x=2,z=3)`, terrain altitude 12 → one
    null-occupant record at `(40,12,56)`, radius 100. Changing only strength leaves those parameters unchanged.
12. **Hotspot effect creation:** empty previous set, nonempty name N, both creations succeed → create N,
    transform scale 100 / flags 2,3, Start(false); create N again, scale 200 / flags 2,3, Start(false).
    Release the temporary reference without Stop; retain the second effect. Highlights changed = true.
13. **Hotspot repeated/removal:** repeat vector 12 at identical coordinates → temporary creation/start,
    Stop(false) on the old retained effect, second creation/start, release replaced references, changed=true.
    Next crime query false → no creation, stop/release retained effect, changed=true. Another false with no
    other highlights → no hotspot effect work. Empty name → no create/start, but successful polls still change.
14. **Hotspot creation failures:** first creation false, second true → retain scale-200 effect; first true,
    second false → retain scale-100 temporary effect; both false → retain record with no effect. Each successful
    crime query still contributes radius 100, and false effect returns do not suppress the bitmap update.
15. **Independent colours:** EF `0x80FF0000`, EE zero, occupant cell rectangle `(2,3,3,4)`, city 16×16 cells,
    bitmap 32×32 → footprint `[4,9)×[6,11)`, no ring. Black opaque destination inside becomes `0xFF800000`.
    A hotspot contributes no such footprint. EE `0x00FF0000` with EF zero still enables red rings and no
    footprints. World width 256, bitmap width 32, hotspot `(40,12,56)`, radius 100 → ring arguments
    `(5,7,12.5)`, independent of bitmap height; named-effect transforms are unchanged by either colour.
16. **Ring blend:** ring arguments `(u=4.5,v=4.5,r=2)` in a sufficiently large clip: pixel `(7,5)` has
    `e=0`, weight 255; pixel `(5,5)` is untouched (`e=2`). EE `0x000000FF` over `0xFF000000` at `(7,5)`
    produces `0xFF0000FE`, demonstrating both the ignored source alpha and the division by 256.
17. **Building retention:** collect A(radius 10, position P), then collect the same non-null A with
    radius 20 and position Q. First reconciliation changes; second does not. Retained radius/position
    remain 10/P. Reordering distinct A/B inputs does not change comparison or recreate their effects.
18. **Building multiplicity and empty:** previous `[A,A,B]`, new `[B,A]` removes one A occurrence and
    reports changed. Kind 0 after a nonempty collection removes all building records; repeating empty
    returns unchanged. Kinds 6–9 supply radius +0.0. A false producer result with an empty list follows
    the same empty reconciliation; it is not an allocation-failure simulation.
19. **Ramp endpoints and duplicate:** stops `(2,0x00000000),(4,0xFFFFFFFF),(4,0xFF000000),
    (6,0xFF0000FF)` give palette[0..2]=0, [3]=`0x80808080`, [4]=`0xFF000000`,
    [5]=`0xFF000080`, [6..255]=`0xFF0000FF`. No sorting or duplicate averaging.
20. **Channel rounding/premultiplication:** stops `(0,0),(2,0x01010101)` give [1]=`0x01010101`;
    reverse the colours and [1] is still `0x01010101`. Straight `0xFFFFFFFF` premultiplies to
    `0xFFFEFEFE`; straight `0x80808080` premultiplies to `0x80404040`.
21. **Resampling:** a 1×2 unsigned grid `[0,100]` resampled to 1×4 gives `[0,25,75,100]`.
    The same shape with signed `[-100,100]` gives palette indices `[28,78,178,228]`.
    A constant 1×1 grid fills the entire destination. For source x rows `[0,1]`, `[1,2]`,
    resampling 2×2 to 3×3 gives centre value 2 (inner-axis halfway results 1 and 2, then 2),
    discriminating two-stage rounding from the unrounded bilinear centre value 1.
22. **Float index wrap:** samples `[-1.75,-0.75,255.75,256.75,511.75,NaN,+Inf,-Inf,2^63]`
    give `[255,0,255,0,255,0,0,0,0]` under masked invalid exceptions. These cases must not be
    implemented as clamping or finite-domain rejection.
23. **Overlay cells:** final grid x rows `[0,1]`, `[2,3]`, straight colours
    `[0x00FFFFFF,0x01000000,0xFF000000,0x00000000]` gives coordinates `[(0,1),(1,0)]`
    and corresponding colours `[0x01000000,0xFF000000]`. With sizes 32×16, the first spans
    world x=[0,32], z=[16,32], with terrain/water height from overlay-views §2.2, independent
    of minimap rotation. With all palette alphas zero the replacement cell and colour arrays are empty.
