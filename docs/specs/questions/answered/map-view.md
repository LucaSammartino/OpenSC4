# map-view: answered questions

<!-- from: core-map-view5b (2026-10-23) -->
- **QV14 - §5.2: ring intermediate precision:** which x87 precision-control and rounding-control modes
  apply to the ring helper, and at which operations (including square roots) are intermediates rounded?
  Does the stated formula require 24-, 53- or 64-bit significands between its explicit binary32 stores?
  Please add a near-threshold pixel vector that discriminates these modes. [literal formula implemented
  with the explicit binary32 stores and binary64 intermediates; §10.15-16 pass; this is a portable numeric
  choice, not a claim of native edge-pixel/weight equivalence for all coordinates; invalid domains remain QV10]

  **ANSWERED 2026-10-24 (re-map-view-qv18):** DIFFERS — §5.2 and vectors 45–48 specify inherited
  PC/RC, every arithmetic/square-root rounding point, explicit binary32 stores, and integer bias
  conversion. CRT PC53 is superseded by framework PC24; neither setter changes RC. Later contexts
  can change PC, so inheritance, not a permanent global PC53 or PC24, is the native contract.
  Nearest-mode vectors yield weights 254/255/255 and 254/254/255 under PC24/53/64, distinguishing
  all three choices. Corrected candidate bounds to floor lower / ceil upper. No new question split;
  the already bounded invalid geometric domains remain outside this ordinary finite contract.

<!-- from: re-mapview (2026-10-20) -->
- **QV3 - §1, §3.2:** the panels `0x8A2871B1`-`0x8A2871C4`, `0xAA32BCD4`/`0xAA32BCD5`; the groups' remembered views
  behind the buttons `0x5005`, `0x5008` and `0x8002`; the toggle `+0x9AC` of `0x8100`/`0x8101`; the command
  `0x6A935CF1`. [as read]

  **ANSWERED 2026-10-24 (re-map-view-qv18):** DIFFERS — §§1.2/3.2.1/3.3 and vectors 40–44 specify
  the exact visibility matrix, one-time first-show setup, independent requested-view memories,
  subgroup enable/exclusion, normal/alternate traffic counts, reset precedence and Cancel dispatch.
  Subgroup choices do not update the top-level memory, failed selections still update the requested
  memory, and resetting the alternate flag does not bypass same-view suppression. No question split.

<!-- from: re-mapview (2026-10-20) -->
- **QV2 - §4.1:** source 76 (traffic volume): which grid it draws for the kept travel types (`0x6B7C5CDD`) and how.
  [as read]

  **ANSWERED 2026-10-24 (re-map-view-qv18):** DIFFERS — §4.1 and vectors 36–39 specify fresh
  cell-sized volume/direction grids, travel-to-network eligibility, unsigned edge records, scale bias/cap,
  signed balance clamps, the view-0x54 ferry path with wrapping volume and per-point saturation,
  alternate counts, damaged-road sprite/marked cells, phase animation and final palette overrides.
  This is a constructed grid, not a direct simulator map; no new question split was needed.

<!-- from: re-mapview (2026-10-20) -->
- **QV1 - §2, §3.1:** what the terrain (slots 84, 86), zone manager (slots 17, 18) and `0x4A0B47E6` calls display.
  (The 3D view filter a data view installs is occupant-manager.md §13.2.8, re-occfilters 2026-10-21.) [as read]

  **ANSWERED 2026-10-24 (re-map-view-qv17):** DIFFERS — §§2/3.1 and vector 35 specify plumbing
  coverage and surface-water display flags, configured-default versus current zone opacity, reset
  behavior, setter refreshes and F1's filter-only override after E3's city/terrain selection.

<!-- from: core-map-view4 (2026-10-23) -->
- **QV11 - §4.1: developer layers:** which raw grid does the documented default layer 1 select? Are layers 3
  and 4 distinguished beyond both selecting slot 13, and what is the layout/geometry/iteration rule for the
  slot-11 entries used to construct the layer-6 debug grid multiplied by 12? [default/unknown layers refused;
  preserve selected layer 3/4 in the raw request; debug selects layer 6 but does not construct its grid]

  **ANSWERED 2026-10-24 (re-map-view-qv17):** DIFFERS — §4.1 and vector 34: layer 1/unknown selects
  no grid; 3/4 share one path. Layer 6 uses slot-13 geometry, zero fill, slot-10 availability and
  ordered signed-word triples from slot 11; last duplicate wins, value×12 wraps to signed 16 bits,
  and interpolation is disabled only during this draw.

