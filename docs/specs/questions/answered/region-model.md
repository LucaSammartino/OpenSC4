# region-model: answered questions (archive)
<!-- Archive for [region-model.md](../../region-model.md). Lanes do not read this file. Open questions are in ../region-model.md. -->

<!-- from: region-model spec (2026-10-21) -->
- ~~**RM4 - §5.1:** `SC4ReadAppLoadSaveVersion` (`0x005DBF30`) returns 0 or 65 when the package has no version
  record, depending on a second package call (DB segment slot `+0x4C`). What is that call, and is a city file
  without a version record accepted (65) or refused (0)? Do files saved by the retail 1.1.641 game always carry
  the record? [accepted as 65]~~
  **ANSWERED 2026-10-21** in region-model.md §5.1: no version record counts as 65, an unreadable one as 0; all shipped city files carry the record (found while writing regional-city.md §9.1).
