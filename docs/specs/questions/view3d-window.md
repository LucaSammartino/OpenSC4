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
- **VW12 - §3.1, §6k:** which hash/bucket traversal determines trigger pair order in the version-4 save record?
  What are the overflow/absent-retraction rules for trigger counts? The logical core uses wrapping uint32
  increment and absent retract as no-op, and preserves pairs without claiming original byte ordering. Also,
  does SizeOf's `RZAtofA` support a locale-specific decimal separator or hex floats? The provisional parser uses
  dot-decimal prefix semantics and NaN/Inf. [spec gap]

<!-- from: core-view3dwin5 (2026-10-23, round 2) -->
- **VW14 - §§6a–6b failure paths and overlay lifetime:** which camera/control/renderer/script-window/helper
  allocation or QI failures abort city/window initialization, and which skip dependent calls? The core currently
  keeps null fields and skips dependent configuration, with balanced successful references. Also, §6b releases
  the unstored full-size overlay creation reference immediately and omits its destruction from RemoveGUI, but
  requires no AddGUI-created window to survive RemoveGUI. Should RemoveGUI explicitly destroy the parent-owned
  overlay, or does that requirement apply after the city window's base teardown? [null-safe skips; retain only a
  borrowed stable overlay handle and destroy it at RemoveGUI without releasing that borrowed handle]

<!-- from: core-view3dwin5 (2026-10-23, round 3) -->
- **VW17 - §6i remaining menu inventory:** what script/catalog kinds, groups, backgrounds and remembered-state
  fields do buttons `0x36`/`0x8A32DDDB`, `0x49E95D2B`, `0x8991EE08`, `0x2B79E398`, `0xEA6E379D`,
  `0xABB27B91`, `0x09DE87AE` and `0xE9DE87A7` select? Which landscape catalogs correspond to the five default
  state pairs at `+0x2A8` through `+0x2C8`? Which catalog identity is used for utilities and zoning, and which
  background is used where none is listed? What caller-data argument do the scripted menu buttons pass?
  What are the bus-stop and pipe buttons' command-server IDs (VW9)?
  [unidentified buttons return unhandled; specified typed bus/pipe tools are callable; catalog 0/background 0
  means unspecified to the foreign provider, not a claimed resource identity; unspecified state fields use
  logical per-button memory keys; exact inventory remains open]

<!-- from: core-view3dwin5 (2026-10-23, round 3) -->
- **VW18 - §§1.1, 5.1, 6, 6f, 6j–6l remaining opaque contracts:** how is mission activation data1 converted
  into UpdateMissionProgress's first argument? What payload fields select the advice-link subpanel/graph/budget,
  and what is the terrain debug-string contract (only its slot name is given)? Do renderer/model-maker and
  accelerator getters AddRef, or return borrowed handles? (Filter and input-control getters specify this.)
  Which background mode is selected on zoom refresh? What are the coverage-helper recalculation arguments,
  visibility/clear guards and spatial set operations beyond the two effect IDs/radius 32? The remaining §6l
  constructors are named with addresses but no owner-visible setup/teardown contracts; which extra view actions
  do they require beyond the already specified GUI lifetime? [raw MissionActivation retained without guessing a
  derived value; advice, debug/info and opaque helper operations stay foreign/open; background mode is supplied
  by the host, first-valid-picture selection and reference lifetime are implemented]
