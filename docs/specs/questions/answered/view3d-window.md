# view3d-window: answered questions
<!-- Answered entries moved from ../view3d-window.md. -->

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
  **ANSWERED 2026-10-23 (re-view3dwin-vw5):** DIFFERS — §6l: mission argument 1 is the borrowed IGZUnknown attachment, data1 remains the flag; advice uses type/data1, debug slot is empty, all three getters borrow, background mode latches, and coverage has zero-argument service-list recalculation with literal cache quirks. Auxiliary ownership/setup is specified. VW-T33–T39.

<!-- from: the cSC4View3DWin window lane (re-view3dwin4, 2026-10-21) -->
- **VW6 - §6b:** (narrowed 2026-10-21, re-view3dwin4: the `AddGUI` tail `0x007EED41`-`0x007EF1B6` and `+0x190`/`+0x194`/`+0x3D0` are now in §6b) still open: who creates `+0x2EC` and `+0x2F0` (no direct store in any `cSC4View3DWin` method; `RemoveGUI` releases them), (the object `0x00B43C9C` is the graphic system, re-view3dwin 2026-10-21: slot 3 `CreateBuffer` makes the `+0x3D0` surface, slot 12 `IsFullScreenMode` gates edge scrolling, §6c), and the per-panel extras of the five-panel loop between `0x007EE39B` and `0x007EE610` (flag `0x800` slots 23/24 calls). What settles it: a walk of `0x007EE39B`-`0x007EE610` and a search for indirect stores to `+0x2EC`/`+0x2F0`.
  **ANSWERED 2026-10-23 (re-view3dwin-vw5):** MATCHES — §6m: no owner creator for +0x2EC/+0x2F0; mayor/MySim extra children use transient 0x800 around SendToBack. VW-T26.

<!-- from: re-view3dwin (2026-10-21) -->
- **VW7 - §6i:** what message `0x470BD214` (data1 0 or 1, posted by two landscape items and the global terrain tool
  cases 5 and 6) asks for, and who handles it. [as read]
  **ANSWERED 2026-10-23 (re-view3dwin-vw5):** MATCHES — §6m: terrain handles a whole-map erosion request, data1 0 includes water erosion and 1 selects the alternate pass; both refresh. VW-T29.

<!-- from: re-view3dwin (2026-10-21) -->
- **VW8 - §6d, §4:** the objects at `+0x184`, `+0x188` and `+0x410` (stopped and released at `Shutdown`); no
  function read in this lane creates them. [as read]
  **ANSWERED 2026-10-23 (re-view3dwin-vw5):** MATCHES — §6m: all three fields are dormant zero-initialized references; no mandatory creator or active operation is required. VW-T26.

<!-- from: core-view3dwin5 (2026-10-23) -->
- **VW9 - §3 step 2, §6g:** Init registers 104 exact `(id, name, 0)` records, but §6g groups several commands
  without their full IDs/names (zone, network and place-lot commands), and the registry order beyond the first
  nine is absent. Please give the complete ordered 104-record table, including records deliberately unhandled
  here. An implementer can route typed commands literally but cannot reproduce exact registration or numeric
  dispatch from the current inventory. [spec gap]
  **ANSWERED 2026-10-23 (re-view3dwin-vw5):** MATCHES — §6g supplement: exact ordered 104-record ID/name table, every third word zero, including bus-stop and pipe command IDs. VW-T28.

<!-- from: core-view3dwin5 (2026-10-23) -->
- **VW12 - §3.1, §6k:** which hash/bucket traversal determines trigger pair order in the version-4 save record?
  What are the overflow/absent-retraction rules for trigger counts? The logical core uses wrapping uint32
  increment and absent retract as no-op, and preserves pairs without claiming original byte ordering. Also,
  does SizeOf's `RZAtofA` support a locale-specific decimal separator or hex floats? The provisional parser uses
  dot-decimal prefix semantics and NaN/Inf. [spec gap]
  **ANSWERED 2026-10-23 (re-view3dwin-vw5):** DIFFERS — §§3.1/6k supplement in §6l: unsigned wrap and absent retract no-op; modulo-current-bucket traversal, initial 193 buckets; bundled CRT locale decimal parser rejects hex floats, NaN and Inf. VW-T30–T31.

<!-- from: core-view3dwin5 (2026-10-23, round 2) -->
- **VW14 - §§6a–6b failure paths and overlay lifetime:** which camera/control/renderer/script-window/helper
  allocation or QI failures abort city/window initialization, and which skip dependent calls? The core currently
  keeps null fields and skips dependent configuration, with balanced successful references. Also, §6b releases
  the unstored full-size overlay creation reference immediately and omits its destruction from RemoveGUI, but
  requires no AddGUI-created window to survive RemoveGUI. Should RemoveGUI explicitly destroy the parent-owned
  overlay, or does that requirement apply after the city window's base teardown? [null-safe skips; retain only a
  borrowed stable overlay handle and destroy it at RemoveGUI without releasing that borrowed handle]
  **ANSWERED 2026-10-23 (re-view3dwin-vw5):** DIFFERS — §6m: required factory failures try concrete fallbacks, unchecked allocation/QI failures are fatal preconditions; explicitly guarded optional operations skip. Overlay is parent-owned through base teardown, not explicitly destroyed by RemoveGUI. VW-T27.

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
  **ANSWERED 2026-10-23 (re-view3dwin-vw5):** DIFFERS — §6i supplement in §6l: exact script/menu/catalog/background/state mapping, button ID caller data, dormant +0x2C0 pair and literal no-op/generic-catalog routes. VW-T32.

<!-- from: core-view3dwin5 (2026-10-23); moved by re-view3dwin-vw5 -->
- **VW10 - view3d.md §5.1 step 14, §5/§6j:** which paint-tail operations, guards, callback contracts and returns?
  **ANSWERED 2026-10-23 (re-view3dwin-vw5):** DIFFERS — §6n: the function ends at 0x007F6405;
  the tail drains at most one safe deferred message, reposts only when enabled, and returns true.
  No child walk or cursor/mission update; the old account crossed function boundaries. VW-T19.
- **VW11 - §2, §6f and view3d.md §5.1:** paint period default/unit/comparison and inverted zoom limits?
  **ANSWERED 2026-10-23 (re-view3dwin-vw5):** DIFFERS — §6f: 0.0f microseconds by default;
  nonzero period is due strictly after equality. Zoom uses minimum-first alternatives, not two sequential
  clamps; inverted limits are retained. VW-T20–VW-T21.
- **VW13 - §6j:** cursor priority/ties, fade alpha/rounding/hidden clear and years at/below start year?
  **ANSWERED 2026-10-23 (re-view3dwin-vw5):** DIFFERS — §6j supplement: descending unsigned priority,
  newest-first equal priorities, existing IDs stay in place; fade uses trunc(remaining × float32 0.17),
  white with retained alpha until next paint, including hidden clears. Nonpositive relative years pass through.
  VW-T22–VW-T24.
- **VW15 - §5 PreCityShutdown:** state/reference/subscription/painting effects of 0x007E9780?
  **ANSWERED 2026-10-23 (re-view3dwin-vw5):** MATCHES — §6a: clear deferred queue, remove all normal
  controls including current, then persistent controls; no GUI/render teardown or paint/subscription changes. VW-T25.
- **VW16 - §1.1 window slot 143:** GZOnCommand arguments, effects, return and relation to other dispatch routes?
  **ANSWERED 2026-10-23 (re-view3dwin-vw5):** MATCHES — §1.1: two ignored 32-bit arguments;
  returns false without effects or forwarding; distinct from the two message routes. VW-T25.

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
