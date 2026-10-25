# `cSC4ScriptedWindowCOMDirector`: the game's scripted-window classes (behaviour spec)

Scope: the `cSC4ScriptedWindowCOMDirector` owner as tools/coverage.py assigns it (30 functions, 2,705 bytes), plus the
owners `cSC4WinAlertBorder` (11 functions, 455 bytes), `cGZWinKnob` (13 functions, 1,176 bytes),
`fn:SC4GetSC4ScriptedWindowCOMDirector` and four class-ID getters. The director registers the game's own window classes
so that UI scripts (ui-script-service.md) can create them by class ID; the owner also holds several of those classes'
constructors, which §3 describes. The alert border (§4) and the knob (§5) have no other spec.

Related: app-directors.md (`cRZCOMDllDirector`, class lookup §2.3, the game director's start-up order §7.1);
widget-comdirector.md (the GZ window library's own class table); widget-toolkit.md (`cGZWin`); the window specs named
in §2.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function of the owners was read in the decompile; the director's constructor, the
factories, the map-view constructor, the alert border's paint and the knob's frame choice were checked on the
instructions. Written 2026-10-24 (re-gzscriptwin7).

## 1. The director *[compat]*

- **Getter** `SC4GetSC4ScriptedWindowCOMDirector` (`0x00466400`): a function-local static at `0x00B21548`, guarded by
  bit 0 of `0x00B21594`; the first call constructs it and registers `0x00A7AE20` (the `cRZCOMDllDirector` destructor on
  the static) with `atexit`. No reference is added. The game director adds it first of its 14 child directors and
  starts it at once (app-directors.md §7.1).
- **Constructor** (`0x004662B0`): `cRZCOMDllDirector`'s; vtables `0x00A87F58` and `0x00A87F2C` (`+4`); then the
  `AddCls` calls of §2, in that order (kind 1, a no-argument factory).
- **`GetDirectorID`** (`0x004663F0`, slot 13): `0xE8C5B006`. Every other slot is `cRZCOMDllDirector`'s or a shared
  stub; slot 4 (start) returns true and does nothing.

## 2. Class table *[compat]*

Each factory allocates the object (global heap) and constructs it; a failed allocation returns null. The returned
pointer is an interface inside the object.

| Class | Factory | Object, size | Constructor | Returned | Spec |
|---|---|---|---|---|---|
| `0xCA1492AC` (`GZCOM_cSC4WinAdviceListCLSID`, `0x00793070`) | `0x00793D20` | advice list, `0xE8` | `0x00793CB0` | `+0xD8` | advice-list.md |
| `0xCA318388` (`GZCOM_cSC4WinMiniMapCLSID`) | `0x007A8AF0` | minimap, `0x96C` | `0x007A8920` | `+0xD8` | minimap.md |
| `0xAA5D16A9` (`GZCOM_cSC4WinAuraBarCLSID`, `0x00797BB0`) | `0x00797F20` | aura bar, `0xF8` | `0x00797E60` | `+0xE0` | §3 |
| `0xCA5D3294` (`GZCOM_cSC4WinAlertBorderCLSID`, `0x00793EA0`) | `0x007941C0` | alert border, `0xEC` | `0x00794060` | `+0xE0` | §4 |
| `0xAA7CECFD` (`GZCOM_cSC4WinTextCLSID`, `0x007BE6F0`) | `0x007BE740` | `cSC4WinText`, `0x114` | in the factory | `+0xD8` | §3 |
| `0x28C5A41F` | `0x00466080` | data map view, `0x9E8` | `0x007A0D50` | `+0xE0` | map-view.md |
| `0xC918C6FF` | `0x004660B0` | lot configuration chooser, `0x154` | `0x0047C990` | `+0xD8` | lot-configuration-editor.md |
| `0x4918C704` | `0x004660E0` | lot configuration editor, `0x1E8` | `0x0047DA80` | `+0xD8` | lot-configuration-editor.md |
| `0x8918C708` | `0x004E13E0` | none: the factory returns null | — | — | §6 |
| `0xE918C70C` | `0x004E13E0` | none: the factory returns null | — | — | §6 |
| `0x29AAD0F6` | `0x00466110` | network lot chooser, `0x154` | `0x0049A460` | `+0xD8` | (developer tool) |
| `0x29AB0142` | `0x00466140` | network lot editor, `0x1F8` | `0x00499E00` | `+0xD8` | (developer tool) |
| `0xC7A0E17E` | `0x00466170` | RCI demand bar, `0x134` | `0x007A9770` | `+4` | §3 |
| `0xAA5C2F86` | `0x004661A0` | trend bar, `0x10C` | `0x007BF5E0` | `+4` | §3 |
| `0x89E1567C` | `0x004661D0` | transparent generic window, `0x128` | `0x0079C560` | the object | §3 |
| `0x2AC45173` | `0x004661F0` | the game's IME candidate list, `0x15C` | `0x0079C6A0` | `+4` | §3 |
| `0xCBCBF1E0` | `0x00466220` | knob, `0x108` | `0x007628E0` | `+4` | §5 |

The CLSID getters each return their constant. `0x2AC45173` is also registered by the GZ window director (its plain
IME candidate list, widget-comdirector.md §2); the root asks its children in the order they were added and this
director is added first (app-directors.md §2.3, §7.1), so a root lookup gets the game's subclass.

## 3. Constructors *[compat]*

All zero their fields unless a value is given. "Lists" and "maps" are empty `std::list` / `std::map` headers (a
sentinel node that points to itself).

- **Advice list** (`0x00793CB0`): `cGZWin`; vtables `0x00AB58B0` and the list interface at `+0xD8`; `+0xDC` = 0; the row
  list at `+0xE0` (0x78-byte sentinel); `+0xE4` = 0. advice-list.md §1 gives the fields.
- **Aura bar** (`0x00797E60`): `cGZWin`; a reference count at `+0xD8`; an interface at `+0xE0` (vtable `0x00AB6488`);
  the double at `+0xE8` = 0.5; `+0xF0` = null. Its destructor (`0x00797EC0`) restores the three vtables, releases
  `+0xF0`, ends the reference count and the `cGZWin`, and frees the memory when bit 0 of its argument is set. Its other
  methods are not in these owners.
- **`cSC4WinText`** (in `0x007BE740`): a `cGZWinText` (widget-text.md) with the game's vtables (`+0` and `+0xD8`).
- **Data map view** (`0x007A0D50`): vtables at `+0`, `+4` (`0x00AB8140`), the `cGZWinGen` at `+8` and `+0xE0`; byte
  `+0x140` = 1; `+0x148` = 3; maps at `+0x968` and `+0x974`; a vector at `+0x988` with room for 8 bytes; `+0x984` = an
  embedded interface (vtable `0x00A80810`); `+0x9A0` = 99 and `+0x9A4` = 76; a list at `+0x9C8`; `+0x9DC` = `clock()`.
  The bytes `+0x142` (active), `+0x9AC` (the toggle) and `+0x9E0` (developer grid) start at 0 (map-view.md §1, §3.2,
  §4.1).
- **Lot configuration chooser** (`0x0047C990`) and **network lot chooser** (`0x0049A460`): `cGZWinGen`; vtables; two
  maps (`+0x134` and `+0x140`, respectively `+0x134` and `+0x148`); the rest zero.
- **RCI demand bar** (`0x007A9770`): interface at `+0`, `cGZWin` at `+4`, another interface at `+0xDC`.
- **Trend bar** (`0x007BF5E0`): same shape; `+0xE0`…`+0xF4` zero; a response curve (`cSC4ResponseCurve`) at `+0xF8`.
- **Transparent generic window** (`0x0079C560`): a `cGZWinGen` (widget-wingen.md) with new vtables only.
- **The game's IME candidate list** (`0x0079C6A0`): the GZ candidate list's constructor, `+0x154` = `+0x158` = 0, new
  vtables.

## 4. The alert border (`cSC4WinAlertBorder`) *[compat]*

A `cGZWin` that frames the 3D view with a picture while something needs attention; the 3D view chooses the picture
(view3d-window.md `UpdateAlertBorder`).

- **Layout:** `cGZWin` (vtable `0x00AB5B48`); a reference count at `+0xD8` (`AddRef` `0x00793EB0`, `Release`
  `0x00793EC0`, which act on it); the border interface at `+0xE0` (vtable `0x00AB5B20`, IID `0xCA5D3290`); `+0xE4`
  the picture (null); `+0xE8` a counted reference to the global settings object `0x00B43CC4` (taken by the
  constructor).
- **`QueryInterface`** (`0x00793ED0`): `0x22BA0121` → the window; `0xCA5D3290` and 1 → the border interface; else
  `cGZWin`'s. Each success adds a reference.
- **Border interface:** slot 3 (`0x00793F80`) returns the window; slot 4 sets the picture (assign with reference
  counting; a non-null picture gets magenta, RGB (255, 0, 255), as its colour key, its slots 30 and 23); then the
  window is invalidated (slot 92). Slots 0–2 forward to the window's.
- **`GZPaint`** (`0x00794100`): when there is a picture and the settings byte at `[+0xE8].+0x0C → +0x45C` is set, the
  buffer to draw to (`+0x68`) is locked (slot 6, `0x8010`); `RZDrawBltEdge(buffer, picture, &corner, &drawArea, 0)`
  with `corner` = (0, 0, picture slot 9 ÷ 3, picture slot 10 ÷ 3), unsigned, and `drawArea` the window's `+0x24`;
  unlock (slot 7). The picture is thus a 3 × 3 grid of edge pieces. Returns true.
- **Destruction** (`0x00793F30`, deleting `0x007940E0`, thunk `0x00793F90`): releases `+0xE8`, then `+0xE4`, then
  the `cGZWin`.

## 5. The knob (`cGZWinKnob`) *[compat]*

A picture-strip control: it shows one of N frames chosen by a value between a minimum and a maximum.

- **Layout** (offsets from the object): the knob interface at `+0` (vtable `0x00AB4900`, IID `0x0BCBF1DF`); the
  `cGZWin` at `+4`; `+0xDC` the picture; `+0xE0` the font; `+0xEC` the frame count; `+0xF0` minimum (0.0); `+0xF4`
  maximum (1.0); `+0xF8` value (0.0); `+0xFC` the current frame (0); `+0x100` the font ID (`0x68963C4C`). The
  constructor also sets flag `0x200000` of the window (widget-toolkit.md §3).
- **`QueryInterface`** (`0x00762490`, thunk `0x00762960`): `0x0BCBF1DF` → the knob interface; else `cGZWin`'s.
- **Knob interface:** 3 the window; 4 `SetPicture(p)` (`0x00762680`: assign with reference counting, colour key
  magenta from the graphic system, invalidate); 5 `GetPicture`; 6/7 `Set`/`GetFrameCount`; 8/9 `Set`/`GetMin`;
  10/11 `Set`/`GetMax`; 12/13 `Set`/`GetValue`.
- **Setters** (`0x00762A20`, `0x00762A40`, `0x00762AB0`, `0x00762B20`): an unchanged count, minimum or maximum does
  nothing. Otherwise the new bound is stored and the value clamped to [min, max], the minimum tested first (below it →
  the minimum, else above the maximum → the maximum); `SetValue` clamps the same way and does nothing when the clamped
  value is unchanged. Each change then chooses the frame.
- **Frame choice** (`0x00762770`): frame 0 unless there is a picture, the frame count N > 0, the window is enabled
  (slot 72) and max > min. Otherwise `f = trunc((value − min) / (max − min) · (N − 1)) + 1`, then f := 1 when f < 1
  and f := N − 1 when f > N − 1 (unsigned compares). So frame 0 is the disabled picture and frames 1…N−1 cover the
  range, the maximum sharing the last frame. A change of frame invalidates the window (slot 92).
- **`Init`** (`0x00762570`, once): clears window flag `0x10000`, sets flag 4; `cGZWin::Init`; the font from the
  window-control manager by `+0x100` (its slot 5), else the font `0x68963C4C`.
- **`Shutdown`** (`0x00762630`) releases the picture and the font; then `cGZWin::Shutdown`.
- **`GZPaint`** (`0x00762830`): with a picture and N ≠ 0, frame width `w` = picture slot 9 ÷ N, height `h` = slot 10;
  `k` = the current frame when enabled, else 0; the draw context (`+0x6C` of the window) slot 38 copies the source
  (k·w, 0, k·w + w, h) to (0, 0, w, h). Returns true.
- **Destruction** (`0x007629A0`, deleting `0x00762B80`, thunk `0x00762990`): shuts down if initialised, releases what
  is left, then the `cGZWin`.

## 6. Quirks

- Classes `0x8918C708` and `0xE918C70C` are registered with a factory that always returns null. Class lookup does not
  check the factory result before calling its `QueryInterface` (app-directors.md §2.3), so asking for either
  dereferences null *[quirk]*. A reimplementation should report failure *[choice]*.
- With a minimum above the maximum the knob's clamp yields the minimum for values below it and the maximum otherwise;
  the frame is then 0 (max > min fails).
- The alert border draws nothing while the settings byte `+0x45C` is clear, whatever picture it holds.

## 7. Function inventory

- Director: `0x00466080`, `0x004660B0`, `0x004660E0`, `0x00466110`, `0x00466140`, `0x00466170`, `0x004661A0`,
  `0x004661D0`, `0x004661F0`, `0x00466220`, `0x004662B0`, `0x004663F0`, `0x0047C990`, `0x0049A460`, `0x007628E0`,
  `0x00793CB0`, `0x00793D20`, `0x00794060`, `0x007941C0`, `0x00797E60`, `0x00797EC0`, `0x00797F20`, `0x0079C560`,
  `0x0079C6A0`, `0x007A0D50`, `0x007A8920`, `0x007A8AF0`, `0x007A9770`, `0x007BE740`, `0x007BF5E0`.
- Alert border: `0x00793EB0`, `0x00793EC0`, `0x00793ED0`, `0x00793F30`, `0x00793F80`, `0x00793F90`, `0x00793FA0`,
  `0x00793FB0`, `0x00793FC0`, `0x007940E0`, `0x00794100`.
- Knob: `0x00762490`, `0x00762570`, `0x00762630`, `0x00762770`, `0x00762830`, `0x00762960`, `0x00762990`,
  `0x007629A0`, `0x00762A20`, `0x00762A40`, `0x00762AB0`, `0x00762B20`, `0x00762B80`.
- Others: `0x00466400` (getter), `0x00793070`, `0x00793EA0`, `0x00797BB0`, `0x007BE6F0` (class IDs).
