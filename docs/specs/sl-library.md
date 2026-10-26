# The SL library: the global service cache and the `cSL*` resource helpers (behaviour spec)

Scope: the owners `SL` (15 functions, 2,248 bytes), `cSLBuffer` (8, 1,235), `cSLWinLocationSaver` (13, 1,069),
`cSLString` (7, 854), `cSLCursor` (5, 660), `cSLTokenSerialRecordIO` (2, 117), `cSLTokenField` (1, 44) and
`cSLTokenFieldBase` (1, 41) as tools/coverage.py assigns them. `SL` caches the application's, the framework's, the
city's and the 3D view's service pointers in globals that the whole game reads; the `cSL*` classes are small
helpers that load a localized string, an image, or a cursor from resources, and remember window positions.

Related: application.md (the app object, `0x00B43C94`), lua-host.md §10 (the detokenizer), neighbors.md §6 and
advisors.md §8 (`cSLString` users and its `Load` rule), persistence-manager.md (the resource manager slots),
sc-resources.md §5–§6 (the token records), widget-toolkit.md (window slots), the public gzcom-dll headers
(`cISC4City`, `cISC4App`, `cISC4View3DWin`, `cISC43DRender`, `cIGZBuffer`; their slot order was checked against the
slots the specs above already confirm).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function of the owners was read in the decompile; `SL::OnView3DInit`,
`SL::OnAppInit1`'s tail, `cSLBuffer::Load`, `cSLCursor::Load`, `cSLString::Load`'s replacement loop and the vtable
`0x00AA6E74` were read in the disassembly. Written 2026-10-24 (re-sllib7).

## 1. The global cache (`SL`) *[compat]*

Each `On…` function returns true. The globals hold **borrowed** pointers: no reference is kept (the service
lookups below go through temporary smart pointers released before return).

### 1.1 Application and framework

| Function | Effect |
|---|---|
| `OnAppCreate(app)` (`0x00601C00`) | `0x00B43C94` := app |
| `OnAppInit1()` (`0x00602290`) | the table below, in order |

| Global | Source |
|---|---|
| `0x00B43CAC` | app slot 33 `GetCheatCodeManager` |
| `0x00B43CB0` | framework service `0xEB903A32` (IID `0xCB90253A`), the command server; set even when null |
| `0x00B43CC4` | app slot 44 `GetRenderProperties` |
| `0x00B43CD0` | app slot 41 `GetGimexFactory` |
| `0x00B43CD8` | app slot 38 `GetPreferences` |
| `0x00B43CDC` | app slot 42 `GetStringDetokenizer` |
| `0x00B43CE0` | app slot 43 `GetWinLocationSaver` (§5) |
| `0x00B43CE4` | app slot 46 `GetGlyphTextureManager` |
| `0x00B43CE8` | app slot 47 `GetLuaInterpreter` |
| `0x00B43C9C` | framework service `0xC416025C` (IID `0x0073283C`), the graphics system; set even when null |
| `0x00B43CA0` | when `0x00B43C9C` is set: its interface `0xC47B747C` (the 3D device); the reference from that query is released at once |
| `0x00B43CA4` | when `0x00B43CA0` is set: its slot 3 (the graphics layer) |
| `0x00B43CA8` | framework service `0x056B906E` (IID `0x656B8EFC`), the resource manager; only when non-null |
| `0x00B43C98` | app slot 32 `GetFeatureManager` |
| `0x00B43CB4` | service `0x8414A2C2` (IID `0x6054A2FB`), the real-time debug display; only when non-null |
| `0x00B43CB8` | service `0x23CECBA0` (IID `0x106077D2`), the sound system; only when non-null |
| `0x00B43CC8` | service `0x64063387` (IID `0xC1085722`), the message server; only when non-null |
| `0x00B43CCC` | service `0x04FA845B` (IID `0x652294C7`), the message server 2; only when non-null |
| `0x00B43CD4` | service `0x1AC0E11A` (IID `0xFAC0E219`); only when non-null |

Services come from the framework's `GetSystemService` (slot 5); a missing framework leaves the value null.
Directory helpers (no result; nothing when the app global is null): `AppDirectory(out)` (`0x00602210`, app slot 51),
`DataDirectory(out)` (`0x00602230`, app slot 53 with index −1), `UserDataDirectory(out)` (`0x00602270`, app slot
58).

### 1.2 City

