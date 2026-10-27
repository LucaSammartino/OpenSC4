# Dispatch vehicle TrackInfo

Portable owned record for `cSC4DispatchVehicleView::TrackInfo`. Foreign tokens
carry no implicit native AddRef/Release. The view transfers the scratch record
into stable kind order; the unique processor is never accidentally cloned.

| Spec section | Code item | Worked-case test / boundary |
|---|---|---|
| dispatch-vehicle-view §1.2 | `TrackInfo`, `Vertex`, `Rect`, `Icon`, `flags` | `worked_kinds_and_track_defaults` |
| §2 | `TrackInfo::for_tracking` | `worked_kinds_and_track_defaults` (§8 case 1) |
| §3.12 | owned string/quads; non-Clone processor ownership | move into view; lifecycle test in view module |
