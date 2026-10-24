# tutorial-window: answered questions (archive)
<!-- Archive for [tutorial-window.md](../../tutorial-window.md). Lanes do not read this file. Open questions are in ../tutorial-window.md. -->

<!-- from: re-tutwin7 (2026-10-23) -->
- ~~**TUW6 - §5.1:** interface `0x49D98264` gets slot 18 `(-1, 1)` from the reset walker and `(task slot 19, 0)`
  from the hint walker. Which window class has it, and what is the value? [unknown]~~
  **ANSWERED 2026-10-23 (re-catview7):** `0x49D98264` is the catalog view `cSC4WinCatalogView` (catalog-view.md §2).
  Its slot 18 `(id, on)` sets every item except `id` to `on`, so the reset walker enables every item and the hint
  walker disables every item but the task's item ID (task slot 19). tutorial-window.md §5.1 and §8 updated.
