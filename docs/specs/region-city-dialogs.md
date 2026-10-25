# Spec: the city lifecycle dialogs and the obliterate-city effect (`cSC4NewRegionDialog`, `cSC4ImportCityDialog`, `cSC4EstablishCityWinProc`, `cSC4ObliterateCity`)

Status: **behaviour spec**, round 1 (re-region-city-dialogs, 2026-10-21), clean-room input. This document describes
*behaviour*, not code. It was written by the analysis team from the Windows 1.1.641 executable (the GOG build,
SHA-256 `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`, read in the instruction bytes and in a
Ghidra 12.1.4 project), and checked against the game's data (UI scripts, localized strings, the obliterate-city tuning
exemplar and three icon images, all read with the project's package reader). Implementers must work from this spec,
the public LGPL gzcom-dll SDK headers, the game's data files and general knowledge only. They must **not** consult
decompiled or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven (read from the bytes, not seen running);
*[quirk]* is surprising original behaviour that is kept on purpose; *[compat-limit]* depends on something
FreeCity4 cannot reproduce (the C library's random generator, heap addresses, the file system's listing order), so
only the rule is matched, not each value.

Related specs: [region-model.md](region-model.md) (the region these dialogs create, load and fill: §5 to §7 first;
§7.3 is `MoveCity`, which the import command ends in) and [region.md](region.md) (the independent analysis of the same
class), [region-screen.md](region-screen.md) (the window that opens the two region dialogs; its §5 and §6.5 give the
buttons and the import in outline, this spec adds what the dialogs hand back and what the screen does with it),
[regional-city.md](regional-city.md) (the city object: the new-city preferences block of §7.1; `regional-city-records.md`
holds the record's read and write), [nation.md](nation.md) (`AddNewRegion`, `SetActiveRegion`, the region list),
[application.md](application.md) (§11.6 `DoCityObliteration`, the work that follows the obliterate scene; §11.15
registers a modal dialog with the same global as §1.4 here), [modal-dialogs.md](modal-dialogs.md) (the modal loop,
the button command `0x287259F6`, which dialogs a harness can close), [widget-toolkit.md](widget-toolkit.md) (`cGZWin`:
flags, message types), [legacy-control.md](legacy-control.md) (the UI script format and the tree-view control the
import dialog uses), [ui-windows.md](ui-windows.md) (window creation from a script, §5.1), [city-tools-input.md](city-tools-input.md)
(the base class `cSC4ViewInputControl` of the obliterate tool), [demolition.md](demolition.md) (`DemolishRegion`, which
the tool calls), [zones.md](zones.md) (`PlaceZone`), [effects.md](effects.md) (the two particle effects),
[foundations.md](foundations.md) (§4 response curves; §11.6 the reconciliation messages), [flora.md](flora.md)
(§10.3 trees survive an obliteration), [city-situation.md](city-situation.md) (the situation indicator the tool hides),
[audio.md](audio.md) (§7.2 the two messages the tool posts), [view3d-window.md](view3d-window.md) (the panel commands).
The terrain edge reconciliation that runs after a city is established or loaded (`cSTEEdgeReconcile*`) has no spec yet.

## 0. Summary

Four small classes, three dialogs and one tool, all tied to the life of a city tile:

| Class | What the player does with it | Where it is opened |
|---|---|---|
| `cSC4NewRegionDialog` | names a new region and picks plains or water; the region folder is created | the region screen's "new region" button |
| `cSC4ImportCityDialog` | picks an existing city of the right size from any region or from the Downloads folder, to move onto an empty or occupied tile | the region screen's "import city" command |
| `cSC4EstablishCityWinProc` | names the city and the mayor and picks a difficulty before the city leaves God mode | the 3D view, when the mayor panel is first switched on for an unestablished city |
| `cSC4ObliterateCity` | confirms, then watches the city being razed; the application then rebuilds the tile as a bare landscape | a 3D-view command, only for an established city |

The two region dialogs share a small base class, `cSC4DialogWinProcBase` (§1), which creates the window from a UI
script, runs it modally and hands back an integer.

Seven results that are easy to get wrong:

1. **Creating a region** is `AddNewRegion(name)`, then `Init`, `SetBaseTerrainType`, `Shutdown` on the new region.
   The shutdown is what writes `region.ini` (§2.5). The name must be unique by its **display name, ignoring case**;
   the folder name is made unique separately by region-model §6.4.
2. **Opening the import dialog initialises and shuts down every inactive region** to list its cities (§3.3). That
   loads their city files, moves overflow city files to `Regions\Downloads\` and rewrites each region's `region.ini`.
3. **Pressing "Import City" with nothing, or a region folder, selected closes the dialog as a cancel** (§3.4).
4. **The establish dialog only checks that both names contain a visible character.** It does not look for duplicates
   and does not restrict characters (§4.3). OK writes the names and the difficulty to the city **and** to the
   new-city preferences, saves the preferences and marks the city established (§4.5).
5. **The obliterate tool razes the city only for show.** The application discards the whole city and rebuilds it
   from its terrain once the scene ends (§5.1, application.md §11.6). The scene lasts a few tens of seconds and posts
   one message when it ends (§5.5.3, §5.6).
6. **Escape stops the scene at once, with whatever has been removed so far left in place.** No message is posted,
   so nothing is rebuilt (§5.7, §5.8).
7. **The import command ends in `MoveCity` of the active region** with the chosen city and the selected tile's top-left
   cell; the city that was on the tile is deleted with its file (§3.5).

## 1. The shared dialog base `cSC4DialogWinProcBase`

### 1.1 Identity and slots *[compat]*

A window procedure object (`cIGZWinProc`) that owns one modal dialog window. `cSC4NewRegionDialog` (object `0x1C` bytes)
and `cSC4ImportCityDialog` (`0x68` bytes) derive from it, as do the load-region, delete-region and delete-city dialogs
(whose own bodies are not described here). The base keeps the dialog **window** (set on attach, cleared on detach), the
instance number of the dialog's UI script (type 0, group `0x96A006B0`), the ID given to the script's root window, and the
ID of the control that takes the keyboard focus. Vtable slots (the derived classes override the starred ones):

| Slot | Method | Base behaviour |
|---|---|---|
| 0 to 2 | `QueryInterface`, `AddRef`, `Release` | shared bodies |
| 3 | `DoWinProcMessage(window, message)`* | §1.3 |
| 4 | `DoWinMsg` | shared "return false" stub |
| 5 | `Display(parentWindow)` | §1.4 |
| 6, 7 | `GZOnAttach`*, `GZOnDetach` | stubs; both dialogs override slot 6, the import dialog also slot 7 |
| 8 | `GZOnCommand(buttonId)`* | §1.5 |
| 9 | cancel | ends the dialog with 0 (the new-region dialog overrides it with -1) |
| 10 | `GZOnOK`* | ends the dialog with 1; both dialogs override it |
| 11 | deleting destructor | |

### 1.3 Window-procedure messages *[compat]*

`DoWinProcMessage` handles three kinds of message and returns false for everything else:

- **Attach `0xA2BF8AD5`**: store the window, then `GZOnAttach` (slot 6); returns true.
- **Detach `0xA2BF8AD6`**: clear the window, then `GZOnDetach` (slot 7); returns true.
- **Command (type 3)** whose data1 is `0x287259F6` (the button-pressed code of modal-dialogs.md §2.6): call slot 8
  with data2, the button's ID; returns false (the message is not consumed from the sender's point of view).
- **Set focus (type `0x11`)** on the dialog window: if the window is **not** yet marked modal (flag `0x1000`,
  widget-toolkit.md §3) and a focus control is set, the control with that ID is given the focus through the window
  manager; returns true. Once the modal loop has set flag `0x1000`, focus messages are ignored.

The new-region dialog adds one message of its own (§2.3).

### 1.4 `Display(parent)` *[compat]*

The sequence that every derived dialog goes through, and what it returns:

1. AddRef the procedure object.
2. Create the dialog window from the script `{type 0, group 0x96A006B0, instance of the dialog}` as a child of `parent`, with
   the root window ID (ui-windows.md §5.1). If the window cannot be created the result is **0**.
3. Ask the window for its window-procedure holder interface (`0x5386D516`). If it has none, the result is 0.
4. With the **busy cursor** (cursor ID 15) shown, attach this object as the window's procedure. That triggers the
   attach message of §1.3, so `GZOnAttach` (and with it, for the import dialog, the whole list building of §3.3)
   runs under the busy cursor.
5. Register the window with the dialog registry (global `0x00B43CE0`, slot `+0x28` with `(window, 1)`,
   application.md §11.15), run it with the window manager's modal loop (`DoModalWin`, modal-dialogs.md §2.2) and
   unregister it (slot `+0x18`). The value `DoModalWin` returns is the result.
6. Remove the window from its parent, release the window, and release the procedure object once (undoing step 1);
   the creator (`0x00777FF0`, `SC4DisplayImportCityDialog`) then drops its own reference, which destroys it.

The result is whatever the dialog gave to the "end modal" request, which every derived dialog sends as the pair
`(0xF, window, value)` to the window manager (the end message, `EndModal`, modal-dialogs.md §2.4). A window that could not be
created or has no procedure interface gives **0**, which is also a valid result for the new-region dialog (a region
index) *[quirk]* (§2.6).

### 1.5 The default command handler *[compat]*

Slot 8 in the base handles two button IDs and returns false for any other:

| Button ID | Meaning | Action |
|---|---|---|
| `0x8A5B9959` | the confirm button of the dialog family ("Create Region", "Import City", ...) | slot 10 (`GZOnOK`), returns true |
| `0xCA5B9966` | the "Cancel" button | slot 9 (cancel), returns true |

The new-region dialog overrides slot 8 to add its two toggle buttons (§2.4).

## 2. `cSC4NewRegionDialog`

### 2.1 Identity and resources *[compat]*

- Object size `0x1C`; vtable `0x00AB5340`. Created and run by `0x00777FF0`. Despite its name that function is **not
  a constructor**: it builds the object, calls `Display(parent)` (§1.4), releases it and returns `Display`'s result
  (the sibling for the import dialog is the free function `SC4DisplayImportCityDialog`, `0x00778D80`).
- Script: type 0, group `0x96A006B0`, instance **`0x8A5AB1CB`**, root window ID `0xEA5BA0D1` (area 330 x 168
  pixels; "defaultkeys" off, so neither Escape nor Enter produces a command). The default focus control
  is `0x2000`.
- Controls (checked in the script):

| ID | Kind | Role | State in the script |
|---|---|---|---|
| `0x2000` | single-line text edit | the region name | initial text "New Region" (localized, LTEXT group `0x0A554AE8`, instance `0xAA6CD8EA`) |
| `0x1000` | radio-style toggle button | **Plains** (terrain type 0) | on |
| `0x1001` | radio-style toggle button | **Water** (terrain type 1) | off |
| `0x8A5B9959` | button | **Create Region** (LTEXT group `0x6A231EAA`, instance `0x6A51203B`) | enabled |
| `0xCA5B9966` | button | **Cancel** | enabled |

  Labels with no ID: the header "Region Name", the group caption "Initial Terrain".
- The dialog vtable overrides `DoWinProcMessage` (`0x00778450`), `GZOnAttach` (`0x00778630`), slot 8 (`0x00777F40`),
  slot 9 (`0x00777CC0`, ends with **-1**) and `GZOnOK` (`0x00778470`). `0x00778390` (`EnableControls`) is the
  helper of §2.3.

### 2.2 Opening: `GZOnAttach` (`0x00778630`) *[compat]*

1. Take the name edit (`0x2000`). Its current text, the script's "New Region", is the **base name**.
2. Ask the nation (the application's `GetNation`, nation.md) whether a region **named like the candidate** exists
   (§2.4). While one does, the candidate becomes `base + " " + n` with `n` counting 1, 2, 3, ... (`"New Region 1"`,
   `"New Region 2"`, ...); the base is **not** extended, each candidate is built from the unchanged base. The first
   candidate is the base itself.
3. Put the candidate into the edit, set the edit's maximum length to 128 characters *[inferred]* (the call takes the
   value `0x80`; the script itself says `maxtext = -1`), select the whole text, and make the dialog window the
   notification target of the edit and of the two toggle buttons, so their messages reach the procedure.
4. Run `EnableControls` (§2.3).

### 2.3 Validity of the name: `EnableControls` (`0x00778390`) and the filter *[compat]*

`EnableControls` sets the **enabled** flag (flag `0x2`) of the Create button to `IsFileNameValid(text)` (the general
function `0x0091977C`, second argument 0). It runs once on attach and then on every message of type **`0x9B1552D8`**
that reaches the procedure (`DoWinProcMessage`, `0x00778450`, returns true for it; every other message goes to the
base, §1.3). The text edit sends that message when its text changes *[inferred]* (the attach step sets a flag on the
edit that is consistent with a "notify on change" option; CY3).

`IsFileNameValid(name)` accepts a name when **all** of the following hold:

| Rule | Detail |
|---|---|
| characters | none of `< > : \ / " \| * ?` (`IsFileNameCharAllowed`, `0x009180D4`, the same set that `SC4SanitizeFileName` removes, region-model §6.4). Bytes are tested one by one as signed bytes; any byte other than these nine passes, including controls, blanks and non-ASCII bytes |
| reserved names | the text up to the **first `.`** (the whole text when there is none), lower-cased, is none of `con prn aux clock$ nul com1`..`com9 lpt1`..`lpt9` (23 names). So `CON`, `con.txt` and `Com3` are refused, `Console` and ` con` are not |
| not blank | the text contains at least one character that is **not** `.` or a blank. `""`, `"."`, `".."` and `"   "` are refused |
| length | **a text longer than 256 bytes is accepted without any other check** *[quirk]* (the rules above are skipped; the length test returns "valid"). The edit's own limit keeps real input at 128 characters |

*[inferred]* The reserved-name rule belongs to a file-system test that the function normally makes ("FAT", "HPFS"
and "NTFS" are all accepted by it); with the second argument 0, as both callers pass, it assumes FAT and always applies
the rule. A non-zero second argument skips the reserved-name rule (CY4).

There is no check for leading or trailing blanks: `" Foo "` is valid, and the region's display name keeps the blanks
(the folder name is cleaned by `SC4SanitizeFileName`, region-model §6.4).

### 2.4 The toggles and the duplicate test *[compat]*

- **Slot 8** (`0x00777F40`): pressing `0x1000` (Plains) sets Plains on and Water off; pressing `0x1001` (Water) does
  the opposite; so exactly one is on, as for a radio group, and the script starts with Plains on. The Create button
  (`0x8A5B9959`) calls `GZOnOK`, Cancel (`0xCA5B9966`) the cancel slot, as in §1.5. Any other button ID returns false.
- **Duplicate test** (`0x00777E50`): the name is compared with `GetName()` of **every region of the nation, in the
  nation's list order**, with `cIGZString::IsEqual(name, caseSensitive = false)`: equal length and equal text after
  lower-casing both (ASCII lower-casing; the compare is on bytes). The directory name is **not** compared, and the
  active region is included. One match is enough.

### 2.5 OK: `GZOnOK` (`0x00778470`) *[compat]*

1. Read the text of the name edit. If `IsFileNameValid` refuses it, **do nothing** (the dialog stays open; the button
   is disabled in that state anyway, §2.3).
2. If a region with that display name exists (§2.4): show the generic notification dialog (`nSC4UI::GenericNotificationDialog`)
   with the title "Cannot create region" (LTEXT `0x6A231EAA / 0x4A78B7E8`) and the text "A region already exists with
   that name. Please choose a different name." (`0x6A231EAA / 0x8A78B80E`), and leave the dialog open. Like every
   generic notification it shows nothing when the application's popup gate is off (advisors.md §6.8.1,
   modal-dialogs.md §4), so in an unattended run the dialog then simply does not close *[inferred]* (CY12).
