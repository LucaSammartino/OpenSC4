| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| close-legacyctl29: legacy replay (engine) | Codex / close-legacyctl29 / detached; code a2df43ca5; ABI unchanged | ready for partial landing: cargo 7884/0, clippy clean, CTest 124/124; final H/C clean, ref_warnings=0, shadow 72/72 and 2/2 exact; whole row unverified | 1. Coordinator enqueue pinned tip and review method credit. 2. Resolve LC8/LC17/LC18 and remaining capture before whole-row credit. |

Code: recording widget host/private replay transport, small-method hooks and per-method counters; 5367aa903..a2df43ca5. Default-off legacyctl_shadow; no public ABI export/version change.
Final qbuild all PASS: Rust 7884/0, clippy 0 diagnostics, native 78/78 edges, CTest 124/124; owned fmt 4/4. Logs: build-runs/qbuild-{test,clippy}.log and build-runs/build-results/qbuild-20261026-114847-build-ctest.verdict.
Final game evidence: C:/Users/tlori/FreeCity4/build-runs/game-results/; H=close-legacyctl29-r4-house2032, C=close-legacyctl29-r4-city2 (.log/.verdict); both DLL SHA256 7a1b83a055bb470ff038cbcd162a65d090fce5aced8dacdd9420b7bb02cae4b6.
H: clean through 2032-01-08, ref_warnings=0, 72/72 exact monthly summaries; one recovered 8328-ms no-OnTick stall (.log:6001,6112). C: Kensington clean through 2006-05-02, ref_warnings=0, watchdog_stalls=0, 2/2 exact summaries.
Table counts are complete/calls, identical in H and C; every monthly method line has mismatches=0 and resyncs=0. Empty/unhooked entries receive no credit.
| Interface slots | Method and complete/calls | Limit |
|---|---|---|
| Primary 0,1 | QueryInterface 2367/2367; AddRef 3270/3270 | Exact observed calls |
| Primary 3 | SetParameters 0/775 | Final trees/defaults and temporary metadata lifetimes unobserved |
| Primary 5,6 | GetID 204/204; SetID 0/0 | SetID unreached |
| Primary 7,8,9 | SetOwnControl 775/775; ImplementInterface 0/0; SetParent 775/775 | ImplementInterface unreached |
| Primary 2,4,10 | Release; GetControlVocabulary; scalar deleting destructor | Unhooked/unmeasured |
| Control-area 0..7 | QI/AddRef/Release; stubs 3/4/7; SetArea 5; GetArea 6 | Unpatched/unmeasured: LC17 |
| Legacy 0,1,2,4 | QI/AddRef/Release/SetOwnControl thunks | No separate-address evidence |
| Legacy 3 | GetControl 1564/1564 | Includes 775 explicit post-build getter probes |
| Legacy 5,6 | ForceControl 14/14; UseWinProc 775/775 | Exact observed calls |
| Legacy 7 | GetScriptFragment 0/0 | Populated fragment/lifetime comparison missing; LC8 |
Interpreters: FlatRect 34/34; Common 775/1550 (second pass only complete); Slider 1/2 (resource-bearing first pass incomplete); Outline/Folders 0/0. All compared effects exact; fifteen other type captures absent.
Missing acceptance: complete final trees (class/ID/rect/flags/parent order), all interpreter/resource/complex payload effects, private-list/variant lifetimes, populated fragment serialization comparison; whole row remains unverified.
Earlier r2-month1 corruption and r2-month2's 68 FlatRect differences precede the private-list/defined-byte/pass-local fixes; they are excluded from acceptance. r2-month3/house2032/city2 were also exact on earlier DLL 32c4761c.
Questions remain LC8/LC17/LC18 in docs/specs/questions/legacy-control.md; no new questions or source changes this final round. No outstanding game/build request. Next: coordinator review partial landing; defer whole-row credit until missing observations are implemented.
