| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| corpus35 / TOOLING corpus reach | Codex / corpus35 / detached HEAD, supervisor publishes corpus35; inherited b160c21e7, 02454aaf2, 941ebf5c6; main merge 821bc9c61; ABI unchanged; seams none | blocked: five original pilots remain queued beyond timeout + 15 min; pytest 406 passed; no completed game/repeat/shadow results; questions none | 1. Unblock the six visible requests. 2. Resume c35-p1, c35-rp1, c35-ap1 and c35-m2. 3. Fix refusals, repeat saves and measure reach before admitting corpus35 rows or removing clockless. |

Round 2 merged main first; FC4Schedule.cpp/.h match main. RECORDING was absent by 06:14 UTC; coordinator golden requests continued to starve the lane. No engine or tooling code changed this round.
Inherited changes: reach_gain baseline/gap reports and overlap tests; corpus35 set accepted; Magnolia leads with establish/funds; Tegel is funded before construction; docs/ops/ui-drivers.md explains founding. No new suite rows admitted; Magnolia retains clockless.
Gate: `python build-runs/run-logged.py pytest-c35-r2.log python -m pytest tools/tests -q` printed `406 passed in 633.47s (0:10:33)`, exit 0.
No qbuild rerun: no new engine changes. Previous round ran qbuild all once: Cargo 8410 passed, Clippy clean, native PASS 3/3, CTest 147/147.
Real input: `python build-runs/run-logged.py reach-r2-final.log python tools/reach_gain.py --gaps --summary --owners C:/Users/tlori/FreeCity4/build-runs/run4-owners-current.tsv --json build-runs/c35-r2-final.json` exited 0.
Printed current total: 492 reference files, 10159 functions, 2268040 / 5584842 game-code bytes (40.611%). Two old truncated references retained complete rows only; this is a lower bound, not the quoted historical census percentage.
Gap priorities printed: windows/panels 295159 bytes; city tools/input 165245; region 83614. The earlier owner analysis found 72999 unreached SubBudgetWindow bytes.
Ran `python tools/suite.py resume c35-p1 --slots 3` and the same for c35-rp1/c35-ap1; no game output. Ran `python build-runs/run-corpus35.py c35-m2 paradis-magnolia`: printed `staged 1 requests`; same d79069d9 DLL/script/seed as c35-p1.
Queue Status confirmed all six requests visible behind coordinator requests; at 06:23 the oldest waited 61m57s. All six remained unclaimed at final inspection; no verdict, reach or end-save files. Waiting clients interrupted (exit 1), preserving requests and manifests. These are incomplete runs, not failed game verdicts.

| Scenario | Before refused/total | After refused/total | Pilot tags | Marginal functions/bytes | Repeat |
|---|---|---|---|---|---|
| paradis-magnolia (fixed) | 0/0; census stalled, 29 pending | unmeasured | c35-p1, c35-m2 | unmeasured | both queued |
| berlin-tegel (fixed) | 8/29; all insufficient funds | unmeasured | c35-ap1 | unmeasured | pending |
| corpus35-budget (candidate) | prior corpus34 pilot 10/39 | unmeasured | c35-p1 | unmeasured | pending |
| corpus35-region (candidate) | new | unmeasured | c35-rp1 | unmeasured | pending |
| corpus35-signs (candidate) | new | unmeasured | c35-ap1 | unmeasured | pending |

Total measured candidate gain: unavailable; **0 functions / 0 bytes credited**. No after-refusal, clean-run or repeat claims. New candidate scripts remain in ignored build-runs/.
Next: unblock the queued requests, then `python tools/suite.py resume TAG --slots 3` for all four tags above. Use build-runs/check-corpus35.py for outcomes/repeats; run-corpus35.py pins c35-p1/dll/FreeCity4.dll (d79069d9).
After qualification measure against main/build-runs/golden with the owners inventory above; verify end dates, zero refusals and target entries before admitting rows. FC4Schedule.cpp/.h untouched; coordinator records references after landing.
