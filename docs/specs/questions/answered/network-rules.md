# network-rules: answered questions (archive)

<!-- from: glue-netrules-apply4 (2026-10-21) -->
- **NR15:** §11.2 (NR14) specifies slot 30 `SetPiece` for the ordinary (slot 54, class 0) occupant of the surface
  types. Do the underground types, Pipe (4) and Subway (7), create their occupants through the same class and apply
  their solved pieces through the same slot 30 before `InsertOccupant`, or through another path (another class or
  vtable, a different flag sequence)? Until answered, our tool does not apply solved pieces to Pipe/Subway occupants
  (logged as `unsupported`).
  **ANSWERED 2026-10-22 (re-netrules-underground4):** DIFFERS — network-rules.md §11.3 and tests 32–35.
  Pipe and Subway create `cSC4PipeOccupant` (vtable `0x00AA97B8`) and `cSC4SubwayOccupant`
  (`0x00AAA3C8`) through derived slot 54/82 factories; their slot 30 target is `0x0065ADB0`.
  It copies a u32 id and low-byte orientation and recomputes underground draw height, without the surface target's
  flag/path-cache/texture/alpha work. Native caller/target bytes and executable vtable words prove application
  before insertion through the common commit path (`0x0062797F` before `0x00627A38`) and underground fallback
  (`0x00659747` before `0x00659ABC`). The second pass reuses found occupants without reapplying their piece or
  reinserting them; it releases both found and new local results. No underground game run or replacement credit
  is claimed by this answer.

<!-- Archive for [network-rules.md](../../network-rules.md). Lanes do not read this file. Open questions are in ../network-rules.md. -->

<!-- from: From the network rules spec (2026-10-16) -->
- ~~**NR1:** the texture an ordinary piece id selects, and how the orientation byte turns it (network-rules.md §11).
  [not specified]~~ **ANSWERED 2026-10-16 (re-answers24), simulation part:** the piece id (or an intersection
  piece's path override) also selects the occupant's **path info**: `GetPathInfo` (network occupant slot 49,
  `0x0061B510`) maps it with `MapPath`, loads the path resource `{0x296678F7, 0x69668828 or 0xA966883F, id}` and
  makes the occupant's transformed copy, which is where the orientation turns the paths (network-rules.md §11.1). The
  texture an ordinary piece id selects is drawing only and is left to the renderer specs. The path-info internals are
  filed as NR6.

<!-- from: From the network rules spec (2026-10-16) -->
- ~~**NR2:** the placement of intersection models (`InsertHighwayIntersectionPiece(s)`, `0x006276D0`, `0x0062C4B0`),
  the rule's tile grid and map, `AutoTileBase`, `AutoPathBase` and the constraint layout's consumer (§10.3).
  [not specified]~~ **ANSWERED 2026-10-16 (re-answers24):** specified in network-rules.md §10.3 (the tile codes, a
  byte `row × 16 + col + 1`, and the map values `col × 0x100 + row × 0x10`) and §10.9 (each occupied cell gets piece
  `AutoTileBase + tile`, path override `AutoPathBase + tile` when `AutoPathBase` is set, the prebuilt-model class,
  group `0xA8434037`, the flags and the anchor). The constraint layout's consumer is `InsertIntersectionConstraints`
  (`0x00644710`) in the smoothing loop, height geometry; the codes' meaning is filed as NR7.

<!-- from: From the network rules spec (2026-10-16) -->
- ~~**NR3:** the exact `AddTypes` ring links (§10.8). [inferred: a second circular list through `+0xF4`/`+0xF8`]~~
  **ANSWERED 2026-10-16 (re-answers24):** the shape MATCHES: `+0xF4`/`+0xF8` form a second circular list of rule
  ids (the type ring). The spec's premise that `AddTypes` is read only without a `RotationRing` **DIFFERS**: every
  line is tested on its own (`RotationRing` first, then `AddTypes`, case-insensitively), so both may share a section.
  An `AddTypes` line forms its own rotation ring, and its k-th known rule enters the type ring of the k-th rule of
  the previous `RotationRing`/`AddTypes` line of the file, right after it (a file-wide cursor). network-rules.md
  §10.8, test 27.

<!-- from: From the network rules spec (2026-10-16) -->
- ~~**NR4:** `ResolveOneWayConflicts` and the preview's `SolveMultiTypeSeparation` (`0x00633FA0`). [not
  specified]~~ **ANSWERED 2026-10-16 (re-answers24):** network-rules.md §6.5 (the flow codes of `+0x50`, the codes
  `AssignEdgeMajorTypes` writes for the one-way tool, the propagation from the run's first and last steps into the
  existing one-way roads, and the per-cell conflict sweep, with a quirk: the sweep queues the current key instead of
  the neighbour's) and §8.3 (`SolveMultiTypeSeparation` returns the first evaluator's self-targeting output for one
  cell; it only draws and changes no state). Tests 26 and 29.