| Function | Effect |
|---|---|
| `OnCityCreate(city)` (`0x00601C10`) | `0x00B43CEC` := city |
| `OnCityInit()` (`0x00601C20`) | each global below := the city getter (`cISC4City` slot); no null checks |
| `OnCityShutdown()` (`0x00601FF0`) | every global below and `0x00B43CFC` := 0; the city global stays |
| `NetworkManager()` (`0x00602680`) | city slot 38 `GetNetworkManager`, live (0 without a city) |
| `TerrainView()` (`0x00602160`) | null terrain → 0; else terrain slot 66 (terrain.md §11) |

| Global | City slot | | Global | City slot |
|---|---|---|---|---|
| `0x00B43CF0` | 83 surface water | | `0x00B43D64` | 52 budget |
| `0x00B43CF4`, `0x00B43CF8` | 84 terrain (twice) | | `0x00B43D68` | 53 building development |
| `0x00B43D00` | 31 demolition | | `0x00B43D6C` | 54 commercial |
| `0x00B43D04` | 39 dispatch manager | | `0x00B43D70` | 55 crime |
| `0x00B43D08` | 33 lot manager | | `0x00B43D74` | 56 demand |
| `0x00B43D0C` | 34 occupant manager | | `0x00B43D78` | 57 fire protection |
| `0x00B43D10` | 35 prop manager | | `0x00B43D7C` | 58 flammability |
| `0x00B43D14` | 36 zone manager | | `0x00B43D80` | 59 flora |
| `0x00B43D18` | 37 lot configuration manager | | `0x00B43D84` | 60 industrial |
| `0x00B43D1C` | 85 effects manager | | `0x00B43D88` | 61 land value |
| `0x00B43D20` | 79 lot developer | | `0x00B43D8C` | 62 neighbours |
| `0x00B43D24` | 80 tract developer | | `0x00B43D90` | 63 ordinance |
| `0x00B43D28` | 78 landfill developer | | `0x00B43D94` | 64 plumbing |
| `0x00B43D2C` | 77 airport developer | | `0x00B43D98` | 65 police |
| `0x00B43D30` | 76 seaport developer | | `0x00B43D9C` | 66 pollution |
| `0x00B43D34` | 81 advisor system | | `0x00B43DA0` | 67 power |
| `0x00B43D38` | 82 tutorial system | | `0x00B43DA4` | 68 residential |
| `0x00B43D3C` | 72 disaster layer | | `0x00B43DA8` | 69 traffic |
| `0x00B43D40` | 74 park manager | | `0x00B43DAC` | 70 weather |
| `0x00B43D44` | 43 vehicle manager | | `0x00B43DB0` | 75 zone developer |
| `0x00B43D48` | 44 pedestrian manager | | `0x00B43DB4` | 73 civic building simulator |
| `0x00B43D4C` | 45 aircraft manager | | `0x00B43DB8` | 41 prop developer |
| `0x00B43D50` | 46 watercraft manager | | `0x00B43DBC` | 86 24-hour clock |
| `0x00B43D54` | 40 traffic network | | `0x00B43DC0` | 32 history warehouse |
| `0x00B43D58` | 42 network lot manager | | | |
| `0x00B43D5C` | 50 simulator | | | |
| `0x00B43D60` | 51 aura simulator | | | |

Not cached: the network manager (read live), the MySim agent simulator, the automata controller manager and script
system, the city situation manager. `0x00B43CFC` is cleared by `OnCityShutdown` but never set here.

### 1.3 The 3D view

| Function | Effect |
|---|---|
| `OnView3DCreate(view)` (`0x00602120`) | `0x00B43DC4` := view |
| `OnView3DInit()` (`0x00602590`) | false when the app window global `0x00B43CBC` is null; else `0x00B43DC4` := the app window's 3D view (`0x004E0960`), `0x00B43DC8` := its slot 3 (`AsIGZWin`), `0x00B43DCC` := slot 5 (the model maker), `0x00B43DD0` := slot 4 (the renderer), `0x00B43DD8` := the renderer's slot 8 (view utilities), `0x00B43DDC` := its slot 9 (the lighting manager); true. No null checks |
| `OnView3DShutdown()` (`0x00602130`) | those six globals := 0 |
| `ViewManager()` (`0x00602180`) | the view's slot 6 `GetViewManager` (no null check) |

### 1.4 `SL::AEMS` (`0x0046B4A0`)

Despite the name, a walker: starting at the given object, return its `+0x64` when non-zero, else repeat with its
parent `+0x0C`; 0 at the top. Used by the dispatch vehicle view, the airplane, the helicopter and the city
situation manager.

