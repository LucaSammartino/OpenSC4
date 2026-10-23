| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-effects-commands5 — effects script command contracts | Codex / detached re-effects-commands5 | in progress: round 2 checkpoint, EFP1–EFP6 published, EF15 incomplete | Resume testEffect and remaining definition/nested handlers; no ready queue while EF15 remains open. |

## Report

Published effects-parser.md §§9–13 and effects.md §19. Answered EFP1–EFP6
in the requested answered/effects.md archive, with 73 independent static
conformance rows across both rounds. Native reader matching is exact-case, marks the first matched
dash to plus and retains every record. Directory names fold case within each type;
anonymous registration appends a per-type index and retains an object reference.
Corrected the old AddRef/Release attribution and identified actual destruction.
Generic registration owns descriptors and calls initialization hooks; nested lookup
searches inward-to-outward before root fallback, whose post-switch policy differs.
Round 2 adds all five rotation instruction/store contracts, four resource-ID
maps, effectID/instance/messageTrigger, resource block enter/exit/import, camera,
deferred priority, and root effect enter/exit. Rotation values are turns multiplied
by binary32 2π; row-dependent product stores are explicit. New alias failures
retain an inserted zero map entry. Camera expands one distance to five by halving
and truncates/clips parallax spans. Priority duplicates keep the first exact-case
name; matched declarations override inline range checking and may rename effects.
Resource exit restores the base collection rather than its entry collection.
The default effect constructor leaves the last two message words indeterminate.

Validation: questions index/check and doccheck check passed at each committed milestone.
Cargo, clippy and CTest: n/a (spec-only changes). No game run or request, no
native oracle, no shadow comparison, no ABI export/row changes. All predictions
are explicitly unexecuted. No REA call was needed; evidence used local export,
instruction bytes and PE data/imports. Initial q.py invocation failed with Git Bash
CreateFileMapping error; bounded PowerShell calls succeeded. Git reported a
packed-refs.lock permission diagnostic at checkpoint but created detached commit
62390a5d successfully; no shared ref/config change was attempted.

EF15 remains open: this is not whole-command closure. EFP2 is now answered;
EFP7 was outside this task. The remaining handler contracts and their numeric defaults,
validation, foreign-call ordering and vectors must be published before ready.

## Handoff
- Goal: publish effects-script command behavior for the engine lane; no engine edits.
- Done: 62390a5d publishes reader/directory/lifetime; 16f94ae2 adds dispatch/error contracts. Both pass doccheck/questions checks.
- Published total: effects-parser §§9–13 and effects §19; 73 static vectors; EFP1–EFP6 answered, EF15 and EFP7 open.
- Done round 2: effects-parser §13 answers EFP2: all five rotation branches, turns not degrees, asymmetric intermediate stores, six static vectors; questions/doccheck passed.
- Done round 2: eee0dd22 closes EFP2; effects §19.4–19.6 now documents seven identifier/trigger handlers plus resource enter/exit/import, 18 further static vectors.
- Done: 790c6cff publishes identifier/resource contracts; §19.7–19.8 adds camera and priority declarations, ten static vectors; camera span truncates/clips, priority exact-name duplicates keep first.
- Exact next step: re_query asm 0059D1D0 --bytes --max 56, then page by 56; testEffect pseudocode is badly stack-aliased, so verify transforms and every service call from instructions before publishing.
- EF15 remains open: §19 covers 14 of 21 root descriptors; testEffect and the six other definition blocks still require execution contracts, plus nested handlers and full class inventory.
- Corrected prior spec error: AddRef/Release are F3C0/F3D0, not F3E0/F3F0; destructor 005A27F0 -> 0059F4E0.
- Static evidence only; no game requests, no ABI changes. EFP7 deliberately outside task.
- Ignored helpers: build-runs/peek.py (PE data/import/reference reads), inventory.py (registration navigation only).
- inventory.py only handles puVar1/PTR_FUN registrations: select, particleSequence and folded generic handlers NEED manual proof; do not publish its stale descriptor guesses.
- Dispatch evidence: generic root registrar 00676DB0; nested 00676B80; lookup 00679120; wrappers 007E3BE0/007E3B10/006773D0; end 00676110.
- EFP2 fully published in effects-parser §13; PC/RC/nonfinite qualification remains EFP7. Six RO rows are unexecuted, including two arithmetic-stage assertions.
- Done c5ad01a0: effect enter/exit §19.9–19.10 and EC35–EC42; doccheck/questions passed, tree clean after commit. No merge queue until whole-family closure.
- Gate: doccheck check and questions check passed. No outstanding process or game label. Do not enqueue partial work as ready.
- EFP1–EFP6 archived in questions/answered/effects.md; EF15 open in questions/effects.md, EFP7 in questions/effects-parser.md.
- Explicitly permitted engine read: main checkout effects_parser/README.md only.
- q.py cannot start Git Bash (CreateFileMapping access denied); use bounded PowerShell calls.
- Round 2 commits: eee0dd22 rotations, 790c6cff identifiers/resources, e51d6a67 camera/priority, c5ad01a0 effect entry/exit. Budget reserved for checkpoint/continuation; no pending process or game labels.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-effects-commands5-r2-manifest.json.
