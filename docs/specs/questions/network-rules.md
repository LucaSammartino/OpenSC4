# network-rules: open questions
<!-- Questions about [network-rules.md](../network-rules.md). Append new entries at the END; when one is answered, move it to answered/network-rules.md. Rules: README.md. -->

## From the network rules spec (re-specs17b lane, 2026-10-16)

Spec: [network-rules.md](network-rules.md) §14. RE-side follow-ups; nothing here blocks an implementer of §2–§10.

<!-- from: From the network rules spec (2026-10-16) -->
- **NR6:** the path resource (`{0x296678F7, 0x69668828 or 0xA966883F, MapPath(id)}`, interface `0xA9667793`): its
  file format, and what path-info slots 25 `CreateTransformedCopy`, 26 `RemoveWrongWayPaths`, 27
  `InitializeNetworkConnectionBitmap`, 29 `OffsetAndScissor` and 40 `VerifyPathEdges` do; also who writes the
  override offset `+0x168`/`+0x16C` that slot 29 receives (network-rules.md §11.1). [not specified; the traffic
  rebuild borrows the original's path infos]

<!-- from: From the network rules spec (2026-10-16) -->
- **NR7:** the constraint layout's codes 1–8 as `InsertIntersectionConstraints` (`0x00644710`) applies them: which
  vertex heights each code makes equal and which slope limits it sets (network-rules.md §10.3; network-geometry.md
  §9.4 step 4). [not specified]

<!-- from: glue-netrules-shadow5 (2026-10-23) -->
- **NR16 - sections 2.4 and 5, native record-only shadow boundary:** please publish the GOG 1.1.641
  interoperability contracts needed to compare our parsed rules with the original's live tables and replay
  original tile evaluations without replacing the solver. Section 2.4 establishes that Evaluate is a direct
  thiscall with two arguments and no GZCOM interception point, but does not specify its return channel/type,
  the neighbourhood's concrete layout or callable accessors, or the output-list representation and ownership.
  Please specify those contracts, including how to distinguish no match from a matched empty-output rule;
  the evaluator tree's node/header links, payload offsets, condition/output element layouts, and traversal
  order (including equal keys); and the cIntRule map bucket/node/value layouts if those tables are included.
  Also specify a version-checked observation or replay entry point with the necessary lifetime/threading
  rules, and how the shadow knows that rule loading has completed: section 2.1 loads lazily at original tool
  Init, not necessarily at city load. Any native addresses/layouts used by engine glue need interface facts
  in docs/abi per CONTRIBUTING.md. [blocked: no netrules_shadow implementation or exactness claim; do not
  guess private container layouts, native signatures, or patch boundaries]