<!-- from: core-map-view4 (2026-10-23) -->
- **QV5 - §4.2, §10: contradictory 16-bit conversions:** at scale 1024 the written signed factor is
  floor(0x7FFF00 / 1024) = 8191, and (-512 * 8191) arithmetic-shifted right 16 is -64, hence index 64;
  §10 instead requires 65. At scale 256 the written unsigned/signed factors are 65535/32767, whereas the prose
  says the default factors are 256 and unsigned values are divided by 256. Which formulas, default factors and
  negative rounding rule are authoritative? [both 16-bit conversions Unsupported; no replacement convention]

  **ANSWERED 2026-10-24 (re-map-view-qv17):** DIFFERS — §4.2 and vector 1: absent or out-of-range
  scale initializes both factors to 256; present valid scale uses the written quotients, including
  65535/32767 at explicit 256. Signed products use arithmetic shift (floor); scale 1024, −512 gives 64.

<!-- from: core-map-view-close12 (2026-10-24) -->
- **QV16 - §4.3 / §10.21 fixed-point centre:** for S=2, D=3 the prescribed
  q=trunc(131072/3)=43690 and j=1 samples 32767, not 32768. For x rows
  [0,1], [1,2], the two inner rounded values are 0 and 1, and the outer
  rounded value is 0. Vector 21 instead requires 2 using exact halfway samples.
  Which rule is authoritative? [literal §4.3 truncated-step formula implemented;
  vector's 2-to-3 expected centre remains unsettled; exact halfway two-stage
  rounding is tested separately using a 2-to-1 resample]

  **ANSWERED 2026-10-24 (re-map-view-qv17):** DIFFERS — §4.3 and vector 21: truncated integer
  step and integer half-step are authoritative. The 2-to-3 centre is 0; exact-half two-stage
  rounding gives 2 in a 2-to-1 reduction. The earlier 3×3 expected value was erroneous.

<!-- from: re-mapview (2026-10-20) -->
- **QV4 - §4.1, §4.3:** the rule of `BlockMapByLot` (`0x0079FEF0`) and its argument `0x4A0B47E8`, the crime incident
  markers (`0x0079E600`), and the developer grid of layer 5 (`0x007A2080`). [as read]

  **ANSWERED 2026-10-24 (re-map-view-qv11):** DIFFERS — §§4.1/4.3 specify lot-type mask selection,
  inclusive rounded byte means with minimum 1, ordered native-size incident icons with rotation/clipping
  and division-by-256 blending, and the signed population/built-capacity percentage grid; vectors 24–26.

<!-- from: core-map-view4 (2026-10-23) -->
- **QV6 - §2: raw property and resource failures:** except scale and desirability layer, what defaults apply to
  absent fields, and what happens with wrong property types, duplicate IDs, odd ramp lists, invalid terrain
  modes, travel-type lists longer than eight or entries outside a byte, resource-key overflow and allocation
  failures? [only documented scale/layer defaults and block-property presence used; other missing/malformed
  consumed fields explicitly Unsupported; raw lists retained without inventing ramp or travel conversions]

  **ANSWERED 2026-10-24 (re-map-view-qv11):** DIFFERS — §2.1 specifies per-field absence/retention,
  unchecked variant aliases and one holder lookup per ID, terrain-mode carry, byte count/entry narrowing,
  wrapped resource identity and nontransactional failure boundaries. Unsafe malformed resources are not
  successful empty views or owner validation failures; ramp details remain in §4.3; vectors 27–29.

