| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-legacy-control4 (portable LegacyControl owner) | Codex / core-legacy-control4 (engine lane, detached HEAD) | in progress: guard stopped round 2 at 152k/150k; workspace cargo 5162/0 and owner 22/22 at 34dc4033; nine-file fmt passed before lint cleanup; clippy redundant_locals fix committed 628ec2a1 but unverified; native/CTest incomplete; ABI unchanged; LC9/LC10 retained | (1) Reconcile original all by PID/creation/command, no duplicate; (2) focused clippy/fmt after it ends; (3) authorized stable full gate before ready; no queue written |

## Handoff
- Goal: finish portable LegacyControl owner validation; exact 45-entry scope, no GUI/game/network/ABI/native/default/coverage work.
- Original clean ce60e13a remains ancestor; source milestones 02bd46f2 fixtures before bf469091 production, d8d3da6f source/tests.
- Round 2 commits: 8949db4a validator reconciliation, 34dc4033 audit/fmt, 628ec2a1 actual two-line Clippy fix and full 45-entry report below.
- Old all/fmt did not survive; limited-query OS audit found no own validator. Old logs moved byte-identically to r1-*; failures and UTF16 q.py transport evidence preserved.
- Exactly ONE replacement all launched on clean 34dc4033, tree 736c76bcac18bc6173e2063d2d16853fc6cf9bcb. No quick or second all launched.
- Cargo workspace completed: 5162 passed, zero failed/ignored/measured/filtered; 22 summaries. LegacyControl 22/22 passed, exact393 vocabulary.
- Nine-file absolute owner fmt passed before final lint cleanup: exit0 files9 diff_lines1 (path warning). Shared lib.rs not reformatted; engine configuration unchanged.
- Actual Clippy failure: redundant_locals tests.rs:225/226, let mut f=f and let mut g=g. Removed only those two statements in 628ec2a1; all fixtures/assertions/tests/production preserved. No lint suppression.
- Focused post-fix clippy/fmt NOT run: original all continued into native. Never overlap checks. Original failed qbuild-clippy.log retained with hash in report below.
- Original runner PID2284 created 2026-10-22T23:30:30.413866Z, qbuild PID29620 created 23:30:30.519874Z; commands/cwd this WT. Session9365 is not liveness proof.
- Last OS native: fc4verify PID21424 created 23:52:18.665579Z, CMake PID38668 created 23:52:22.117063Z, release core rustc PID25156 created 23:52:24.690336Z CPU25.8125s; own engine cwd.
- Native rc/complete progress/CTest expected count/end/full wrapper rc still uncollected. Configure succeeded. No denominator or pass fabricated.
- Functions cell42's 300-second collection completed with no output and session9365 still running at that point; report's older 'outstanding' wording is superseded here.
- Test-only lint fix changes source bundle after failed Clippy; do not treat original all as a passing stable final-tip gate. Runner will print original qbuild rc plus before/after HEAD/tree/dirty/hashes at exit.
- NEXT EXACT STEP: inspect actual OS liveness/creation/command/cwd and own saved all/native logs; collect surviving operation without launching duplicate. Old PID/session alone is insufficient.
- Once it ends: preserve failed clippy log, run focused qbuild clippy and absolute touched-test fmt, then obtain authorization for required stable full all (round2 used its one replacement). Report real wrapper failures accurately.
- Full 45-entry classification below:42 complete safe-domain,3 partial; LC4 twelve unknown interpreters Unsupported, LC5 parent opaque, LC8 nineteen unknown writers partial, LC9/LC10 remain open.
- Source/report committed 628ec2a1; final attempted status/log/hash inspection was DENIED before execution by context guard152k/150k. No final native or clean-tip result was collected.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-legacy-control4-r2-manifest.json. Guard denial requires stop, Handoff commit, continue result, NO merge-queue file.

# Portable LegacyControl owner report

