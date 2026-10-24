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
  the panels specified in §1.2; showing also redraws the legend and resets the data view (§3.3).

`Init` does not propagate the window generator's return value or the three subscription results. If it needs
the message server and that server is absent, it returns false, after setting its initialization latch; it
does not undo window initialization. Otherwise it initializes the marked-cell view and returns true. Allocation
of that view is assumed to succeed: a null allocation is subsequently dereferenced, rather than producing a
recoverable false result. These are native failure boundaries, not a transactional initialization guarantee
(`0x0079E43F`–`0x0079E489`, `0x0079E49C`–`0x0079E53E`).

### 1.1 Construction and repeated initialization (QV9) *[compat]*

Construction sets the current-view identity to zero (none), source zero, overlay enabled but not registered,
inactive, dirty false, debug false, and the last-redraw clock to a fresh native clock reading. It does not
require a caller-supplied notification baseline. Notification `0xA6B79602` compares the second 32-bit word
of its two supplied data records; unequal words mark dirty, equal words do not. This is a comparison of
the message's old/new values, not comparison with a retained owner value. Evidence: constructor
`0x007A0D50`–`0x007A0EFA` (current identity store `0x007A0ECE`, clock store `0x007A0EF4`), notification
`0x007A55E7`–`0x007A55FF`; message-target offsets are four bytes below the complete object's offsets.

Every `Init` repeats the base-window initialization and window-flag calls. Its latch protects the
subscriptions and marked-cell allocation: once set, later calls return true without repeating that work.
The latch is set **before** the missing-server false exit; a second Init after that false exit therefore
returns true without repairing the missing setup (`0x0079E43F`–`0x0079E484`, `0x0079E53A`).
`HookUpdateMessages` has no such guard: every call registers the animation target, appends its source's
message ID to the existing vector, sorts the vector, then subscribes once per entry, including duplicates.
Two hooks without an intervening unhook therefore repeat subscription attempts for the accumulated entries;
the owner neither rejects repeated hooks nor promises that the foreign message server delivers duplicates
(`0x007A17B9`–`0x007A17FE`, `0x007A193A`–`0x007A1974`).

### 1.2 Panel visibility and opening/closing (QV3) *[compat]*

The four layout operations set the following child visibility flags (window flag 1). A dash means
**leave the existing flag alone**, including on a previously visible child; it does not mean hide.
These are child flags, not a promise that an ancestor is visible. IDs, rather than inferred artwork
names, identify the panels. `L` is the current legend-content flag (`+0x9B6`, maintained by §6).

| Child ID | Small | Standard large | Desirability group | Traffic group |
|---|---|---|---|---|
| `0x8A2871B1` (B1) | show | hide | hide | hide |
| `0x8A2871B2` (B2) | show | show | show | show |
| `0xAA32BCD4` (D4) | hide | show | hide | hide |
| `0xAA32BCD5` (D5) | hide | hide | show | show |
| `0x8A2871C3` (C3, legend container) | hide | show | show | show |
| `0x8A2871C4` (C4, legend rows) | hide | L | hide | hide |
| `0xCB72F247` (CB) | — | — | hide | show |
| `0x8100`, `0x8101` | — | hide | hide | show |

Visibility setters search below the root window and ignore missing children. None of these four
operations selects a view, updates memories, or changes subgroup enable/selection flags.
Evidence: standard large `0x0079DE70`–`0x0079DF06`, desirability `0x0079DF10`–`0x0079DFAD`,
traffic `0x0079DFB0`–`0x0079E04D`, small `0x0079E050`–`0x0079E0BD`.

`ShowWindow` shows B1 only when C3 is currently hidden, always shows B2, assigns this window as
message target to each existing child in its 82-entry control-ID table, and requests view zero's
legend. It does not indiscriminately apply one of the four layouts. When desirability memory D is
still its construction sentinel 99, it additionally selects buttons 5001/6001/8001/8100/8003 where
their button interfaces exist, disables 8003–800C and changes D to `0x28`. Later shows preserve
those selections, subgroup enable states, traffic memory and alternate byte. Every show sets the
map-enabled byte and calls ResetDataView (`0x007A6290`–`0x007A64BE`).

`HideWindow` first requires child B1 to exist; otherwise it does nothing. With B1 present it hides
`0x4200`, B1, B2, D4, D5, C3 and C4, clears the map-enabled byte, then calls ResetDataView.
The group's memories, CB and the alternate controls are not individually reset/hidden by this
operation (`0x007A64D0`–`0x007A657F`).

Accepted commands `0xCA2871F0`/`0xCA2871F1` refresh highlights and bitmap, then apply standard large
and consume the message. `0xA32CACD4`, `0x0A32CAC3`, `0xEA8B80A7` and `0xEA8B80A8` apply small.
Commands `0x4A32CA92`/`0xEA8B80A6` expand according to the controls/current source: a selected
queryable 5005 takes precedence and applies desirability with D; otherwise source 76 applies traffic
with T, source 9 applies traffic with 9, and every other source applies standard large with M.
Each of those expansions requests the chosen view and legend but does not update D/T/M or change
subgroup enable/selection. A failed 5005 button query takes the source-based path. The hide buttons
`0xEA2871D9`/`0xEA2871E9` invoke HideWindow. Except for the two consumed CA commands, these accepted
paths continue through the base window handler (`0x007A5E4B`–`0x007A60BF`).

## 2. The data view exemplar *[compat]*

Data view `n` is the exemplar with key (type `0x6534284A`, group `0x690F693F`, instance `0x4A0B6800 + n`), loaded
through the resource manager (`0x00B43CA8`, slot 3, IID `0xA52160F5`). Its properties:

| Property | Type | Use |
|---|---|---|
| `0x4A0B47E5` | u32 | the **source** (§4.1) |
| `0x4A0B47E4` | u32 pairs | the colour ramp: (value, ARGB) pairs, interpolated into a 256-entry palette |
| `0x4A0B47EC` | u32 | the **scale** `s` (§4.2); absence/invalid range leaves conversion factors 256; explicit valid 256 recomputes them |
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
| `0x4A0B47E3`, `0x4A0B47F1` | u32 0–2 | E3 selects city level plus terrain display; F1 subsequently overrides the filter's level options only; 0 → 1 normal, 1 → 3 pipes, 2 → 2 subway (§3.1) |
| `0x6B7C5CDD` | u32 list | up to eight bytes kept for the source (the travel types of the traffic volume views) |
| `0x4A0B47E6` | u8 | zone display alpha; absent → zone manager's default, not the previous view's alpha (§3.1) |
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