3. Otherwise, in this order:
   1. `index := nation.AddNewRegion(name)` (nation slot 13): the region object is built by `CreateRegion`
      (region-model §6.4: sanitised and de-duplicated folder name, the folder, a random built-in `config.bmp`) and
      appended at the **end** of the nation's list; `index` is its position (the old count).
   2. `region := nation.GetRegion(index)`; `region.Init()` (region-model §6.1). With no `region.ini` yet the name set
      by `CreateRegion` stands; the locations are read from the exported picture; no cities are found.
   3. `region.SetBaseTerrainType(t)` with `t := 1` when the Water button is on and `0` otherwise
      (region-model §8.1; `Plains` is 0, matching `Terrain type = 0` in the shipped `region.ini` files).
   4. `region.Shutdown()`: this **writes `region.ini`** (region-model §3.2) with the name as typed, the terrain type
      and the default water range 60 and 100. The region is left shut down.
   5. End the dialog with the result `index` (the pair `(0xF, window, index)`).

The water range cannot be chosen in the dialog; the new region always has the defaults 60 and 100.

### 2.6 Cancel and the result *[compat]*

Cancel ends the dialog with **-1** (`0x00777CC0`). The result is therefore `-1` (cancelled) or the new region's
index (`0` or more). *[quirk]* The failure result of `Display` (§1.4) is `0`, which a caller cannot tell from "region
0 was created".

