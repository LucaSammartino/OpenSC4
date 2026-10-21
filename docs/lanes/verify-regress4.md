| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-regress4 (train17 vs train19 shadow regression: netmgr 73->71/73, aura 71->70/71, netmgr_shadow ini warning) | `verify-regress4` / (engine lane) | in progress | see Handoff |

## Handoff

- Goal: explain netmgr 73->71/73 and aura 71->70/71 (train17 fed3a354 vs train19 832c7238), fix the
  netmgr_shadow ini warning; fix from specs or file questions. Evidence: build-runs/evidence-run4/.
- Done: 28971497 (aura state-3 lockstep + capi test; FC4Config registers netmgr_shadow/_order/_wide; netmgr NM11
  lot-difference diagnostics; questions NM10, NM11 in questions/network-manager.md, AU16 in questions/aura.md).
  Gate `all` at 28971497: cargo 3577/0, clippy 0, ctest 62/62.
- Diagnosis: no engine code delta for netmgr/aura between the heads; both differences are timing of the game run
  (aura cycle 2 took 11 days vs 9; netmgr lot-state order). Aura: the shadow ran all of state 3 at the 3->4 move,
  the original reads one department per call; a landfill pair inserted in between = 208 tracts / 188 (spec 5.3
  example). Fixed (plugin reads departmentCursor +0x7C, core 3->3 step). Netmgr: lot diffs at 20,103 / 16,93 fell
  outside the NM10 first-row cell set because the lot state message on those cells came later in train19
  (train17 classified the same cells under NM10). Not suppressed: filed NM11, added diagnostics.
- Outstanding game request: vr4-a1 (turbo to 2027-01-01, simulator + aura_shadow + netmgr_shadow).
- Next: qwait build-runs/game-results/vr4-a1.verdict --fresh; grep vr4-a1.log for "aura shadow 20", "netmgr shadow
  (month)", "unknown key"; then finish (lane report, merge-queue json).
