| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| maps-cities30: census protection/plumbing/power; kind engine | Codex; .claude/worktrees/maps-cities30; detached HEAD; reserved branch maps-cities30; code 8af3d0d50, questions bfef34c6e | READY for committed fixes only; ABI unchanged; gate PASS; strict census acceptance NOT MET, no owner credit | 1. Collect live maps-cities30-5. 2. Resolve numeric PR/PW questions and retry incomplete cases. 3. Require every census key exact with zero resyncs before credit. |

- Merged main first (8af3d0d50), preserving both additive test-seam cases; existing fixes eba3b7333 (plumbing timing), 1b022d134 (building interface), 5f3a56f52 (query-list ownership) are retained. Comparisons remain strict.
- This round made no engine changes after that merge; 0ccbd0661, e9e069452 and bfef34c6e contain numeric question requests, not guessed arithmetic changes.
- Before: protection 163/176, plumbing 137/176, power 175/176 exact (`C:/Users/tlori/FreeCity4/build-runs/suite/sh28-census3/per_key.tsv`).
- After full census: NOT VERIFIED; the 176/176 target is unmet for every key. No strict suite-verified owner credit is requested.
- Live batch: E=`C:/Users/tlori/FreeCity4/build-runs/suite/maps-cities30-5/`; 52 cases = union of all 39 plumbing, 13 protection and 1 power findings, with reach, shared queue, two in flight (`E/manifest.json`).
- Snapshot S=`build-runs/maps30-round-snapshot.txt`: 12/52 verdicts, 11 suite-clean/end=1; 8 raw-clean, 3 known quit faults, 1 incomplete. Later completions are not included in S.
- Plumbing: 12/12 observed scenarios exact; 11/12 accepted completed scenarios; resynced lines=0 (S, E per-scenario JSON/verdicts). Only 8 of the 39 affected cases have completed validly.
- Protection: 9/12 observed exact, 8/12 accepted; resynced lines=0. Bevopolis, Darris and Indereast remain inexact (S; E corresponding JSON `per_key.protection`).
- Power: 11/12 observed exact, 10/12 accepted; resynced lines=0. Indereast is newly inexact; West Weburg remains pending in this batch (S; `E/maxisland-indereast.json`).
- Provisional baseline plus valid reruns: plumbing 145/176, protection 163/176, power 174/176; this combines old and new evidence, not a completed current census (S; reporter `build-runs/summarize-maps30.py`).
- All 12 observed runs: ref_warnings=0, rtc=0; foreign exceptions max=1 in the three suite-classified known quit faults. Preserve raw corrupt and normalized clean separately (E per-scenario JSON, S).
- Incomplete `gen27-spandau-s2-default`: stalled/end=0, 468 actions pending; fallback says frozen at 2001-09-09 while final plugin heartbeat says 2002-09-04. No acceptance credit; retry under a new label (`E/gen27-spandau-s2-default.json`, shared verdict).
- Darris and Indereast: raw clean, end=1, no known quit faults; 28/1 and 29/0 actions ok/refused respectively; their remaining shadow differences are compatibility findings, not exceptions (E corresponding JSON/logs).
- Quick PASS: 681 capi tests, 226/226 native edges (`build-runs/fc4verify-quick-test.log`, quick build output). Owned Rust fmt: two files, zero diff.
- Full gate run once: Rust 8206 passed/0 failed, clippy 0 diagnostics (`build-runs/qbuild-test.log`, `build-runs/qbuild-clippy.log`).
- Native gate: 29/29 build edges, CTest 142/142 PASS (`build-runs/fc4verify-build.log`, `build-runs/fc4verify-ctest.log`). The tree-change notice was a docs-only commit; engine source stayed unchanged.
- Suite DLL SHA256: bae49534439bbfa7ddd0c888c504e3c8588576759aa5d15b4214df73b9519c49 (`E/manifest.json`); subsequent full-build DLL: 93c685b6faf0aa860df514b6b09fd42bcb0819df053aea34969d6658076d3281, same engine sources.
- PR-MAPS30-01 (`docs/specs/questions/protection.md`): numeric saved-cell/rebuild/cone tuples and original x87 control word requested; item stopped pending values.
- PW-MAPS30-01 (`docs/specs/questions/power.md`): numeric West Weburg ageing intermediates and original control word requested; item stopped pending values.
- PW-MAPS30-02 (same file): new Indereast block-order numeric transcript requested; state 1 has 460 next cells not exact and wear 0.16816721856594086/0.16816723346710205 (`E/maxisland-indereast.log:6681`); item stopped, no guessed fix.
- Earlier invalid results remain separate in `build-runs/maps30-failed-checks.json`; old corrupt suite maps-cities30-2 supplies no acceptance evidence. The reporting wrapper failure is separately logged in S; direct reporting succeeded.
- Ready enqueue is for the finished committed fixes per FINAL ROUND, with the passing gate; it does not assert strict census acceptance or flip owner credit.
- Next: collect the already-running exec session 42144 (or wait for `E/summary.tsv`), do NOT relaunch `build-runs/rerun-plumbing.py` or reuse the tag; retry incomplete cases under new labels, obtain the numeric answers, then require every census key exact with zero resyncs before credit.
