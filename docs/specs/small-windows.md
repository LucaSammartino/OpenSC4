# Small game windows: credits, clock, splash screen, height-field loader and others (behaviour spec)

Scope: eight small owners as tools/coverage.py assigns them, each a section here: `cSC4CreditsWinProc` (§1, 6
functions, 955 bytes), `cSC4TimeDisplayMiniWindowWinProc` (§2, 7, 1,275), `cSC4WinSplashScreen` (§3, 13, 1,288),
`cSC4WinLoadHeightField` (§4, 10, 1,365), `cSC4MissingPluginPacksDialog` (§5, 1, 435), `cSC4WinGenTransparent`
(§6, 2, 266), `cSC4PanelUIWinProc` (§7, 3, 257), `cSC4WinGameDelay` (§8, 3, 161) and `cSC4WinTertiaryMenuFrame`
(§9, 1, 92).

Related: ui-windows.md (`SC4CreateWindowFromScript`, the class inventory), modal-dialogs.md (`DoModalWin`, the end
message `0xF`), widget-toolkit.md (`cGZWin` slots and flags), game-preferences.md (opens the credits),
intro-video.md (the screens before the splash), application.md §11.15 (the height-field picker's caller),
missing-plugin-reporter.md (the caller of §5), view3d-window.md (the panel commands of §7).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function of the owners was read in the decompile; the mangled bodies (the credits'
`InitDialog`, the clock's update and opener, the splash's `DoMessage`, the height-field `Init`) were checked on the
instructions. Written 2026-10-24 (re-gzsmallwin7).

`P` is the preferences object (`0x00B43CD8`); `L` the window-location store (`0x00B43CE0`: slot 10 `(window, 1)`
restores a saved position, slot 6 `(window)` saves it, ui-helpers.md); "end with v" means the window manager's
`GZWinMsgPost(0xF, window, v)` (cIGZWin slot 6 gives the manager, its slot 9 posts).

## 1. Credits (`cSC4CreditsWinProc`) *[compat]*

A 16-byte proc (vtable `0x00AB4AFC`: `+4` reference count, `+8` the window, `+0xC` an HTML control).

- **`ShowDialog(id)`** (`0x0076A2C0`): looks `id` up in the one-entry table `0x00AB4AEC` `{id 0, script instance
  0xCA551016, window ID 0x0A592004}` (other IDs → false); creates the window from `(0, 0x96A006B0, 0xCA551016)` with
  no parent; `InitDialog`; the proc becomes the window's proc. True.
- **`InitDialog`** (`0x0076A120`): the placeholder `T` = child `0x0A5D0E13` (slot 34) and its parent `Q` (slot 11).
  When both exist and no control is held: the HTML control is created (GZCOM `CreateInstance(0xAA12E5F5,
  0x4A11FD4A)`); `T`'s caption is set to the localized credits text (`cSLString(0x8A4924F3, 0x4A5D648F)`, slot 74),
  read back (slot 73); the control gets the seven font sizes 8, 10, 12, 16, 19, 24, 48 (`0x00AB4AD0`, its slot 33
  *[inferred: HTML sizes 1–7]*), `T`'s area (window slot 54), the fill colour RGB (200, 220, 220) (window slot 107),
  flags `0x40` and `4` on (slot 28) and `8` off (slot 26); it is added to `Q` (slot 14), given the text as its
  document with an empty base (slot 22), and `T` is deleted from `Q` (slot 16).
- **`DoModalWin`** (`0x00769F70`): with the game paused (`cSC4AutoGamePause`), the window's saved position restored
  (`L`), run modally (window manager slot 41), position saved. True.
- **`DoWinProcMessage(win, msg)`** (`0x0076A0C0`): only for its own visible window; a command (type 3) returns true and
  ends the dialog when it is `0x287259F6` from button `0xEA59207D`; other types false.
- **`EndDialog`** (`0x00769FE0`): a modal window (flag `0x1000`) is ended with 1; the window manager destroys the
  window (slot 23); the window and the control are released; the self reference dropped.
- **`Release`** (`0x0076A080`): the last reference releases the control and the window and frees the object.

## 2. The clock (`cSC4TimeDisplayMiniWindowWinProc`) *[compat]*

A small floating window that shows the real (wall-clock) time. Proc: 0x38 bytes, vtable `0x00AB54D8` (3
`DoWinProcMessage`, 5 destructor, 7 `Shutdown`); `+0xC` the window, `+0x10` its text child, `+0x14` the language
manager, `+0x18` a timer (unit argument 5), `+0x30` the last time shown.

- **Opening** (`DoModelessWin`, `0x0078C590`, static; called from two application commands): with the window
  `0x00B43CC0`: when its child `0xCA5E6261` exists, it is shown (slot 69), raised (flag `0x800` on, slot 23, flag
  off) and the result is false. Otherwise the window is created from `(0, 0x96A006B0, 0xAA5E60D1)` as child
  `0xCA5E6261`; a new proc holds it, its text child `0x4A5E623E` (as `0x212CDC1F`, slot 37) and the language manager
  (service `0x441E5070`, IID `0x0054B7D5`); the saved position is restored (`L`), the window raised, the proc
  attached, the time shown. True.
- **`UpdateTimeDisplay`** (`0x0078C3F0`): `now` = the current time in seconds; when it differs from `+0x30`: stored;
  local = now − `0x00B18BA8` (the time-zone offset), plus 3600 when daylight saving is on (`cRZTime::IsDST`); the
  language manager's formatter (its slot 38 `(0)`) writes `hour = (local mod 86400) / 3600` and
  `minute = (local mod 3600) / 60` with seconds −1 (none) into a string (formatter slot 9); the text child gets it
  (its slot 4).
- **`DoWinProcMessage`** (`0x0078C510`): the close button (`0x287259F6`, `0x42B7C356`) → `EndDialog`, true;
  `0xA2BF8ACD` → when the timer shows more than 1 unit, restart it and update; then the window is raised (slot 23);
  false; `0xA2BF8AD1` → `Shutdown`, false.
- **`EndDialog`** (`0x0078C2B0`): a modal window is ended with 1, otherwise its parent deletes it (slot 16); then
  `Shutdown`.
- **`Shutdown`** (`0x0078C230`), once: the position is saved (`L`), `P+0x4EE` := 0 (the clock no longer shown), the
  window and text released, the proc detached (generic slot 7, null).
- Constructor `0x0078C340`, destructor `0x0078C3A0`.

## 3. The splash screen (`cSC4WinSplashScreen`) *[compat]*

The full-screen title window after the intro movies (intro-video.md). `cGZWin` with a message target at `+0xD8`;
`+0xDC`…`+0xE4` counted pointers; a callback timer at `+0xE8`; `+0x110` "timer started"; `+0x114` a paint counter.
The constructor (`0x007BE4E0`) clears flag `0x10000`.

- **`QueryInterface`** (`0x007BE180`, thunk `0x007BE540`): 1 and `0x452294AA` → `+0xD8`; `0xAA38326E` → the window;
  else `cGZWin`'s.
- **`Init`** (`0x007BE5D0`), once: `cGZWin::Init`; the area becomes the main window's size (slot 55 with
  `(0, 0, W, H)`); `SetupScriptWindow`; `+0xE0` := the surface service `0x00B43C9C` (counted) and `+0xE4` := its
  interface `0xC47B747C`; the timer's callback is `TransitionTimerCallback(this)`.