## 2. `cSLString`: a localized string *[compat]*

A string (vtable `0x00A85478`; text storage `+0x04`…`+0x0C`, initially empty) with a resource key `+0x14`
(type, group, instance).

- Constructors: `(key&, convert)` (`0x00603520`) and `(group, instance, convert)` (`0x00603580`, type
  `0x2026960B`, LTEXT); both call `Load(convert, 0)`. Destructor `0x00414250` frees the text.
- `GetKey(out&)` (`0x00602F70`) / `SetKey(key&)` (`0x00602F90`).
- **`Load(convert, mode)`** (`0x00603300`): neighbors.md §6 steps 1–3: the language offset (language manager
  slot 7) is added to the group; the private resource (manager slot 4, IID `0x089B7DC8`) is tried with it, then,
  when the offset is non-zero, with the plain group; the text is copied; with `convert` and a detokenizer the text
  is detokenized in place with the detokenizer's mode set to `mode` (restored after); then each UTF-8 U+2022 is
  replaced by U+200B. True. Without a resource manager or a resource: the text is emptied and the result is false.
- **`ConvertTokens(detokenizer, mode)`** (`0x00602FB0`): the given detokenizer, or the global one; with neither,
  nothing. Otherwise: save the detokenizer's mode (its slot 11), set `mode` (slot 10), detokenize the text in place
  (slot 9), restore. The result is always false *[quirk]*.

## 3. `cSLBuffer`: an image from a file or a resource *[compat]*

Fields: `+0x04` the buffer (held), `+0x08` key, `+0x14` a file name, `+0x28` the wanted colour type (1 = as
stored), `+0x2C` "private copy" flag.

- Constructors (vtable `0x00A856CC`, deleting destructor `0x0042FBB0`): `(private)` (`0x00602AA0`: key 0, colour 1,
  no load); `(key&, colour, private)` (`0x00602B00`); `(type, group, instance, colour, private)` (`0x00602B70`);
  `(fileName, colour, private)` (`0x00602C30`, the name copied from a `cIGZString`); the last three call `Load`.
  `SetResKey(key&)` (`0x006026F0`), `SetResKey(type, group, instance)` (`0x00602710`). Destructor `0x00602BE0`.
- **`Load`** (`0x006027C0`) returns whether a buffer is held. A held buffer is kept unless the private flag is set.
  - With a file name: the Gimex factory (`0x00B43CD0`) slot 4 `LoadFromFile(name, &buffer, colour)`; the result
    replaces the held buffer.
  - Otherwise the key, with type 0 read as `0x856DDBAC`. When the render properties' flag at
    (`*(0x00B43CC4 + 0x0C)` + `0x43C`) is set, group 1 is used if the resource manager has that key (slot 14
    `TestForKey`). Then `GetResource` (slot 3) or, with the private flag, `GetPrivateResource` (slot 4), IID
    `0x06F1568E`, with the colour as the fourth argument; on success the resource's buffer (its slot 3) is held.
  - Conversion, when a buffer is held and the wanted colour is not 1: nothing when the buffer already has the
    wanted type, or when type 3 is wanted and the buffer is 16 bits. Otherwise a new buffer (class `0xC470D325`) is
    initialised at the same size in the wanted type; the source's transparent colour, if any, is removed for the
    copy; the whole image is copied (lock `0x8010`, `Blt`); the transparent colour is put back **on the source**,
    the new buffer replaces it and the source is released, so the converted image has no transparent colour
    *[quirk]*. A failed creation, `Init` or lock drops the held buffer (the result is then false) *[quirk]*.

## 4. `cSLCursor`: a cursor resource *[compat]*

Fields: `+0x04` the `cRZCursor` (held), `+0x08` key, `+0x14` a file name. Constructors `(key&)` (`0x00602EB0`)
and `(group, instance)` (`0x00602F10`, type `0xAA5C3144`) call `Load`; destructor `0x00602CC0`, deleting
destructor `0x00602E60` (vtable `0x00AA6E54`).

**`Load`** (`0x00602D00`) returns whether a cursor is held; a held cursor is kept.

- With a file name: a new `cRZCursor(fileName)` (`0x00892911`), held.
- Otherwise (type 0 read as `0xAA5C3144`): the resource manager's segment holding the key (slot 32); if found, the
  record is written to a temporary file (`MakeTempPathName` with suffix `.cur`, then `SC4ReadBinaryRecord`), a
  cursor is made from that file, and the file is deleted; the segment is released.
