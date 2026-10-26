| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| close-legacyctl29: legacy replay (engine) | Codex / close-legacyctl29 / detached 019420a6a; ABI unchanged | continue: round time guard; Rust 7884/0, clippy 0, native pending, no game runs | Collect gate; fix Gen capture accounting; complete replay and game measurement. |

Code commits: 5367aa903, 7029926a3, 8d377c4a2, 019420a6a. All code committed before the guard stopped edits.
Gate: qbuild all once; build-runs/qbuild-test.log: 7884 passed; qbuild-clippy.log: diagnostics=0; four owned Rust files pass fmt.
Pending native request: qbuild-20261026-090054-build-ctest; original exec session 42790. Collect before starting another check.
Game: none submitted; full house to 2032, second city load, exactness and ref_warnings unmeasured. No coverage credit.
Replay transport uses production core and a recording Foreign host; property order/duplicates/null/U32/I32/String retained; other types unsupported.
Capture: factory, window writes, QI/ref effects; Common/FlatRect/Outline/Slider/Folders entry detours; switch remains default-off.
Per-method eligibility below is UNMEASURED, not earned credit:
Primary slots 0 QI, 1 AddRef, 7 SetOwnControl, 8 ImplementInterface: pure owner comparisons.
Primary 5 GetID, 6 SetID, 9 SetParent; legacy 5 ForceControl, 6 UseWinProc: state and primitive-call replay.
Primary 3 SetParameters: complete=0 always; final trees/defaults and temporary metadata lifetimes remain missing.
Primary 2 Release, 4 GetControlVocabulary, 10 deleting destructor: no new complete evidence.
Control-area 0..7: unpatched; LC17 filed in docs/specs/questions/legacy-control.md for overlapping table extent.
Legacy 0 QI, 1 AddRef, 2 Release, 4 SetOwnControl thunk: no separate address credit; legacy 3 GetControl retains owner replay.
Legacy 7 GetScriptFragment: non-null calls counted only; native fragment and lifetime comparison missing (LC8/Q9).
Common first pass remains partial (variant/list lifetimes); second pass may complete. FlatRect/Outline and resource-free Slider/Folders may complete.
Other 15 types, resources and complex payloads remain uncaptured; Folders String font metadata also remains partial.
Next: successful Gen QI currently marks a trace exhaustive after installing only slot7; mark it partial before game runs (fix denied by guard).
Next: finish native gate, fix failures, run shared MAIN game queue iterations; implement remaining capture/tree/fragment comparisons.
