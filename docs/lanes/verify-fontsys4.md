| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-fontsys4: completed cGZFontSys verification | `verify-fontsys4` / detached reserved worktree (engine lane) | in progress | 1. Establish published owned factory/QI/lifetime route within 15 calls. 2. Verify production FontSys or stop on precise contract gap. |

## Handoff
- Goal: verify only completed cGZFontSys, drawing-fonts section 12/12.1, against independent original.
- Worktree reused, detached HEAD; initial status clean and no commits beyond main.
- Baseline qbuild all: Rust tests and clippy passed; native ctest 68/68 passed (fc4verify-ctest.log). Build finished with the route probe added, off by default.
- Published route established: spec section 1 factory creates a new 0x70-byte instance; SDK cIGZCOM GetClassObject, service IID 0x287FB697 then primary QI 0x00673960 (this+4 relationship); section 12.1 supplies QI/refcounts/Shutdown/destruction. Typed SDK MSVC calls supply thiscall; no private getter.
- d916e22d: opt-in owned route probe, balanced acquired/created views, Init/idempotence, negative QI, empty catalogue/cache, Shutdown/re-init/final Release.
- Route game label fontsys4-route-20261022a1: clean, ref_warnings=0, date 2035-01-03; load and shutdown each 23 exact, 0 differences/refusals/fallback/resync, natural 0.
- Added persistent actual GzFontSys C API transport and constructed native comparison; raw read-only Windows font metadata enters provider once, no native query/selection answers enter Rust.
- 7c84f857: production bridge committed before call 30. Focused fontsys4-focused-20261022b1 exited -1 during first bootstrap AddFont, no exception/panic/ref warnings and no production comparisons. SDK overloaded virtual textual order is not a slot contract under MSVC. Changed overloaded calls to explicit published section 12.1 slots 9/10/11/13/14; cGZFont no-arg GetStyles uses published slot 7.
- Added full read-only system font catalogue bootstrap once, actual directory positive/negative calls, renderer same-pointer/null/Shutdown lifetime tests, config key recognition. No metrics/raster claim. Natural 0 remains separate.
- Initial post-bridge full gate: Rust tests/clippy pass; ctest 67/68, ret_channel_check false positive treated font GetSize as lot GetSize. Replaced that call with published font slot 10 (no shared checker changes).
- bc7d8149 stable full gate PASS: qbuild all 4961 Rust tests / 0 failures, clippy 0 diagnostics, ctest 68/68, ret_channel_check passes. Summary build-runs/fontsys4-finalgate-summary.txt.
- Frozen DLL build-runs/dll/fontsys4-stable/FreeCity4.dll SHA256 A7D4536F5F3A775EC9818BAB3A697000532E95951058D144D2815F8853B3FC48; paired PDB preserved. No further code/build mutations planned.
- Outstanding request fontsys4-stable-20261022c1 (native attempt 3/3 counting route and exited focus): immutable DLL, 2032 end date, scripted full save 2026-03-01; qwait invoked, no verdict yet. Never discard an existing verdict.
- Formatting: new module passes qbuild fmt. Root lib.rs check traverses unrelated modules (645 diff lines); scoped skip_children check identifies pre-existing test-module ordering, untouched. No mass reformat.
- Three verification-only C exports added; ABI version and public ABI tables untouched. No binaries committed.
- Next: consume outstanding verdict, file exact questions for differences/contract gaps, do not make a fourth native attempt; original control and current40 preset/reload controls remain outstanding. No current40 preset or reload action found in allowed operational docs/engine.
- FN18-FN22 excluded. Do not read analysis sources or run tools/ scripts.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-verify-fontsys4-r1-manifest.json.
