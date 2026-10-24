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

<!-- from: glue-netrules-shadow5 Round 2 (2026-10-23) -->
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
