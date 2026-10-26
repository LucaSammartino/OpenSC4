# region-screen: open questions
<!-- Questions about [region-screen.md](../region-screen.md). Append new entries at the END; when one is answered, move it to answered/region-screen.md. Rules: README.md. -->

<!-- from: re-regionscreen (2026-10-20) -->
- **QR1 - §1, §2:** the window construction: what `InitApplicationServices` (`0x007ACC90`) starts and
  `ShutdownApplicationServices` (`0x007AC380`) stops, the windows `LoadGUI` (`0x007B0470`) builds, and what the
  tuning values `0xCA383CA3`-`0xCA383CB0` other than the clouds' opacity control. [as read]
- **QR2 - §6.5:** the import path after `SC4DisplayImportCityDialog`: the size check against the tile, which region
  slots place the read record, and what happens to the city that was on the tile. [as read]
- **QR3 - §5:** the buttons `0x4A779A09`, `0x4A779A1A`, `0x4BB92C1F`, `0xA98F4F88`, `0xCA5CFEE2`, `0xEBB91356`, the
  panels of `ShowPanel` (`0x007ABDF0`), and what `MakeWholeRegion` (`0x007AEDD0`, command `0x6A9757C2`) builds.
  [as read]

<!-- from: region-city-dialogs (2026-10-21) -->
- **QR4 - §6.5 (re QR2):** the import path after `SC4DisplayImportCityDialog` is described in
  [region-city-dialogs.md](../region-city-dialogs.md) §3.5: the dialog filters by the tile's size (no later size check);
  a city of a region or of the Downloads folder is placed by `cSC4Region::MoveCity` on the active region with the tile's
  top-left cell (a Downloads file is read only to carry its path and serial number), and the tile's previous city is
  deleted with its file by `MoveCity`. What the screen shows afterwards (CY8 there) is open. [the maintainer may close QR2]

<!-- from: impl-region30 (2026-10-26) -->
- **QR5 - §3:** `ComputeBaseLimits` sets scroll limits from region size and cloud area, but the spec gives no
  projection, padding or clamp formula. What are the resulting limits, including for an empty region? The core
  stops this item at `Unsupported::Open("QR5")`; snapshot masks separately remain open RGV5.
