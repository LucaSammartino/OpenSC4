tag: run-26
run: 26
date: 2026-10-13
lane: run
dll: 569FF4D8…
pacing: unknown
verdict: crash
switches: `budget_default=1, enable = power, power_report=1, observe=1`, DLL `569FF4D8…`
result: **crash** `0xC0000409`, no `AUTOTEST VERDICT` (the line that looked like one is a periodic heartbeat). Power *did* work up to the crash — 14 `power recompute` lines, first reading `production 5997 consumption 1038 demand 1038 powered 385 unpowered 0`, closing run 22's "production 0" gap — but those numbers were written **before** the death. The log ends with two threads interleaved (`tid 23860` in `BudgetSimulator::GetFundingOf`/`GetBudgetItemForPurpose`, `tid 8328` in `GetAllGroups`), which is the cross-thread race found and fixed in run 27. *(This row was originally recorded as clean by the coordinator in `c7d1e51`; that was wrong and is corrected here.)*
