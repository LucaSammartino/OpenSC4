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

<!-- from: core-overlay-views4 (2026-10-22) -->
- **NOV3 - §2, SC4TrainJunctionView missing draw bodies:** publish exact Init draw state, entry-direction
  selection, route coordinate space, last-three-points distance/chord rule, GetArrowBitmapQuad
  width/corners/winding/UVs/transform, chosen-route ordering and +0.01 origin, marker model choice/transform,
  train current/target matching, empty/degenerate routes, resource failure/ownership, bounds, setters and
  lifecycle. [Core acquires literal keys, builds midpoint unit directions and computes the literal
  last-three-point chord classifier and 0.8/0.5 transparency. Init, quad, junction draw and Update fail
  with TrainGeometryNov3; no injected same-owner transforms or placeholder success. No owner/runtime credit.]
