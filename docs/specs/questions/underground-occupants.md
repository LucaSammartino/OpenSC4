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
