tag: run-84
run: 84
date: 2026-10-14
lane: run
dll: 7c5c4da6
pacing: unknown
verdict: clean
switches: main `5ee5c52` (DLL `7c5c4da6`), `build-runs/repro74.ps1 -Label run84-soak-x87 -After 900` with all thirteen replacements plus the default simulator and budget (Konradshohe, `observe = 1`): **soak 3 again after the x87 fix**
result: **clean, 0 problems, x87 leftovers 0 for 35 game years** (2026-02 → 2061-09, 428 months; no `x87 leftovers at` line, no stack-fault line): run 74's fault did not recur. **The city path differs from runs 77/78**: the same start, then higher tax income from about 2041 (R$ and I-D taxes; `tools/compare_soaks.ps1`), and the collapse that took runs 77 (ours) and 78 (original) to about −80k by 2061-09 comes later: funds peak at 163k in 2053 and are +79,924 at 2061-09, with R/C/I 20219/2314/5289 against 16134/1583/3205. Runs 77 and 78 already differ by up to 50% in single tax lines, so one soak cannot separate variance from a bias: run 85 repeats the all-original soak with this DLL.
