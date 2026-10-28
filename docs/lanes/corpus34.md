| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| corpus34: corpus from reach (tools) | Codex / .claude/worktrees/corpus34 / corpus34 (detached); commits aa24885e7..81a2394e9; ABI unchanged | continue: context guard blocked queue write; all gates passed; pytest 405; cargo 8410; clippy pass; ctest 147/147; c34-r1/r2 four clean dated pairs, DBPF 552/552 exact; shadows n/a; questions none | 1. Resume and write the ready merge-queue entry at the actual HEAD. 2. Coordinator records corpus34 references using the corrected driver. 3. Qualify budget-panel and region-view drivers. |

Historical clean census plus admitted traces: **42.348% -> 44.047%**, 2,365,094 -> 2,459,937 bytes, +319 functions / +94,843 bytes (177 census traces).
Frozen golden references separately: 277 reach files, 9,988 functions / 2,222,200 bytes (39.790%); admitted union 10,147 / 2,267,281 (40.597%).
Byte inventory: MAIN/build-runs/run4-owners-current.tsv, attributed by coverage-ranges/map; denominator 5,584,842. Snapshot and SHA256 provenance: build-runs/corpus34-baseline/ (two truncated reference tails explicitly reported; complete rows retained as lower bounds).
Both final batches used DLL 10a83d18c075d1e9db27a5bc10a0e9cb463620ad5c6df271364ebb5ca08a2abd, schedule=fixed / code a33857dd12297904, fixed 64-delivery calendar, seed 0x5EED1234, reach=1, terminate after dated full save.

| Scenario | End date | Marginal functions | Marginal bytes | Refused / total before -> after | Repeat exact / different records |
|---|---|---:|---:|---|---|
| corpus34-tools | 2000-01-15 | 65 | 27,187 | pilot 10/44 -> 0/44 | 135 / 0 |
| corpus34-options | 2000-01-15 | 93 | 17,780 | new -> 0/24 | 132 / 0 |
| corpus34-volcano | 2000-01-15 | 1 | 114 | new -> 0/9 | 132 / 0 |
| berlin-konradshohe (fixed; pilot funded-house) | 2026-04-29 | 0 | 0 | census 11/29 -> 0/30 | 153 / 0 |
Total against frozen references: **159 functions / 45,081 body bytes**, overlap counted once; 214/214 admitted actions across both batches, no pending actions, exact requested dates and clean/ref_warnings=0 verdicts.
New subsystem bytes include tools/input 11,585 and windows/panels 13,511. These are measured entry gains; mouse receipts alone do not assert successful lot construction. No Exact-replaced or shadow credit claimed.
Driver: recover normal client bounds while the fixed schedule keeps windows minimized; never restore/paint. Suite now pins reach_contracts to the requesting checkout, preserving explicit overrides (cached DLLs carried another worktree's prefix).
Real input CLI: python tools/reach_gain.py --owners build-runs/corpus34-baseline/owners.tsv --references build-runs/corpus34-baseline/golden --summary --json build-runs/reach-cli-qualified.json build-runs/suite/c34-r1/corpus34-tools.reach.tsv build-runs/suite/c34-r1/corpus34-options.reach.tsv build-runs/suite/c34-r1/corpus34-volcano.reach.tsv build-runs/suite/c34-r1/corpus34-funded-house.reach.tsv; printed union gain +159 functions / +45081 bytes (snapshot aggregates 277 files into one trace).
Ran python build-runs/qualify-corpus34.py c34-r1 c34-r2: ADMIT tools/funded-house/options/volcano, EXCLUDE panels; comparisons use golden_dbpf.compare_saves with no gameplay masks. Ran census-gain-corpus34.py: census 42.3484496, combined 44.0466713.
Ran python -m pytest tools/tests -q: **405 passed in 227.14s**; targeted suite/reach checks after row promotion: **32 passed**. Earlier tool-only version: 404 passed; retained separately.
Ran MAIN/.claude/lane-bin/qbuild.py --wt WT all once at the end: **8410 passed, 0 failed; clippy exit 0/0 diagnostics; ctest 147/147; qbuild PASS**. Native quick builds were separate development checks.
Excluded budget pilot: c34-r1/r2 both clean/end reached but 10/39 controls unavailable; identical 152-record saves do not qualify refused actions. No panel row added.
Earlier attempts retained separately: c34-p1/p2 wrong calendar/export; c34-f1/f2 corrected calendar but export/input failures; c34-d1 rejected window-selection approach; c34-d2/d3 missing reach contracts and volcano date overrun. Final r1/r2 correct all admission requirements for the four table entries.
Missing: qualified region-view and full budget-subpanel activation. The other historical refused actions remain; census overall was 8,080/30,174 executed actions refused. No changes to FC4Schedule.cpp/.h, coverage-map.tsv, regress.ps1 or C ABI exports.
Artifacts: build-runs/corpus34-evidence.json (per-address/subsystem gains, full repeat diffs), corpus34-census-gain.json, reach-cli-qualified.json, suite/c34-r1 and c34-r2, suite/states/c34-r1 and c34-r2. Coordinator records references after landing; old pinned UI bounds refuse these minimized mouse inputs.

## Handoff

Goal: land the measured corpus work. Code and qualification are complete; no merge-queue file was written because the guard denied that exact operation at context 201k/200k.
Done: aa24885e7 reach_gain.py/tests; e2d17805a initial driver/CLI; 807928fe4 corrected minimized-input bounds plus per-checkout reach contracts/tests; 81a2394e9 qualified scripts/rows; 4c5a1dd04 lane evidence. The rejected window-enumeration approach is absent from the final source.
Next: obtain the actual 40-character HEAD after this handoff commit; create build-runs/merge-queue/corpus34.json with lane/branch corpus34, kind tools, abi false, state ready, that tip, summary of +159 functions/+45081 bytes over references and census 42.348% -> 44.047%. Do not rerun already-passing gates or game repeats without source changes.
Game runs for the queue: suite-c34-r1-corpus34-tools-plain, suite-c34-r2-corpus34-tools-plain, suite-c34-r1-corpus34-options-plain, suite-c34-r2-corpus34-options-plain, suite-c34-r1-corpus34-volcano-plain, suite-c34-r2-corpus34-volcano-plain, suite-c34-r1-corpus34-funded-house-plain, suite-c34-r2-corpus34-funded-house-plain. Exclude panels.
Facts: all spawned shell operations have terminal results. Full pytest 405 passed; qbuild all was run once, cargo 8410 passed, clippy exit 0, ctest 147/147, qbuild PASS. Worktree was clean at 4c5a1dd04; only this handoff is newer. Supervisor publishes detached HEAD to reserved branch corpus34. Schedule sources and ABI unchanged. No question IDs. Missing panel/region activation is explicitly documented above.
