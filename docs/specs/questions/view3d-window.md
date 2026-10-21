# view3d-window: open questions
<!-- Questions about [view3d-window.md](../view3d-window.md). Answered entries move to answered/view3d-window.md. -->

<!-- from: the cSC4View3DWin window lane (re-view3dwin4, 2026-10-21) -->
- **VW2 - §3 step 8:** identity of GZCOM object `0x42E3EA4B` and of class `0x42E967BE` / IID `0xE2C1B3C4` that
  `Init` looks up (`0x007E9CF1`, `0x007E9D37`) and stores at `+0x114`; window `DoMessage` later calls its slot 9 with
  an unconsumed key (`0x1B` messages). *[inferred: the key-binding / accelerator manager]* What settles it:
  `re/class_id_table.tsv` or the class whose factory registers `0x42E967BE`.

<!-- from: the cSC4View3DWin window lane (re-view3dwin4, 2026-10-21) -->
- **VW3 - §3 step 2:** the length of the command-name table at `0x00B0DD98` (records `{id, name, 0}`) and the
  meaning of the `0x68` argument (record count 104, or something else), plus the registry's identity (framework
  lookup class `0xCB90253A`, IID `0xEB903A32`). What settles it: walk the table to its terminator and read the
  registry's slot 7.

<!-- from: the cSC4View3DWin window lane (re-view3dwin4, 2026-10-21) -->
- **VW4 - §6.3:** the branches of the message filter `0x007F58E0` (968 bytes) for IDs `0x287259F7`, `0x8988BC94`,
  `0x2988BC85` and `0x4988BC6A` (the last pushes `0x8810`), and which window message types besides 3 and `0x12`
  it inspects. What settles it: a walk of `0x007F58E0` from `0x007F591A`.

<!-- from: the cSC4View3DWin window lane (re-view3dwin4, 2026-10-21) -->
- **VW5 - §6a step 2:** identity of GZCOM class `0xE9C6262A` (constructor `0x007FF930`, 1068 bytes) that
  `DoPreCityInit` stores at `+0xEC` and hands to the renderer; and the condition by which `DoPostCityInit` picks
  `DoCityEstablished` or `DoCityObliterated` (`0x007ED320`–`0x007ED33A`). What settles it: the class ID table entry
  for `0xE9C6262A` and the compare before `0x007ED333`.
