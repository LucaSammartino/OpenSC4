| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-netocc4 (network occupants core: identity, §2 piece behaviour, power line) | `core-netocc4` / (engine lane) | ready at dbc452ef: engine/core/src/network_occupants (identity 13/13 classes; behaviour for 5 of 13 owner classes, partial; 1 more partial for control vars only); no ABI, no glue, game=false; gate cargo 3595 passed / clippy 0 / ctest 62/62 PASS; questions NO9-NO14 filed | 1. §2.3 base record (needs an S3DTransform codec and the lot-records base). 2. Underground family §3. 3. Pole object §4.1-§4.4. 4. Pylon §6. 5. Glue/shadow once a record exists. |

## Report

**What changed** (engine/core only; no capi, plugin, CMake or FC4Subsystems edits; ABI unchanged):

- `engine/core/src/network_occupants/identity.rs`: §1 for all thirteen classes of §1.1 — class IDs,
  `GetGZCLSID` (none for the abstract underground base), occupant types (pipe/subway connection only after
  `Init`), `QueryInterface` chains per §1.2 (sub-object offsets, property-holder/model fallthrough, the pole
  delegate, the pole/pylon null-`this` quirk), `IsOfType` as a flag test, `IsManagedGlobally`.
- `piece.rs`: `NetworkPiece` for `cSC4NetworkOccupant`, `…WithPreBuiltModel`, `…BridgeOccupant`,
  `…TunnelOccupant` (§2.2 ctor flags, §2.5 `Init`/`Shutdown`, §2.7 `SetNetworkFlag`/`ClearNetworkFlag` with the
  usable message and slot 58, §2.6 `AddPathConnection`/`PathConnection` with wildcards, §2.4 orientation/flip/
  rotation/variation, `GetOccupiedCell` with the float32 magic-number rounding, demolition cost). A `PieceHost`
  trait carries the calls into the game. Not implemented: the §2.3 save record, geometry, triangles, bbox,
  lighting, drawing, path info, the bridge structure list and the tunnel portal pairing.
- `power_line.rs`: `cSC4PowerLineOccupant` slots 3-7, `SetPosition`, degenerate bbox, `Shutdown`, and the v2
  record codec as a literal field list (the live-object ↔ record mapping is NO13).
- `power_pole.rs`: §4.3 `InitPowerLineControlVariables` only (six properties, defaults, exemplar key). The pole
  object itself is not implemented.

**Owner rows** (do not count as whole-family coverage): identity tables cover all 13; behaviour implemented for
`cSC4NetworkOccupant`, `cSC4NetworkOccupantWithPreBuiltModel`, `cSC4NetworkBridgeOccupant`,
`cSC4NetworkTunnelOccupant` (each partial: no record) and `cSC4PowerLineOccupant` (slots + record, mapping open);
`cSC4PowerPoleOccupant` control variables only. Not implemented: underground, pipe, subway, the three connection
classes, pylon, §8 type info, §9 routines, §10 model instance.

**Measured:** 17 new unit tests (conformance 1, 2, 3, 5, 6, 7, 8, 9, 10, 18 + record round trip). Gate
`qbuild all`: cargo test 3595 passed 0 failed, clippy 0 diagnostics, fc4verify build 180/180, ctest 62/62.
No game runs (game=false; no glue).

**Questions filed** (docs/specs/questions/network-occupants.md): NO9 underground `IsManagedGlobally`; NO10 pylon
QI IIDs; NO11 tunnel ctor `~0x400` vs prebuilt bit `0x400000`; NO12 path table shape and wildcard count; NO13
power line offsets/record/version/rounding; NO14 conformance 6 slot 24 vs 25. Numbered from QUESTIONS.md's next
free (NO9); note §12 also uses NO9/NO10 labels for two settled notes.
