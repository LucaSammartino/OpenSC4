# view3d-window: answered questions
<!-- Answered entries moved from ../view3d-window.md. -->

<!-- from: the cSC4View3DWin window lane (re-view3dwin4, 2026-10-21) -->
- **VW1 - §3 step 3:** what each of the thirteen cheats does in `ProcessCheatCode` (`0x007F3510`, 244 decomp lines).
  The Ghidra output types the cheat ID as a float, so branch targets must be read on the instructions. Known so
  far: `You don't deserve it` (`0x4A82253E`) sets byte `+0x446` and returns `0x4A822501`; `WatchMeMove`
  (`0x4A972F37`) sets a byte flag and returns `0x4A972F01`; one branch sets the camera control's value to `64.0`
  or `0.9f` (`0x3F666666`); two branches build a `cSC4RealisticOccupantFilter` at `+0x320`. What settles it: a
  byte-level walk of `0x007F3510` mapping each `CMP` against the IDs of §3 to its effect.
  **ANSWERED 2026-10-21 (re-view3dwin4):** DIFFERS — view3d-window.md §3.1: the earlier notes were wrong on two points: both byte-flag cheats return true (not an ID), and the 0.9/64 values are SizeOf's clamp of its argument; TastyZots/Zoneria toggle one realistic-filter flag each and return false.

<!-- from: the cSC4View3DWin window lane (re-view3dwin4, 2026-10-21) -->
- ~~**VW2 - §3 step 8:** identity of GZCOM object `0x42E3EA4B` and of class `0x42E967BE` / IID `0xE2C1B3C4` that
  `Init` looks up (`0x007E9CF1`, `0x007E9D37`) and stores at `+0x114`; window `DoMessage` later calls its slot 9 with
  an unconsumed key (`0x1B` messages). *[inferred: the key-binding / accelerator manager]* What settles it:
  `re/class_id_table.tsv` or the class whose factory registers `0x42E967BE`.~~
  **ANSWERED 2026-10-21 (re-view3dwin):** MATCHES — `0x42E3EA4B` is the interface of the key-configuration resource (type
  `0xA2E3D533`, group `0x6A231EAA`, instance `0x6A9362EF`) and class `0x42E967BE` (`0xE2C1B3C4`) the key accelerator,
  made into object `+0x120` (the handler's `+0x114`) and filled by the resource's slot 3; window `DoMessage` hands it
  the keys no control took (view3d-window.md §3 step 8).

<!-- from: the cSC4View3DWin window lane (re-view3dwin4, 2026-10-21) -->
- ~~**VW3 - §3 step 2:** the length of the command-name table at `0x00B0DD98` (records `{id, name, 0}`) and the
  meaning of the `0x68` argument (record count 104, or something else), plus the registry's identity (framework
  lookup class `0xCB90253A`, IID `0xEB903A32`). What settles it: walk the table to its terminator and read the
  registry's slot 7.~~
  **ANSWERED 2026-10-21 (re-view3dwin):** MATCHES — `0x68` is the record count: the table holds 104 records, every one listed
  with what `ProcessCommand` does in view3d-window.md §6g; the registry is the command server (command-system.md §5).

<!-- from: the cSC4View3DWin window lane (re-view3dwin4, 2026-10-21) -->
- ~~**VW4 - §6.3:** the branches of the message filter `0x007F58E0` (968 bytes) for IDs `0x287259F7`, `0x8988BC94`,
  `0x2988BC85` and `0x4988BC6A` (the last pushes `0x8810`), and which window message types besides 3 and `0x12`
  it inspects. What settles it: a walk of `0x007F58E0` from `0x007F591A`.~~
  **ANSWERED 2026-10-21 (re-view3dwin):** MATCHES — `0x287259F7` is a panel-tab toggle; the IDs are the tabs (options
  `0x8988BC94` panel 3, MySim `0x4988BC6A` panel 2, mayor `0xC988BC79` panel 1 with the establish dialog first in an
  unestablished city, God `0x2988BC85` through `DoPanelCommandGUI`); type `0x12` is eaten and a window whose slot 67
  test is true lets the message pass (view3d-window.md §6h).

<!-- from: the cSC4View3DWin window lane (re-view3dwin4, 2026-10-21) -->
- ~~**VW5 - §6a step 2:** identity of GZCOM class `0xE9C6262A` (constructor `0x007FF930`, 1068 bytes) that
  `DoPreCityInit` stores at `+0xEC` and hands to the renderer; and the condition by which `DoPostCityInit` picks
  `DoCityEstablished` or `DoCityObliterated` (`0x007ED320`–`0x007ED33A`). What settles it: the class ID table entry
  for `0xE9C6262A` and the compare before `0x007ED333`.~~
  **ANSWERED 2026-10-21 (re-view3dwin):** MATCHES — `0xE9C6262A` is `cS3DCamera` (camera.md), the S3D camera the renderer is
  wired to; `DoPostCityInit` picks `DoCityEstablished` when the city is established (city slot 22), else
  `DoCityObliterated` (view3d-window.md §6a).