<!-- from: From the network rules spec (2026-10-16) -->
- ~~**NR5:** the order of several `[RuleOverrides]` patterns with the same first id (§9.1). [file order,
  inferred]~~ **ANSWERED 2026-10-16 (re-answers24):** file order MATCHES. `cRZIniFile::EnumEntries` (`0x0091D91C`)
  keeps every line, repeated keys included, in file order; the multimap insertion (`0x0063C920`) puts an equal `idA`
  after the existing entries; `PatchTilePair` walks the equal range forward, so the first matching line wins.
  network-rules.md §9.1, test 25.

<!-- from: From the enable=/spec-contradiction wave (filed by the coordinator from the implementer lanes' reports, 2026-10-16; answered by re-answers31) -->
- ~~**NR8** (`network-rules.md` §6.5 step 2.4): the step says only "skip `s`", but test 26 needs the east flow
  code dropped. Does a skipped side lose its flow code, on all three skip reasons or only missing-neighbour?~~
  **ANSWERED 2026-10-16 (re-answers31): DIFFERS.** A skipped side **keeps** its code, for all three reasons.
  The propagation only reads the code field; the sole zeroing is the conflict sweep of step 3
  (`decomp/cSC4NetworkTool.c:11343`). Test 26's X east drop is that sweep (edge word `0x00000202` maps sides 0
  and 2 both to partner side 2 with contradictory flows). Drop the `[choice]` in `solve.rs`. Written into
  [network-rules.md](../../network-rules.md) §6.5 step 2.4.

