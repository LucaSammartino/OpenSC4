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
| `0x4A0B47EA` | string | the plop mode name of the highlighted buildings (`"PlopMode_Police_Existing"` …) |
| `0x4A0B47EE`, `0x4A0B47EF` | u32 | two highlight colours *[inferred]* |
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
  their alpha (`c × a / 255`, rounded); the city view gets the filter and the terrain the view mode; then
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
domains and the meanings of reset primitives remain the separate questions QV1 and QV6–QV8.

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
| float | the truncated value |

With the default scale 256 the factors are 256 (so an unsigned 16-bit value is divided by 256). Each index selects
the alpha-multiplied palette entry (`MakeMapColorBuffer`).

### 4.3 Interpolation, lots and the 3D overlay

- With **interpolate** set and a grid coarser than the city's cells (city slots 93, 94), the grid is first
  resampled to one value per cell (`SC4InterpolateGrid`) and drawn at the cell size.
- With **block by lot**, a copy of the grid is made in which every lot's cells carry one value (`BlockMapByLot`,
  with the property's argument) *[summary]*.
- While the 3D overlay is on (`+0x140`), the marked-cell view is added once to the renderer (slot 32, layer 5,
  priority 1000) and every coloured cell is also marked in the city at the grid's cell size.

## 5. Highlights (`UpdateHighlights`, `0x007A1A00`) *[compat]*

By the highlight kind, each building of a kind is collected with its coverage radius:

| Kind | Buildings | Radius |
|---|---|---|
| 1 | schools (residential slot 47) | `GetOccupantCoverage` (residential slot 45) |
| 2 | hospitals (slot 49) | the same |
| 3 | fire stations (fire slot 22) | fire slot 21 at level 0.5 |
| 4 | police stations (police slot 21) | police slot 20 at level 0.5 |
| 5 | the crime hot spot (crime slot 8 `GetCrimeHotspot`, with the effects manager) *[summary]* | — |
| 6 | garbage buildings (pollution slot 46) | — |
| 7 | power plants (power slot 25) | — |
| 8 | water producers (plumbing slot 28) | — |
| 9 | transit switches (traffic simulator slot 28) | none |

The result is compared with the previous set (`0x007A1390`); a change marks the map dirty.

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

## 8. Quirks

1. The zones view and the four generic desirability views draw nothing (§4.1).
2. A view with a scale of 1 or less, or of `2^24` or more, keeps the default factors (§2).
3. The highlights are polled on every animation tick, not on a message (§7).

## 9. Open questions

See [questions/map-view.md](questions/map-view.md): QV1 (the 3D view filter options and the terrain and zone display
calls), QV2 (the traffic volume source 76 and its travel types), QV3 (the panels, the group buttons and the toggle
`0x8100`/`0x8101`), QV4 (`BlockMapByLot` and the crime and traffic markers).

## 10. Conformance tests

1. **Index:** with scale 1024, an unsigned 16-bit value of 512 gives index 127; a signed 16-bit value of −512
   gives index 65.
2. **Mayor rating:** a signed 8-bit value of −128 gives index 0, +127 gives 255.
3. **Throttle:** two pollution messages 200 ms apart cause one redraw at the first tick after 1000 ms.
4. **Empty view:** choosing view `0x14` (residential desirability) leaves the map empty.