- **`SetupScriptWindow`** (`0x007BE210`): the child from `(0, 0x96A006B0, 0x8AA9AA14)` with ID `0xAAA9C9D9`, centred
  (`max((W − w)/2, 0)`, `max((H − h)/2, 0)`), raised, flag `0x800` off. True when made.
- **`GZPaint`** (`0x007BE3A0`): the first paint starts the timer for 3000 ms. Each paint clears the screen through
  the graphics driver (`RZ::GS()` as `0xC47B747C`, then its device: slots 12 `(0, 0, 0, 0)`, 15 `(1)`, 11
  `(0x4000)`); `+0x114` counts two paints (0 → 2, minus 1 each time) and invalidates (slot 91) while it is not 0.
- **`TransitionTimerCallback`** (`0x007BE480`) posts `0x09DC338F` to `+0xD8`. **`DoMessage`** (`0x007BE370`) for
  `0x09DC338F`: the application (`0x00B43CBC`) `DestroyCurrentScreen`, then `CreateRegionScreen`; true; other
  messages false.
- **`GZOnMouseDownL`** (`0x007BE6A0`): the timer stops and the callback runs at once (a click skips the wait).
  **`GZOnKeyDown`** (`0x007BE1E0`) only forwards the key to the notification target (`SendMsg(target, 5, vk,
  modifiers, 0)`); it does not skip.