### 2.1 Missing and malformed fields (QV6) *[compat]*

The following are the effects of a **successful nonzero exemplar load** with absent properties. They are
not defaults applied before the resource lookup, nor a reset of all owner memory on each selection.

| Absent field | Result |
|---|---|
| source E5 / layer ED / highlight E9 | 0 / 1 / 0 |
| interpolate E7 / block E8 | false / disabled; old mask storage is irrelevant while disabled |
| scale EC | scale 256 and the default factors already specified above |
| effect EA / footprint EF / ring EE | empty string / zero / zero |
| ramp E4 | retain both existing palette arrays; constructor does not initialize their entries |
| occupant list E0 | no added types; show-all default; a present empty list instead starts show-only with no types |
| invert E1 / hide E2 / force bases F0 | retain the list-derived show-all choice / false / false |
| terrain E3 | mode 1; F1 starts with the resulting E3 mode and changes it only for values 0, 1 or 2 |
| travel list `0x6B7C5CDD` | retain the previous count and bytes (construction count is zero) |
| zone E6 | use the currently queried zone-display option |
| custom EB | no new custom object; the previous one has already been shut down/released |

The two terrain fields each map 0→1, 1→3, 2→2; any other value leaves the mode entering that field
unchanged, rather than failing selection. The travel count is first narrowed to its low **eight bits**,
then capped at eight; that many initial u32 entries contribute their low byte. Thus a 256-element list
selects zero types, a 257-element list selects one, and nine selects eight. There is no range check on
an entry's other bits (`0x007A4D23`–`0x007A4E42`, `0x007A4E6E`–`0x007A4EA6`).

Except for separately documented helpers such as the legend title, the owner obtains each property by
one ID lookup and directly uses the requested mutable scalar alias or raw array pointer/count. It does
not check the variant type, convert a mismatched type, iterate duplicate IDs, or validate their agreement.
The property holder's returned property is the one consumed; duplicate records in a serialized resource
are a resource/holder concern, not multiple map-view assignments. The native variant aliases themselves
do not type-check: a scalar alias addresses inline payload, whereas an array alias loads the stored
pointer ([variant-storage.md](variant-storage.md) §4). A wrong-type scalar can therefore expose raw
payload bits; a wrong-type array can dereference a meaningless address. There is no general malformed-input
false result or substitute zero. Odd/empty ramps follow §4.3, including its unsafe read boundary.
Missing ramp means retention, not invocation with an empty ramp. Evidence: `0x007A4C22`–`0x007A4CFD`,
`0x007A4F03`–`0x007A4F45`, `0x007A50F8`–`0x007A533B`.

The resource instance is the 32-bit sum `0x4A0B6800+n` for a nonzero request, with wrapping and no overflow
rejection (`0x007A49CE`–`0x007A49E0`). An overflow to zero enters the none-selection path. A failed resource
lookup returns false after the teardown already described in §3.1; it does not restore hooks/custom state.
Allocations are not a recoverable all-or-nothing selection protocol: required constructor nodes, marked-cell
objects, the filter and layer-5 grid can subsequently be dereferenced after null allocation; containers and
temporary bitmap operations likewise have no owner-wide rollback. Do not manufacture a successful empty
view or a retry-preserving dirty state from an allocation fault. Native normally returning foreign false
results remain governed by their individual rules (§§1, 3, 5–7). See `0x007A4BF1`–`0x007A4D5E`,
`0x007A2080`–`0x007A2103`, and §1's marked-cell allocation boundary.

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

#### Terrain and zone display (QV1) *[compat]*

These settings affect the **3D city**, independently of the minimap's data source. E3 chooses the
city-view level and the two terrain display flags. Start at normal level 1; E3 values 0/1/2 choose
normal/pipes/subway levels 1/3/2. An absent or unrecognized E3 leaves normal level 1.

| E3 / selected level | Plumbing coverage display (terrain slot 86) | Surface water display (terrain slot 84) |
|---|---|---|
| absent, 0, or invalid / 1 | off | on |
| 1 / 3 (pipes) | on | off |
| 2 / 2 (subway) | off | off |

The plumbing flag enables the terrain view's pipe-coverage rendering (terrain-view §8.2), rather
than changing the source grid of this map. The water flag controls the visible surface-water pass;
turning it off does not remove water from the simulation. Slot 86 stores a global display byte;
slot 84 stores the terrain's water-display byte and, on change, emits `0x6AB97795` carrying the new
byte so the terrain view updates. Repeating the same flag does not emit another water-state change.
F1 is read **after** these terrain flags and the city level have been set. Its mapped mode controls
the occupant filter's underground/level options; it does not rerun the terrain settings. Thus
E3=1,F1=2 keeps the plumbing display on and surface water off, while using the subway filter options.
The detailed occupant-filter type rules remain in occupant-manager §13.2.8.

E6 is an **opacity byte**, not a zone-type mask or a choice of zone grid. Before each nonzero view,
obtain the zone manager's default alpha (slot 17, its retained configuration default). A present E6
replaces that byte; apply the result through slot 18. Zero makes zone display transparent; 255 is
full alpha. The setter changes the common current zone alpha and refreshes affected zone/lot visuals
only when the byte differs. It does not change the retained default. Selecting none restores the
default alpha, disables plumbing display and enables surface water, alongside the normal city level
and default filter. An absent E6 likewise restores the configured default even after an explicitly
transparent prior view. No owner-side alpha interpolation or alpha scaling is performed here.

