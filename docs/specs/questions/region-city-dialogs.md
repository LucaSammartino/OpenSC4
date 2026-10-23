# region-city-dialogs: open questions
<!-- Questions about [region-city-dialogs.md](../region-city-dialogs.md). Append new entries at the END; when one is answered, move it to answered/region-city-dialogs.md. Rules: README.md. -->

<!-- from: region-city-dialogs (2026-10-21) -->
- **CY1 - §3.3.2:** the import dialog calls `Init` on every region that is not the active one, lists its cities, and
  then calls `Shutdown`. By region-model §6.1, §6.2 that loads the region's city files, moves overflow city files to
  `Regions\Downloads\` and rewrites `region.ini` with the resolved name. Does opening the dialog really change the
  shipped regions on disk (Timbuktu loses its second city on tile (6, 5) and every `Name = #0x...` token is replaced by
  text), and is that visible to a player (the Downloads rows of the same dialog would then show the moved file)?
  [yes, both halves are read from the bytes; the engine does the same]
- **CY2 - §3.3.1:** `GetAllCities` of the **active** region returns its live cell map. The region screen inserts a
  placeholder regional city (not established, name "New City", no file) when a tile is played (region-screen.md §6.1).
  Is such a placeholder still in the map when the import dialog opens (a tile played and left without saving), is it
  listed as a row of the active region's group, and what does `MoveCity` do when one is picked (it has no file,
  region-model.md §7.3)? [as read: nothing in the dialog filters them out; unknown whether one can survive]
- **CY3 - §2.3, §4.2:** both name edits get a flag set with the value `0x800` through the edit control's slot at
  `+0x130`, and the dialogs react to a `0x9B1552D8` message. Is that the "notify on text change" option of the UI script
  (`notifyonchange`, legacy-control.md §4.2, word `0x416`), and does `0x9B1552D8` carry the edit window in data1? Also: is
  slot `+0x84` with `0x80` the maximum text length, and slot `+0x12C` with `(0, length)` "select all"? [as read: yes
  to all four, tagged *[inferred]*]
- **CY4 - §2.3:** `IsFileNameValid(name, flag)`: with `flag = 0` the function assumes the file system is FAT and applies
  the reserved device-name list; a non-zero flag skips that list. It accepts any name longer than 256 bytes without
  testing the characters. Is the second argument a file-system-name pointer in the original source, who passes a
  non-zero value (the only callers found are the two new-region dialog paths, both with 0), and is the long-name
  acceptance intended? [as read: reserved-name rule always on in the game; long names accepted]
- **CY5 - §4.4:** `DoCommand` treats the button codes `0x42B7C353` (cancel) and `0x42B7C356` (accept) like Cancel and
  Establish City. Which input produces them for a window with "defaultkeys" on (Escape and Enter, the title-bar close
  button)? The dialog's character handler handles Enter itself, so is `0x42B7C356` ever delivered? [unknown which
  inputs; the spec treats them as the generic close and accept commands]
- **CY6 - §3.3.1:** the region groups are added at the "sorted" position (position code -4) of the tree-view control.
  What ordering does the control use (case-sensitive byte order, or case-folded; does the Downloads group sort in with
  the region names)? [unknown; the engine sorts case-insensitively *[choice]*]
- **CY7 - §4.5:** the establish dialog writes the city name, the mayor name and the difficulty into the application
  preferences at `+0x6C`, `+0x26C`, `+0x46C`, which line up with the new-city preferences block of `SetupPreferences`
  (slot 23) and `UpdateCityCache(prefs)` (slot 25) of regional-city.md §7.1. Who calls those two regional-city slots (the
  new-city path of the region screen, `cSC4App::NewCity`)? The establish dialog does not. [unknown callers; the
  structure is the preferences block at `+0x6C`]
- **CY8 - §2.7, §3.5:** after a new region or an import the region screen asks the application for the region view
  (`RequestGoToRegionView(false)`, application slot 12). The screen is already up, so does that call rebuild it (re-run
  `PutCitiesInView`, region-screen.md §3, so the imported city's thumbnails and the new region show) or only switch the
  view? [unknown; the command sequences themselves are read from `GZOnCommand` and tagged *[inferred]*]
- **CY9 - §5.3:** `Init` registers a callback with the application (application slot 69, `+0x114`) that only clears the
  "tuning was read" flag of the obliterate tool. The registration appends `(function, argument)` to a list at
  application `+0x5C0`. When is that list run (application shutdown, city shutdown, a preferences change)? Does a
  second obliteration in the same session re-read the exemplar? [unknown; the spec says "not read again until the flag
  is cleared"]
- **CY10 - §5.5:** how long does the particle effect `obliterate_finale` run? Its end message `0xEA623590` ends the
  scene and cancels the 10-second bulk removal when it comes first. The effect directory (effects.md §3) holds its
  components; the game record only holds a 15-second hidden pause. What are the emitter lifetimes? [unknown; the scene is
  at most 40 seconds with the shipped tuning]
- **CY11 - §5.6:** the two messages the tool posts at `Init`, `0xAA650861` (data1 = 1) and `0x2A611B51`, are handled by
  the audio event handler and forwarded to the radio service (slot 30, a byte setter, and slot 31, `StopPlayback`).
  What does the byte at radio-service `+0x54` do, and is it reset when the new city starts? [unknown; read as "stop the
  radio and block it while the scene runs"]
- **CY12 - §2.5:** with the popup gate off (advisors.md §6.8.1) the "Cannot create region" notification shows nothing, so
  OK with a duplicate region name does nothing visible and the new-region dialog stays open. Is that the observed
  behaviour in an unattended run (a harness that closes dialogs with button 0 would end it as a cancel, result -1)?
  [as read; unattended runs should not open the dialog]
