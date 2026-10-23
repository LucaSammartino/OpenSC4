# network-overlay-views: open questions
<!-- Questions about [network-overlay-views.md](../network-overlay-views.md). Append new entries at the END; when one is answered, move it to answered/network-overlay-views.md. Rules: README.md. -->

<!-- from: re-netoverlays (2026-10-20) -->
- **NOV1 - §1, §2:** which cheat code makes the automata manager create the path-info view, what the path key's
  bytes and type nibble denote in traffic-network.md's terms, and which turn each arrow-picture pair shows.
  [as read]

<!-- from: core-overlay-views4 (2026-10-22) -->
- **NOV2 - §1, SC4PathInfoView exact geometry/state:** publish numeric light-green/light-red RGBA,
  red-to-white pulse waveform/channel mapping, red-cross axes/alpha/altitude, line vertex format,
  other packed-state fields, path/cell/occupant order, coordinate space, and whether arrows/bars use
  capped 25 points or the full path. Are arrow lengths/directions in x-z or 3D? How are repeated/tiny
  first/end segments treated? Publish setters and lifecycle/defaults. [Core retains opaque key bytes,
  known blue/yellow/magenta with alpha 200, caps to 25, searches backward for last x-z segment >1mm,
  omits zero-segment bars, traverses rows with x varying first. Light/pulse colours fail with
  PathInfoColourNov2; native line format stays None. Cross provisional reading: x/z axes ±4m, alpha 200,
  same +0.5m height. No complete-owner credit.]

  **Native evidence 2026-10-22 (re-overlay-views-contracts4):** §§1.1–1.2 prove exact colours/pulse
  update, format/state/defaults, full-vector 3D arrow/bar rules, ±2m opaque cross, x-outer traversal,
  empty/repeated/nonfinite and scratch-alpha boundaries. Residual: pulse static initialization,
  actual public filter/depth setters and shared cleanup/refcount lifecycle. The constructor does not
  reset the process-wide pulse; no initial-draw pulse value is certified yet.
  **Qualification 2026-10-23 (re-overlay-views-contracts4):** overlay-views §3.4 proves shared
  QI/AddRef/Release and path-info deleting cleanup individually. NOV2 remains partial for the static
  pulse initializer and actual public filter/depth setters; geometry vectors remain UNEXECUTED.

<!-- from: core-overlay-views4 (2026-10-22) -->
- **NOV3 - §2, SC4TrainJunctionView missing draw bodies:** publish exact Init draw state, entry-direction
  selection, route coordinate space, last-three-points distance/chord rule, GetArrowBitmapQuad
  width/corners/winding/UVs/transform, chosen-route ordering and +0.01 origin, marker model choice/transform,
  train current/target matching, empty/degenerate routes, resource failure/ownership, bounds, setters and
  lifecycle. [Core acquires literal keys, builds midpoint unit directions and computes the literal
  last-three-point chord classifier and 0.8/0.5 transparency. Init, quad, junction draw and Update fail
  with TrainGeometryNov3; no injected same-owner transforms or placeholder success. No owner/runtime credit.]

  **Native evidence 2026-10-22 (re-overlay-views-contracts4):** §2.1 proves packed Init state,
  full-path horizontal chord classifier, exact quad/order/UV/height, .7/.4 plain versus .8/.5 selected,
  frame-wide .1+.01 stacking, first-route origin/direction, model choice, resource releases and failure
  boundaries. Residual: marker rotation-helper handedness/model-state details, actual setters and
  identification of the stored cell pairs as train current versus target, destruction/refcount integration.
  **Qualification 2026-10-23 (re-overlay-views-contracts4):** overlay-views §3.4 proves shared
  refcount-to-deletion dispatch and the train deleting body (models +0x48 then +0x44, base/free,
  without Shutdown or six-binding cleanup). NOV3 remains partial for public cell setters/identities,
  marker rotation/model state and caller Shutdown discipline. No complete-owner promotion follows.
