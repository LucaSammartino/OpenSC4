# tutorial-window: answered questions (archive)
<!-- Archive for [tutorial-window.md](../../tutorial-window.md). Lanes do not read this file. Open questions are in ../tutorial-window.md. -->

<!-- from: re-tutwin7 (2026-10-23) -->
- ~~**TUW6 - §5.1:** interface `0x49D98264` gets slot 18 `(-1, 1)` from the reset walker and `(task slot 19, 0)`
  from the hint walker. Which window class has it, and what is the value? [unknown]~~
  **ANSWERED 2026-10-23 (re-catview7):** `0x49D98264` is the catalog view `cSC4WinCatalogView` (catalog-view.md §2).
  Its slot 18 `(id, on)` sets every item except `id` to `on`, so the reset walker enables every item and the hint
  walker disables every item but the task's item ID (task slot 19). tutorial-window.md §5.1 and §8 updated.

<!-- from: re-tutwin7 (2026-10-23) -->
- ~~**TUW5 - §8:** the `cSC4TutorialTask` slot uses are read only from this window (3 build, 7 allowed ID, 8 and 9
  texts, 11 and 12 hint window, 14 start, 16 end, 19 value, 20 type, 22 zoom, 23 and 24 offset). Do the task's own
  bodies confirm them, and what are task types 1, 4, 8, 0xB–0xD and 0x10? [inferred]~~
  **ANSWERED 2026-10-23 (re-tuttask7):** confirmed by tutorial-task.md §1 (slot 5 is the area, slot 9 the try-again
  message). The type is the Lua `function_index`. What the task itself does per type is in tutorial-task.md §5:
  - 1, 2, 3, 8, 0x11 highlight the area;
  - 5 also hidden-pauses the simulator;
  - 9 sets a fire in the nearest building;
  - 4 puts `target_guid` first in the button list;
  - 2 and 0x11 also build a cell grid.

  Types 0xB–0xD and 0x10 only change the window's hint handling (tutorial-window.md §5.2). tutorial-window.md §8
  updated.