<!-- from: core-map-view4 (2026-10-23) -->
- **QV9 - §1, §3.1, §7: state outside the specified prefix:** what are the constructor's initial current view,
  notification value and last-redraw clock, repeated Init/Hook behavior, tick clock width/wrap semantics, and
  the timestamp/dirty commit points after empty sources, failed/partial updates or filter-reset calls? Does
  draw-nothing clear or preserve an existing bitmap? [portable construction takes explicit clock/notification
  baselines and an unknown current view; no native defaults claimed; clock regression and repeated Init/Hook
  refused; none stops at QV1 reset boundary; partial redraw intents retain dirty and last-redraw state]

  **ANSWERED 2026-10-24 (re-map-view-qv11):** DIFFERS — §§1.1/7.1 specify initial none/current clock,
  comparison of message old/new records, latched Init and accumulating Hook calls, unsigned wrapping
  wall-clock milliseconds, active-entry dirty clear, and distinct missing-child/developer/empty-source
  timestamp/image commit points; vectors 30–33. Normally returning partial draws do not retain retry state.

<!-- from: core-map-view4 (2026-10-23) -->
- **QV7 - §5: highlight comparison and collection:** what fields, ordering, duplicates and radius equality does
  0x007A1390 compare? What radius data belongs to kinds 6-8 (dash rather than kind 9's explicit none), what is
  kind 0's behavior, and how are collection failures handled? [collect raw matching building records and the
  documented coverage radii for kinds 1-4; kinds 6-8 retain unspecified radius, kind 9 has none; no equality,
  deduplication, ordering or dirty-change decision claimed, including on repeated identical inputs]

  **ANSWERED 2026-10-24 (re-map-view-qv11):** DIFFERS — §5.0 specifies a pointer-keyed ordered multiset,
  retained equal non-null records without radius/position comparison, zero radius for kinds 6–9, empty
  kind-0 collection, and producer-result versus unsafe allocation-failure behavior; vectors 17–18.

<!-- from: core-map-view4 (2026-10-23) -->
- **QV8 - §2, §4.3: owned palette and resampling algorithms:** what are the ramp endpoint, duplicate-stop,
  ordering, interpolation and rounding rules of InterpolateRampToPalette, and the coarse-grid resampling
  algorithm/boundary rules of SC4InterpolateGrid? [nonzero SetDataView stops at ramp generation; raw supplied
  256-entry palettes exercise lookup/emission helpers only; interpolation Unsupported, not delegated]

  **ANSWERED 2026-10-24 (re-map-view-qv11):** DIFFERS — §4.3 specifies supplied-order stops, endpoint
  extension, duplicate advancement, 8-bit fixed-point channel weights, division-by-256 premultiplication
  and centre-aligned, edge-clamped 16-bit fixed-point separable grid resampling; vectors 19–21.

<!-- from: core-map-view4 (2026-10-23) -->
- **QV10 - §4.2-§4.3: out-of-domain floats and overlay cells:** what happens for nonfinite or truncated float
  indices outside 0-255, which colours count as coloured cells, and what origin/extent/depth transformation
  maps the grid to marked cells? [refuse unsupported float indices; overlay helper plans only explicitly
  supplied coloured cells with complete raw geometry, without inferring inclusion, depth or transforms]

  **ANSWERED 2026-10-24 (re-map-view-qv11):** DIFFERS — §4.2.1 specifies signed-qword truncation and low
  byte wrap, with zero on masked invalid conversion. §4.3 specifies straight-palette alpha inclusion,
  grid coordinates and world cell sizes, and the marked-cell terrain/water/depth contract; vectors 22–23.

<!-- from: core-map-view5 (2026-10-23) -->
- **QV12 - §§1-3, §6-7: interface execution and summary edges:** what rollback/return behavior applies when
  a foreign window/renderer/simulator primitive fails, what initialization/attachment follows creation of the
  custom-view class with IID 0x8A175D6B, and does ResetDataView bypass SetDataView's same-view guard? Is the
  legend title F4 a singleton u32 list as §2 says, and what happens for unequal color/string list lengths?
  Must button handling continue with legend/panels after selection failure? [literal portable execution
  stops on foreign failure, retains any completed prefix and reports an opaque adapter error; no native
  failure code or rollback claimed. Only successfully created custom handles are shut down/released.
  Reset calls SetDataView normally; F4 requires one list element and unequal rows are Unsupported.
  Buttons stop on selection failure. Native custom attachment remains open; render settings pass only
  the semantic filter/terrain/zone/lot intents, without claiming unknown native reset calls (QV1).
  For QV8, only full-range strictly increasing ramps with exact integer linear channel results are built;
  fractional interpolation, unordered/duplicate stops and endpoint extension remain Unsupported.]

  **ANSWERED 2026-10-23 (re-map-view-qv5):** DIFFERS — [map-view.md](../../map-view.md) §§1–3 and 6,
  vectors 5–10: failure is not transactional and normally returning primitive results are selectively
  ignored. Missing exemplars return false after teardown, retaining the old identity. Custom creation
  success calls no-argument slot 3; no attachment call; false initialization does not undo selection.
  Failed creation/child QI does not abort. Reset uses the ordinary same-view guard (MATCHES that part).
  Buttons continue with requested legend/panels after selection failure. F4 is scalar u32 with default
  0x2A5E854F, not a list. Colours drive rows; zero skips a row, extra strings are ignored, and index equal
  to string count is an unchecked access, not a defined empty-label policy. Invalid required pointers are
  not recoverable native false returns. Separate QV1/QV6–QV8 domains remain open.