### 2.7 What the caller does *[inferred]*

The region screen opens the dialog from its new-region button (command `0x2A5B0000`, region-screen.md §5). The
load-region button (`0x2A5B0001`) shares the code after the dialog, so the two behave alike:

1. A negative result (cancel) ends the command.
2. Otherwise the result is a region index: `nation.SetActiveRegion(index)` (nation.md §4 slot 9: the old active region
   is shut down and the new one initialised, so the dialog's own `Shutdown` of §2.5 is undone at once), the new region's
   name goes into the user preferences (region-screen.md §7), and the application is asked for the region view again
   (`RequestGoToRegionView(false)`, application slot 12).

So a created region becomes the active one and the screen shows it empty (its tiles come from the exported picture).
Whether `RequestGoToRegionView(false)` rebuilds a region screen that is already up is not decoded (CY8).

## 3. `cSC4ImportCityDialog`

### 3.1 Identity and resources *[compat]*

- Object size `0x68`; vtable `0x00AB5454`. Created and run by the free function `SC4DisplayImportCityDialog`
  (`0x00778D80`), which takes `(parentWindow, sizeX, sizeZ, &regionIndex, &x, &z, &path)`, returns a bool (the dialog's
  result was non-zero) and copies the chosen record out (§3.4).
- The constructor (`0x00778C10`) sets the script `{type 0, group 0x96A006B0, instance 0x8A5AB1CD}`, the root window
  ID `0x0A5BA192`, the default focus control `0x1000`, the two size arguments (`+0x1C`, `+0x20`), an empty record
  list, an empty selected-record slot and eight icon numbers set to "not loaded" (-1).
- Controls (checked in the script):

| ID | Kind | Role |
|---|---|---|
| `0x2000` / `0x2001` / `0x2002` | static header, hidden in the script | "Small City Import" / "Medium City Import" / "Large City Import" (LTEXT group `0x6A231EAA`, instances `0x6A7A268A`, `0xAA7A26BF`, `0xAA7A26CE`); exactly one is made visible |
| `0x1000` | tree view | the city list; no root row, node icons on, tree lines on, one row 16 pixels high, single selection |
| `0x8A5B9959` | button | "Import City" (LTEXT group `0x0A554AE8`, instance `0xEAA59DE0`) |
| `0xCA5B9966` | button | "Cancel" |

- Overrides: `GZOnAttach` (`0x00779060`), slot 7 `GZOnDetach` (`0x007781A0`), slot 8 (`0x00777C90`, the base command
  handler of §1.5 shared with the other dialogs), slot 9 cancel (`0x00777CF0`, ends with **0**), slot 10 `GZOnOK`
  (`0x00778CD0`). `DoWinProcMessage` is the base's (`0x00777BD0`).

### 3.2 The size filter and the header *[compat]*

The two size arguments are the city size in region cells, `1`, `2` or `4` (the region screen passes
`1 << sizeExp` for both, region-model §4.2). The dialog lists **only cities whose `GetCitySize()` equals exactly
`(sizeX, sizeZ)`**. The header control matching `sizeX` is made visible (flag `0x1`): 1 shows `0x2000`, 2 shows
`0x2001`, 4 shows `0x2002`; any other value shows no header.

### 3.3 Building the list: `GZOnAttach` (`0x00779060`) *[compat]*

On attach, in this order:

1. Take the tree view (`0x1000`), the nation, its active region and its region count. Make the header visible (§3.2).
2. **Load the icons** if their numbers are still -1: three images of type `0x856DDBAC`, group `0x46A006B0`, instances
   `0x1441624A`, `0x1441624B`, `0x1441624C` (16 x 16 PNG each in the shipped data), added to the tree's icon table.
   Icon A (`...4A`) is the region row's normal and expanded icon, B (`...4B`) its selected and expanded-selected icon;
   C (`...4C`) is the icon of all four states of a city row. (The numbers are only separate fields so a missing
   image falls back to the first.)
3. For each region of the nation in the nation's list order, index `i`:
   1. `region := nation.GetRegion(i)`. If `region` is **not the active region**, call `region.Init()` (region-model
      §6.1).
   2. `region.GetAllCities(list)` (region-model §5.4: the distinct cities of the cell map).
   3. Add the **region group** (§3.3.1) with the region's display name, the list and the index `i`.
   4. Release the list; if `region` is not the active region, call `region.Shutdown()` (region-model §6.2).
4. Then the **Downloads group**: its title is the LTEXT `0x6A231EAA / 0x2A8F9A7D` ("Downloads"; the literal text
   `Downloads` when the string is missing). The cities are read straight from the nation's download directory with
   `LoadCitiesFromDirectory` (region-model §5.1: every `*.sc4` file that reads as a regional city, **clearing the
   read-only attribute** of each file it accepts), and added as a group with region index **-1** and the "store the
   file path" flag set. Afterwards each of these cities gets `Shutdown` (slot 4 of the table in region-model §7), and the list is released.

#### 3.3.1 One group

For each group, a **parent row** is added at the top level (parent 0), at the **sorted** position (position code -4,
legacy-control.md §9.2), with the group title, no user data, and the four state icons A, B, A, B. Then, for every city
of the group's list **in list order** (position code -3, "last"), if `city.GetCitySize() == (sizeX, sizeZ)`:

- the row text is the city's name (`GetCityName`);
- the row's **user data** is a record `{ regionIndex, x, z, path }`, appended to the dialog's record list:
  `regionIndex` is the group's index (or -1), `(x, z)` is `city.GetPosition()`, and `path` is the city's file path
  **only for the Downloads group** (empty for a region); the four state icons are all C.

The order of the rows inside a region group is the order of `GetAllCities`, which depends on object addresses
(region-model §5.4, *[compat-limit]*); inside Downloads it is the file system's listing order. FreeCity4 *[choice]*:
order region cities by position (x, then z), Downloads cities by file name. The order of the groups is the tree
control's own sorted order (CY6).

The active region's list is its **live** cell map. The region screen inserts a placeholder record (not established,
named "New City", no file) only when a tile is played (region-screen.md §6.1), never for the import command itself;
whether such a record is still in the map when the dialog opens, and so is listed as a "New City" row, is not decoded
(CY2).