Source milestones: independent literal fixtures `02bd46f2` preceded production
`bf469091`; source/tests finalized in `d8d3da6f`. The original round stopped at
`ce60e13a`, preserved as an ancestor. Round 2 reconciled validation in `8949db4a`
and recorded the audit/format result in `34dc4033`. Round 2 removed only two
redundant mutable local rebindings from the vocabulary test after Clippy reported
them. Production, fixtures, all assertions/counts/tolerances and all 22 test
functions remain intact. No ignored tests or lint suppressions were added.

## Exact 45-entry inventory

Complete means the published portable effects in the declared safe domain,
including explicitly allowed deterministic choices. It does not mean native
memory layout, unsafe CRT inputs, whole native UI behavior, or default readiness.
Conservatively: 42 complete entries, 3 partial entries, 0 wholly unsupported
entries within these 45; 12 further owned interpreters remain Unsupported (LC4).

1. QueryInterface — complete: four IIDs, interface identity, AddRef, untouched output on failure.
2. Release — complete: decrement above one; count one or zero destroys without first decrementing.
3. Destructor/scalar deleting destructor — complete portable detach/shutdown/ref order; Rust tombstone/storage replaces native deletion.
4. GetControlVocabulary — complete borrowed getter.
5. SetOwnControl — complete flag setter.
6. SetOwnControl legacy thunk — complete same setter.
7. SetParent — complete release/null old, legacy QI, acquire control, release temporary, unconditional true.
8. UseWinProc — complete equality no-op, new AddRef before old Release.
9. ForceControl — complete same ref ordering, leaves own flag unchanged.
10. GetID — complete null/default/read channel.
11. SetID — complete null/no-op/write channel.
12. DoExceptions — complete true-only QI, one-shot procedure handoff, procedure then interface release.
13. Constructor — complete defaults, lazy creation, exact registration order, vocabulary reuse AddRef, usable on failure.
14. ImplementInterface — complete ignored-argument true stub.
15. SetParameters — partial: complete published known/no-interpreter builder; successful unknown per-type QI stops explicitly Unsupported (LC4).
16. GetScriptFragment — partial overall: complete allocation/reference/output wrapper and known frame/common; unknown successful writer QI marks PartialLc8.
17. ControlVocabularyCommon — complete 64 words, no map inserts.
18. ControlVocabularyFlatRect — complete registration and map inserts.
19. ControlVocabularyText — complete registration; interpreter remains LC4.
20. ControlVocabularyTextEdit — complete registration; interpreter remains LC4.
21. ControlVocabularyListBox — complete registration; interpreter remains LC4.
22. ControlVocabularyCombo — complete registration; interpreter remains LC4.
23. ControlVocabularyBtn — complete registration; interpreter remains LC4.
24. ControlVocabularyBMP — complete registration; interpreter remains LC4.
25. ControlVocabularySlider — complete registration.
26. ControlVocabularyScrollbar — complete registration; interpreter remains LC4.
27. ControlVocabularyLineInput — complete registration; interpreter remains LC4.
28. ControlVocabularyOptGrp — complete registration; interpreter remains LC4.
29. ControlVocabularySpinner — complete registration; interpreter remains LC4.
30. ControlVocabularyOutline — complete registration.
31. ControlVocabularyTextTicker — complete registration; interpreter remains LC4.
32. ControlVocabularyFileBrowser — complete registration.
33. ControlVocabularyTreeView — complete registration.
34. ControlVocabularyFolders — complete registration, no own property words.
35. ControlVocabularyGrid — complete registration.
36. ControlVocabularyCustom — complete registration, no own property words.
37. ControlVocabularySC3WinGen — complete registration, SC3WinGen/GZWinGen aliases share their class.
38. InterpretPropertiesCommon — complete published safe effects, explicit area-zero choice.
39. InterpretPropertiesSlider — complete: max before min, direction, resource references/keys, signed second-pass value.
40. InterpretPropertiesTreeView — complete published unique-key effects, retained positions, maps/root/fonts/depth-first build; duplicate keys explicitly Unsupported (LC10).
41. InterpretPropertiesFileBrowser — complete: raw getter defaults, font precedence, mappings and pair-position filter keys.
42. InterpretPropertiesGrid — partial: safe published effects implemented; uninitialized scratch/resource and unsafe malformed cells Unsupported (LC10).
43. InterpretPropertiesFolders — complete: fonts, image-key effects/references, explicit unparsed-field zero choice, repeated second-pass elements.
44. InterpretPropertiesFlatRect — complete: all repeated type-6 styles ORed, unconditional style setter, edge colors.
45. InterpretPropertiesOutline — complete: nonzero-style setter, colors, 32-node cap, explicit unparsed-component zero choice.

