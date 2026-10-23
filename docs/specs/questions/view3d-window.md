# view3d-window: open questions
<!-- Questions about [view3d-window.md](../view3d-window.md). Answered entries move to answered/view3d-window.md. -->

<!-- from: the cSC4View3DWin window lane (re-view3dwin4, 2026-10-21) -->
- **VW6 - §6b:** (narrowed 2026-10-21, re-view3dwin4: the `AddGUI` tail `0x007EED41`-`0x007EF1B6` and `+0x190`/`+0x194`/`+0x3D0` are now in §6b) still open: who creates `+0x2EC` and `+0x2F0` (no direct store in any `cSC4View3DWin` method; `RemoveGUI` releases them), (the object `0x00B43C9C` is the graphic system, re-view3dwin 2026-10-21: slot 3 `CreateBuffer` makes the `+0x3D0` surface, slot 12 `IsFullScreenMode` gates edge scrolling, §6c), and the per-panel extras of the five-panel loop between `0x007EE39B` and `0x007EE610` (flag `0x800` slots 23/24 calls). What settles it: a walk of `0x007EE39B`-`0x007EE610` and a search for indirect stores to `+0x2EC`/`+0x2F0`.

<!-- from: re-view3dwin (2026-10-21) -->
- **VW7 - §6i:** what message `0x470BD214` (data1 0 or 1, posted by two landscape items and the global terrain tool
  cases 5 and 6) asks for, and who handles it. [as read]

<!-- from: re-view3dwin (2026-10-21) -->
- **VW8 - §6d, §4:** the objects at `+0x184`, `+0x188` and `+0x410` (stopped and released at `Shutdown`); no
  function read in this lane creates them. [as read]

<!-- from: core-view3dwin5 (2026-10-23) -->
- **VW9 - §3 step 2, §6g:** Init registers 104 exact `(id, name, 0)` records, but §6g groups several commands
  without their full IDs/names (zone, network and place-lot commands), and the registry order beyond the first
  nine is absent. Please give the complete ordered 104-record table, including records deliberately unhandled
  here. An implementer can route typed commands literally but cannot reproduce exact registration or numeric
  dispatch from the current inventory. [spec gap]

<!-- from: core-view3dwin5 (2026-10-23) -->
- **VW10 - view3d.md §5.1 step 14, §5/§6j:** the paint tail is described only as a child walk and calls at
  numeric addresses. Which owner operations run, in what order and under which guards, including deferred-message
  replay and cost/cursor/mission updates? Please specify the callback contracts and return behavior; the current
  core exposes a foreign tail boundary rather than inventing those operations. [spec gap]

<!-- from: core-view3dwin5 (2026-10-23) -->
- **VW11 - §2, §6f and view3d.md §5.1:** what is the constructor/default paint period at `+0xFC`, its unit and
  its exact due comparison? What does SetZoom/SetZoomLimits do with `min > max`? The core currently uses zero
  microseconds for the unspecified period and literal sequential min/max comparisons; these are provisional
  choices, not measured defaults. [spec gap]
