| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-effects-commands5 — effects script command contracts | Codex / detached re-effects-commands5 | in progress: EFP1–EFP6 published, EF15 incomplete | Continue remaining root/nested owners; no merge queue until complete. |

## Report

Published effects-parser.md §§9–12 and effects.md §19. Answered EFP1 and
EFP3–EFP6 in the requested answered/effects.md archive, with 31 independent static
conformance rows. Native reader matching is exact-case, marks the first matched
dash to plus and retains every record. Directory names fold case within each type;
anonymous registration appends a per-type index and retains an object reference.
Corrected the old AddRef/Release attribution and identified actual destruction.
Generic registration owns descriptors and calls initialization hooks; nested lookup
searches inward-to-outward before root fallback, whose post-switch policy differs.
Two root execution contracts are published: renderprop and effectGroup.

Validation: questions index/check and doccheck check passed at both milestones.
Cargo, clippy and CTest: n/a (spec-only changes). No game run or request, no
native oracle, no shadow comparison, no ABI export/row changes. All predictions
are explicitly unexecuted. No REA call was needed; evidence used local export,
instruction bytes and PE data/imports. Initial q.py invocation failed with Git Bash
CreateFileMapping error; bounded PowerShell calls succeeded. Git reported a
packed-refs.lock permission diagnostic at checkpoint but created detached commit
62390a5d successfully; no shared ref/config change was attempted.

EF15 remains open: this is not whole-command closure. EFP2 remains open, and EFP7
was outside this task. The remaining handler contracts and their numeric defaults,
validation, foreign-call ordering and vectors must be published before ready.

## Handoff
- Goal: publish effects-script command behavior for the engine lane; no engine edits.
- Done: 62390a5d publishes reader/directory/lifetime; 16f94ae2 adds dispatch/error contracts. Both pass doccheck/questions checks.
- Done: effects-parser §§9–12, effects §19; 31 static vectors; EFP1/EFP3/EFP4/EFP5/EFP6 answered.
- Done round 2: effects-parser §13 answers EFP2: all five rotation branches, turns not degrees, asymmetric intermediate stores, six static vectors; questions/doccheck passed.
- Next: EF15 root resource/name commands, then effect/group blocks and nested setters. Commit checkpoint contains EFP2 closure.
- Then EF15: trace remaining root and nested handlers; current §19.2 covers ONLY renderprop and effectGroup; keep EF15 open.
- Corrected prior spec error: AddRef/Release are F3C0/F3D0, not F3E0/F3F0; destructor 005A27F0 -> 0059F4E0.
- Static evidence only; no game requests, no ABI changes. EFP7 deliberately outside task.
- Ignored helpers: build-runs/peek.py (PE data/import/reference reads), inventory.py (registration navigation only).
- inventory.py only handles puVar1/PTR_FUN registrations: select, particleSequence and folded generic handlers NEED manual proof; do not publish its stale descriptor guesses.
- Dispatch evidence: generic root registrar 00676DB0; nested 00676B80; lookup 00679120; wrappers 007E3BE0/007E3B10/006773D0; end 00676110.
- EFP2 navigation: rotateX lookup 00598A38, next rotateY 00598B70, rotateZ 00598C95. Three branches multiply ParseFloat ST0 by f32 at 00A9F384, FCOS then f32 temporary, FSIN stays x87 before product stores. Do not flatten to a host matrix multiply.
- Last asm pages inspected: skip 96/152/208 at max 56 on 005989D0; next skip 264, but reread skip 40/56 for full X stack derivation. No numeric rotation contract published yet.
- Guard OVER at 152k/150000; checkpoint 5685fd80 committed before denial. Attempt to read skip 264 was denied and did not execute; next remains skip 264. No merge queue written.
- Gate: doccheck check and questions check passed. No outstanding process or game label. Do not enqueue partial work as ready.
- EFP1–EFP6 are in questions/effects-parser.md, EF15 in questions/effects.md.
- Explicitly permitted engine read: main checkout effects_parser/README.md only.
- q.py cannot start Git Bash (CreateFileMapping access denied); use bounded PowerShell calls.
- Round 2 resumed 2026-10-23 at detached HEAD 5685fd80; existing publication retained.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-effects-commands5-r2-manifest.json.