<!-- from: core-map-view5 (2026-10-23) -->
- **QV13 - §5 crime hotspot and effects:** what raw hotspot data is obtained from crime slot 8 and which
  effects-manager primitives/parameters render or remove it? How do the stored plop name and two highlight
  colors affect hotspot/building rendering? [kind 5 remains Unsupported::CrimeHotspot; no guessed effect
  geometry/style/lifecycle. Other kinds collect the specified raw buildings/radii but comparison remains
  QV7. Plop name and highlight colors are retained as raw exemplar properties, without invented effects.]

  **ANSWERED 2026-10-23 (re-map-view-qv5):** DIFFERS — [map-view.md](../../map-view.md) §§5.1–5.2,
  vectors 11–16: crime slot 8 returns byte strength and signed cell x/z; strength is ignored by map rendering.
  Hotspot record has null occupant, radius 100 and centred-cell world position with terrain altitude.
  EA names an effect resource: manager slot 7 creates; effect slots 8/3 transform/start(false), slot 4
  stops(false). Each successful poll attempts a temporary scale-100 effect and a retained scale-200 effect;
  repeated hotspots stop/replace the old retained effect and mark changed even at identical coordinates.
  Creation failures preserve the documented candidate behavior. EF blends bitmap building footprints;
  EE draws bitmap coverage rings, including hotspots, and ignores its alpha byte. Neither changes the
  named effect. Geometry, blend arithmetic, ordering and failure boundaries are specified; general
  building comparison remains QV7 and invalid numeric domains remain QV10.

<!-- from: drawshadow29 (2026-10-26) -->
- **DS29-1 - Same-invocation MapView snapshot/interface binding:** reach selected this owner
  (house: 5,574 reached bytes; gen27: 15,665, including 24 Update entries; other candidates: zero).
  For Update `0x007A2F60` and both DoMessage entries `0x007A54D0`/`0x007A56E0`, please publish
  the complete native-state field offsets/types and subobject adjustments needed to initialize
  the Rust owner and compare every changed field: selection/source/layer, dirty and last clock,
  palettes, highlight/traffic state, retained collaborators and ordered pending view requests.
  The spec publishes some individual offsets, but docs/abi/layouts.md has no MapView record.
  Also identify the bitmap surface/presentation interface boundary for Update's raster output;
  section 4 describes bitmap drawing rather than calls on a widget's cGZDrawContext.
  No pilot comparison or complete method credit is claimed pending this binding; native drawing
  continues unchanged. This requests interface facts, not original implementation code.

  **ANSWERED 2026-10-26 (re-ds29):** DIFFERS — [map-view.md](../../map-view.md) §1.3 publishes
  the 0x9E8-byte owner layout, 0/−4/−8 entry adjustments (factory interface −0xE0), live palettes,
  highlight/string/vector/list records, retained collaborators and same-invocation comparison rules.
  +0x9CC stores an exemplar instance; pending requests are inserted at the front and preserve duplicates.
  §§4.4–4.5 and vector 49 define the buffer creation/lock/pixel-write sequence and presentation through
  cSC4WinMiniMap IID 0xCA318385 slot 4 SetOverlay, followed by child slot 9 Update only for source 76.
  SetOverlay retains the buffer and dirties the child; later minimap composition/paint is a separate
  boundary. The worked layer-change case gives exact owner deltas and four pixel words. No pilot
  execution, complete-method credit or UI verification is claimed by this static binding answer.
