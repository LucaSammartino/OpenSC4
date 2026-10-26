# underground-occupants: answered questions (archive)

<!-- from: core-netocc-close5 round 2 (2026-10-23) -->
- **UG5 - connection owner closure, network section 3.6 and underground sections 1,4-9:** please
  publish the creation-path cell/geometry build and eight-neighbor decisions, callback `0x00653DA0`,
  SetupPackedRenderState values and ModelStateChanged/GetModelInstances/transform/bounds/message/view-dependent
  behavior. UG1 already leaves drawing/model building open. Are the seven record wall writes exactly the
  section 3.4 u32-count/24-byte helper? Which versions does connection Read accept, which IIDs do the two
  refs use, and how do adjusted serializable refs map to live +C4/+C8 versus released +C8/+CC? The core
  uses the documented wall helper, preserves the read version without an invented rejection, reads generic
  serializable refs and separates raw record fields from runtime references. Shutdown, subscriptions and
  hidden triangles follow the explicit contract. No original-owner result callbacks stand in for geometry.
  [missing/ambiguous; network section 3.6 and underground UG1/section 8 remain open; no owner closure]
  **Progress 2026-10-23 (re-netocc-answers5):** network §3.6.1 and underground §4/tests 9–13 specify exact
  record-version/ref adjustment rules, creation surface inputs and ordered eight-neighbor decisions,
  callback latch preservation, three packing tuples, linked-reference changes, model-state notification,
  model enumeration and position copies. Remaining: complete descriptor-dependent geometry, Draw,
  transform/view-dependent behavior, bounds and message decisions. Kept open for those owner requirements.
  **Progress 2026-10-23 (re-netocc-poles5):** network §3.6.2/tests 49–50 publish standalone unattached
  connection floor/wall generation, attached early exits and shared-A-drop wall UVs. The remaining
  descriptor-dependent geometry, Draw, transform/view behavior, bounds and messages are still open.
  Export address `0x00648360` is the pipe-interface query, not a transform getter; do not infer a
  transform contract from its stale decompiler label.
  **Progress 2026-10-24 (re-netocc-answers5d):** network §3.6.4/tests 60–61 and underground §4
  specify all four message decisions, query-reference lifetime, equal-pointer reattachment, the
  own-primary-pointer fallback, removal lower-height update, linked-removal teardown and context
  +4-word comparison. The model bounds wrapper delegates after −0x24C adjustment; `0x00654160`
  is attached Refresh, not GetWorldBounds. Geometry, Draw and generated transforms/bounds remain open.
  **ANSWERED 2026-10-26 (re-no29):** DIFFERS — underground §4 and network §§3.6.5–3.6.6 now give
  descriptor/build/deformation/UV rules, bounds, packed words and Draw passes with worked cases, in
  addition to the record/ref/creation/message rules already published. The exact matrix/attached-ceiling/
  template/shade input remainder is narrowed to runtime observation NO40-C, with capture sites,
  lengths and input variants. Full geometry-owner closure remains contingent on that observation;
  archiving this question does not credit an original-owner mesh or draw callback.
<!-- Archive for [underground-occupants.md](../../underground-occupants.md). Lanes do not read this file. Open questions are in ../underground-occupants.md. -->

<!-- from: re-underground (2026-10-19) -->
- ~~**UG1 - §0, §2.3:** the two base classes, `cSC4UndergroundOccupant` (`QueryInterface` `0x0065A080`, `Write`
  `0x0065B140`, `Read` `0x0065E510`, `Draw` `0x0065B290`) and `cSC4UndergroundConnectionOccupant` (`0x00654030` …):
  their save records, drawing and model building, and how they use the static model descriptors and the diagonal
  ratio tables. The pipe and subway records cannot be written without them.~~
  **ANSWERED 2026-10-21 (re-ugbase):** network-occupants.md §3.1–§3.6 now give both base classes: the underground
  occupant's fields, slots, `Init` forms, ceiling, ratio tables, neighbour rules, `Refresh` and draw passes (§3.1,
  §3.2), its record with every field named (§3.4), and the connection occupant's fields, record and behaviour (§3.6).
  The model builders' exact geometry is NO34.

<!-- from: re-underground (2026-10-19) -->
- ~~**UG2 - §2.2:** what network flag bits `0x02000000` and `0x04000000` (`+0x13C`) mean; either one makes an
  occupant's sort value its piece id instead of its class ID.~~
  **ANSWERED 2026-10-21 (re-ugbase):** they make the network occupant draw the first and the second triangle of its
  textured quad (`0x1000000` choosing the diagonal); `SetQuadDraw` sets both for surface pieces and the underground
  base sets them from the piece id and rotation (main slot 88). So the sort value is the piece id exactly when the
  surface quad is drawn (network-occupants.md §2.11, §3.2).

<!-- from: re-underground (2026-10-19) -->
- ~~**UG4 - §4:** the values `0x2A183167` (pipe connection) and `0x8A13918A` (subway connection) that `Init` stores at
  `+0x24` before the base `Init`.~~
  **ANSWERED 2026-10-21 (re-ugbase):** `+0x24` is the occupant type that occupant slot 7 `GetType` returns: the
  connection classes derive straight from `cSC4Occupant`, so each subclass's `Init` writes its own type there
  (network-occupants.md §3.6).
