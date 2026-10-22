| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-fonts-contract-complete4: cGZFont published contracts | Codex / `core-fonts-contract-complete4` (engine lane) | in progress | Complete FN15-FN17; audit 64 slots; run full gate. |

## Handoff
- Goal: complete cGZFont core against drawing-fonts sections 7, 9, 12, including FN15-FN17; core only.
- Worktree reused at detached HEAD e517da6; no branch/ref changes permitted.
- Done: b688951c: FN15 owned 477-name map, FN16 integer x87 product/branching, FN17 native counter/return rules; independent tests.
- Baseline all: cargo 4238 passed, clippy 0 diagnostics, CMake 206/206, ctest 67/67. Tree changed during baseline; final stable-tree all required.
- First quick: cargo 2371 passed; link LNK1168 while baseline ctest held DLL; retry after baseline finished.
- Done: all 64 slots traced to font/layout/cache/helper bodies. Corrected unsigned size adjustment, insert-unique loaded records, borrowed renderer getter, failed-Init holder destruction, and zero-line-height fault adapter.
- Independent tests now cover full map and aliases, PC boundaries and all rounding controls, fixed/loaded/failure/done-error paths, renderer reference counts, failed-init final release, lock failures/wrap/null/mode edges.
- Quick retries exposed a missing test-provider method (fixed) and a test oracle assuming distinct codes had distinct names (fixed; normative table shares some names). Final verification pending.
- Next: sequential quick/fmt, stable final all; full 64-slot checklist/report and wip queue if contract residuals remain.
- FN18 appended: nonfinite/out-of-i64 `_ftol2` results unspecified; core returns typed conversion errors, font adapters panic rather than invent output.
- FN19-FN22 appended: paragraph pen/result/trimming, AA precision, incomplete encodings, invalid backing/progress domain. No cGZFont whole-owner or blanket family promotion.
- No game requests outstanding; no ABI changes allowed; cGZFontSys credit remains separate.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-fonts-contract-complete4-r1-manifest.json.
