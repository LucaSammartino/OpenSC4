| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| tools-coverage-functions4: exact function coverage | Codex, `tools-coverage-functions4` | in progress | Implement strict shared entry resolver and validate accounting |

## Handoff
- Goal: conservative specified/implemented exact-entry evidence; no owner promotion or engine changes.
- Done: strict tracked ledger/resolver, union tally/report/audit, split treemap, docs and Registry withholding review implemented. Existing row rules unchanged.
- Focused `python build-runs/validate-functions4.py focused`: 25 tests, OK, return 0; source stable. Log/hash in `build-runs/functions4/focused.json`.
- Earlier fixture attempt: 24 tests, one duplicate-test failure because its helper overwrote qualification. Corrected fixture; no hidden production failure.
- Mechanical Attribution measured Registry 00473B50/70/90 = 32/25/19 B, owner cSC4GameDataRegistry, rule vtable; `build-runs/functions4/metadata.json`.
- Empty ledger: public raw source-matched qualification/logs not supplied as tracked proof; all pilots withheld. No Directory/HTML or map edits.
- Implementation milestone: `9ec5bc8c` (committed normally on detached HEAD; Git printed packed-refs.lock permission diagnostic but did create the commit; no refs/config/maintenance changed).
- ONE full tools suite: `python build-runs/validate-functions4.py full`: 232 tests in 38.609s, OK (skipped=1), return 0, stable source; `full.json` and `full.log` SHA256 881f7d7aef73b9f5596c5f4681344736eeb7123d65dff0653f8adf5eebe84fb8.
- Two actual report and treemap runs pinned to regress-train115.out: all return 0, repeat report/SVG/audit hashes identical. Baseline/audit/headline unchanged: denominator 29536/5584842 B, exclusions 6100/924106 B, all additions zero.
- Report normalized hash c02203611fae009ad008646444815764cf4fc7918f0b5d9ace177a2698e877ab; SVG 30c071bdabbda568e2832c5fb9f788a77bf3bcf2bad730ffec6241cdfcea4d33.
- Next: retain exact integer treemap payload parity check, inspect final diff, replace Handoff with full report and commit; enqueue own READY entry only when clean.
- q.py cannot start Bash (E_ACCESSDENIED); use PowerShell directly. No engine tests/game requests. Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-tools-coverage-functions4-r1-manifest.json`.
