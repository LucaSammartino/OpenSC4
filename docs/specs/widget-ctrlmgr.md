# `cGZWinCtrlMgr`: the window-control manager (behaviour spec)

Scope: the `cGZWinCtrlMgr` owner as tools/coverage.py assigns it on `bb641878` (48 functions, 6,805 bytes;
inventory in §8). It is the framework service that holds the UI's shared resources (system fonts, system
pictures, named colours, default texts, font-name mappings), creates the standard controls with their house
defaults, and keeps a list of plug-in control creators. Widgets reach it as `RZ::WCM()`.

Related:
- application.md AP13 (`cSC4App::SetupWinCtrlMgr` fills the tables at start-up);
- system-service-pointer.md (`cRZSysServPtr<cIGZWinCtrlMgr>`: service `0xC2C2EB0F`, interface `0x22C2EB1F`);
- widget-button.md, widget-lineinput.md, widget-scrollbar.md, widget-wingen.md, widget-controls.md (the controls it
  creates; their slots are cited from there);
- legacy-control.md §8 (UI scripts read fonts and colours through it).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; `CreateOptGrp`'s area and colour pushes and the
text-edit and `0x68963E45` branches of `CreateDefaultCtrl` were checked on the instructions. The vtables
`0x00ADC128` (`cIGZWinCtrlMgr`), `0x00ADC1D4` and `0x00ADC1EC` were read. Written 2026-10-23 (re-gzwinctrl7).

Conventions:
- Offsets are from the object start; the `cIGZWinCtrlMgr` interface is at `+0x1C` and its methods are written
  for it. "Slot n" is a `cIGZWinCtrlMgr` slot.
- `GetColor(id)` is slot 32, `SystemFont(id)` slot 5, `SystemBMP(id)` slot 7.
- "Create `(class, iid)`, else `new X (size)`": `GZCOM()->CreateInstance(class, iid, &p)` (COM slot 3); on failure
  the class is constructed directly (`GlobalHeapAlloc(size)` and its constructor; a failed allocation is not
  checked before use) and AddRef'd. The control interface is returned counted.

## 1. Identity and fields *[compat]*

- A system service (`cRZSystemService`, service ID `0xC2C2EB0F`, priority 0) with three vtables: `+0`
  `0x00ADC1EC` (the service: 8 `Init`, 9 `Shutdown`), `+4` `0x00ADC1D4` (its slot 3 is the deleting destructor
  thunk `0x00997D4B`), `+0x1C` `0x00ADC128` (`cIGZWinCtrlMgr`).
- **`QueryInterface`** (`0x00996CFB`): `0x22C2EB1F` → `+0x1C` (null for null), AddRef'd; else
  `cRZSystemService::QueryInterface`. Thunks `0x00997D43`, `0x00997D53`.
- **Constructor** (`0x009988BF`); **destructor** (`0x00997CD4`, deleting form `0x00998654`): restore the vtables,
  `Shutdown`, destroy the creator list and the five tables, release the two services.

| Offset | Meaning | Initial |
|---|---|---|
| `+0x20` | initialised (byte) | 0 |
| `+0x24` | the graphics system (`cIGZGraphicSystem`, counted) | null |
| `+0x28` | the sound manager (`cIGZSndMgr`, counted) | null |
| `+0x2C` | colour table: ID → packed `0x00RRGGBB` | empty |
| `+0x40` | system fonts: ID → font (counted) | empty |
| `+0x54` | system pictures: ID → `cIGZBuffer` (counted) | empty |
| `+0x68` | default texts: ID → string (counted) | empty |
| `+0x7C` | font mappings: ID → font-name string (counted) | empty |
| `+0x90` | control creators (a list of counted objects) | empty |
| `+0x94` | double-click time (ms) | **750** |

The tables are hash tables keyed by a 32-bit ID (helpers `0x00968ED8` find, `0x00997866` insert,
`0x00997C95` find-or-insert).

## 2. The `cIGZWinCtrlMgr` slots *[compat]*

Vtable `0x00ADC128`, 43 slots. Slots 0–2 are the QI thunk and the shared AddRef/Release; 3 and 4 are thunks to
`Init` and `Shutdown` (`0x00997D5B`, `0x00997D63`).