- **`Shutdown`** (`0x007BE2D0`), once: stops the timer when running, deletes the children (slot 18), releases the
  three pointers, `cGZWin::Shutdown`. Destructors `0x007BE570`, `0x007BE6D0`.

## 4. The height-field loader (`cSC4WinLoadHeightField`) *[compat]*

The terrain import picker (application.md §11.15 runs it modally). `cGZWin`; `+0xD8` the file browser; `+0xDC` the
current directory (string); window ID `0xEA9660F3` (constructor `0x0079D810`).

- **`Init`** (`0x0079D8D0`), once: area (20, 20, 424, 288); private buffer on (slot 100); fill colour RGB (25, 3,
  220) (slot 107). The browser is created (GZCOM `CreateInstance(0x1AA52EA4, 0x3AA52E64)`, false on failure), gets
  ID `0x9AEDEF7C`, area (8, 8, W − 8, H − 8), its slot 73 with `(4, 4, 4, 4)`, the filters `*.bmp`, `*.png`,
  `*.gif`, `*.jpg`, `*.jpeg`, `*.tga` (slot 21 each), the title `cSLString(0x6A231EAA, 0x4A9F1428)` (slot 22), slot
  26 `(0)` and the directory `+0xDC` (slot 15); it is added as a child.
- **`DoMessage`** (`0x0079D740`): command `0x287259F6` from `0xBAA55D58` (OK) while the browser's selection (slot 18)
  is a file (type 2) → end with 1; from `0xBAA55D59` (Cancel) → end with 0. Message `0x7ABE7204` (the browser's
  choose) → end with 1 when the selection is a file; true either way. Everything else → `cGZWin::DoMessage`.
- **`GZOnKeyDown`** (`0x0079D700`): Escape ends with 0; other keys go to the base.
- **`GZPaint`** (`0x0079D6B0`): fills the draw area with the fill colour (buffer slot 20 under lock `0x8001`).
- **`GetSelection`** (`0x0079D650`): the browser's slot 18, or 0 without a browser. **`GetInitialDirectory`**
  (`0x0079DB60`) refreshes `+0xDC` from the browser (slot 16) and copies it out; **`SetInitialDirectory`**
  (`0x0079DBB0`) copies a string into `+0xDC` (before `Init`, which hands it to the browser). **`Shutdown`**
  (`0x0079D670`) saves the directory the same way and releases the browser. Destructor `0x0079D880`.

## 5. Missing plugin packs (`cSC4MissingPluginPacksDialog::DoDialog`, `0x0048C510`) *[compat]*

The warning shown after a city loads with content from missing packs (missing-plugin-reporter.md gives the caller).
With the window `0x00B43CC0`: the dialog from `(0, 0x96A006B0, 0xEA89B6C3)`, ID `0x2A5CFB2C`; window slot 116
(`0x1D0`); its proc is cleared (generic slot 7, null). The list `0x6A89FB3A` (as list box `0x4132242B`, slot 37)
gets one row per missing pack number `n` in the reporter's set: `"%s %d"` of the localized label
`cSLString(0x6A231EAA, 0x6A89C0D9)` and `n` (list slot 7 with index −2, ID 0), in the set's order. Then it runs
modally (registered with `L`), the window is destroyed (window manager slot 23) and the result is whether the modal
result was `0xAA89FAF5` (its only button, modal-dialogs.md). No window → false.

## 6. The transparent generic window (`cSC4WinGenTransparent`) *[compat]*

