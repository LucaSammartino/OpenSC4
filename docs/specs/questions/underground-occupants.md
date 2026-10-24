# underground-occupants: open questions
<!-- Questions about [underground-occupants.md](../underground-occupants.md). Append new entries at the END; when one is answered, move it to answered/underground-occupants.md. Rules: README.md. -->

<!-- from: re-underground (2026-10-19) -->
- **UG3 - §3.3:** which subway pieces are `0x300` and `0x4B00` (their tunnel floor comes from the second vertex
  list), and what the zipper piece type (main slot 99) being 0 means for them.

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