| Slot | Address | Behaviour |
|---|---|---|
| 5 `SystemFont(id)` | `0x00997B55` | the font, **not AddRef'd**; null when absent |
| 6 `SetSystemFont(id, font)` | `0x00998670` | replace: release and erase an existing entry; a non-null `font` is AddRef'd and inserted |
| 7 `SystemBMP(id)` / 8 `SetSystemBMP(id, image)` | `0x00997B70` / `0x009986ED` | the same for pictures |
| 9 `CreateButton(id, caption, image, rect, key)` | `0x00996E39` | slot 10 with style 2 (push button) |
| 10 `CreateButton(style, id, caption, image, rect, key)` | `0x00996D52` | §3.1 |
| 11 | `0x00996E56` | slot 10 with style 3 (toggle) |
| 12, 22, 23 | `0x00996E73` | slot 10 with style 4 (check box / radio) |
| 13 `CreateLabel(id, caption)` | `0x00996E90` | §3.2 |
| 14 `CreateOptGrp(id, caption, rect*)` | `0x00997667` | §3.3 |
| 15 `CreateWinBMP(id, image, sizeToImage)` | `0x00996F75` | §3.4 |
| 16 `CreateSlider(id, orientation, a, b, c)` | `0x0099701E` | §3.5 |
| 17 `CreateScrollbar(id, orientation, a, b, c)` | `0x009970E8` | §3.5 |
| 18 `CreateSpinner(id, a, b, c, d, e)` | `0x009971AE` | §3.6 |
| 19 `CreateLineInput(id, editable)` | `0x009972C6` | §3.7 |
| 20, 21 | `0x00996E39`, `0x00996E56` | as 9 and 11 |
| 24 `RegisterCtrlCreator(c)` | `0x00997C44` | §4 |
| 25 `RemoveCtrlCreator(c)` | `0x009977F2` | §4 |
| 26 `CreateCtrl(type, iid, out)` | `0x009977A9` | §4 |
| 27 `SetSystemSndVol`, 28 `GetSystemSndVol`, 29 `SystemSndID`, 30 `SetSystemSndID` | `0x009975DC`, `0x009975EB`, `0x009975FA`, `0x0099760B` | forward (arguments untouched) to the sound manager's slots 4, 5, 6, 7; without it the getters return 0 |
| 31 `DefaultColor(id, &r, &g, &b)` | `0x00997D6B` | the three bytes; 0, 0, 0 when absent |
| 32 `GetColor(id)` | `0x00997B8B` | the graphics system's slot 34 `(r, g, b)`: a **native** colour; 0 when absent or without the graphics system |
| 33 `SetDefaultColor(id, r, g, b)` | `0x0099876A` | replace the entry with `r << 16 | g << 8 | b` |
| 34 `SetDefaultColor(id, native)` | `0x0099761A` | graphics-system slot 36 turns `native` into `r, g, b` (0, 0, 0 without it), then slot 33 |
| 35 `DefaultText(id)` / 36 `SetDefaultText(id, s)` | `0x00997BC4` / `0x009987C5` | as slots 5/6, for strings |
| 37 `GetFontMapping(id)` / 39 `SetFontMapping(id, name)` | `0x00997BDF` / `0x00998842` | as slots 5/6, for font-name strings |
| 38 `GetFontMappingID(name)` | `0x00997BFA` | the first ID (table order) whose name matches: `name` slot 10 `(entry, 1)`; 0 when none |
| 40 `GetDoubleClickTime` | `0x0099B951` | `+0x94` (a shared 6-byte getter) |
| 41 `SetDoubleClickTimeMilliseconds(ms)` | `0x00996D48` | stores `+0x94` |
| 42 `CreateDefaultCtrl(type, iid, out)` | `0x00997DD1` | §5 |

`getFontOrSystemFont(id)` (`0x00996D25`, internal): `SystemFont(id)`, or `SystemFont(0x68963C4C)` when that is null.
`0x68963C4C` is the system font every widget falls back to.

## 3. The control factories *[compat]*

### 3.1 `CreateButton(style, id, caption, image, rect, key)` (slot 10)

Create `(0x22ECFC47, 0x8810)`, else `new cGZWinBtn (0x1B4)`. With `w` its window (button slot 3):
`font := SystemFont(0x68963C54)`; a null `image` becomes `SystemBMP(0xE8963EF1)`; `w.SetID(id)` (window slot 64),
`w.SetCaption(caption)` (74), `w.SetFillColor(GetColor(0x28963F93))` (106); button `SetFont(font)` (slot 19),
`SetStyle(style, image, rect)` (12); `w.SetKeyEquivalent(key)` (79). Returns the button.

