| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gdriver-rd13 (RD6/RD7/RD12 contracts) | Codex / `re-gdriver-rd13` | ready; doccheck passed; docs only, ABI unchanged | Land; core lane can implement the three closed contracts |

## Report

Closed RD6, RD7 and RD12 in renderer-driver.md, moved their entries to the answered
question file and regenerated the question index. RD12 was imported directly from the
committed core-gdriver-close12 question file. Its owner closure table was read first;
all three open owner obligations now have rules and independent vectors.

- §9.6: ordinary source/key channel layouts, truncation and bit replication; DXT1/DXT3
  interpolation in destination precision, upward midpoints, independent DXT3 alpha,
  retained endpoint-order branch and whole-block/stride behavior.
- §12.6: CopyColorBuffer lifetime probe cache, factory/borrowed ownership, every return
  class, clipping and black-fill quirk, 16-bit division and unusual 24-bit byte ordering.
- §14.2 with lifecycle/region cross-references: primary, target and depth bindings,
  AddRef versus owned GetRenderTarget, null type-2 slot, loss and restoration order.
- Added 13 prose conformance vectors, DX7-50–62. These are specifications, not executed
  engine tests or game observations.

Evidence used the main checkout's read-only export queries, decompilation and instruction
bytes, with local PE/table inspection for shared context getters and conversion tables.
No REA call was attempted: the brief explicitly states that no binary is open there.
No engine, plugin, capi, ABI table or coverage-map files were read or edited for this work.
No ABI row proposal is needed. No in-game runs were requested or performed.

Validation: `python tools/questions.py index` regenerated the index;
`python tools/doccheck.py check` passed. Cargo, clippy and ctest are not applicable to
this prose-only lane. RD1, RD3 and RD4 remain outside this task.

## Handoff

- Done: `97bcebf06` CopyColorBuffer/bindings; `c26c03147` arithmetic, vectors and question archival.
- Next: coordinator lands this documentation, then core consumes §§9.6, 12.6 and 14.2.
- RD12's originating branch can introduce its old open entry during train integration;
  retain this answered entry and remove that superseded open entry if it reappears.
- CopyColorBuffer ECX is owner+4; Flush ECX is owner+C. Do not confuse their raw offsets.
- Detached HEAD is intentional; supervisor publishes commits to the reserved lane branch.
