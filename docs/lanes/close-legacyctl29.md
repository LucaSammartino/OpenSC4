| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| close-legacyctl29: legacy replay (engine) | Codex / close-legacyctl29 / detached code a2df43ca5; ABI unchanged | wip: final all waiting for Cargo slot; fmt 4/4; earlier quick 5946/0; final game evidence pending | 1. Collect session 2710. 2. Measure corrected DLL, 2032 and second city. 3. Complete remaining capture; resolve LC8/LC17/LC18. |

This round: 71c1d540c, 79abe847f, 12aabc1dd, a2df43ca5; default-off switch and public ABI unchanged.
Fixed Gen completeness, unsafe private-comment-list SDK casts, failed-factory out reads, byte colour capture and pass-local resource exclusions.
Prior all collected: build PASS 316/316; CTest FAIL 123/124 (ret_channel_check); build-runs/build-results/qbuild-20261026-090054-build-ctest.verdict. This is superseded source.
Quick at 79abe847f: PASS 5946/0, build 47/47; build-runs/build-results/qbuild-20261026-094602-quick.verdict. Current fmt 4/4; return-channel check 0 unreviewed, 98 reviewed.
Final qbuild all: session 2710, waiting over 30 minutes for one of three occupied Cargo slots; no current Rust/clippy/native verdict. Collect this operation before another gate; existing qbuild-test/clippy logs belong to the previous round.
Game evidence below is MAIN/build-runs/game-results/; both requests completed through MAIN's shared queue, no game request outstanding.
close-legacyctl29-r2-month1: corrupt at startup, ref_warnings=0; unsafe private comment-list virtual call removed afterward; .log/.verdict retained.
close-legacyctl29-r2-month2: DLL 991c6ff3 (79abe847f), clean to 2026-03-04, ref_warnings=0, watchdog_stalls=1; shadow 0/2 exact, 68 FlatRect differences; .log/.verdict retained. Not final-source acceptance.
Per-method table below is complete/calls at month2's final line; exact means mismatches=0 and resyncs=0 in that earlier DLL only:
Primary 0 QueryInterface 2367/2367 exact; 1 AddRef 3270/3270 exact; 5 GetID 204/204 exact.
Primary 7 SetOwnControl 775/775 exact; 9 SetParent 775/775 exact.
Primary 2 Release, 4 GetControlVocabulary, 10 deleting destructor: unhooked/unmeasured.
Primary 3 SetParameters 0/775; final tree/defaults and temporary metadata lifetimes remain unobserved.
Primary 6 SetID and 8 ImplementInterface: 0/0, unreached; no credit.
Control-area slots 0..7: unpatched/unmeasured (LC17 overlapping vtable extent).
Legacy 0 QI, 1 AddRef, 2 Release, 4 SetOwnControl thunks: no separate-address evidence.
Legacy 3 GetControl 1564/1564 exact, including 775 explicit post-build probes; 5 ForceControl 14/14 exact; 6 UseWinProc 775/775 exact.
Legacy 7 GetScriptFragment 0/0; populated fragment/lifetime observation missing, per-type serializer LC8 remains open.
Common 550/1550 exact partial calls; FlatRect 34/34 with 68 mismatches; Outline/Folders 0 calls; Slider 0/2. Colour masks/pass filters were corrected after this run and remain unmeasured.
Questions: existing LC8/LC17; new LC18 in docs/specs/questions/legacy-control.md requests private comment-list reader/lifetimes. No guessed list interface remains.
Missing: full trees/defaults, fifteen per-type captures, resources/complex payloads, fragment comparison; whole row is unverified and no final method credit is claimed.
Next: collect all; fix failures, repeat one-month run, then full house to 2032 and a second city load with final DLL; preserve failed/earlier-source evidence separately.