### 3.2 `CreateLabel(id, caption)` (slot 13)

Create `(0x592, 0x212CDC1F)`, else `new cGZWinText (0x114)` (its interface `+0xD8`). `font := SystemFont(0x68963C4C)`
and `fontID :=` the font's slot 12 (0 without a font). Window `SetID`, `SetCaption`; text slots 18 `(font)`, 10
`(fontID)`, 21 `(1)`, 16 `(0)`, 7 `(2, 0)`, 5 `(0, 0)`.

### 3.3 `CreateOptGrp(id, caption, rect*)` (slot 14)

Create `(0xA1336CC1, 0xA1336CC0)`, else `new cGZWinOptGrp (0x11C)`. Window `SetID`, `SetCaption`; with a rectangle
`SetArea(rect.l, rect.r, rect.t, rect.b)` (window slot 55). *Quirk*: the right and top edges are exchanged (the
pushes read `+8` before `+4`). Option-group slot 5 `(1, 1)`, `(4, 0)`, `(8, 1)` (flags: main caption on,
draw-fill off, outline on, legacy-control.md §9.20); slot 21 `(GetColor(0xA8963FAD), GetColor(0xA8963FAD),
GetColor(0x08963FAF), GetColor(0xA8963FAD))` (outline colours); slot 28 `(0, rect ? rect.b − rect.t : 128)`.

### 3.4 `CreateWinBMP(id, image, sizeToImage)` (slot 15)

Create `(0x82FE68C4, 0xC12CEA13)`, else `new cGZWinBMP (0xFC)`. Window `SetID`; bitmap slot 4 `(image)`; with
`sizeToImage` and an image, `SetSize(image W, image H)` (window slot 53; both values through leftover pushes).

### 3.5 `CreateSlider` (slot 16) and `CreateScrollbar` (slot 17)

- Slider: create `(0x21325208, 0x21325207)`, else `new cGZWinSlider (0x124)`. `SetID`; slider slot 10
  `(orientation)`; slot 12 `(SystemBMP(orientation == 1 ? 0x88963EC1 : 0xA8963EC4), 0)`; slot 6 `(a)`, slot 8 `(b)`,
  slot 5 `(c)`.
- Scroll bar: create `(0x61325A2E, 0x61325A2D)`, else `new cGZWinScrollbar (0x150)` (the **old** scroll bar; its
  interface at `+0x124`). From the interface: the window (slot 3) and a second interface (slot 4). `SetID`; on the
  second interface slot 10 `(orientation)`, slot 12 `(SystemBMP(0xE8963EC7), 0)`, slot 6 `(a)`, slot 8 `(b)`; on
  the first, slot 5 `(c)`. (`cGZWinScrollbar2` is not created here; widget-scrollbar.md.)

### 3.6 `CreateSpinner(id, a, b, c, d, e)` (slot 18)

Create `(0x612CE0C4, 0x612CE0C3)`, else `new cGZWinSpinner (0x170)`. Spinner slots: 14 `(SystemBMP(0x68963ED6), 0)`;
10 `(b, c)`; 9 `(e)`; 12 `(d)`; 4 `(a)`; 31 `(dark, dark, light, light)` with dark `GetColor(0xA8963FAD)` and light
`GetColor(0x08963FAF)`; 23 `(2, GetColor(0x28963F93))`; then window `SetID(id)`.

### 3.7 `CreateLineInput(id, editable)` (slot 19)

Create `(0x21335C5A, 0x21335C59)`, else `new cGZWinLineInput (0x150, editable 1)`. `SetEditable(editable)` (slot 28),
`SetOutlineType(2)` (11), `SetOutlineColor(0, GetColor(0xA8963FAD))`, `SetOutlineColor(1, GetColor(0x08963FAF))`
(14); window `SetID(id)` (widget-lineinput.md §2).

## 4. Control creators

- **`RegisterCtrlCreator(c)`** (slot 24): null → false. `c.QueryInterface(0xA2ED067F, &tmp)`: failure → false (the
  queried reference is **never released** on success either). Then `RemoveCtrlCreator(c)` (no duplicates), `c`
  slot 4 (init); true → add `c` to the creator list (counted), true; false → false.
- **`RemoveCtrlCreator(c)`** (slot 25): find `c`; when present, `c` slot 5 (shut down), release, remove; returns
  whether it was found.
