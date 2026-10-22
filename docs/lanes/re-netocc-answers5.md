| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-netocc-answers5 (network owner questions) | Codex / detached `re-netocc-answers5` | in progress: harvesting PR 10 | Finish harvest; trace NO33/NO36/UG5 and validate |

## Report

Updated `docs/specs/network-occupants.md` with three complete question answers, corrected live layouts,
record tables, failure/ownership rules, placement transitions and expanded prose conformance vectors
16, 17, 19 and 20. No implementation, ABI exports or ABI tables changed; no proposed ABI rows.

- **NO34** (`c2d33055`): demolition quote reads signed-int64 property `0x0A2ECED8` directly from the
  control exemplar during one-time StaticInit. The float property `0xE9A18D3F` does not supply the quote.
  Test 17 now covers zero/missing/failed lookup, exact signed words and one-time loading.
- **NO37** (`ddbbc96d`): pylon serialization receives primary+0xC. The first alleged transform is a bool;
  there is one transform at live +7C, a u16 call at +14C, and a GZ string at +1AC rather than an int64.
  Source IID is 0x26BB767E; owner IID is 1. v3/v4 preserve absent tail; reached releases are not rolled back.
  Placement bits/results, renderer +70 insertion/+74 removal, repeat Init, packing tuples and shutdown
  resources are explicit. Renderer +28's return is discarded.
- **NO35** (`015ea80f`): pole record/live offsets reconcile with the same +C adjustment. Spec now covers
  counted line-reference list, two collections of counted xyz lists, exact three wall-helper records,
  four vectors through the selected shared +E4 table, distinct destination/line IIDs and delegate cleanup.
  Corrected quirk 11 and test 16; live +B0 vector end is not the released delegate at live +BC.

All three entries moved to `docs/specs/questions/answered/network-occupants.md`, marked DIFFERS with
date/lane, and the questions index regenerated. `questions.py find` confirms all three are archived.
No new questions filed.

**Validation:** `python tools/questions.py index` and `check` passed after every milestone;
`git diff --check` passed. Numeric widths, signed-byte extension, ref IIDs, placement masks/call slots,
and record operation order were checked against exported instruction bytes. PE data confirms initial
quote words are zero. Sources were the main checkout's export and scoped decomp functions; REA was
not required. Cargo, clippy, ctest and in-game/shadow runs: n/a (spec-only lane). These are specified
conformance vectors, not executed engine tests or a claim of owner closure.

**Remaining:** NO33 stays open for pole connection algorithms, tables, geometry, drawing/picking and
resource closure; only layout/record facts were corrected here. NO36 stays open for pylon texture/model,
draw/pick/message and other owner algorithms; lifecycle and packing inputs alone do not close it.
UG5 remains open and the underground spec is unchanged: connection geometry/eight-neighbor decisions,
callback, view/model/rendering and record/reference closure still need a dedicated trace. NO12, NO13,
NO14 and NO29–NO32 were not reached. Work was bounded to finish and land verified record/placement
answers within the measured lane budget rather than declaring these wider questions answered.

## Handoff
- Harvest commits: `3393a1a2` (a8b30439), `36a3a300` (a33fa02e), author amarret retained.
- A failed UTF-8 patch generation left conflict text in the second harvest commit; normalization in the next commit removes it. Initial doccheck passed only after normalization; do not use the intermediate tree.
- Normalization unions symbol rows (pre-round addresses win), retains NO34/35/37 answers/vectors, renumbers PR questions NO32-37 to NO38-43, regenerates QUESTIONS. Coverage additions set spec only; core/shadow/replace remain '-'.
- Validation after normalization: questions index/check and doccheck check passed. No ABI edits.
- Round 2 manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-netocc-answers5-r2-manifest.json`.
- Harvesting a8b30439 then a33fa02e, preserving amarret authorship. Duplicate PR questions NO32-35 become NO38-41.
- REA returned target_unavailable (no app open); use export/decomp fallback, no new tools/session changes.
- Conflict: keep PR rounding/path table (0061B010/00619790 instruction bytes); retain local pole ref-kind rule.
- Land the ready spec commits; do not treat pole/pylon/connection owners as closed.
- Next: NO33 primary methods, NO36 texture/model/geometry, then UG5 connection owner trace.
- Use primary-relative offsets consistently; pole/pylon Read/Write receive serializable primary+0xC.
- Pylon +1AC is an embedded string; pole's four record xyz values overwrite the selected shared +E4 table.
- Evidence helper `build-runs/read-pe.py` and query output are ignored; no RE output is committed.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-netocc-answers5-r1-manifest.json`.
