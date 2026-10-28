| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| corpus35 / TOOLING corpus reach | Codex / corpus35 / detached HEAD, supervisor publishes corpus35; b160c21e7, 02454aaf2, 941ebf5c6; ABI unchanged | blocked: reference recording starved game requests past timeout + 15 min; pytest 406; cargo 8410; clippy clean; ctest 147/147; game/shadow/repeats unmeasured; questions none | 1. Resume the three staged batches. 2. Fix refusals, repeat on the pinned DLL and compare end saves. 3. Measure gains, remove Magnolia clockless only after qualification, register passing corpus35 rows. |

Changed: reach_gain supports baseline-only reports and byte-ranked remaining subsystem gaps, with overlap/conservation tests; suite accepts corpus35.
Magnolia now leads with the inherited establish action and funds; Tegel receives funds before its eight money-refused census actions. Founding prerequisites documented in docs/ops/ui-drivers.md.
No new suite rows admitted; Magnolia retains clockless pending two clean advancing runs. Candidate scripts remain in build-runs/.
Gate: `python -m pytest tools/tests -q` printed `406 passed in 570.88s`; focused reach_gain tests printed `4 passed in 0.27s`.
Gate: `python C:/Users/tlori/FreeCity4/.claude/lane-bin/qbuild.py --wt C:/Users/tlori/FreeCity4/.claude/worktrees/corpus35 all` ran once: Cargo 8410 passed, Clippy 0 diagnostics, native build PASS 3/3, CTest 147/147, qbuild PASS.
Real input: `python build-runs/run-logged.py reach-gap.log python tools/reach_gain.py --gaps --summary --owners C:/Users/tlori/FreeCity4/build-runs/run4-owners-current.tsv --json build-runs/c35-gap-baseline.json` exited 0.
It printed current 358 reference files, 10155 functions, 2267839 / 5584842 game-code bytes (40.607%); two truncated references retained complete rows only: a lower bound during RECORDING, not the historical census percentage.
Gap priorities printed: windows/panels 295159 bytes; city tools/input 165441; region 83614; SubBudgetWindow accounts for 72999 unreached bytes in the owner analysis.
Pilots ran `python build-runs/run-corpus35.py c35-p1 paradis-magnolia corpus35-budget`, `... c35-rp1 corpus35-region`, `... c35-ap1 berlin-tegel corpus35-signs`: printed staged 2, 1, 2 requests respectively; all stayed queued, no game verdict by 05:56 UTC.
Queue Status confirmed all five visible behind coordinator golden requests. Awaiters were interrupted (exit 1); requests, staged scripts, reach paths and export destinations remain intact. These are incomplete runs, not failed game verdicts.

| Scenario | Before refused/total | Pilot tag | Marginal functions | Marginal bytes | Repeat |
|---|---|---|---|---|---|
| paradis-magnolia (fixed) | 0/0; census stalled, 29 pending | c35-p1 | unmeasured | unmeasured | pending |
| berlin-tegel (fixed) | 8/29; all insufficient funds | c35-ap1 | unmeasured | unmeasured | pending |
| corpus35-budget (candidate) | prior corpus34 pilot 10/39 | c35-p1 | unmeasured | unmeasured | pending |
| corpus35-region (candidate) | new | c35-rp1 | unmeasured | unmeasured | pending |
| corpus35-signs (candidate) | new | c35-ap1 | unmeasured | unmeasured | pending |

Total measured candidate gain: unavailable; **0 functions / 0 bytes credited**. No reach files or repeat saves exist yet, so no after-refusal figures are claimed.
Next: `python tools/suite.py resume c35-p1 --slots 3`, then resume c35-rp1/c35-ap1; use build-runs/check-corpus35.py for outcomes/repeats. New repeats via run-corpus35.py pin c35-p1/dll/FreeCity4.dll (d79069d9), not the later main DLL.
After qualification use tools/reach_gain.py against main/build-runs/golden with the owners inventory above; inspect actual save dates and target-method entries before admitting rows. FC4Schedule.cpp/.h untouched; main was already the specified 0a970d24c base.
