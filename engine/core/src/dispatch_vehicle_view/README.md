# Dispatch vehicle view

Clean-room portable `cSC4DispatchVehicleView`. No C ABI or plugin wiring.
`TrackInfo` is a separate owner module. Foreign identities are opaque tokens;
resource-handle identity remains separate from the resolved texture identity.
Exclusive mutation and immutable getters use no interior mutable state.

| Spec section | Code item | Worked-case test / boundary |
|---|---|---|
| dispatch-vehicle-view §1.1–1.2 | `DispatchVehicleView`, stable low-byte `insert` | `low_byte_map_is_stable_for_equal_keys` |