#### 3.3.2 Side effects *[compat]* *[inferred]* (both halves read from the bytes, not seen running)

Because a region is initialised only to be listed, opening the dialog:

- loads every inactive region's cities and **moves any overflow city file to `Regions\Downloads\`** (region-model
  §5.3, §7.5): for the shipped data, the second Timbuktu city on the same tile (region-model §12 test 2) leaves the
  Timbuktu folder the first time the dialog opens, or the first time Timbuktu is made active;
- ends with `Shutdown` on each inactive region, which **rewrites its `region.ini`** (region-model §3.2): a shipped
  `Name = #0x...` token becomes the language's literal text at that moment. This is the case RM1 asks about (CY1); it
  happens for every region except the active one. The same pair of calls is what `SetActiveRegion` does when the
  active region changes (nation.md §4 slot 9), so this dialog is a second, unannounced trigger of it;
- reads the Downloads folder and clears the read-only attribute of its city files.

### 3.4 The result: OK, Cancel, detach *[compat]*

- **OK** (`0x00778CD0`): take the tree's selected row; if there is one **and** it has user data, copy its record into
  the dialog's selected-record slot and end the dialog with **1**. Otherwise (no row selected, or a group row, which
  has no user data) end it with **0**: pressing "Import City" without choosing a city is a cancel *[quirk]*.
- **Cancel** ends with 0 (§3.1).
- **Detach** (`0x007781A0`): clears the tree and sets all eight icon numbers back to -1, so a later attach reloads
  them.
- Only the two buttons are handled; a double click on a row, Enter and Escape do nothing in this dialog.
- `SC4DisplayImportCityDialog` returns true when the modal result was non-zero and then copies out the selected
  record: the region index, `x`, `z`, and the path string (empty for a region city).

### 3.5 What the caller does *[inferred]*

The region screen's import command (`0x4A560002`, region-screen.md §5, §6.5) works on the **selected tile** (the tile
object holds its size code and its top-left region cell):

1. If the region already has a record at the tile (looked up without creating one, region-screen.md §6.1), it first
   asks "Importing a city will replace the current city at this location. Are you sure you want to continue?" (a
   generic confirmation; title "Small/Medium/Large City Import" by the tile's size code, button "Continue"; LTEXT group
   `0x0A554AE8`, instance `0x4AA59E23`, and group `0x6A231EAA`, instance `0xEA8DFC1B`). "No" ends the command, nothing
   changed. The record is not touched until step 3, so a Cancel in the dialog also leaves the tile alone.
2. It runs the dialog with `sizeX = sizeZ = 1 << sizeCode` of the tile. This is the only size check: nothing is
   compared again afterwards.
3. On a non-zero result it calls **`activeRegion.MoveCity(source, city, tileX, tileZ)`** (region-model.md §7.3, which
   is also where an occupant of the tile is deleted with its file) with `(tileX, tileZ)` the **tile's top-left region
   cell**:
   - **path empty (a city of a region):** `source` is region `regionIndex` of the nation, initialised first when it is
     not the active region; `city` is the one at `(x, z)` of that region (`GetCity`). A source equal to the active
     region counts as none, as in `MoveCity`. The source is shut down by `MoveCity` and not initialised again.
   - **path not empty (a city of the Downloads folder):** the file is read as a regional city (`SC4ReadRegionalCity`,
     region.md §8; `regional-city-records.md` for the record), given the path (regional-city slot 32) and passed with
     a null source; afterwards the object is shut down and released. The file itself is moved by `MoveCity`, so the
     record read here only carries the name, serial number and path.
4. Only when `MoveCity` returns true: the view position is saved to the preferences (region-screen.md §7) and the
   application is asked for the region view again (application slot 12 with 0). On false nothing more happens.

This is the public evidence for RM7 (`questions/region-model.md`): `MoveCity` is reached from the import command, with
the source region (or null for a Downloads city), the city, and the tile's top-left cell. It is the only call site
found. region-screen.md §6.5 and its QR2 say the record is "read and placed in the region": that is true for a Downloads
file only; a region city is moved by renaming its file, and the tile's previous city is deleted by `MoveCity`.

## 4. `cSC4EstablishCityWinProc`

### 4.1 Identity and resources *[compat]*

- Object size `0x20`; vtable `0x00AB4B34` (only slots 0 to 5 are real: `DoWinProcMessage` `0x0076A9D0`, `DoWinMsg`
  stub, deleting destructor `0x0076A4F0`). Unlike the dialogs of §1 it is **not** a `cSC4DialogWinProcBase`: the
  caller builds the window, attaches the object and runs the modal loop itself
  (`cSC4View3DWin::DoEstablishCityDialog`, `0x007EB320`, the only caller of the constructor `0x0076A840`).
- Fields: `+0x18` the **difficulty** chosen so far (0, 1, 2), `+0x1C` the window-procedure holder of the dialog window
  (held), `+0x08` an unused string.
- Script: type 0, group `0x96A006B0`, instance **`0x2A41436B`**, root window ID `0x6A414973` (area 434 x 234 pixels,
  "defaultkeys" **on**). Controls:

| ID | Kind | Role |
|---|---|---|
| `0xAA4149BF` | text edit | **city name**, `maxtext` 128, initial text "New City" (LTEXT `0x6A231EAA / 0xAA738C4E`) |
| `0xAA4149D2` | text edit | **mayor name**, `maxtext` 128, initial text "Defacto" (`0x6A231EAA / 0xEA8C765A`) |
| `0x2A4144BE` | button | **Establish City** |
| `0x8A414491` | button | **Cancel** |
| `0x2AB6C373` / `0x2AB6C374` / `0x2AB6C375` | radio-style buttons with tool tips | difficulty **Easy** / **Medium** / **Hard** (0, 1, 2) |

  Static texts: "City Name", "Mayor Name", "Difficulty Level", and the warning "Warning! You will not be able to
  use the God terraforming tools or change your city's difficulty level once a city is established. ..."
  (`0x6A231EAA / 0xEC047F4B`).

### 4.2 Opening: `DoEstablishCityDialog` and `InitDialog` *[compat]*

`DoEstablishCityDialog` (3D view, `0x007EB320`) prepares the dialog; the constructor then runs `InitDialog`
(`0x0076A7E0`):

1. Create the window from the script as a child of the 3D view; centre it in the view.
2. **Difficulty**: set the city's difficulty level to the value stored in the application preferences
   (`cISC4App::GetPreferences`, offset `+0x46C`: the default for new cities, regional-city.md §2, the difficulty field), so the radio
   group shows the preference. `InitDialog` reads the city's difficulty back into `+0x18`, makes the dialog window the notification
   target of all its children, and presses the radio button `0x2AB6C373 + difficulty` (releasing the other two).
