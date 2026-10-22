| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-budgetwindow-ownership4: BW14/BW15 ownership and BW16 visibility | Codex / `core-budgetwindow-ownership4` (engine lane) | blocked: edits uncommitted, git index.lock Permission denied; cargo 4177/0, clippy 0 diagnostics, ctest 67/67, fmt PASS; BW17; no ABI/game changes | 1. Restore permitted git writes and commit preserved patch. 2. Resolve BW17 before whole-owner completion. |

## Report

Resumed quota WIP 720d37b0 (lane text only; no preserved implementation). The relevant reviewed spec on current main and this worktree has no differences. All implementation edits are in engine/core/src/budget_window; there are no plugin, CAPI, tools, ABI or unreconciled-file edits.

BW16 now uses flag bit 0 and unsigned ID <= 100 or nonzero signed 64-bit amount. Income selects GetIncome; every other type selects GetFullExpenses. The same predicate selects counted and built rows. Tests 26-27 cover IDs 99/100/101, zero/25 amounts and flags 5/4/1 in both lists, with extra high-half/nonzero negative 64-bit cases.

BW14/BW15 now retain typed Arc reference operations independent of the mutable host. A saved +0x38 pair and each +0x2C slider row acquire temporary +1, copied element +1, then temporary -1; null departments are skipped. Only B builds sliders: one per shown header/item, with a shared nine-row limit across departments. Slider IDs carry no owned window reference. Shutdown releases sliders then pairs front to back; final release / Drop releases pairs then sliders, before nested/child/content/dialog windows. Drained vectors and taken window handles make shutdown/final release/Drop idempotent. A Drop after the host is gone retains the same typed cleanup operations.

BW13 remains a documented safe divergence: failed content creation retains the pause and runs the group mode branch; painting into absent content is skipped while sliders still become children of the dialog. BW12's strict ordinance versus inclusive group/neighbour arrow bounds and unconditional DestroySubBudgetWindow HiddenResume are preserved.

Validation: baseline qbuild all PASS (cargo 4172/0, clippy 0 diagnostics, ctest 67/67). Final quick PASS (2321 core tests, dependents compiled, CMake 4/4); qbuild fmt PASS for all four touched Rust files (absolute paths required by this wrapper). Five added tests exercise conformance cases 19-27, including 64 hidden departments across vector growth, exact append events, X/Y/Z refcount deltas 1/4/7, repeated initialization, shutdown/re-init, explicit final release then Drop, both destruction orders, host-independent Drop, content failure and BW16. Final qbuild all PASS: cargo 4177/0, clippy 0 diagnostics, CMake 4/4 and ctest 67/67 (128.3 seconds). Logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log and fc4verify-ctest.log; baseline logs retained as baseline-test.log, baseline-clippy.log and baseline-ctest.log.

No game request: no plugin/CAPI implementation uses BudgetWindowHost, so the pure-core changes cannot produce a game shadow trace. No native replacement or game-validation claim is made.

Question BW17 is appended to docs/specs/questions/budget-window.md. The question index is not edited or regenerated: this lane may only append the question file and cannot run tools/ scripts. The coordinator must regenerate it. Existing BW11 remains open and was not changed. The full published-owner audit follows; delegated original-owner outputs explicitly remain incomplete and cannot be used as native inputs.

Commit blocker: every git add/commit attempt fails creating C:/Users/tlori/FreeCity4/.git/worktrees/core-budgetwindow-ownership4/index.lock with Permission denied. No successful new commit exists; HEAD remains 720d37b0. All six modified source/report/question files are preserved in the worktree; a binary-capable patch will also be saved at build-runs/core-budgetwindow-ownership4-r2.patch. This is WIP, not a ready merge candidate. build-runs/msg-core-budgetwindow-ownership4-1.txt contains the prepared UTF-8 commit message and required co-author line.

Exact remainder: restore allowed git writes and commit the tested changes/report; regenerate the question index from the coordinator; obtain BW17's missing raw-input and full method/slot contracts; replace the explicitly incomplete same-class callbacks listed below with core algorithms and public external window/department/simulator calls; implement the documented child/property lookup rather than accepting get_window_indiv_item outputs; then add independent conformance coverage for those bodies and rerun fullgate. Native COM slots/thunks and game integration require a separate authorized glue lane. No restart, merge or push was attempted.

## Published owner inventory audit

This inventory covers every owner method/slot described by budget-window.md. The spec does not publish the entire budget-panel vtable or all owner bodies; a complete native-owner inventory cannot be asserted. BW17 requests that missing inventory. No whole-owner replacement credit is claimed. Same-class callback outputs used by unit-test fakes are not acceptable native inputs to a future replacement adapter.

