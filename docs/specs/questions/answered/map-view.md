# map-view: answered questions

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