Evidence: none reset `0x007A4A67`–`0x007A4AB5`; E3 level/terrain setup
`0x007A4D7B`–`0x007A4DA5`, later F1/filter options `0x007A4DB2`–`0x007A4E42`;
E6 default/override/application `0x007A4EBC`–`0x007A4F00`; terrain setters
`0x00741FF0`–`0x00741FFB`, `0x007434E0`–`0x0074354C`; zone default getter
`0x004DC370`–`0x004DC376` and current-alpha setter `0x004DCF60`–`0x004DD04F`.

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
domains and reset primitives are specified in §2.1 and the QV1 subsection above;
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

### 3.2.1 Panels and remembered group selections (QV3) *[compat]*

There are three independent remembered **requested** view IDs: desirability `D` (owner `+0x9A0`),
traffic volume `T` (`+0x9A4`) and top-level mode `M` (`+0x9A8`). Construction gives
`D=99,T=0x4C,M=0`, and the alternate-traffic byte `A` (`+0x9AC`) is zero. The sentinel 99 is replaced
by `0x28` on the first `ShowWindow`; it is not a selectable data view. These memories are separate
from the currently loaded view and from ResetDataView's saved/queued request.

Button exclusion selects the named button and deselects the others in its contiguous group, querying
IID `0x8810`; absent/nonbutton children are skipped. The groups are `0x5001`–`0x5011`,
`0x6001`–`0x600C`, `0x8001`–`0x8002`, `0x8003`–`0x800C`, and `0x8100`–`0x8101`.
The `0x51xx`/`0x61xx` aliases use the canonical `0x50xx`/`0x60xx` button for exclusion.

| Accepted button | Selection, legend and memory behavior |
|---|---|
| Ordinary `0x50xx`/`0x51xx` in §3.2 | Exclude in the top-level group; request its mapped view and legend; if `0xAA32BCD5` is visible, switch to the standard large layout (§1.2); set `M` to the requested ID. Leave `D,T` alone. |
| `0x5005` / `0x5105` | Exclude in the top-level group; show the desirability layout; request `D` and its legend; set `M=D`. |
| `0x6001`–`0x600C` / `0x6101`–`0x610C` | Exclude in the desirability group; request the mapped view and its legend; set `D` to that ID. Leave `M,T` and the layout alone. |
| `0x5008` / `0x5108` | Exclude in the top-level group; show the traffic layout. If button `0x8001` can be queried and is selected, request 9 and its legend and set `M=9`; otherwise request `T` and its legend and set `M=T`. Leave `T` alone. |
| `0x8001` | Exclude in `0x8001`–`0x8002`; disable `0x8003`–`0x800C`; request view 9 and its legend. Leave `D,T,M` and the layout alone. |
| `0x8002` | Exclude in `0x8001`–`0x8002`; enable `0x8003`–`0x800C`; request `T` and its legend. Leave `D,T,M` and the layout alone. |
| `0x8003`–`0x800C` | Exclude in the volume subgroup; request the corresponding view `0x4C`–`0x55` and its legend; set `T` to that ID. Leave `D,M` and the layout alone. |
| `0x8100` / `0x8101` | Exclude in the two-button group; set `A=0` / `A=1`; call ResetDataView. Leave `D,T,M` alone. |

Enable/disable acts on window flag 2, querying IID `0x22BA0121` for each of the ten volume controls;
missing controls are skipped. The handler itself does not reject a synthetically delivered volume
button merely because its window is disabled. Group entry does not recompute the subgroup's enable
state. All memories, exclusion, legend requests and layout changes above still occur when SetDataView
returns false or uses its same-view shortcut. Consequently selecting a subgroup can leave `M` stale
until its top-level group is entered again. The alternate byte selects the traffic simulator's
normal/alternate density data (and ferry counts in §4.1); it is not a new view ID. Resetting does not
force a bitmap draw past the ordinary same-view guard.

The accepted command button `0x8A2871D5` dispatches registered command **Cancel**, `0x6A935CF1`, through
the command service (`0x00B43CB0`, slot 20) with arguments zero/zero. This path does not itself select
view zero, hide a panel or change a remembered group; the registered command handler performs the
cancellation (see [view3d-window.md](view3d-window.md), command inventory).
As with the other accepted button paths, processing then continues through `cGZWinGen::DoMessage`.
A tutorial-filter rejection consumes the message before these effects.

Evidence: initial memories in constructor `0x007A0D50`–`0x007A0F05`; exclusion `0x0079E540`–`0x0079E59B`;
ordinary/subgroup helpers `0x007A0F30`–`0x007A101C`; group entries `0x007A57D6`–`0x007A5918`;
traffic enable/selection `0x007A5C0A`–`0x007A5CC7`; alternate flag and Cancel dispatch
`0x007A5E12`–`0x007A5EFC`; enable helper `0x0079E5A0`–`0x0079E5FB`.

### 3.3 `ResetDataView` (`0x007A5610`)

Select the front queued view request if there is one; otherwise select the saved view (`+0x9D4`)
while the map-enabled byte (`+0x9D8`) is set, or zero while it is clear. This is not a lookup of `M`
or the selected mode button. A queued request takes precedence even while the map is hidden.
Controls `0x8A2871D6` and `0x8A2871C5`, if present, receive visibility flag 1 as follows: only the
saved-view case shows D6 and hides C5; the queued and disabled/no-queue cases hide D6, show C5 and
invalidate C5. Redraw the selected request's legend if `0x8A2871C3` is visible
(`0x007A5615`–`0x007A56CC`).

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
| 76 | fresh volume/direction grids from the kept travel types; ferry view `0x54` uses routes (below) | unsigned 16-bit, with animated bands |

The developer types of sources 64–75 are `0x1010`, `0x1020`, `0x1030` (R), `0x3110`, `0x3120`, `0x3130` (CS),
`0x3320`, `0x3330` (CO), `0x4100`, `0x4200`, `0x4300`, `0x4400` (I), in that order.

Sources 11, 13–23 and 31–63 draw nothing *[quirk: the shipped zones and the four generic desirability views
(sources 11, 20–23) leave the map empty]*. For sources 64–75 the layer (`0x4A0B47ED`, or 6 when the debug flag
`+0x9E0` is set) picks the developer's grid: 2 → slot 14 (unsigned 16-bit), 3 and 4 → slot 13 (signed 16-bit),
5 → a grid built by `0x007A2080` (signed 16-bit), 6 → a debug grid of slot 11's entries × 12 (signed 16-bit).

