| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-legacy-control5b | Codex / re-legacy-control5b (re, detached worktree) | ready documentation increment; LC5 and LC9–LC13 answered, LC8 partial; doccheck/questions passed; cargo/clippy/ctest n/a; game n/a; ABI unchanged | Complete LC8 emission/value/record contracts before full serializer closure |

## Report

The requested seven-question closure is **partial: six answered, LC8 remains open**. This increment is
ready to land as documentation; it does not establish full owner/round-trip equivalence or completion
of the implementation. No engine, plugin, capi, ABI tables or coverage-map files were read or edited.
The prior RE/core lane reports supplied the implementation-side closure context.

### Changes and implementer consequences

- LC5 (§6): default attachment uses the manager's current main/root window, a borrowed result; manager
  replacement changes later default parenting. Confirmed manager slot 3 and GetMainWindow forwarding.
- LC9 (§7.5): portable policy requires valid UTF-8 with unchanged bytes and explicit unsupported outcomes
  for invalid bytes or out-of-range scan conversions. Existing partial-scan defaults and defined wrapping
  arithmetic remain intact. This is a design choice, not claimed native UTF-8 validation.
- LC10 (§§9.2/9.5): TreeView duplicate metadata keys are first-insert-wins, not rejected; later widget
  effects still occur. Grid text notifier descriptors retain empty text. Axis notifier records copy all
  three numbers with column/row orientation, including the column endpoint ignored by its sizing setter.
  Undefined Grid style/key consumers are unsupported; established carry-over stays defined.
- LC11 (§13.3): explicit unsupported edge/allocation outcomes replace provisional permission to reject
  undefined arguments. No fabricated successful payload or transactional rollback is promised.
- LC12 (§13.1): six-slot default SC3 procedure contract, accepted QI IDs, callback returns, count and
  generator ownership/replacement notifications. Structured callback always false; scalar callback true
  only for 0xA2BF8AD0/0xA2BF8AD1. Same-pointer replacement has no effects.
- LC13 (§§9.4/9.9): named-font metadata owns copied String bytes/length in a type-0x800E variant at
  0xFAA3BE85 + selector. Base SetParam stores metadata without resolving/applying a font.
- Added LCV34–LCV43: ten recording-interface vectors, bringing the spec total to 43. These are static
  predictions, not executable tests, native game results or pixel comparisons.
- LC8 (§7.7): confirmed output vocabulary for all 19 writers and option-walker words; documented bare
  autofit, unescaped quoted output and per-option definition/move/size grouping. Exact conditions,
  complete value sources and repeated-record formats remain explicitly open. Do not emit every
  inventoried word unconditionally or treat an empty suffix as equivalent.

### Evidence, validation and limits

Export queried first with re_query.py, followed by bounded decomp reads from the main checkout. Numeric
and width-sensitive facts were checked against bytes/instructions: String copy/type, manager stored
pointer and forwarding, procedure message constants/returns, generator notification ordering, 12-byte
axis payload copies and Grid scratch test. The default procedure's six vtable addresses were read from
the working executable, then targets queried in the export. No REA query was needed; no REA error/fallback.
Local evidence helper is ignored at build-runs/evidence.py; no decompilation or game data was committed.

- Milestones: 7d36db6f (six answer contracts/vectors), 18af0344 (question moves/index and LC8 boundary).
- PASS: python tools/questions.py index; python tools/questions.py check; python tools/doccheck.py check;
  git diff --check. Cargo, clippy and CTest are n/a for this spec-only change.
- No game runs requested or performed; no shadow/replacement or exactness verdict. ABI exports unchanged;
  no proposed ABI rows, so no abi-rows TSV.
- Six entries moved to docs/specs/questions/answered/legacy-control.md with dated MATCHES/DIFFERS rules.
  LC8 updated in docs/specs/questions/legacy-control.md; no new question IDs.
- Investigation stopped when guard reported 103280/150000 measured context (first reading beyond the
  two-thirds target); remaining work was documentation, question bookkeeping, checks and handoff.
- Git commits succeeded despite a packed-refs.lock permission diagnostic. Detached HEAD retained;
  no branch/ref/config maintenance attempted. Queue uses actual HEAD; supervisor publishes lane branch.

## Handoff

- Goal: close legacy-control questions LC5 and LC8–LC13 with evidence and explicit portable policy.
- Done: 7d36db6f specified LC5/LC9–LC13 policies, native map/notifier rules, font copy and SC3 procedure lifetime; added LCV34–LCV43.
- Done: moved six answered entries; question index/check and doccheck passed; inventoried confirmed words for all 19 LC8 writers without claiming complete emission contracts.
- Next: LC8 successor should query writer getters/metadata gates starting with Slider 0x0095F4EC,
  TextTicker 0x00960C31 and Folders 0x00962E9F; then finish the other writers, optionswalk and Grid/TreeView records.
- Facts: manager slot 3 is current main window; named-font metadata is copied type-0x800E bytes, no base-window font application.
- SC3 procedure vtable image words: 0094c813 0094c7ce 0094c7e5 0062c980 0094c835 0094c851.
- TreeView unique maps are first-wins; duplicate alone must not be rejected. Grid axis helpers copy all three numbers.
- Edit only specs and lane record; no engine reads/edits, no ABI changes planned.
- Main evidence: C:/Users/tlori/FreeCity4; worktree reserved at detached HEAD.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-legacy-control5b-r1-manifest.json.