- A failed allocation of the cursor object is not checked *[quirk: a null dereference]*.

## 5. `cSLWinLocationSaver`: remembered window positions *[compat]*

The service behind app slot 43 (`0x00B43CE0`). A map from window ID to (x, y) (`+0x10`), the main window (`+0x0C`,
held) and an "initialised" byte (`+0x08`). The preferences hold 200 slots of (ID, x, y) at `+0x4F0`.

| Slot | Address | Behaviour |
|---|---|---|
| 0 `QueryInterface` | `0x00603650` | IIDs 1, `0xAA601692`, `0xAA6016B1` |
| 2 `Release` | `0x00603630` | a count of 1 or less destroys (slot 11) and returns 0 |
| 3 `Init` | `0x00603810` | once: hold the app's main window (app slot 26); copy the preference slots into the map, stopping at the first ID 0 (at most 200) |
| 4 `Shutdown` | `0x00603910` | when initialised: release the main window; when the preferences global (`0x00B43CD8`) is set, write the map, in ID order, into the 200 preference slots, the rest zeroed (entries beyond 200 are dropped); not initialised any more |
| 5 `SaveWindowLocation(id, x, y)` | `0x00603D70` | ID 0 → false; else map[id] := (x, y) |
| 6 `SaveWindowLocation(window)` | `0x00603680` | null → false; else slot 5 with the window's ID (window slot 63) and its left and top (slots 43, 44) |
| 7 `GetWindowRestoreLocation(id, &x, &y)` | `0x00603A20` | ID 0 or absent → false |
| 8 `GetWindowRestoreLocation(window, &x, &y)` | `0x006036D0` | null → false; else slot 7 with the window's ID |
| 9 `RestoreWindowLocation(id)` | `0x00603890` | the main window's descendant with that ID (slot 35), then slot 10 with it; false when none |
| 10 `RestoreWindowLocation(window, clamp)` | `0x00603710` | null or no saved position → false. Without `clamp`: move the window there (slot 56). With `clamp`: when the window has a parent, the position is used only if x ≤ parentW − 8, y ≤ parentH − 8, x + w > 7 and y + h > 7, else the window goes to (8, 8); without a parent nothing moves. True |
| 11 | `0x00603CD0` | deleting destructor (body `0x006039C0`; map nodes freed by `0x006038D0`) |

## 6. Token record helpers *[compat]*

- `cSLTokenSerialRecordIO::Read(record, &value)` (`0x00980DC7`) reads one token with separator `,` and parses it
  with `%d` (`0x00980DD5`, which takes the separator); a missing token keeps the read's false, a token that does not
  parse gives false and value 0.
- `cSLTokenFieldBase` constructor (`0x0097F991`, vtable `0x00AD9B70`): copies the 8-byte field descriptor to `+4`
  and makes the name string `+0x0C`. `cSLTokenField<u32>` constructor (`0x0097FB89`, vtable `0x00AD9C78`): the base,
  then a value array of (descriptor `+6`, u16) × 4 bytes. The token record format is sc-resources.md §5–§6.

## 7. Function inventory

`SL`: `0x0046B4A0`, `0x00601C00`, `0x00601C10`, `0x00601C20`, `0x00601FF0`, `0x00602120`, `0x00602130`,
`0x00602160`, `0x00602180`, `0x00602210`, `0x00602230`, `0x00602270`, `0x00602290`, `0x00602590`, `0x00602680`.
`cSLBuffer`: `0x006026F0`, `0x00602710`, `0x006027C0`, `0x00602AA0`, `0x00602B00`, `0x00602B70`, `0x00602BE0`,
`0x00602C30`. `cSLWinLocationSaver`: `0x00603630`, `0x00603650`, `0x00603680`, `0x006036D0`, `0x00603710`,
`0x00603810`, `0x00603890`, `0x006038D0`, `0x00603910`, `0x006039C0`, `0x00603A20`, `0x00603CD0`, `0x00603D70`.
`cSLString`: `0x00414250`, `0x00602F70`, `0x00602F90`, `0x00602FB0`, `0x00603300`, `0x00603520`, `0x00603580`.
`cSLCursor`: `0x00602CC0`, `0x00602D00`, `0x00602E60`, `0x00602EB0`, `0x00602F10`. `cSLTokenSerialRecordIO`:
`0x00980DC7`, `0x00980DD5`. `cSLTokenField`: `0x0097FB89`. `cSLTokenFieldBase`: `0x0097F991`.