#### Remaining developer layers (QV11) *[compat]*

**Layer 1 selects no grid.** It is the exemplar default but is not a handled developer layer; like
any layer other than 2–6 it leaves the newly cleared map empty. It is neither slot 12 nor a synonym
for layer 3. Layers **3 and 4 are identical** at this owner: the same slot-13 signed-16 grid, geometry,
conversion, lot handling and overlay are used. The selected layer need not be collapsed in stored
exemplar state, but no distinct rendering rule exists for those two values.

Layer 6 constructs a fresh zero-filled signed-16 grid. Its x/z dimensions are the slot-13 grid's
dimensions (grid slots 12/13); its world cell width/height are also that grid's (slots 14/15).
Developer slot 10 is the availability predicate: if false, no debug values are drawn. If true,
developer slot 11 supplies an ordered flat sequence of **12-byte records**, three signed 32-bit words
`(x,z,value)`. For each record, replace cell `(x,z)` with the low 16 bits of `value×12`, interpreted
as signed 16-bit. Unmentioned cells remain zero; later duplicate coordinates overwrite earlier ones.
There is no summation, sorting, normalization or saturation. The owner does not bounds-check record
coordinates; malformed coordinates/storage are unsafe inputs, not clipped records. The rendering walk
is the ordinary x-outer/z-inner grid walk, independent of record order after these replacements.

Layer 6 temporarily disables interpolation while passing this grid through the ordinary signed-16
conversion/drawing path, then restores the previous interpolation flag. It does not change the
exemplar's saved layer. The debug override chooses layer 6 even when ED is absent or names another
layer. Evidence: layer dispatch and shared 3/4 branch `0x007A4408`–`0x007A44A4`, debug-grid construction,
entry replacement and interpolation restoration `0x007A44E1`–`0x007A45F1`.

#### Layer 5: capacity percentage (QV4) *[compat]*

Layer 5 reuses a lazily allocated signed-16 simgrid with cell-size shift 1 and instance `0x89D5C471`.
Every refresh first copies developer slot 15's population map into it, then combines developer slot 14's
built-capacity map with it. This is **population / built capacity × 100**, not maximum-capacity occupancy.
At matching geometry, let P be the copied word interpreted as signed 16-bit and C the capacity word
interpreted as unsigned 16-bit: C=0 gives zero; otherwise store the low word of truncation toward zero of
`P×100.0f/C`. No saturation or division-by-zero marker is added. For example P=3,C=8 gives 37;
P=65535,C=2 gives -50, because the copied population word is interpreted as -1.
Evidence: owner `0x007A2080`–`0x007A213D`; copy `0x0079DDE0`; percentage arithmetic
`0x0079DDF0`–`0x0079DE30` (signed destination load, unsigned divisor, x87 multiply/divide and qword
truncation), constants `0x00A81054`=0.0f and `0x00A80AA0`=100.0f.

If the source shift differs from the owned shift, each combining walk converts geometry before invoking
its callback: a coarser destination uses the mean of the corresponding power-of-two square of source
cells, summing in 32 bits and arithmetic-shifting right by the shift difference twice (floor mean);
a finer destination replicates each source cell across its power-of-two square. Population samples are
sign-extended, capacity samples zero-extended; a callback consumes the resulting low word. Equal shifts
walk corresponding cells, x outer and z inner. This conversion is independent for the two inputs, before
the percentage operation (`0x0068C270`, `0x006B4710`; signed/unsigned loads `0x0068C3D8`, `0x006B4878`,
two arithmetic shifts `0x0068C403`/`0x0068C40A`, `0x006B48A3`/`0x006B48AA`). The resulting grid then uses
the ordinary signed-16 display path; its palette conversion is separate (QV5).

#### Traffic volume (QV2) *[compat]*

Source 76 is a reconstruction, not a simulator-owned scalar map. Each update creates three zero-filled
city grids at tract size 1 (shift 0, one value per city cell): unsigned-16 volume `V` and signed-16 directional
balances `X,Z`. There is no reuse of the preceding update's values. The retained travel-type bytes
from §2 are an ordered list, including duplicates. The alternate-count flag is true exactly when
`+0x9AC == 1`; the two traffic buttons set it to 0 or 1 (§3.2).
Construction and widths: `0x007A3621`–`0x007A376F`; tract-size geometry `0x00465643`–`0x00465699`.

**Ordinary views (current view other than `0x54`).** At each cell, a travel type contributes only when
traffic slot 23, `GetCapacity(networkType,x,z)`, returns a value strictly greater than zero. Capacity
is only an eligibility test; there is no division by it. The mapping is:

| Retained travel byte | Capacity network type |
|---|---|
| 0 | 0 |
| 1, 2, 4 | 1 |
| 3, 5 | 2 |
| 6 | 3 |
| 7 | 4 |
| 8 | 5 |
| any other byte | 1 |

For an eligible entry, traffic slot 12 `GetTrafficEdgeDensity(x,z,travelByte,alternate)` supplies
17 unsigned words. Write `h[k]` for its word at byte offset `2k`; these are raw directional counts,
not signed shorts. The scalar count is `h[16]`. Its two balance contributions are:

- `X = 2(h[2]−h[8])−h[4]−h[9]−h[12]−h[11]+h[3]+h[1]+h[6]+h[14]`.
- `Z = 2(h[7]−h[13])−h[9]−h[12]−h[1]−h[14]+h[4]+h[3]+h[11]+h[6]`.

Sum each balance across the eligible list in signed 32-bit arithmetic, then saturate each final
cell balance to `−32768…32767`. Before adding each eligible scalar count, if the scalar accumulator
is still zero, replace it with `B = low16(((s+383) mod 2^32) >> 8)`. After the list, cap the scalar
accumulator at `L = low16(((s×255) mod 2^32) >> 8)` and store it in `V`. Here `s` is the stored unsigned
scale field, including an absent scale's default 256; this bias/cap is independent of the palette
factors in §4.2. No eligible entry means zero, even when `B` is nonzero. Duplicate entries contribute
again. The per-cell sums fit signed 32 bits for the eight retained entries; the scale products/addition
have the explicit unsigned wrapping just stated. Mapping, eligibility and record arithmetic:
`0x007A3AB2`–`0x007A3D6D`; scale shifts `0x007A371A`–`0x007A373C`.

