| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-automatarefs4: the 238 ref warnings of the saved-city control run (controller manager teardown) | engine implementer, `.claude/worktrees/verify-automatarefs4` | in progress | see Handoff |

## Handoff

- Goal: sg4-c1 (glue-simgrids4, 36 subsystems, no simgrids, script `2031-06-01 save`) gives ref_warnings=238;
  find the owner of the last references and fix only from the public contracts.
- Diagnosis (sg4-c1.log, refaudit summary lines 13358-13361 / 56373-56374): all 238 are the controller
  manager core's references to game controllers read from the save (generator 0x4A1628C5: 214 acquired by
  ReadRef's QueryInterface, 214 last-ref releases; attractor 0x6A1628BA: 25 / 24), released by
  `fc4_automata_cm_destroy` -> HostRelease (cFC4Automata.cpp:53) from ~cFC4AutomataControllerManager.
- Lifetime contract: foundations.md 2.7 SV1 (the load's object table keeps +1 until the loaded segment
  closes; a save closes it, building-development.md 1.x table "loaded, then saved": the holder's release is
  the last) + automata.md 3.4 (the manager keeps the reference Read returned) + 7.1 (removed controllers are
  shut down and released by the manager). So after a save the manager's reference is legitimately the last.
- Plan: cm host gets `user` = the manager; a `destroying_` flag around fc4_automata_cm_destroy; the cm
  release of a game object while destroying uses FC4_RELEASE_AT_DESTRUCTION (audit still requires the
  acquisition). Test via a seam op. File AT21: where the original drops controller refs at city close.
- Done: 7893f2db fix (CmHostRelease + destroying_), seam op 69 + plugin_load_test checks, question AT21.
  Gate: baseline all PASS (cargo 3756, clippy 0, ctest 63/63); after fix all PASS (3756, 0, 63/63).
- Outstanding: game request vam4-c1 (exact copy of sg4-c1.req, script build-runs/sg4-save-j1.txt).
- Next: wait `qwait.py <WT>/build-runs/game-results/vam4-c1.verdict` (no --fresh); grep vam4-c1.log for
  "released last" and "Release (at our destruction)"; then a no-save control vam4-n1.
