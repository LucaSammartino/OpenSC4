| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-watercraft-close14 (watercraft owner integration) | `Codex` / `core-watercraft-close14` (engine lane) | in progress: new implementation and fixtures committed; initial Rust gate failed E0277, test corrected; Clippy clean; native 52/52, CTest 81/81; ABI unchanged | (1) Run final full gate on corrected source; (2) fix remaining diagnostics; (3) enqueue tested batch with actual HEAD. |

## Handoff

Goal: close cSC4Watercraft; guard stopped round 4 after collecting all operations. No merge-queue entry was written.
Prerequisites already merged: 4ee9852ce (close12), f075b206b (WC10-WC12 answers). Previous source eb4957aa3 passed 7462 Rust tests and 81/81 CTest.
This commit adds lifecycle.rs: independent inherited initialized predicates, concrete model loading, first-model effect-controller binding/discovery/update, occupant Init, then owner setup despite inherited failure.
Init clears each old effect reference before its factory call; fixtures observe release order through Weak references.
EndTowing preserves its link across slot 53, rereads/clears a reentrant replacement, then clears its own tug; callback and reciprocal-state fixtures added.
Movement/collision share the specified outside-corner order; movement preserves its two short-circuit top-state queries. Added corner/flag/top-state vectors.
Player command processing executes rope-throw transform/start/independent refresh through a borrowed optional effect; complete-transform and missing-effect fixtures added.
Animation advancement/publication now share ModelStorage.event_counter (+0x1C0); strict-100 publication and full animation two-time/removal/lighting fixtures added.
Added concrete wake-controller execution with six interleaved stop/update calls and case-insensitive name fixtures; collision size now requires a group.
Formatting passed for 13 explicitly owned Rust files before the final small panic-test correction. qbuild fmt paths are core/src/watercraft/<file>.rs relative to engine.
Focused Cargo session 12121 TERMINAL: E0277 from catch_unwind capturing inherited SaveObject references; log build-runs/wc14-r4-test-1.log.
Full qbuild all session 75638 TERMINAL: FAIL compiling the same earlier catch_unwind snapshot (integration_tests.rs:300); log build-runs/wc14-r4-all-1.log.
That full run passed Clippy with zero diagnostics, native build 52/52 edges (284.8s), CTest 81/81 (267.3s), total native 554.2s. No operations remain outstanding.
Correction ALREADY in source: catch_unwind replaced by separate #[should_panic(expected = "no-tow refresh requires the connection effect")] test. Partial-loader fixture also now expects one release for the failed second acquisition.
EXACT NEXT: run python C:/Users/tlori/FreeCity4/.claude/lane-bin/qbuild.py --wt C:/Users/tlori/FreeCity4/.claude/worktrees/core-watercraft-close14 all on corrected source and collect its terminal result. Repetition is justified by measured E0277 and changed test input; preserve r4-all-1 evidence.
Helper build-runs/wc14-check.py captures numbered logs: all 2 runs the next full gate, fmt 4 checks owned formatting. Read bounded log excerpts.
Whole-owner closure remains incomplete: destructor/interface forwarding, inherited Shutdown/state-enter result, complete presentation execution, remaining route/docking/collision compositions and vectors. README was not edited.
No ABI/plugin wiring, new questions, game requests or shadow measurements. Manifest C:/Users/tlori/FreeCity4/build-runs/run4-core-watercraft-close14-r4-manifest.json reached 154k/150k OVER.