**Ferry view `0x54`.** The view identity, rather than a travel byte, selects this path. Obtain the
ordered routes in use from traffic slot 31. For each route, traverse its first compressed path and
then its second with `cSC4CompressedRouteReader` (`SetData`/`GetNext`). Every returned point adds the
path's unsigned count to `V` modulo 65536. The first/second counts are the route words at offsets
`0x14/0x16`, or `0x18/0x1A` with alternate counts. There is no ordinary-path bias, cap, capacity test
or travel-list filtering. Repeated points, including endpoints, contribute again. Each point's outgoing
direction byte contributes to one balance: 0 subtracts the count from X, 1 subtracts it from Z,
2 adds it to X, 3 adds it to Z; any other direction (including the terminal `0xFF`) changes neither.
Saturate the affected signed balance **after each point**, preserving route/path order, rather than
saturating a final sum. Coordinates are the reader's signed 16-bit cell coordinates and index the
grids directly; out-of-city routes are unchecked native inputs, not a clipping policy.
Evidence: `0x007A3789`–`0x007A3AAD`, direction tables `0x007A4920/30`, reader `0x006D6510`.

**Drawing and animation.** Before drawing V, stamp the damaged-road sprite into the bitmap at every
cell for which traffic slot 16 is true (`0x0079F4D0`). Load the bitmap key stored at `0x00B0DB80`
once per update, on the first damaged cell; a null result stops this sprite pass only. Place the
sprite centred at `(trunc(x×q),trunc(z×q))`, where `q = binary32(bitmapWidth/cityCellWidth)`, rotating
by the negative camera orientation modulo four, and clip to the bitmap. Sprite blending uses its
alpha `a`: each RGB channel is `floor((D×(256−a)+S×a+128)/256)` and output alpha is
`min(255, Da−floor((Da×a+128)/256)+a)`. See `0x0079F509`–`0x0079F5E4`, sprite helper `0x0079E600`.

When the 3D overlay is enabled, collect marked cells **before** animation: every damaged cell is
included with colour zero; an undamaged cell is included iff V is nonzero, with palette index
`low16(trunc(V×255/s))`. This index is not clamped; a zero scale or index outside the owned palette
is an unsafe numeric/access domain, not a native fallback colour. Publish the cells, parallel
colours and the temporary grid's cell sizes (`0x007A3D8B`–`0x007A3F84`).

A process-wide phase counter increments on each source-76 draw and wraps from 15 to 0. For `V>10`,
compare `abs(X/V)` with `abs(Z/V)`. Strictly larger X selects coordinate x; ties select z. A selected
ratio greater than binary64 `0.9` uses that coordinate; less than binary64 `−0.9` uses its bitwise
complement; otherwise the value is unchanged. If the chosen coordinate's low four bits equal the
phase, replace V by `low16(s)`. X/V is stored to binary64 before its threshold comparisons; Z/V
remains in x87, and the magnitude comparison uses the unspilled quotients. Thus ordinary integer
cases follow the rule above without imposing a binary32 ratio rounding. Values 0…10 never animate
(`0x007A376F`–`0x007A3789`, `0x007A3FF0`–`0x007A40D4`).

Finally draw the modified V using the standard unsigned-16 conversion (§4.2) and the temporary grid's
cell sizes. The 3D overlay flag is temporarily disabled for this draw and restored afterwards, so
the standard draw cannot replace the separately collected marked cells. The view's interpolation
and lot-block settings still apply. Before drawing, straight palette entry 255 is set to `0xCFFFFFFF`
and its premultiplied counterpart to `0xFFFFFFFF` (an explicit override, not the usual premultiplication
of `0xCFFFFFFF`). These two entries persist until a later selection rebuilds them. The conversion
factor is unchanged: absent scale 256 and explicit valid scale 256 still differ as in §4.2.
The sprite remains visible where the later palette draw is transparent. All three grids are shut down
and released. Palette overrides/overlay suppression: `0x007A3FB2`–`0x007A3FC6`,
`0x007A40ED`–`0x007A415D`; normal factor use `0x007A2BBF`–`0x007A2BE8`.

### 4.2 Values to palette indices

| Grid | Index |
|---|---|
| unsigned 8-bit | the value |
| signed 8-bit | value + 128 |
| unsigned 16-bit | `min((v × ⌊0xFFFF00 / s⌋) >> 16, 255)` |
| signed 16-bit | `128 + ((v × ⌊0x7FFF00 / s⌋) >> 16)` (64-bit, arithmetic shift), clamped to 0–255 |
| float | low byte of the signed-64-bit truncation; see §4.2.1 |

The formulas in the table apply only to a **present** scale with `1 < s < 2^24`.
Every new nonzero selection first sets both 64-bit factors to **256**, independently of the stored scale
field (initially 256). An absent scale leaves those factors unchanged; a present out-of-range value
(`0`, `1`, or at least `2^24`) is retained in the scale field but also leaves both factors 256.
A present valid value replaces them with the two quotients in the table. Consequently absent scale
and explicit scale 256 are observably different: explicit 256 gives unsigned factor 65535 and signed
factor 32767. With default factors, the unsigned index is `min(floor(v/256),255)` and the signed index
is `clamp(128+floor(v/256),0,255)`. The previous selection's factors never carry into a new nonzero view.