The 21 vocabulary functions are table-driven core algorithms with 393 exact
ordered entries, 20 class and 20 interpreter map entries. Common has no map
entry; aliases add words without extra owner credit. Custom's shared bare-return
interpreter, AddRef/GetControl, area support and property readers are implemented
support, not additional named owner bodies. Broader 205-row/104642-byte
attribution is not implementation credit for this 45-entry scope.

## Core ownership and retained quirks

Core owns typed linear property scans, numeric/field parsing, type selection,
lookup order, defaults, maps, build ordering, two passes, resource/reference
decisions and interpreter payloads. Foreign provides individual factory/QI/ref,
field/raw-widget operations, fonts/colors/resources, notifier helper operations
and string allocation/appending. It never decides builder/interpreter outcomes.
Tests use a foreign recorder with literal event vectors and independent raw
getter/factory answers, not an outcome-producing host interpreter.

Successful SetParameters still returns false. Fallback attempts only the lowest
same-IID class, then Custom, while storing requested CLSID. The second build
forgets the old control without Release. Type effects precede common effects,
procedure handoff, Init, attach/front; type/common repeat in their second pass.
Pending parent/procedure lifetimes, equality no-ops, one-shot handoff,
owned-control detach/shutdown versus release-only, and zero-count destruction
remain literal.

LC4's 12 unknown interpreters are Btn, OptGrp, TextEdit, Spinner, LineInput,
ListBox, SC3WinGen, Combo, TextTicker, Text, Scrollbar and BMP. Failed QI is the
published no-effect path. Successful unknown QI is released then Unsupported.
Prior build effects are precisely old-control overwrite, factory/fallback,
requested-ID parameters, own=true, QI/Release. Common/procedure/Init/attach/front
and second pass have not run; pending parent and procedure remain retained.

LC5's manager-slot-3 parent is opaque. LC8's 19 unknown foreign writers remain
partial after successful writer QI; common output is never described as complete
native output. The fragment wrapper preserves true with a control in its valid
foreign-result domain. Resolution scans all 21 rows when either ID is missing.
LC9 retains UTF8/bounded numeric and indeterminate-field choices. LC10 retains
duplicate-map and Grid scratch/notifier limits. No question was closed, no new
question filed in round 2, and no shared question index was edited.

## Validation attribution

The original all/fmt did not survive the round transition. OS PID/creation/command
audit with limited-query fallback found no matching validator; no other check
was killed. The old all/native logs end in compilation without return codes or
test results; the mixed quick log has 3717 NUL bytes. All five overwrite-prone
logs were moved byte-identically to r1-* names, verified by SHA256. Original
lifetime/header failure snapshots and the UTF16 q.py pre-launch Access denied
transport failure remain intact. Details: legacy-control-validation-reconciliation.md.

Replacement fmt: qbuild fmt, nine absolute owner paths, exit=0, files=9,
diff_lines=1 (canonicalize-path warning only). Engine max_width=120; shared lib.rs
was not mass-formatted. Original 22 test functions remain unignored and exercise
production algorithms; the vocabulary fixture has 21 header rows + 393 entries.