- **`CreateCtrl(type, iid, out)`** (slot 26): each creator in list order, slot 6 `(type, iid, out)`; the first true
  wins. None → `CreateDefaultCtrl(type, iid, out)`.

## 5. `CreateDefaultCtrl(type, iid, out)` (slot 42) *[compat]*

Creates the control for a control-type ID, then `QueryInterface(iid, out)` on it; the function's own reference is
released, so `*out` holds the only one. Returns the query's result; unknown types return false.

| Type | Control | Set-up after a successful query |
|---|---|---|
| `0x88963E15` | slot 9 `(0, "")` (push button) | — |
| `0xC8963E1D` | slot 10 `(2, 0, "")` (push button) | — |
| `0xE8963E24` | slot 10 `(3, 0, "")` (toggle) | — |
| `0x68963E21` | slot 11 `(0, "")` (toggle) | — |
| `0xA8963E4B`, `0x68963E48` | create `(0x22ECFC47, 0x8810)`, else `new cGZWinBtn` | below |
| `0x48963E33` | slot 13 `(0, "")` (label) | — |
| `0x28963E2C`, `0x48963E30` | slot 17 `(0, 1 or 2, 0, 200, 100)` (scroll bar, orientation 1 / 2) | — |
| `0x48963E28` | slot 19 `(0, 1)` (editable line input) | — |
| `0x28963E36` | create `(0x598, 0x4132242B)`, else `new cGZWinListBox (0x168, 0)` | below |
| `0xC8963E3C` | create `(0xA1336CC1, 0xA1336CC0)`, else `new cGZWinOptGrp` | flags `(1, 1)`, `(4, 0)`, `(8, 1)` and outline colours as §3.3 |
| `0xE8963E41` | create `(0x231A1C58, 0x231A1C57)`, else `new cGZWinTextEdit (0x290, 0x11FF)` | below |
| `0x68963E45` | `CreateCtrl(0xE8963E41, 0x231A1C57)` (a text edit, through the creators) | below |
| `0x88963E39` | create `(0x4386D516, 0x5386D516)`, else `new cGZWinGen (0x128)` | — |
| `0xE8963E4D` | create `(0x82FE68C4, 0xC12CEA13)`, else `new cGZWinBMP` | — |

- **Check box / radio** (`0xA8963E4B`, `0x68963E48`): `image := SystemBMP(0x68963E48 ? 0xA8963EF3 : 0xC8963EF6)`;
  `font := getFontOrSystemFont(0xC8963C5C)`; frame `rect` = the image's rectangle (buffer slot 11), or without an
  image `{0, 0, 128, font line height or 32}`. Button slots: 19 `(font)`; 23 `(4, GetColor(0x08963FB9))`,
  `(2, GetColor(0x28963FBC))`, `(0, GetColor(0x08963FB7))`; 27 `(2)`; 30 `(2, 2)`; 12 `(4, image, &rect)`.
- **List box**: background `DefaultColor(0xA8963FB2)` as r, g, b into window slot 107; list-box slots 38
  `(SystemFont(0x68963C4C))`; 39 `(0, GetColor(0x08963FB7))`, `(2, GetColor(0x28963FBC))`, `(1, GetColor(0x08963FB9))`;
  42 `(GetColor(0x88963FB4))`; 46 `(1.0f)`; 51 `(1)`; 53 `(1)`; 55 `(GetColor(0xA8963FAD), GetColor(0x08963FAF))`.
- **Text edit** (`0xE8963E41`): text slots 9 `(SystemFont(0x68963C4C))`; 78 `(0x200)` (clear flag); 21
  `(GetColor(0xA8963FB2))`; 23 `(GetColor(0x88963FB4))`; 12 `(4, GetColor(0x28963FBC))`; with
  `c = GetColor(0x08963FB7)`: 12 `(0, c)`, 13 `(c)`, 18 `(c)`, 37 `(1, c)`.
- **One-line text edit** (`0x68963E45`): `font := SystemFont(0x68963C4C)`; `h :=` its line height × 1.5 (truncated),
  16 without a font; text slots 9 `(font)`, 76 `(0x200)` (set flag), 6 `(0, 0, 16, h)` (the area), 43 `()`.

## 6. Lifecycle

**`Init`** (`0x009973AA`), once: hold the graphics system and the sound manager (both may be null), then
`SetDefaultColor` (slot 33) for 22 IDs in this order:

| ID | RGB | ID | RGB |
|---|---|---|---|
| `0x68963F7D` | 0, 0, 0 | `0xE8963F9A` | 255, 255, 0 |
| `0x28963FC5` | 128, 0, 0 | `0xC8963FA2` | 0, 0, 255 |
| `0xA8963F82` | 0, 128, 0 | `0x08963FA5` | 0, 255, 255 |
| `0x48963F84` | 128, 128, 64 | `0x28963FA8` | 255, 255, 255 |
| `0x68963F8A` | 0, 0, 128 | `0x88963FAB` | 64, 64, 64 |
| `0xA8963F8C` | 128, 0, 128 | `0xA8963FAD` | 64, 64, 64 |
| `0x68963F8F` | 0, 128, 128 | `0x08963FAF` | 192, 192, 192 |
| `0x08963F91` | 128, 128, 128 | `0xA8963FB2` | 255, 255, 255 |
| `0x28963F93` | 192, 192, 192 | `0x88963FB4` | 192, 192, 192 |
| `0xE8963F95` | 255, 0, 0 | `0x08963FB9` | 160, 160, 160 |
| `0x28963F98` | 0, 255, 0 | `0x28963FBC` | 255, 255, 255 |

The application then sets its own palette over these (application.md AP13). Returns true.

**`Shutdown`** (`0x00997A30`): release every value of the font, picture, text and font-mapping tables and clear
all five tables; remove every creator (slot 25 on the list's first entry until the list is empty); release the
two services; initialised := 0. True.

## 7. Quirks and cases

Quirks:
1. `CreateOptGrp` exchanges the rectangle's right and top edges (§3.3).
2. `RegisterCtrlCreator` keeps the reference its query added (§4).
3. The getters (slots 5, 7, 35, 37) return borrowed pointers.
4. The fallback constructions do not check the allocation.

Cases:
1. Before the application's palette: `GetColor(0xA8963FB2)` is the native form of white; `DefaultColor(0x08963FB9)`
   gives 160, 160, 160.
2. `CreateDefaultCtrl(0x48963E28, 0x21335C59, &p)`: an editable line input with a 3D outline (type 2) in the dark
   and light outline colours.
3. `CreateOptGrp(1, s, {10, 20, 110, 70})` sets the area `SetArea(10, 110, 20, 70)`, and the option height 50.
4. With a creator registered whose slot 6 accepts type `0xE8963E41`, `CreateDefaultCtrl(0x68963E45, …)` builds the
   one-line text edit on that creator's object.

## 8. Function inventory (owner `cGZWinCtrlMgr`, 48)

`0x00968ED8` table find, `0x00996CFB` QI, `0x00996D25` `getFontOrSystemFont`, `0x00996D48` slot 41, `0x00996D52`
slot 10, `0x00996E39` slots 9/20, `0x00996E56` slots 11/21, `0x00996E73` slots 12/22/23, `0x00996E90` slot 13,
`0x00996F75` slot 15, `0x0099701E` slot 16, `0x009970E8` slot 17, `0x009971AE` slot 18, `0x009972C6` slot 19,
`0x009973AA` `Init`, `0x009975DC`, `0x009975EB`, `0x009975FA`, `0x0099760B` (slots 27–30), `0x0099761A` slot 34,
`0x00997667` slot 14, `0x009977A9` slot 26, `0x009977F2` slot 25, `0x00997866` table insert, `0x00997A30`
`Shutdown`, `0x00997B55` slot 5, `0x00997B70` slot 7, `0x00997B8B` slot 32, `0x00997BC4` slot 35, `0x00997BDF`
slot 37, `0x00997BFA` slot 38, `0x00997C44` slot 24, `0x00997C95` table find-or-insert, `0x00997CD4` destructor,
`0x00997D43`, `0x00997D53` QI thunks, `0x00997D4B` deleting-destructor thunk, `0x00997D5B`/`0x00997D63` `Init` and
`Shutdown` thunks, `0x00997D6B` slot 31, `0x00997DD1` slot 42, `0x00998654` deleting destructor, `0x00998670`
slot 6, `0x009986ED` slot 8, `0x0099876A` slot 33, `0x009987C5` slot 36, `0x00998842` slot 39, `0x009988BF`
constructor.

Not covered: the slots of the created controls that have no spec yet (label, slider, old scroll bar, spinner,
option group, bitmap, list box, text edit beyond widget-controls.md), cited by number.
