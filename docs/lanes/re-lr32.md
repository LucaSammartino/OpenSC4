| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lr32: lot road-access contract; kind docs (RE) | Codex / .claude/worktrees/re-lr32 / reserved re-lr32, detached HEAD | Ready; spec commit 751ee081a; doccheck passed; lot-records question diagnostics 0; global question diagnostics 14 inherited; cargo/clippy/ctest n/a; game/shadow n/a; ABI exports unchanged, no seam/ABI reservations; replace-lot29 LR8 answered as LR16 | 1. Engine lane use network slot 9 and translate T89–T93 plus travel case into tests. 2. Run the specified paired observation/rerun for acceptance (a). |

- §13.2.5 now requires HasAnyNetworkFlag(0x449), network slot 9 (+0x24), for normal road access.
- Native evidence: mask/call/branch 0x006C1BD1–006C1BDB; any-bit body 0x00604D30–00604D42; all-bits body 0x00604D10–00604D25.
- §17.10 adds T89–T93: individual/extra flags, negative masks, cache refresh, purpose short-circuit and QI/list order; residential counter example fails on visit 3 only with false road access.
- Static mechanism: false direct access can feed TestTravel and reason-4 abandonment; no claim that this explains all measured income/population loss or city termination.
- §13.2.5 specifies the exact per-lot road/cache/travel observation and predicate-only house rerun needed; acceptance (a) remains a runtime obligation.
- Imported question file from replace-lot29, retaining main's LR6/LR7 and all engine files. Main already archived unrelated LR8/LR9: source LR8 is LR16 (answered), source LR9 is LR17 (preserved open, outside this task).
- Answer archive: docs/specs/questions/answered/lot-records.md; generated index fresh, lot-records open/answered 3/14, next LR18.
- Gates: direct python tools/doccheck.py check PASS; git diff --check PASS; global questions.py check FAIL with 14 diagnostics, byte-for-byte identical to starting a3085b97ff8c5f2642be89d1c2664e7e1850141b; no lot-records diagnostics.
- Baseline comparison helper: build-runs/re-lr32-questions-check.py (ignored). Initial q.py invocation failed in Git Bash CreateFileMapping before doccheck ran; subsequent direct doccheck passed.
- Existing lot29-r6-house-on1/off April DIGEST lines 16634/11925 checked: income 2957/4107, R 12789/12797, C/I and expense equal. No new game run; reported ON ended 2029-04-01 / OFF clean to 2032; shadow n/a.
- Export/decomp supplied the evidence; REA unused. No proposed ABI rows or C ABI changes. No engine files read or edited.
- Next: replace-lot29 applies the specified predicate and tests, then establishes runtime causality before closing acceptance (a); route imported LR17 separately.
