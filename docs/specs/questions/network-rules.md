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

## From the glue-netrules4 lane (2026-10-21)

<!-- from: glue-netrules4 (2026-10-21) -->
- **NR13:** interoperability boundary for the rule solver. `cSC4TileEvaluator`, `cSC4TileRule`, `cSC4TileInfo` and
  `nSC4Networks` (§6, §9, §10) have no rows in docs/abi (no CLSID, IID, vtable or factory), and the spec names
  them only as internals of `cSC4NetworkTool::SolveNetwork`. Is there any game-reachable, supported boundary (a GZCOM
  class/interface the game creates through the COM director, a resource-loader/exemplar hook, or a message) through
  which the original game would call a replacement of these owners? If not, should the landed Rust
  `network_rules` (capi `fc4_network_rules_*`) be wired only as a component of our own network tool
  (engine/plugin/FC4NetToolHost.cpp `InsertOccupantsForSolvedCells`, today piece id 0, active under
  `enable = netmgr`), with no separate `enable = netrules` key? [blocks: glue for nr owners; no interception invented]