<!-- from: From the enable=/spec-contradiction wave (filed by the coordinator from the implementer lanes' reports, 2026-10-16; answered by re-answers31) -->
- ~~**NR12** (cores22): `fc4_network_rules_cell` carries neither `+0x57` (the one-way pass's re-solve mark) nor
  the occupant's paired portal (interface `0x8A4BD52B` slot 59), so a cell that crosses the C boundary loses
  both. Should the cell struct grow those fields, or should a glue lane pass them another way?~~
  **ANSWERED 2026-10-16 (re-answers31): neither belongs in the cell struct.** `+0x57` is per-pass scratch on the
  world-cache cell info (set in §6.5 step 2.1 and again in the sweep at `decomp/cSC4NetworkTool.c:11360`, never
  saved, read by nothing outside `ResolveOneWayConflicts`); the paired portal is a property of the *occupant*,
  reached only when the cell flags carry `0x80000`, through `0x8A4BD52B` slot 59 on the occupant's `+0x1D0`.
  Use a marks side-channel for the former and resolve the latter on demand. Written into
  [network-rules.md](../../network-rules.md) §14.

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
  **ANSWERED 2026-10-21 (re-netrules-boundary4):** MATCHES — no boundary of its own: the evaluator is a 0x10-byte plain object (one-slot vtable 0x00AA8338, deleting destructor only, no RTTI) made only by LoadEvaluator; the rule is a stack struct, cSC4TileInfo free functions, cIntRule an image-global map; every caller is a direct call in the network tool or the puzzle-piece input control, and no image pointer refers to them. Hosts: the rule resources (data) or the whole network tool (reachable only via a replacement network manager 0xC990BD46). Wire network_rules as a component of our own network tool, no separate enable key. Written into [network-rules.md](../../network-rules.md) §2.4, §12, §14.

<!-- from: glue-netrules-impl4 (2026-10-21) -->
- **NR14:** network-rules.md §11 (from a solved cell to an occupant) says the solved piece ID and orientation become the
  occupant's piece and rotation/flip, but no public contract (docs/abi, SDK `cISC4NetworkOccupant`) names a slot that
  writes a piece ID or orientation onto an occupant the game created (`0xC9C05C6E`); only readbacks (`PieceId()`,
  `GetRotationAndFlip()`) are public. Which interface/slot applies a solved `(piece id, rotation, flip)` to an occupant,
  and when relative to insertion? Evidence (run nrimpl4-a1): our solver chose road (4,4)..(14,4) end piece `0x200`/03,
  straight `0x2600`/03 (street `0x05000200`/`0x05002600`); occupants read back 0/0. That run also crashed (AV) after
  insert and at Shutdown; cause was our own context-layout bug (stale object), not a write. Current choice: the solver
  result is logged only, never applied; no replacement credit. [not specified]
  **ANSWERED 2026-10-21 (re-netrules-apply4):** MATCHES — network occupant slot 30 (+0x78, 0x0061B490, docs/abi name SetQuadDraw) is SetPiece(u32 id, orientation byte zero-extended), thiscall ret 8: unless flag 0x200000, set 0x2000000 and 0x4000000; ReleasePathInfo (slot 50); clear 0x2000; +0xFC := id, +0x104 := orientation; SetTextureCoordinates; vertex alphas 0xFF; no message. InsertNetworkOccupant (slot 72) calls it on the occupant slot 54 just created (0x0062797F) with the 12-byte solved record {+0 id, +4 orientation, +8 key}, before AddSupportStructures, ClearNetworkOccsFromCell and occupantManager.InsertOccupant (0x00627A38); slot 54 never writes the fields (fresh occupant reads 0/0). Written into [network-rules.md](../../network-rules.md) §11, §11.2, §15 tests 30-31.

<!-- from: glue-netrules-shadow5 (2026-10-23), question commit 4965eb6a; archived by re-netrules-nr16 before that lane lands -->
- ~~**NR16 - sections 2.4 and 5, native record-only shadow boundary:** please publish the GOG 1.1.641
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
  guess private container layouts, native signatures, or patch boundaries]~~
  **ANSWERED 2026-10-23 (re-netrules-nr16):** DIFFERS — Evaluate returns AL boolean and a borrowed output-vector
  descriptor through its second argument; matched-empty succeeds. §2.5 specifies tree/header/node offsets,
  unsigned and equal-key order, 0x28-byte neighbourhood and accessors, global cIntRule map, instruction-boundary
  observation spans, build checks and synchronous lifetime restrictions. The loaded byte is set before loading,
  so completion must be observed separately. Getter prefetch can populate tool caches; capture only actual native
  queries and replay copied data, reporting absent inputs separately. Written into
  [network-rules.md](../../network-rules.md) §2.5 and §15 vectors 36–44. ABI publication proposals are named in
  [the lane report](../../../lanes/re-netrules-nr16.md); shared docs/abi promotion is for the coordinator.

<!-- from: glue-netrules-shadow5 Round 2 (2026-10-23); archived by re-netrules-nr17 before that lane's filing lands -->
- **NR17 - section 2.5.5, fail-closed observer installation and ABI publication:** NR16 now specifies the
  semantic capture layouts, Evaluate's five-byte span and ReloadRules' six-byte span. Before installing the
  required native observer, please publish the exact expected observation-site byte sequences (including any
  relocation handling), or an approved method for comparing loaded sites with the fingerprinted image. The
  section requires exact site checks but provides no byte reference; existing FC4Compat Identify/CheckCode
  checks PE identity and executable-page membership, not these instruction bytes. Engine authors cannot inspect
  the executable to supply the missing reference. Please also specify a safe observation boundary for static
  cleanup at 0x006232F0 (whole-instruction span/continuation and validation reference, or a supported callback
  interception), so the required readiness invalidation does not depend on guessing its patch boundary. Publish
  the native interoperability facts in docs/abi as required by CONTRIBUTING.md; section 2.5.5 currently says
  that publication is still proposed for the coordinator. [blocked: no hook installed, no netrules_shadow
  exactness claim; retain NR16's capture semantics and do not infer native instructions]
  **ANSWERED 2026-10-23 (re-netrules-nr17):** DIFFERS — exact entry bytes, whole-instruction 5/6/5-byte spans and
  continuations are published in [the observation ABI](../../../abi/network-rules-observation.md), together with
  NR16's native call/layout contracts. The fingerprinted image has no base relocations and requires loaded base
  0x00400000; cleanup's absolute flag operand remains unchanged in a trampoline. All loaded bytes must match,
  without masking. Spec §2.5.6 describes fail-closed validation and cleanup's quirk: it resets a nonzero flag,
  always returns AL=1, and leaves tables allocated; readiness is invalidated on entry even for flag zero.
  Written into [network-rules.md](../../network-rules.md) §2.5.6 and §15 vectors 45–49. No hook/game result is claimed.
