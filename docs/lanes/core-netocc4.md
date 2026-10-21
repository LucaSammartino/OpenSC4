| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-netocc4 (network occupants core: identity, §2 piece, §2.3 base record, §3 underground behaviour, power line) | `core-netocc4` / (engine lane) | ready at ce36e023: engine/core/src/network_occupants (identity 13/13; no owner class fully implemented: §2.3 record codec v5-v8 in stream order, live mapping open; underground/pipe/subway behaviour partial, records blocked NO17); no ABI, no glue, game=false; gate cargo 3601 passed / clippy 0 / ctest 62/62 PASS; questions NO9-NO17 | 1. Answers NO15-NO17, then the live<->record mapping and the v4 legacy transform. 2. §3.4 underground + pipe v3/subway v2 records. 3. Pole object §4. 4. Pylon §6. 5. Glue/shadow. |

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

## Round 2 (2026-10-21, ce36e023)

- `xform.rs`: the `S3DTransform` stream form from lot-records.md §2.1 (mask; matrix/translation/scale in that
  order). Mask kept as data, omitted parts left at identity (NO15). Legacy load (v<5) not implemented (NO15).
- `record.rs`: the §2.3 base record v8 in stream order: occupant base (reuses `crate::automata::OccupantBase`),
  xform, 4 quads + tail, `+0x100` sub-record (v2 only), triangles (type-column shape, NO16), bbox words, flags
  masked `& 0xFFFF1FFF`, `+0x15C`, v8-only ref (IID 0xA9667793) + i64. Read rejects v<4, fails v4 (NO15),
  reports `flags_need_v5_conversion` (v<6, NO16) and the tail as `LoadFixup` (NO16).
- `underground.rs`: §3.2/§3.3 GetSortValue, GetPosition, GetTerrainHeights, Init height store, SetVertexHeights
  stores, slot 104 model choice, IsImaginaryConnection (found case), pipe SetWateredState, pipe `+0x270`, the
  shared Shutdown. `NetworkPiece::new` now accepts Underground/Pipe/Subway (base ctor, §3.1).
- Blocked: §3.4 underground record and so the pipe/subway records, SetTerrainHeights, uv ratios, zipper type
  (NO17). Honest coverage: no owner class is fully implemented; do not count identity-only families.
- Measured: 7 new tests (tests_record.rs); qbuild all: 3601 passed, clippy 0, build 4/4, ctest 62/62.