3. **City name edit**: switch on the edit's change notification (flag `0x800`) *[inferred]*, attach a key filter
   (below), set the maximum length 128, select all the text and give it the keyboard focus. The text stays the
   script's "New City".
4. **Mayor name edit**: when the preferences hold a **non-empty mayor name** (preferences `+0x26C`, a 512-byte string),
   it replaces the edit's text; otherwise the script's "Defacto" stays. Change notification, the key filter and the
   maximum length are set as for the city name.
5. `InitDialog` runs the name validation (§4.3) once.

The **key filter** is a small object attached to both edits (`0x0076A3C0`; its message hook is `0x0076A390`). It
forwards every character message for **Tab** (`\t`) or **Enter** (`\r`) typed in an edit to the dialog window so the
procedure of §4.4 sees it, and lets all other characters go to the edit.

### 4.3 Validation: `ValidateCityAndMayorNames` (`0x0076A530`) *[compat]*

Each name is read from its edit (an 8-bit string). A name **counts** when it has at least one byte **above `0x20`**
(so a name of only blanks and control characters does not; bytes `0x80` and above count). The **Establish City**
button (`0x2A4144BE`) is enabled (flag `0x2`) exactly when **both** names count; otherwise it is disabled. The function
returns that same truth value; when one of the edits or the button cannot be found, it changes nothing and returns
true. It runs on `InitDialog` and on every message of type `0x9B1552D8` (the edit's text-changed notice) that reaches
the window.

There are **no other rules**: no length minimum, no forbidden characters, no check for an existing city or file with
the same name (the file is named at the next save, which adds ` (n)` to keep it unique, region-model §7.4), and no
trimming at this stage. The text edits are limited to 128 characters and the stored copies are cut to 511 bytes
(§4.5).

### 4.4 Messages and keys *[compat]*

`DoWinProcMessage` (`0x0076A9D0`) acts only on messages for the dialog's own window; others return false. The
cases:

| Message | Action |
|---|---|
| text changed `0x9B1552D8` | validate (§4.3), return true |
| command (type 3) | `DoCommand`, below |
| character (type 4) | `GZOnCharacter`, below |
| set focus (type `0x11`) | give the focus to the **city name** edit; return true |
| attach / detach `0xA2BF8AD5` / `0xA2BF8AD6` | return true, nothing else |

**`DoCommand`** (`0x0076A700`), data1 must be `0x287259F6`, data2 is the button:

| Button | Action |
|---|---|
| `0x2AB6C373`, `0x2AB6C374`, `0x2AB6C375` | `difficulty := 0, 1, 2`; press that radio button, release the other two |
| `0x2A4144BE` (Establish City) or `0x42B7C356` | validate; if both names count, `EndDialog(OK)`; else nothing |
| `0x8A414491` (Cancel) or `0x42B7C353` | `EndDialog(cancel)` |

`0x42B7C353` and `0x42B7C356` are the generic close and accept commands the window layer produces for a window with
"defaultkeys" on; which keys produce them is not decoded (CY5). Other buttons return false.

**`GZOnCharacter`** (`0x0076A880`): **Tab** moves the focus between the two edits (city name to mayor name, any
other focus to the city name); the newly focused edit has all its text selected and the other has its selection
cleared; returns true. **Enter** validates and, when both names count, ends the dialog with OK; returns true whether
or not it ended. Other characters return false.

**`EndDialog(ok)`** (`0x0076A430`): detach the procedure from the window; if the window is modal (flag `0x1000`),
end the modal loop with the value **0 for OK and 1 for cancel** (note the inversion), else remove the window from its
parent; then release the window and the object. The caller treats the result 0 as "established".

### 4.5 OK: what the caller does with the names *[compat]*

`DoEstablishCityDialog` runs the modal loop and, when it ended with OK (result 0), applies the choice in this order:

1. `d := ` the procedure's difficulty; `city.SetDifficultyLevel(d)`; the **preference** difficulty `+0x46C := d`.
2. City name: read the edit, cut it to **511 bytes** (`SC4LimitStringLength`), copy it into the preferences string at
   `+0x6C` (512 bytes), `city.SetCityName(text)` and `city.SetCityNameChanged(true)`.
3. Mayor name: read the edit, cut it to 511 bytes, copy it into the preferences string at `+0x26C`,
   `city.SetMayorName(text)`.
4. `app.SavePreferences()`.
5. `city.SetEstablished(true)`.

The three offsets (`+0x6C`, `+0x26C`, `+0x46C`) are 0, `0x200` and `0x400` bytes into a block that starts at `+0x6C` of
the preferences object: two 512-byte strings and a dword. That is the layout of the "new-city preferences"
structure that `cISC4RegionalCity::SetupPreferences` and `UpdateCityCache(prefs)` take (regional-city.md §7.1:
city name at the start, mayor name at `+0x200`, difficulty at `+0x400`), so *[inferred]* the block is the structure
those two functions are given. The dialog itself never calls them (CY7). The text copied into the preferences is not
trimmed, so a name with leading blanks keeps them.

On **cancel** (result 1) none of this runs; the city stays unestablished and the toggle that started the dialog is
reset by the caller (the mayor panel's switch, command `0x6A935CA4`, which only runs the dialog while the city is
unestablished and the switch is being turned on). On OK, the 3D view later receives `CityEstablished` `0x26D31EC4`
(view3d-window.md), which is outside this spec.

## 5. `cSC4ObliterateCity`

The "Obliterate City" action (a 3D-view command, available only for an established city): it removes every building and network from an **established**
city and returns the tile to its bare landscape, as God mode would start it. The class is the **view input control**
that plays the scene; the actual rebuild is the application's (application.md §11.6).

### 5.1 Trigger and confirmation *[compat]*

- The 3D view command **`0x2A94A5B6`** (`cSC4View3DWin::ProcessCommand`) acts only when the city's
  `GetEstablished()` is true. It then runs the scripted modal dialog (`nSC4UI::DoScriptedModalDialog`, script
  instance `0x2A41436C`, root window `0x27DF05BE`, title "Obliterate City", text "Warning! Removes all traces of
  civilization from your city and returns the landscape to its pristine condition. This is an irreversible
  operation. The current city file will be saved over. Do you wish to continue?"). The buttons are "Obliterate City"
  (`0x6A41BBB5`, the confirmation) and "Cancel" (`0x6A414457`).
- The scripted dialog returns 0 without asking when the popup gate is off (advisors.md §6.8.1), so an unattended run
  that suppresses popups can never reach the tool *[compat]*.
- Any result other than `0x6A41BBB5` ends the command. On the confirmation: disable the 3D view's control `0x4988BC6A` (the toggle button of command `0x6A935CA8`,
  ToggleMySimPanel in view3d-window.md; the pairing is read from `DoPanelCommandGUI`, *[inferred]*), hide panel 4 of the
  3D view when it is the panel shown (which panel that is, is not decoded), build a
  `cSC4ObliterateCity` (`0xA8` bytes), install it as the current view input control with
  `SetCurrentViewInputControl(tool, 2)` (city-tools-input.md §2.3), and release the creator's reference.

### 5.2 Identity *[compat]*

- Object size `0xA8`, vtable `0x00A8F6D8` (the `cSC4ViewInputControl` layout of city-tools-input.md §2.3), control ID
  **`0xCA58C969`**, cursor ID **15** (the busy cursor). `QueryInterface` (`0x00661D70`) answers the message-target IID
  `0x452294AA` with the sub-object at `+0x28`, and everything else through the base.
- The sub-object at **`+0x28`** (vtable `0x00A8F6C4`) is a message target whose `DoMessage` is
  `DoAnimationTick` (`0x0049D820`). It is subscribed to the animation tick manager and to one message (§5.6).
- Overridden slots: `Init` (`0x0049D390`), `Shutdown` (`0x0049C8B0`), `OnKeyDown` (`0x0049C790`), the three mouse
  slots `OnMouseDownL`, `OnMouseUpL`, `OnMouseMove` (the shared "return true" body `0x0049C7B0`), and the deleting
  destructor (`0x0049D020`). The constructor (`0x0049CF90`) runs the base constructor, installs the vtables and sets
  the state to 0 (§5.4), the block iterator to its defaults and the rest to zero.
- State kept (internal, not an interface): the scene **state** (§5.5), three microsecond clocks (scene, smoke, finale), the
  counters of blocks removed, processed and skipped, the finale effect handle, the occupant filter, and the situation
  indicator's visibility saved at `Init`.

### 5.3 Tuning *[compat]*

Read once per process (a static flag; see below) from the exemplar **`0x6534284A / 0x7A4A8458 / 0xCA5A576F`**
(name "ObliterateCity"; listed in INVENTORY-M6.md §6):

| Property | Type | Meaning | Default if absent | Shipped value |
|---|---|---|---|---|
| `0x6A5A6700` | uint32 | **block size**: the edge, in city cells, of the squares the isolated removal works in | 8 | **4** |
| `0x6A5A6701` | float32 array, (x, y) pairs | the **removal curve**: seconds since the start of the scene to the *total number of blocks that should have been removed by then* (foundations.md §4: linear between points, flat outside; the last x is the curve's maximum input) | none; the curve is empty and its maximum input is 0 | (0, 4), (10, 20) |
| `0x6A5A6702` | float32 | the **smoke-forming time** in seconds; stored in microseconds as `trunc(value x 1,000,000)` | 250,000 µs (0.25 s) | **10.0** (10,000,000 µs) |

*[inferred]* With no exemplar (a stripped installation) the removal phase has maximum input 0, so the first tick of
state 1 already moves on to the smoke phase (§5.5). The three values are held in process-wide variables, so they are
read by the first tool that runs after start-up and **not read again** until the flag is cleared. `Init` registers
a callback with the application (application slot 69, `+0x114`, with argument 0) whose only effect is to clear that
flag; when the application runs such callbacks is not decoded (CY9).

### 5.4 `Init` (`0x0049D390`) *[compat]*

Does nothing if already initialised. Otherwise, in this order:

1. If the tuning has not been read, read it (§5.3) and register the flag-clearing callback.
2. Base `Init` (cursor, 3D view, window manager) and `SetCapture` (city-tools-input.md §2.4, §2.6).
3. **`city.DisableSave()`** (city slot `+0x1A0`): the city cannot be saved while the scene runs.
4. Build the **occupant filter** (a 12-byte object, vtable `0x00A8F694`, one reference held). Its type test
   (`0x0049C7E0`) accepts **every occupant type except flora `0x74758926` and fire `0x69D25C5F`**.
5. Cell limits: `maxX := city.cellCountX - 1`, `maxZ := city.cellCountZ - 1`; the whole-city rectangle
   `(0, 0, maxX, maxZ)`.
6. `simulator.HiddenPause()` (simulator slot `+0x28`). The tool then holds references to the demolition utility, the
   occupant manager and the zone manager.
7. Subscribe the `+0x28` sub-object to the animation tick manager at **priority `0xC0`**, and to message
   **`0xEA623590`** (the finale effect's end message) on message server 2.
8. Initialise the **block iterator** (§5.4.1) over the blocks `0 .. (maxX / blockSize)` by `0 .. (maxZ / blockSize)`
   (integer division, both ends included), shuffled.
9. Clear the scene clock, the processed-blocks counter, the skip counter and the removed-blocks counter; state := 1.
10. **Situation indicator**: take the city situation manager (city slot `+0xC4`), remember its
    `GetCSIVisible()` and call `SetCSIVisible(false)` (city-situation.md §8, slot 29).
11. Post two messages (§5.6): `0xAA650861` with data1 = 1, then `0x2A611B51`.

The state is 1 only after `Init`. A tool whose `Init` did not run has state 0 and ticks do nothing.

#### 5.4.1 The block iterator *[compat]* (the set) *[compat-limit]* (the order)

The iterator visits **every block exactly once, in a random order**. A block `(bx, bz)` covers the cells
`[bx x s, bx x s + s - 1] x [bz x s, bz x s + s - 1]`, with `s` the block size, **clipped** to `[0, maxX] x [0, maxZ]`.
A city of 64 x 64 cells with `s = 4` has 16 x 16 = 256 blocks, a 128-cell city 1,024, a 256-cell city 4,096. The
random order comes from the C library's `rand` through a shuffle; FreeCity4 *[choice]*: any uniform random
permutation.

### 5.5 States and timing *[compat]*

The state is driven by the animation tick (§5.6): each tick adds the tick's elapsed microseconds `dt` (data2 of the
message, effects.md §6) to the clock of the current state. The first clock (the "scene clock") runs in states 1
to 3.

| State | Name | Entered | What each tick does | Leaves when |
|---|---|---|---|---|
| 0 | idle | construction | nothing | `Init` |
| 1 | **isolated removal** | `Init` | `scene += dt`; `DoIsolatedRemoval` (§5.5.1) | the scene clock reaches the curve's maximum input (10 s), or the iterator has no block left |
| 2 | **smoke screen forming** | `StartSmokeScreenFormingState` (§5.5.2) | `smoke += dt` | `smoke >= smokeTime` (10 s) |
| 3 | **finale** | the end of state 2 | `finale += dt` (§5.5.3) | the finale effect ends, or `finale > 20,000,000 µs` |
| 4 | ended | §5.5.3 | nothing (ticks are ignored) | |

Total length with the shipped tuning: 10 s of removal, then 10 s until the bulk removal, then until the
`obliterate_finale` effect ends or 20 s pass: **at most 40 s**; shorter if the effect ends earlier (its length is in the
effect directory, CY10).

#### 5.5.1 Isolated removal (`DoIsolatedRemoval`, `0x0049D5A0`)

The removal schedule uses the scene clock in seconds, represented as a float:
`t = sceneClock / 1,000,000`. At or beyond the curve's maximum input, the scene enters
the smoke-forming state without another isolated-removal batch.

Before that deadline, the cumulative block target is the curve value rounded to
the nearest integer, with ties to even under the default x87 rounding mode. A tick
whose target exceeds the previous target processes that increase in one batch;
an unchanged or lower target does no work and leaves the previous target intact.
The recorded target advances to the new value even if fewer blocks remain.

Blocks follow the iterator's random order and their cell rectangles are clipped
to the city. For each block, any sparkles are created before demolition. Sparkles
occur for the first processed block, for an occupied block, or after six skipped
blocks; a sparkle resets the skip count, and a block without one increments it.
Every processed block is demolished, whether or not it sparkles, and contributes
one to the processed count. Exhausting the iterator during a batch starts the
smoke-forming state immediately.

Demolition commits all cells of the clipped rectangle with privilege 0, flags
`0x2000`, no dezoning, the tool's filter, no cost output and no result set; the
remaining three optional arguments are zero (demolition.md §4.4).

With the shipped curve the target is `round(4 + 1.6 t)`: 4 blocks on the first tick, 6 at 1 s, 8 at 2.5 s, 12 at
5 s, 16 at 7.5 s and 20 just before 10 s. That is 20 of the 256 blocks of a small city (8 %), so the city is **mostly
still standing** when the phase ends (the rest goes at the start of state 3). If a tick is long, the loop processes
the whole delta at once. At the first tick with `t >= 10` the function goes straight to the smoke state.

- **`occupied(rect)`** (`IsCellRectOccupied`, `0x0049D0E0`): true when the occupant manager's
  `GetOccupantsByStandardCityCells` returns at least one occupant in the rect (corners inclusive) that the tool's
  filter accepts, at most one asked for. So trees and fires alone do not count.
- **Sparkles** are the particle effect **`sparkles_on_demolish`** (effects.md §9.4) created at the rect's centre in
  metres, `((x0 + x1) x 16 / 2, (z0 + z1) x 16 / 2)` (one city cell is 16 m), at the terrain height; the effect is
  started and the tool keeps no reference. The rule means: the first block always sparkles, then an empty block
  sparkles only after **six** empty ones in a row, and an occupied block always does.
- `DemolishRegion` (demolition.md §4.4) does the removal. With privilege 0 there are no authority checks; with flags
  `0x2000` there is no demolition effect and the 3D view's current filter is ignored. The tool's filter keeps
  **trees and fires** (flora `0x74758926`, fire `0x69D25C5F`). Lots are removed as a whole by the demolition rules, so
  one building may disappear on the first block that touches it.

#### 5.5.2 `StartSmokeScreenFormingState` (`0x0049CD90`)

State := 2; clear the smoke clock; create the particle effect **`obliterate_finale`** at the **centre of the whole
city**, `((maxX + 0) x 16 / 2, (maxZ + 0) x 16 / 2)` metres at terrain height (`CreateParticleEffect`,
`0x0049CC60`, "keep" flag set, the handle kept); and give the effect the end message `0xEA623590` with
the tool as data1 (`SetEndMessage`, effect slot 10, effects.md §11.1; the effect posts it when it finishes, §7 there). If the effect cannot be
created (no effects manager or terrain) the handle stays null.

`CreateParticleEffect(name, position, heightOffset, keep, &handle)` (`0x0049CC60`) creates the named visual effect
(effects.md §4.1), places it at `(x, terrainHeight(x, z) + heightOffset, z)`, starts it, and releases it at once unless
`keep` is set; with no effects manager or terrain the handle stays null.

#### 5.5.3 State 2 to 4 (`0x0049D070`, `0x0049CE20`, `0x0049CC30`)

- **End of state 2** (the smoke clock reaches the smoke time): state := 3; **`occupantManager.RemoveOccupants(filter,
  shutdown = true, 0)`** removes every occupant the filter accepts at once; then `DemolishRegion` over the whole-city
  rectangle with the same arguments as §5.5.1 (`0x0049CEA0`) and then, because the helper's second argument ("dezone")
  is passed as 1 by its only caller, **`zoneManager.PlaceZone(wholeCity, type 0, commit = 1, skipFunds = 1,
  checkSlope = 1, skipSizeCheck = 1, checkLotInside = 0, null, null, null)`** (zones.md §6): the whole city is
  dezoned. (The helper tests its own argument, not `DemolishRegion`'s result.)
  The finale clock is cleared. Trees and fires stay (they are the filter's exceptions; flora survives the
  obliteration, flora.md §10.3).
- **State 3**: `finale += dt`. When it exceeds **20,000,000 µs** and the effect handle is set, the effect is soft
  stopped (`Stop(0)`), released and the handle cleared. Then, if the handle is null and the state is not yet 4,
  **end the scene** (below).
- **Effect end message `0xEA623590`**: delivered to the same message target; if the handle is set, release it, clear
  it and **end the scene**. It can arrive in state 2 too; the scene then ends at once and the bulk removal never runs.
- **End the scene** (`0x0049CC30`): if the state is not 4 already, state := 4, call `EndInput` (slot 27: the 3D view
  removes the tool from its input stack, city-tools-input.md §2.7; that releases it and runs `Shutdown`, §5.8), then
  post **`0xEA65224E`** (CityObliterationSceneEnded, §5.6).

A scene whose finale effect was never created ends on the first tick of state 3, because the handle is null.

### 5.6 Messages *[compat]*

| Message | Direction | Meaning |
|---|---|---|
| `0x86AD10EE` animation tick | received (animation tick manager, priority `0xC0`) | data2 = elapsed microseconds (§5.5); ignored in states 0 and 4 |
| `0xEA623590` | received (message server 2) | the finale effect has ended (§5.5.3) |
| `0xAA650861`, data1 = 1, data2 = data3 = 0 | posted at `Init` | handled by the audio event handler (audio.md §7.2): it sets a byte of the radio service from data1 *[inferred: a flag that, set to 1, mutes or blocks the radio]* |
| `0x2A611B51`, all data 0 | posted at `Init` | handled by the audio event handler: calls the radio service's `StopPlayback` *[inferred]* |
| `0xEA65224E` CityObliterationSceneEnded, all data 0 | posted when the scene ends normally | the application runs `DoCityObliteration` (application.md §6.2, §11.6) |

All three posts go through `PostMessageA` (`0x0049CA70`): a standard message of the given type with data1 to data3,
queued (not sent) on message server 2; nothing is posted without that server. The tool does **not** undo the two audio
posts at `Shutdown`; the application replaces the city afterwards (§5.9).

### 5.7 Input *[compat]*

`OnKeyDown(vkey)` (`0x0049C790`) returns true for **every key**; on **Escape (`0x1B`)** it first calls `EndInput`
(slot 27), which removes the tool and runs `Shutdown` (§5.8). It posts no message. The left mouse press, release and
the mouse move return true (consumed). The right button, the wheel and the key-up are the base's stubs (false), so the
camera can still be rotated and zoomed during the scene. Escape works in every state, including state 3.

### 5.8 `Shutdown` (`0x0049C8B0`) *[compat]*

Only when initialised; always returns true. In order:

1. Unsubscribe the `+0x28` sub-object from message `0xEA623590` and from the animation tick manager.
2. If the finale effect handle is set: **hard stop** it (`Stop(1)`), release it, clear it.
3. Release the demolition utility, the occupant manager and the zone manager; **`simulator.HiddenResume()`**
   (simulator slot `+0x34`); release the occupant filter.
4. **`city.EnableSave()`** (city slot `+0x19C`); take the city situation manager and call `SetCSIVisible(saved)` with
   the value saved at `Init`.
5. `ReleaseCapture`, base `Shutdown`.

Consequences *[quirk]*: **Escape leaves the city partly razed but intact in every other way** (saving is allowed
again, the simulator resumes, nothing is rebuilt). The blocks removed so far and, in state 3, the entire city are
gone. The disabled control `0x4988BC6A` of the 3D view's panel is not re-enabled by the tool.

### 5.9 After the scene (application.md §11.6) *[compat]*

`0xEA65224E` makes the application save the city's **terrain and flora** into a temporary segment (the other city
data is not kept), close the city, delete its file, create a new city from that segment, set it **unestablished**, save
it to the old path, and restore the zoom and rotation. So the file on disk is a fresh God-mode city on the same
landscape, and everything the tool removed is irrelevant to the result. The region's own object for the tile is
untouched until the region is next reloaded (region-model §7.1).

The addresses of the functions of this spec are in `re/symbols/region-city-dialogs.tsv`.

## 6. Conformance tests

1. **Names that `IsFileNameValid` refuses:** `""`, `" "`, `"."`, `".."`, `"a:b"`, `"a/b"`, `"a?"`, `"CON"`, `"con.txt"`,
   `"Com3"`, `"clock$"`, `"nul.x.y"`. **Accepts:** `"Berlin"`, `"New York"`, `"San Francisco"`, `"Console"`, `" con"`,
   `"a.b"`, `"é"`, a 300-byte text that contains `:` (the length rule, *[quirk]*).
2. **Default region name.** With regions named `New Region` and `new region 1` present (case differs), the dialog's
   initial name is `New Region 2`; with none, `New Region`; with only `NEW REGION`, `New Region 1`.
3. **Duplicate test.** `"BERLIN"` against a region whose display name is `Berlin` is a duplicate; `"Berlin "` is not.
4. **Create.** From a nation with 6 regions, OK with `Foo` and Water on: a seventh region, folder `Regions\Foo`,
   `config.bmp` present, `region.ini` with `Name = Foo`, `Terrain type = 1`, `Water Min = 60`, `Water Max = 100`; the
   dialog result is 6. Cancel: result -1, nothing written.
5. **Import list.** For size 2 with the shipped data and every region initialised: the number of rows is the number
   of cities of size `(2, 2)` over the regions plus the Downloads folder's. The six shipped regions give 249 such cities
   (the medium tiles of region-model §12 test 1, all filled once the duplicate Timbuktu file has moved to Downloads,
   region-model §12 test 2). A group row selected, or no row, with OK gives result 0.
6. **Import result.** Selecting the Downloads city `City - New City.sc4` gives `(-1, x, z, "<download dir>\City - New City.sc4")`;
   selecting a Berlin city gives `(Berlin's index, x, z, "")`.
7. **Establish validation.** City name `"A"`, mayor `"B"`: the button is enabled. City name `"   "`: disabled. A name made of
   the single byte `0xE9`: counts. Tab from the city edit selects the mayor edit's text.
8. **Establish OK.** With City `X`, Mayor `Y`, Hard: the city has difficulty 2, name `X`, mayor `Y`, established true,
   the preferences hold `X`, `Y` and 2, and `SavePreferences` ran once.
9. **Removal curve.** Targets `round(4 + 1.6 t)`: t = 0 gives 4, 0.3 gives 4 (4.48), 1 gives 6, 2.5 gives 8, 5 gives 12,
   9.99 gives 20; at t = 10 the state becomes 2 with no further removal. Total over the phase: 20 blocks.
10. **Sparkle rule.** With no occupied block, blocks 1, 8, 15, ... sparkle (the first block, then every seventh: the skip
    counter is 0 after a sparkle and the next block sparkles once it has reached 6). An occupied block always sparkles
    and resets the counter.
11. **Escape.** In state 1, Escape removes the tool, resumes the simulator, re-enables saving, restores the situation
    indicator and posts nothing; in state 3 the city is then empty of everything but trees and fires.
12. **No effect.** With no effects manager the scene still ends: state 3 ends on its first tick.
13. **Import call.** Tile top-left (6, 5), a Berlin city chosen: `MoveCity(Berlin, thatCity, 6, 5)` on the active region;
    a Downloads file chosen: `MoveCity(null, recordFromFile, 6, 5)`; a cancel, or a false result, changes nothing.
14. **New region becomes active.** OK in the new-region dialog, then the screen makes the result the active region; Cancel
    (-1) leaves the active region alone.

## 7. Verified against the game's data

Read with `tools/dbpf.py` from the base packages (`SimCity_1.dat`) and the English locale file:

- The four UI scripts (new region `0x8A5AB1CB`, import city `0x8A5AB1CD`, establish city `0x2A41436B`, obliterate
  confirmation `0x2A41436C`, each type 0 / group `0x96A006B0`): control IDs, kinds, initial states, hidden headers,
  `maxtext` values, "defaultkeys" and the window IDs of §2.1, §3.1, §4.1, §5.1 match the code.
- The tuning exemplar `0x6534284A / 0x7A4A8458 / 0xCA5A576F` ("ObliterateCity", only in `SimCity_1.dat`): `0x6A5A6700 = 4`,
  `0x6A5A6701 = [0, 4, 10, 20]`, `0x6A5A6702 = 10.0`.
- The three tree icons `0x1441624A..C` in group `0x46A006B0` (16 x 16 PNG), the quoted English strings, and the two
  effect names (game records with hidden pauses of 5 s and 15 s, effects.md §9.4).
- Not checked against running code: every behaviour tagged *[inferred]*.

## 8. Open

The open questions are CY1 to CY12 in [questions/region-city-dialogs.md](questions/region-city-dialogs.md) (CY1 to CY12
are all still open; the evidence for RM7 is §3.5 and is filed in `questions/region-model.md` for the maintainer to
close). Not covered here: the region screen and region view windows (region-screen.md), the terrain edge
reconciliation that follows an establish, and the effect records' contents (effects.md). The load-region,
delete-region and delete-city dialogs are §9.

## 9. The base's remaining bodies and the three other region dialogs *[compat]*

Added 2026-10-24 (re-gzsmallwin7), static RE on the instructions of the Steam 1.1.641 exe (same code as GOG).

**Base** (`cSC4DialogWinProcBase`, 8 functions): besides §1.3–§1.5, slot 10 `GZOnOK` (`0x00777D10`) ends the dialog
with 1, slot 9 (`0x00777CF0`) with 0, the new-region slot 9 (`0x00777CC0`) with −1; the deleting destructor
`0x00777D30` (thunk `0x00777CE0`) restores the two vtables and frees on bit 0.

Each of the three dialogs is a 0x1C-byte base object `{vtable, reference count, window, script instance, root window
ID, focus control}` made, `Display`ed (§1.4), released, and its `Display` result returned by one function:

| Dialog | Function | Script instance / root ID / focus | Overrides | Result |
|---|---|---|---|---|
| load region | `0x007780C0` | `0x8A5AB1CC` / `0x4A5BA0E7` / `0x1000` | attach `0x00778810`, OK `0x00778060` (vtable `0x00AB5380`) | the chosen region index |
| delete region | `0x00778130` | `0x8A5AB1CE` / `0x6A5BA20C` / 0 | attach `0x00778420` (vtable `0x00AB53C0`) | true for OK |
| delete city | `0x00778220` | `0x8A5AB1D0` / `0x8A5AB1D0` / 0 | none (vtable `0x00AB5400`) | true for OK |

- **Load region, attach** (`0x00778810`): the list `0x1000` (as list box `0x4132242B`, widget-controls.md) gets one
  row per region of the nation (`GetRegionCount`, `GetRegion(i)`; the row's ID is `i`), with its name; a name that
  starts with `#` gets a second `#` in front (`0x007789C0` inserts it) *[inferred: `#` is the list's escape]*. Then
  the active region's row is selected by ID (list slot 11 with `GetActiveRegionIndex`) and list slot 16 runs.
- **Load region, OK** (`0x00778060`): ends the dialog with the list's selected ID (list slot 13). Cancel ends with 0,
  which is also region 0 *[quirk]*.
- **Delete region, attach** (`0x00778420` → `0x00778290(0x1000)`): the text `0x1000`'s caption (slot 73) is passed
  through the application's slot 42 object (its slot 9, a token substitution *[inferred]*) and written back (slot 74),
  so the question names the region.
- **Delete city** has no body of its own: the script's text is shown as is.
