| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| close-legacyctl29: DBPF regression repair (engine) | Codex / close-legacyctl29 / detached; code e0502c7ee; ABI unchanged | ready: cargo 7901/0, clippy clean, CTest 126/126; r5-final clean, refs=0, DBPF 3/3 exact, legacyctl unchanged; 14:50 enqueue deadline missed | 1. Coordinator decide late landing versus revert. 2. Preserve partial legacy-control credit; whole row remains unverified. |

Code: merged main (da6663fcc); scoped recorder hooks (9b731563c), narrowed to COM factory only (e0502c7ee); regression test added. No public C ABI/export/version changes.
Cause: Setup permanently replaced COM CreateInstance's vtable entry with a DLL address; DBPF correctly rejected Interface(com,4). Restore that slot after the last nested/overlapping capture; keep widget observations installed. DBPF validation is unchanged.
Diagnosis: close-legacyctl29-r5-diagnose.log:2828 reports COM-interface-outside-supported-image; files=1, records=187, refusals=1. The broader r5-fixed attempt restored DBPF but reduced SetParent complete to 0; excluded from acceptance.
Final evidence root: C:/Users/tlori/FreeCity4/build-runs/game-results/; F=close-legacyctl29-r5-final (.log/.verdict), same shadows profile, priority high, turbo, requested 2026-03-01; clean through 2026-04-13, ref_warnings=0, watchdog_stalls=0.
F first DBPF sample 2026-01-27: files=11 opened=11 records=111826 compressed=88769 refusals=0 mismatches=0; packed_roots=10 packed_children=1; packed-root/child discovery present (13 discovery lines across run).
F DBPF: 3/3 samples exact, all refusals=0; full first sample plus February/March rotation. Native image checks were not weakened.
F legacyctl: 3/3 summaries exact; compared=7976 builds=775; all 46 February/March summary+method payloads byte-identical to diagnosis after removing log prefixes. No trace differences; all method mismatches/resyncs=0.
Final qbuild all PASS: cargo 7901/0, clippy 0 diagnostics, native 49/49 edges, CTest 126/126 including legacyctl_hooks_contract. Logs: build-runs/qbuild-{test,clippy}.log, fc4verify-ctest.log, build-results/qbuild-20261026-144746-build-ctest.verdict. Tree-change note reflects committing already-tested source during the gate; no source edits after invocation.
Final DLL SHA256: 192ae15dba503455d30d90514ef84a22dc77fd2fedd33b170836173428c3d0b7.
Deadline: code fixed/committed and final game submitted at 14:48:40, gate passed before 14:50; game verdict arrived after 14:51. The 14:50 merge-enqueue deadline was missed; coordinator must decide late landing versus revert. No further implementation started.
Previous full evidence (older DLL): close-legacyctl29-r4-house2032 clean through 2032-01-08, refs=0, 72/72 exact; close-legacyctl29-r4-city2 Kensington clean, refs=0, 2/2 exact. Current round measures the requested DBPF regression only.
Per-method complete/calls below remain unchanged; unhooked/unreached entries receive no credit. Existing questions: LC8/LC17/LC18 in docs/specs/questions/legacy-control.md; none filed this round.
| Interface slots | Method and complete/calls | Limit |
|---|---|---|
| Primary 0..4,10 | QI 2367/2367; AddRef 3270/3270; SetParameters 0/775 | Release/vocabulary/destructor unmeasured; build incomplete |
| Primary 5..9 | GetID 204/204; SetID 0/0; Own 775/775; ImplementInterface 0/0; Parent 775/775 | Unreached methods retain no credit |
| Control-area 0..7 | All unpatched/unmeasured | LC17 |
| Legacy 0..7 | GetControl 1564/1564; ForceControl 14/14; UseWinProc 775/775; Fragment 0/0 | Thunks unmeasured separately; LC8 |
| Interpreters | FlatRect 34/34; Common 775/1550; Slider 1/2; Outline/Folders 0/0 | First-pass/resource gaps; fifteen other type captures absent |
Whole-row acceptance still missing: final trees/defaults, every interpreter/resource/complex effect, temporary/private-list/variant lifetimes, populated fragment serialization. No outstanding game/build request. Next: coordinator review deadline and partial landing.