| Owner method / slot | Core status and exact remainder |
|---|---|
| Sub-budget vtable slot 0 QueryInterface | No GZCOM adapter in this pure-core module. |
| Slots 1 AddRef / 2 Release | Rust ownership and explicit release_final / Drop cleanup; no native object refcount adapter. |
| Slot 3 cIGZWinProc thunk | No native thunk; Rust do_win_msg models dispatch only. |
| Slot 4 DoWinMsg | Message routing implemented; delegated mode bodies below remain incomplete. Alt delivery at runtime remains BW11. |
| Slot 5 deleting destructor / base cleanup | Department pairs first, sliders second, then nested/child/content/dialog releases; shutdown drains sliders before pairs; retained typed reference provider supports host-independent Drop. |
| Constructor | new / with_slider_guard initialize modeled fields. Native object allocation/refcount remains adapter work. |
| CreateSubBudgetWindow | Generator/window creation flags, output AddRef, selector/context/department state implemented. Generator setter ownership remains unspecified by section 8. |
| on_init | HiddenPause, content setup, BW13 safe continuation, selector order, group counts/build ownership, initial arrows implemented. Non-group population bodies remain incomplete. |
| on_shutdown | Idempotent modeled vectors/arrays cleared; slider then pair references released; window then content released; no HiddenResume. Unmodeled vectors +0x50/+0x5C are not silently counted as implemented. |
| DestroySubBudgetWindow | Deal AddRef/cancel/Release and UI-disable reset, unconditional HiddenResume, conditional end_dialog implemented. |
| on_button_activated | Documented dispatch/returns, apply/cancel, scrolling, camera and nested routing implemented. Popup and mode refresh bodies remain delegated gaps. |
| RestoreStartSettings | Pair and individual record replay implemented; individual record acquisition itself is not. |
| ApplyDealEdits | Documented deal flags/amount/inc transitions implemented for held deals; combo-selected +0x50 construction is not. |
| SetOccupantFundingLevel | Occupant QI/release, service dispatch and percentage conversion implemented through external simulator providers. |
| DoIndividualBudgetWindow | Nested ownership/modal/destroy sequence implemented; individual population missing. |
| Group count/build | Shared BW16 predicate; header rules; separate ordered A/B builds; nine-row cap per list; only B owns slider refs. paint_group_row / create_department_slider are presentation/external window-factory operations, not row-selection providers. |
| initOrdinanceBudget / updateOrdinanceBudget | INCOMPLETE: same-class callbacks return IDs/state/counts/map or perform refresh; need raw external enumeration/filter/map algorithm (BW17). |
| initNeighborBudget / updateNeighborBudget / combo edit | INCOMPLETE: same-class callbacks return deals/counts or rewrite/refresh; need proposed-deal section/count/selection algorithm (BW17). |
| initIndividualBudget / updateIndividualBudget | INCOMPLETE: same-class callbacks supply +0x44 restore outputs or refresh; need occupant enumeration, record fill and ownership (BW17). |
| UpdateValues | INCOMPLETE: same-class callback; group refresh/scroll-row reuse and repeated header visibility are not implemented core bodies (BW17). |
| get_window_indiv_item | INCOMPLETE: host callback stands in for the documented child/property/type/object lookup; must split into external window/property calls. |
| SetEmptyWindow / SetupWindowBig / info popup | INCOMPLETE: same-class helper callbacks; need core setup/failure ownership; presentation alone may remain external (BW17). |
| end_dialog / show_sequence / image/slider/text helpers | Core end/modal/modeless/show flags implemented; external window callbacks handle presentation/factory calls. No reference to a slider window is stored in SliderInfo. |
| Budget-panel DoSubBudgetWindow | Held-dialog/modal/location/focus/release/destroy/refresh sequence implemented. |
| Panel DoWinProcMessage / ShowWindow slot 6 | Selector/tutorial/tab routing implemented. Taxes/loan/remove/scroll/take commands are returned as PanelCommand, not implemented owner bodies. ShowWindow still delegates UpdateBalanceSheetInfo. |
| Panel DoMessage / UpdateBalanceSheetInfo / loan bodies | Dirty coalescing implemented. Balance-sheet computation and section 5.2 loan record/display updates are incomplete; host refresh is a same-class gap (BW17). |
| Remaining panel vtable slots / undisclosed helpers | Not inventoried by supplied spec; require published contracts, not guessed mappings (BW17). |

External slots used by the implemented ownership/visibility work: department QI-base AddRef/Release slots 1/2; funding pointer slot 10; SetFundingPercentage slot 11; ideal funding slot 14; total income slot 16; line items slot 21; budget GetDepartmentBudgetsInGroup slot 31; line-item GetID slot 3, GetType slot 4, GetFullExpenses slot 10, GetIncome slot 14 and GetDisplayFlags slot 17. The core reads full signed 64-bit amounts and unsigned IDs. Rust Vec growth moves ownership rather than copying/destroying old elements; each live pair/slider still owns exactly one reference, with temporary +1/copy +1/temporary -1 at append.


### Coordinator delivery recovery (2026-10-22)
The worker stopped after Git metadata writes were denied; its original result and recovery evidence are retained. Coordinator reviewed and committed only the reported engine/docs files after the worker exited. The documented gate remains the worker measurement; the integration train must independently gate/regress this commit. Whole-owner and runtime credit remain withheld wherever the report lists missing contracts or bodies. No new lane round was used.
