| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-effects-commands5 — effects script command contracts | Codex / detached re-effects-commands5 | in progress: round 3 checkpoint; root contracts published, EF15 nested handlers incomplete | Resume remaining nested handlers and full inventory; no ready queue while EF15 remains open. |

## Report

Round 3 publishes effects §§19.11–19.16: testEffect preview creation, all six
component definition blocks and nested particle effectBase, with 26 additional
static conformance rows EC43–EC68 (99 rows across all rounds). All 21 root
descriptors now have direct or entry/exit execution contracts. The family is
still incomplete: remaining nested handlers and the complete class inventory
are required to answer EF15. No merge-queue entry is written for partial closure.

Measured findings: testEffect retains a preview before setting transforms, adds
binary32 terrain height+4 for two-coordinate positions, and reads speed/target/hard
only after creation and both transforms. Definition inheritance errors retain
the new name/reset state; decal uses the particle-definition error text. Light
fallback lists are supplied at exit even without a collection. Sequence exit
clears/releases its pointer; other definition descriptions remain live. Particle
base assignment copies an uninitialized +1C8 word and retains live +204.

Round 3 validation: doccheck check and questions check passed after each spec
milestone. Cargo/clippy/CTest n/a (spec-only). All vectors are unexecuted static
predictions; no game/shadow/native oracle, game requests, ABI rows or exports.
No REA query was needed. Export/decomp queries and instruction bytes provided
evidence. The ignored PE helper initially read disk bytes beyond a raw section;
it now supplies virtual-tail zero-fill. Published zero constants were checked
with the corrected helper. Git again emitted a packed-refs.lock permission
diagnostic but successfully created detached commits; no refs were changed.

Earlier-round report:

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
- Published total: effects-parser §§9–13 and effects §19; 99 static vectors; EFP1–EFP6 answered, EF15 and EFP7 open.
- Done round 2: effects-parser §13 answers EFP2: all five rotation branches, turns not degrees, asymmetric intermediate stores, six static vectors; questions/doccheck passed.
- Done round 2: eee0dd22 closes EFP2; effects §19.4–19.6 now documents seven identifier/trigger handlers plus resource enter/exit/import, 18 further static vectors.
- Done: 790c6cff publishes identifier/resource contracts; §19.7–19.8 adds camera and priority declarations, ten static vectors; camera span truncates/clips, priority exact-name duplicates keep first.
- Done round 3 48dd5f1f: effects §19.11–19.12 publishes testEffect and EC43–EC50; verified transforms, terrain height+4, retention before setters and failure prefixes.
- Done round 3 a6079da7: effects §19.13–19.15 publishes six definition block entries/exits, constructor defaults, inheritance and EC51–EC64. Final checkpoint adds effectBase §19.16 and EC65–EC68.
- Exact next step: verify nested dictionary registrations and publish child contracts, starting cParticleEffectCommand::RegisterCommands 005958F0; build-runs/inventory.py is navigation only and misses folded/common handlers.
- EF15 remains open: §19 now covers entry/exit or direct execution of all 21 root descriptors, but nested handlers and full class inventory still require publication.
- Corrected prior spec error: AddRef/Release are F3C0/F3D0, not F3E0/F3F0; destructor 005A27F0 -> 0059F4E0.
- Static evidence only; no game requests, no ABI changes. EFP7 deliberately outside task.
- Ignored helpers: build-runs/peek.py (PE data/import/reference reads), inventory.py (registration navigation only).
- inventory.py only handles puVar1/PTR_FUN registrations: select, particleSequence and folded generic handlers NEED manual proof; do not publish its stale descriptor guesses.
- Dispatch evidence: generic root registrar 00676DB0; nested 00676B80; lookup 00679120; wrappers 007E3BE0/007E3B10/006773D0; end 00676110.
- Done c5ad01a0: effect enter/exit §19.9–19.10 and EC35–EC42; doccheck/questions passed, tree clean after commit. No merge queue until whole-family closure.
- Gate: doccheck check and questions check passed. No outstanding process or game label. Do not enqueue partial work as ready.
- EFP1–EFP6 archived in questions/answered/effects.md; EF15 open in questions/effects.md, EFP7 in questions/effects-parser.md.
- Explicitly permitted engine read: main checkout effects_parser/README.md only.
- q.py cannot start Git Bash (CreateFileMapping access denied); use bounded PowerShell calls.
- Next navigation: sequence registrar 00596190 registers descriptors 00A9FCC8 -> 005A2400 and 00A9FCB8 -> 005A20C0 (names via 00B0A400 / 00B0A3FC); execution NOT published yet.
- Round 3 resumed at c5ad01a0; corrected ignored PE helper virtual-tail reads (B22CC0 is zero-fill). No published prior constant was changed.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-effects-commands5-r3-manifest.json.