Products are 64-bit integer products; signed samples are sign-extended and negative right shifts round
**down**, not toward zero. Saturation follows the shift (and the signed path's addition of 128).
Thus explicit scale 1024 gives factors 16383/8191: unsigned 512 gives 127, while signed −512 gives
**64**, correcting the earlier vector's 65. Each resulting byte selects the alpha-multiplied palette
entry (`MakeMapColorBuffer`). Evidence: factor initialization/range/division/stores
`0x007A51FD`–`0x007A5287`; unsigned load/product/shift/clamp `0x007A2BBB`–`0x007A2BE3`;
signed load/product/arithmetic shift/clamp `0x007A2D49`–`0x007A2D78` (QV5).

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
- With **block by lot**, a copy of the grid is made and selected lots' rectangles are replaced by a common
  value, according to the mask/mean rule below.
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

The step is truncated **before** constructing the half-step origin; `q/2` here is integer division.
For S=2,D=3, q=43690 and the three fixed-point positions are −10923, 32767, 76457.
The middle is just below halfway. Do not replace it with exact rational cell-centre coordinates or
round the step to 43691. A 2×2 source with x rows `[0,1]`, `[1,2]` therefore has centre **0** at 3×3:
the inner rounded values are 0 and 1, followed by outer result 0. The same source reduced to 1×1
has exact half fractions on both axes and gives **2**. This latter case distinguishes the two integer
roundings from one bilinear rounding. Evidence: signed step divisions `0x0079EB8B`–`0x0079EBA7`,
integer halves/origins `0x0079EBBE`–`0x0079EBDD`, rounded inner/outer products
`0x0079ECB9`–`0x0079ECF5` (QV16).

#### Lot mask and crime icons (QV4) *[compat]*

For the unsigned-byte path, blocking happens after any cell resampling and before palette lookup. The
property E8 is a **lot-type bit mask**: obtain each lot's type byte from slot 31 and select it when
`mask & (1 << (type & 31))` is nonzero. Iterate the lot manager's slot-15 enumeration. For each selected
lot, obtain its inclusive cell rectangle (slot 14), sum its unsigned source-grid bytes, and let N be its
cell count. Replace the entire rectangle in the destination copy by `max(1, trunc((sum+floor(N/2))/N))`.
Thus half means round upward, and a completely zero lot becomes 1. Unselected cells retain the copied
grid; means read the source rather than previously replaced rectangles. The owner passes byte offset
zero to this helper. Mask zero selects no lots but still forces interpolation (§2). There is no clipping
of a bad lot rectangle or recovery from zero area/jagged data; those are unsafe accesses/division, not
validation failures. Evidence: `0x007A25E1`–`0x007A25F3`, `0x0079FF49`–`0x0079FF6D` (mask),
`0x0079FF70`–`0x007A0013` (inclusive area, byte sum, signed division, minimum 1).

Crime source 4 first draws the crime grid, then walks **all** returned incident-array entries in order;
neither incident type nor visuals-created flag filters them. Each entry supplies cell x/z and its icon
resource key (crime.md §11.4). Request its bitmap from the image service (slot 12, flag 1, using the
destination bitmap's format), then blend the icon centred at `(trunc(x×r),trunc(z×r))`, where r is the
binary32 ratio of map bitmap width to city x-cell count. The same ratio applies to both coordinates.
The rotation argument is the negated renderer orientation, reduced modulo four. The icon keeps its
native pixel size; it is not scaled to cell size. Release the returned icon after use. The service's
return value is not tested before the blit, so a missing/null icon is an unsafe boundary, not a skipped
incident (`0x007A41BD`–`0x007A42B4`; ratio store `0x007A41FC`, coordinate truncations
`0x007A4273`/`0x007A427F`). Incident entries without a lot can contain uninitialized positions; the map
does not repair the producer's quirk (crime.md §11.4).

The icon blitter (`0x0079E600`) swaps width/height for odd quarter turns, puts the upper-left at the
centre minus floor(rotated size/2), and clips the half-open rectangle to the destination. For a zero-origin
source of size W×H, the source coordinates of destination-local `(u,v)` are: turn 0 `(u,v)`, turn 1
`(W−1−v,u)`, turn 2 `(W−1−u,H−1−v)`, turn 3 `(v,H−1−u)`. Empty intersection changes nothing.
Source/destination pixel locks use flag `0x8040`; a false lock result skips the affected blend and does
not stop subsequent incidents. For straight source RGB s, alpha a, and destination channel d, each output
RGB byte is `floor((d×(256−a)+s×a+128)/256)`. Output alpha is
`min(255, destinationAlpha−floor((destinationAlpha×a+128)/256)+a)`. Even a=255 retains a 1/256 contribution
from the destination. Evidence: rotation/clip `0x0079E632`–`0x0079E793`, locks `0x0079E797`–`0x0079E823`,
blend `0x0079E881`–`0x0079E905`. These bitmap incident icons are distinct from the kind-5 hotspot effect.

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
rollback of the selection. The clock/dirty rules for individual early exits follow below; the ordinary false
results and local skip behavior specified in §§1, 3, 5 and 6 must not be replaced by a universal stop-on-error
policy.

### 7.1 Clock and redraw commitment (QV9) *[compat]*

The clock is the Windows CRT `_clock`: system FILETIME minus its CRT baseline, divided unsigned by
10000, with the low 32 bits used by this owner (`0x00932A45` thunk, `0x009F361F`–`0x009F365A`, import
`0x00A801C8` = GetSystemTimeAsFileTime). It is milliseconds of wall time, not simulation time. Tick
eligibility is an **unsigned 32-bit** subtraction `now−last`, wrapping modulo 2^32, compared unsigned
with 1000 or 250 (`0x007A55C0`–`0x007A55CD`). A small clock regression therefore looks like a large
elapsed interval and permits a redraw. No monotonic-clock rejection or saturation is native.

| Update situation | Dirty afterward | Last clock | Existing child bitmap |
|---|---|---|---|
| inactive entry | preserved | preserved | preserved |
| active, child `0x4203` interface lookup false | cleared | fresh clock | preserved |
| active, source zero (none) | cleared | fresh clock | replaced by null |
| active nonzero source with no drawing case (11, 13–23, 31–63, unknown layer with a developer) | cleared | fresh clock | replaced by the cleared temporary bitmap, plus any enabled highlights/custom work |
| missing building-development service/developer in the developer/default-source branch | cleared | preserved | preserved |
| ordinary completed draw, including normally returning false local locks/presentation/custom update | cleared | fresh clock | replacement attempted; no retry inferred from false |

Dirty is cleared immediately on active entry (`0x007A2F72`–`0x007A2F8E`), before child lookup, temporary
bitmap work or any foreign calls. The missing-child path jumps directly to the clock store; the missing
developer path exits without it (`0x007A3003`, `0x007A43EF`–`0x007A4403`). The source-zero path sends a
null bitmap and reaches custom update then the clock store (`0x007A484B`–`0x007A486F`). Other empty
sources still create/clear and install the temporary bitmap. Thus draw-nothing does **not** preserve an old
image on the normal path. The clock is sampled at the end, after the custom slot-5 call, not copied from
the tick's eligibility sample. Filter/reset calls do not themselves commit a timestamp: only the Update
they invoke does. A normally returning partial update does not restore dirty/clock. If a native fault or
exception prevents reaching the tail, there is no rollback guarantee; in particular the entry dirty clear
has already happened. A later notification/highlight change can mark dirty again.

## 8. Quirks

1. The zones view and the four generic desirability views draw nothing (§4.1).
2. A view with a scale of 1 or less, or of `2^24` or more, keeps the default factors (§2).
3. The highlights are polled on every animation tick, not on a message (§7).

## 9. Open questions

See [questions/map-view.md](questions/map-view.md): QV14 (ring precision) and QV15 (missing lot-mean prose).
QV1, QV4–QV13 and QV16 are answered in the relevant sections; see the
[answered record](questions/answered/map-view.md).

## 10. Conformance tests

1. **Index:** with explicit scale 1024, unsigned 512 gives index 127; signed −512 gives 64.
   With scale absent (also with 0, 1 or `0x1000000`), unsigned `[0,255,256,65535]` gives
   `[0,0,1,255]`, signed `[-32768,-257,-256,-1,0,255,256,32767]` gives
   `[0,126,127,127,128,128,129,255]`. With explicit scale 256 the factors are 65535/32767:
   unsigned `[0,1,255,256,512]` gives `[0,0,254,255,255]`, signed `[-512,-1,0,1,256]`
   gives `[0,127,128,128,255]`. Selecting an absent-scale resource after this restores factors 256.
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
    resampling 2×2 to 3×3 gives x rows `[0,0,1]`, `[0,0,1]`, `[1,1,2]`, with centre **0**.
    Resampling that source to 1×1 gives **2** (inner halfway results 1 and 2, then 2),
    discriminating two-stage rounding from the unrounded bilinear value 1.
22. **Float index wrap:** samples `[-1.75,-0.75,255.75,256.75,511.75,NaN,+Inf,-Inf,2^63]`
    give `[255,0,255,0,255,0,0,0,0]` under masked invalid exceptions. These cases must not be
    implemented as clamping or finite-domain rejection.
23. **Overlay cells:** final grid x rows `[0,1]`, `[2,3]`, straight colours
    `[0x00FFFFFF,0x01000000,0xFF000000,0x00000000]` gives coordinates `[(0,1),(1,0)]`
    and corresponding colours `[0x01000000,0xFF000000]`. With sizes 32×16, the first spans
    world x=[0,32], z=[16,32], with terrain/water height from overlay-views §2.2, independent
    of minimap rotation. With all palette alphas zero the replacement cell and colour arrays are empty.
24. **Lot selection/mean:** source lot bytes `[0,0,1,1]`, type 3, mask 8 → all four become 1.
    `[1,2]` becomes `[2,2]`; `[0,0]` becomes `[1,1]`. Mask 4 leaves the type-3 lot unchanged.
    Type 35 is also selected by mask 8. Mask zero changes no lot values but still enables interpolation.
25. **Layer-5 ratio:** matching-geometry population/capacity pairs `(3,8),(7,0),(65535,2),(9,4)`
    produce signed words `37,0,-50,225`, before signed-16 palette conversion. A finer population
    block `[-1,0,0,0]` reduced to one owned cell copies -1 (floor mean), not zero.
26. **Crime placement and blend:** map width 64, city width 128 cells, incident `(11,7)` → centre `(5,3)`.
    A 2×2 icon at turn 0 covers x=[4,6), z=[2,4); no filtering on visuals=false. Source `0xFFFFFFFF`
    over `0xFF000000` yields `0xFFFEFEFE`; source `0x80808080` over `0xFF000000` yields `0xFF404040`.
    Two incidents at the same point blend in supplied order.
27. **Travel narrowing:** list `[256,511,258]` → count 3, bytes `[0,255,2]`; length 9 → first 8;
    lengths 256 and 257 → count 0 and 1. A subsequent absent list retains the prior count/bytes.
28. **Absence/terrain:** after a known palette, select a resource without E4/E5/ED/E7/E8/E9/EA/EE/EF:
    palettes retained; source 0, layer 1, interpolation/block false, highlight 0, name empty, colours zero.
    E3=1,F1=99 → mode 3; E3=99,F1 absent → mode 1. Do not apply prior-view source or highlight values.
29. **Wrapped identity:** nonzero request `0xB5F49800` sums to zero and follows none-selection;
    `0xB5F49801` queries instance 1. Neither is rejected as arithmetic overflow.
30. **Init/hook repetition:** failed first Init for missing message server → false; second Init → true,
    base setup repeated, subscriptions/marked-cell creation still absent. Two source-1 hook calls
    without unhook → two animation registration attempts and three subscription attempts total
    (first vector `[0x2A09B21A]`, second vector has two equal entries).
31. **Clock wrap/regression:** dirty nontraffic owner, last=`0xFFFFFF00`, now=`0x000002E8` → elapsed 1000,
    eligible. Last=1000, now=999 → elapsed `0xFFFFFFFF`, eligible. Elapsed 999 is ineligible; traffic
    elapsed 249/250 is ineligible/eligible, with dirty forced even absent a simulator notification.
32. **Commit points:** active dirty owner, child lookup false → dirty false, clock sampled at exit, old
    bitmap preserved. Missing developer → dirty false, old clock/image retained. Source 20 with valid
    child → cleared replacement image and fresh clock. Source 0 → null image and fresh clock. Inactive
    Update preserves all three. A normally returning custom false does not undo a replacement or restore dirty.
33. **Notification comparison:** supplied old/new records' second words 5/5 leave a clean map clean;
    5/6 mark it dirty, independent of prior notifications or constructor clock.
34. **Developer layers:** valid developer and child, ED absent or 1 or 99 → cleared empty map, no
    developer grid selected. ED=3 and ED=4 with the same inputs produce identical images/marked cells.
    For ED=6, slot-13 dimensions 2×2, slot-10 true, ordered records
    `[(1,0,2),(0,1,-1),(1,0,3000)]` give signed x rows `[0,-12]`, `[-29536,0]` before palette
    conversion (`36000` wraps to −29536). Interpolation initially true is false during this draw
    and true afterwards. An empty available sequence draws zero values; unavailable sequence draws
    nothing. Debug override plus ED=2 uses this same layer-6 result.
35. **3D display flags/alpha:** default zone alpha 96, prior current alpha 0: E3 absent/E6 absent →
    level 1, plumbing off, surface water on, zone alpha 96. E3=1,E6=255 → level 3, plumbing on,
    surface water off, alpha 255. E3=2,E6=0 → level 2, plumbing off, water off, alpha 0.
    E3=1,F1=2 retains level 3/plumbing on/water off while changing only filter options to mode 2.
    Selecting none restores level 1/plumbing off/water on/alpha 96. Applying the same current
    water/alpha bytes again emits no additional setter-side visual refresh.
36. **Traffic eligibility and counts:** stored scale 256, types `[0,1,1,8,99]`; capacities positive
    for network 0/1, zero for 5. Type-0 record has `h[16]=3,h[2]=10`; type-1 has
    `h[16]=5,h[7]=4`; type-99 has `h[16]=7`; all other words zero. Requests use networks
    `[0,1,1,5,1]`, with three separate network-1 reads. Before animation the cell is
    `V=22,X=20,Z=16` (bias 2, then 3+5+5+7). Empty list or nonpositive capacities gives zero grids.
    Scale 1024 with an eligible total 2000 gives `V=1020`; a positive capacity's magnitude has no effect.
37. **Traffic record width and balance clamp:** one eligible record with `h[2]=65535,h[13]=65535`,
    total zero and scale 256 gives `V=2,X=32767,Z=−32768`, rather than signed-word input arithmetic.
    Alternate flag 1 is passed as true to every density request; flag 0 as false. Repeated updates
    rebuild from zero rather than retaining these values.
38. **Ferry accumulation order:** view `0x54`, empty travel list; successive visits to one cell with
    counts/directions `(40000,2),(40000,0),(40000,0)` give `V=54464,X=−32768,Z=0`.
    A subsequent terminal point with count 7 gives `V=54471` with the same balances. Replacing normal
    first/second path counts 3/5 by alternate 7/11 gives V=18 instead of 8 for a shared endpoint.
39. **Traffic animation and draw:** phase before update 0 becomes 1. Cells `(1,4)` with
    `V=20,X=20,Z=0` and `(14,4)` with `V=20,X=−20,Z=0` become V=256 at scale 256;
    `V=10` stays 10. A tie `X=Z=20` selects z, so `(4,1)` animates and `(1,4)` does not.
    The overlay uses the original volumes/colours, with damaged cells colour zero. Final bitmap
    conversion at explicit scale 256 gives indices 19/255 for V=20/256, while absent scale gives
    indices 0/1. Straight/premultiplied palette entry 255 becomes `0xCFFFFFFF`/`0xFFFFFFFF`.
    The overlay flag is restored and all three temporary grids released.
40. **Panel matrix:** begin with every named child visible. ShowSmallWindow leaves B1/B2 visible,
    hides D4/D5/C3/C4 and leaves CB/8100/8101 alone (ID suffixes as in §1.2).
    Desirability then shows D5/B2/C3 and hides B1/D4/C4/CB/8100/8101. Traffic shows
    D5/B2/C3/CB/8100/8101 and hides B1/D4/C4. Standard large with legend-content flag false/true
    shows D4/B2/C3, hides B1/D5/8100/8101, sets C4 false/true and leaves CB unchanged.
41. **Independent memories:** start `D=0x28,T=0x4C,M=0`, normal active selection. Press 5005,
    6002, 5002, 5105 → requests `[0x28,0x29,5,0x29]`, ending `D=M=0x29,T=0x4C`.
    Immediately after 6002, `D=0x29` but `M=0x28`. Missing the 6002 exemplar still remembers
    `D=0x29` and requests its legend; it does not restore D to the previous successfully loaded view.
42. **Traffic group:** `T=0x4C,M=5`, 8001 selected. Press 5008 → traffic layout, request 9, `M=9`.
    Press 8002 → enable all ten volume controls, request `0x4C`, `M` still 9. Press 800B →
    `T=0x54`, `M` still 9. Press 5002 then 5008 → requests 5 then `0x54`, ending `M=0x54`.
    An absent/unqueryable 8001 chooses T on entry. 8001 disables volume controls but preserves T;
    a directly delivered 800C command still selects/remembers `0x55`.
43. **Alternate/reset:** queue empty, saved view `0x54`, enabled, current view also `0x54`.
    Press 8101 → `A=1`, D6 shown/C5 hidden, Reset requests `0x54`, and no forced bitmap rebuild
    occurs through the same-view shortcut. A later actual volume rebuild uses alternate counts.
    Hide with no queue → Reset requests zero, D6 hidden/C5 visible and invalidated; with queued
    request 7 → request 7 instead. These actions do not change D/T/M. 8100 restores `A=0`.
44. **First show and layout commands:** construction gives `D=99,T=0x4C,M=0,A=0`. First ShowWindow
    selects 5001/6001/8001/8100/8003 and disables 8003–800C, replaces D by `0x28`, enables the map
    and resets it. Reopening does not repeat that initialization. With 5005 selected, expand
    0xEA8B80A6 chooses the desirability layout and D even if the current source is traffic.
    Without 5005 selected, source 76 chooses traffic/T, source 9 traffic/9, and other sources
    standard large/M. Cancel button dispatches `(0x6A935CF1,0,0)` without direct memory/layout
    changes; tutorial rejection dispatches nothing.