A `cGZWinGen` (class `0x89E1567C`, scripted-window-director.md) that only takes the mouse where it has children.
`IsPointInWindowWindowCoordinates(x, y)` (`0x0079C5C0`): false outside its own size; otherwise the point is
converted (slot 60) and the answer is true only when some child is visible, does not have flag `0x200000` and contains
it (child slot 62); the child list is read-locked meanwhile. Destructor `0x0079C590`.

## 7. The panel buttons (`cSC4PanelUIWinProc`) *[compat]*

An 8-byte proc (constructor `0x00774550`; destructor `0x00774450`). **`DoWinProcMessage`** (`0x00774470`): while
the 3D view exists (`0x00B43DC4`), a command `0x287259F6` from one of five buttons runs a game command through the
command server (`0x00B43CB0`, slot 20 `(command, 0, 0)`) and returns true; anything else false.

| Button | Command (view3d-window.md) |
|---|---|
| `0x26C10A3D` | `0x6A935E36` `SaveCity` |
| `0xC98F4F48` | `0x6A935E39` `QuitCity` |
| `0x26C10A3E` | `0x6A935E3C` `QuitGame` |
| `0xCA1DA670` | `0x6A935E45` `OpenPhotoAlbumDialog` |
| `0x8A1DA655` | `0x6A935E4B` `OpenSnapshotDialog` |

## 8. The game delay window (`cSC4WinGameDelay`) *[compat]*

The invisible modal window of `SC4GameDelay` (modal-dialogs.md). Fields: `+0xD8` mode (0 = timed), `+0xE0` timer,
`+0xF8` duration, `+0xFC` frame count.
- **`Init`** (`0x005F8660`): `cGZWin::Init`, start the timer, count := count − 1, and end with 0 when it reaches 0.
- **`GZPaint`** (`0x005F8690`): timed: end with 0 once the timer reaches the duration; otherwise count := count − 1
  and end with 0 at 0. True. Destructor `0x005F9110`.

So a counted delay lasts that many paints (the first taken by `Init`), a timed one at least the duration.

## 9. The tertiary menu frame (`cSC4WinTertiaryMenuFrame`) *[compat]*

Only the constructor (`0x0079AFF0`) is in this owner: an interface at `+0` (vtable `0x00AB6D04`), the `cGZWin` at
`+4` (vtable `0x00AB6AA8`), its fields `+0xDC`, `+0xE0`, `+0x118`, `+0x11C`, `+0x120`, `+0x124` zero, and a callback
timer at `+0x128`. The 3D view uses it for the tertiary menus' background (view3d-window.md).

## 10. Quirks

- The clock's opener returns false when it only re-shows an existing clock.
- The splash ignores keys (only a click skips it) and clears the screen on its first two paints.
- The height-field OK button does nothing while a folder is selected.
- The credits dialog builds its HTML control only once per window; reopening re-creates the window.

## 11. Function inventory

- §1: `0x00769F70`, `0x00769FE0`, `0x0076A080`, `0x0076A0C0`, `0x0076A120`, `0x0076A2C0`.
- §2: `0x0078C230`, `0x0078C2B0`, `0x0078C340`, `0x0078C3A0`, `0x0078C3F0`, `0x0078C510`, `0x0078C590`.
- §3: `0x007BE180`, `0x007BE1E0`, `0x007BE210`, `0x007BE2D0`, `0x007BE370`, `0x007BE3A0`, `0x007BE480`,
  `0x007BE4E0`, `0x007BE540`, `0x007BE570`, `0x007BE5D0`, `0x007BE6A0`, `0x007BE6D0`.
- §4: `0x0079D650`, `0x0079D670`, `0x0079D6B0`, `0x0079D700`, `0x0079D740`, `0x0079D810`, `0x0079D880`,
  `0x0079D8D0`, `0x0079DB60`, `0x0079DBB0`.
- §5: `0x0048C510`. §6: `0x0079C590`, `0x0079C5C0`. §7: `0x00774450`, `0x00774470`, `0x00774550`.
  §8: `0x005F8660`, `0x005F8690`, `0x005F9110`. §9: `0x0079AFF0`.
