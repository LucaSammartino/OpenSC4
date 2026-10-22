# region-model: answered questions (archive)
<!-- Archive for [region-model.md](../../region-model.md). Lanes do not read this file. Open questions are in ../region-model.md. -->

<!-- from: region-model spec (2026-10-21) -->
- ~~**RM4 - §5.1:** `SC4ReadAppLoadSaveVersion` (`0x005DBF30`) returns 0 or 65 when the package has no version
  record, depending on a second package call (DB segment slot `+0x4C`). What is that call, and is a city file
  without a version record accepted (65) or refused (0)? Do files saved by the retail 1.1.641 game always carry
  the record? [accepted as 65]~~
  **ANSWERED 2026-10-21** in region-model.md §5.1: no version record counts as 65, an unreadable one as 0; all shipped city files carry the record (found while writing regional-city.md §9.1).

<!-- from: region-model spec (2026-10-21) -->
- ~~**RM7 - §7.3:** `MoveCity` has four arguments and no caller in the executable. The spec reads them as
  `(sourceRegion, city, x, z)` with the destination being `this` (the active region). Which windows call it (the
  region view's drag-and-drop?), what do they pass, and is the serial number change on a move within the same
  region (and the possible ` (n)` rename of its file) really visible in play? [as read]~~
  **ANSWERED 2026-10-21** in region-model.md §7.3: the region screen's import command calls it with the dialog's source region (or null) and the selected tile's top-left cell (found while writing region-city-dialogs.md). The same-region serial change remains unobserved in play.
