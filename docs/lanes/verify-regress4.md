| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-regress4 (train17 vs train19 shadow regression: netmgr 73->71/73, aura 71->70/71, netmgr_shadow ini warning) | `verify-regress4` / (engine lane) | in progress | see Handoff |

## Handoff

- Goal: explain netmgr 73/73 -> 71/73 and aura 71/71 -> 70/71 between train17 (fed3a354) and train19 (832c7238);
  fix the `unknown key 'netmgr_shadow'` config warning; fix from specs or file numbered questions.
- Evidence: build-runs/evidence-run4/regress-train1{7,9}{.out,-shadows.log}.
- Facts so far: netmgr differences are all near lot-state first-row cells (FC4NetMgrShadow.cpp g_nm10Cells, the
  code calls it "NM10 (open)" but NM10 was never filed: QUESTIONS.md next free NM is NM10). train19 mismatches:
  2026-04 lot 20,103 original-only (cfg 0x4A7F8609), 2026-05 lot 16,93 ours-only (cfg 0x4A7F8606); in train17 the
  same 16,93 lot difference was classified NM10 (cell already in the set). train17's previous run was 72/73: flaky.
- Next: confirm cause (cell-set timing), read aura diff line, config key registration.