One replacement all was launched by legacy-control-final-gate.py on clean
HEAD 34dc40338ed4c4c5520781bc1f8d83b156421fd8,
tree 736c76bcac18bc6173e2063d2d16853fc6cf9bcb.
Source bundle SHA256: 200e916d47a25e750903c43a52c9e558cb97a588130d8f84e7d02fbf6d6dd1e0.
Attribution: qbuild PID 29620, created 2026-10-22T23:30:30.519874Z,
parent runner PID 2284, created 2026-10-22T23:30:30.413866Z; both commands/cwd
match this worktree. Rustc PIDs 33708/29016 compile this worktree's core.

Workspace test phase completed: 5162 passed, 0 failed, 0 ignored, 0 measured,
0 filtered, across 22 Cargo summary rows. LegacyControl tests 22/22 passed.
qbuild-test.log: 312613 bytes, no NULs, SHA256
1eddbeade616bf9a85366eaf94fa78943dd8fcb414dea643236bbc9d4e693a35.
No Rust diagnostics in that log. Clippy processes 29416/27140 were created
2026-10-22T23:45:20Z, cwd this worktree/engine. Clippy **failed** under -D warnings
with clippy::redundant_locals at tests.rs:225/226: `let mut f = f;` and
`let mut g = g;`. Both original bindings were already mutable. Removed these two
statements only. Focused Clippy and touched-test fmt after the fix remain pending:
the original all is still in native compilation, so no overlapping check ran.
The original failed qbuild-clippy.log remains unchanged: 1568 bytes, no NULs,
SHA256 09a5622612bb0f79045ed3efe2582999c19eae12180c55e063a15c7b855bf3ae.

CMake configured/generated successfully (0.2s/0.1s); configure log SHA256
23b704df9201df16e5e48025e1f225f0e16cdf4ed6974357e53d7f57f5b63990.
Native build at last OS sample was compiling release core: fc4verify PID 21424,
created 2026-10-22T23:52:18.665579Z; CMake PID 38668, created
2026-10-22T23:52:22.117063Z; rustc PID 25156, created
2026-10-22T23:52:24.690336Z, CPU 25.8125 seconds, cwd own engine. No other check
was killed. No original native rc, completed Ninja progress/full denominator,
CTest N/N/end or full wrapper return code is yet available. CTest had not started.

The full gate was launched on clean 34dc4033; its completed after snapshot is
pending. The final test-only lint fix changes the source bundle after the failed
Clippy phase. Production and vocabulary fixture hashes remain unchanged. Never
use this operation as a passing stable final-tip gate. The runner will print
original qbuild rc and its before/after head/tree/dirty/source hashes on exit.
Session 9365 has an outstanding collection in functions cell 42; neither ID
proves liveness or completion. Recheck OS PID/creation/command/cwd before adoption.

WIP: this round consumed its one replacement full all, which already failed
Clippy; no second all or duplicate quick was launched, and no wrapper gate was
overridden. Actual independent owner tests and workspace totals passed before
the lint-only fix, whose assertions/expected effects remain identical.

## Next steps

1. Collect the existing qbuild all PID 29620 / runner PID 2284 and native operation
   after verifying current creation/command/cwd. Preserve all logs and original
   rc; successful Ninja restat may have partial predicted progress. Do not infer
   success from a progress denominator or start overlapping checks.
2. Once it ends, preserve the failed clippy log before a focused qbuild clippy
   rerun; run qbuild fmt on the absolute touched test path. No fixture/assertion
   rewrite is needed. Do not change tools, parsers or caches.
3. Obtain an authorized passing stable full all on a clean final tip before
   ready. Coordinator regenerates the shared question index. LC4/LC5/LC8/LC9/LC10
   and all native/default/whole-UI exclusions remain; no question was closed.

Relative to original base 82feac1f, shared lib.rs only adds the final module
declaration; engine rustfmt configuration is unchanged. This round does not
claim a passing gate or a complete native/default/whole-owner implementation.

No GUI/game/network run, C ABI/export/version/default/coverage/native glue
change, imported widget answers, whole-UI/pixel equivalence or whole-owner credit.
