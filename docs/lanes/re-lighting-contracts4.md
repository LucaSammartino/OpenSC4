| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lighting-contracts4: LT5–LT11 native lighting contracts | Codex / `re-lighting-contracts4` (RE docs) | in progress | Verify owner inventory, then projection/terrain rectangle and lifecycle/numeric contracts. |

## Handoff
- Goal: resolve LT5–LT11 from original instructions/data; specs only, no engine reads/edits.
- Done: reused supervisor-reserved detached worktree; read short handoff, lane/merge formats and lighting questions.
- Done: individual 43+2 owner checklist and six attributed helpers; LT6 Euclidean/x87 contract, LT7 vector scan, LT8 exact double epsilon/upper clamp, independent vectors added.
- Commits: `69fd2874` inventory/distance/storage/epsilon; `82a5c8c2` lifecycle/statics/terrain rectangle/messages.
- Done: LT5–LT9 and LT11 answered; LT10 partial count-first argument order, input/output storage, basis, signed double cutoff/NaN/failure boundary; nine new independent vectors (10–18).
- Measured: manager 43 bodies/21714B, image 2/517B; local documentation inventory/numeric/question checks PASS; git diff check EOF fix pending recheck.
- Next: commit reconciled questions/projection boundaries, run docs checks, replace Handoff with final wip report and queue.
- Remaining: LT10 complete transform/precision composition still unproved; do not promote complete-owner coverage.
- Facts: epsilon binary64 0.001; red reciprocal binary32 0x3B808081; min scan includes vector padding; RGB test strict greater, unordered fails.
- Evidence sources: main checkout `C:/Users/tlori/FreeCity4`; query export first via tools/re_query.py, decomp fallback.
- Guard: `C:/Users/tlori/FreeCity4/build-runs/run4-re-lighting-contracts4-r1-manifest.json`; max100 calls, one round.
- Must commit substantive proved contract by call20–30; update this handoff and guard after milestones.
- No ABI changes/game runs requested; portable core choices in questions are provisional, not native evidence.
