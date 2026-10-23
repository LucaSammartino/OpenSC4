| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netrules-finish5: finish network rules shadow measurement | Codex / `glue-netrules-finish5` (engine lane) | in progress | Diagnose early game exit, measure through 2032, record gate and READY result. |

## Handoff

- Goal: finish prior netrules shadow with a real-city run through 2032.
- Done: merged glue-netrules-shadow5 at d056c1b9d; working record committed at 57ac32167.
- This lane's stable-source qbuild all TERMINAL PASS: Rust 6696 passed/0 failed, clippy 0 diagnostics; native 223/223 edges; CTest 70/70.
- Native label qbuild-20261024-002209-build-ctest TERMINAL PASS (339.1 s). All session 55736 collected to exit=0; no duplicate verify launched.
- Prior game labels netrules-r3-2032-20261023a/b ended 2028-08-01; 33/33 mismatch-clean, one pre-reload exact=0 line.
- Diagnosis: copied prior logs to build-runs/prior-netrules-a.log and prior-netrules-b.log. b starts funds=834, ends funds=-103615 on 2028-08-01 immediately before unexpected shutdown; bankruptcy evidence, no shadow mismatch.
- Spandau TERMINAL clean/ref0, timeout at 2019-03-29 after 600.969 s, emergency=1. Run 342: netrules-finish5-spandau-2032-20261024a 212/212 mismatch-clean, 101/212 exact, evaluations=24. No 2032 credit; dated script not reached.
- Watcher 69334 timed out (not a game verdict); 61720 printed valid verdict then cleanup failed. Helper corrected to unregister events only, with Renamed notification added. No game outstanding at this commit.
- Next: request Konradshohe, label netrules-finish5-konrad-2032-20261024b, broader 39-provider fixture (run270 profile minus netmgr/netlot/nettool), same script/end date. Keep native network ownership for observation. Wait using corrected build-runs/await-game.ps1.
- Keep profile key out unless every shadow line exact. Existing load line exact=0 is a limitation, not a matched observation.
- ABI inherited from prior lane adds fc4_network_rules_snapshot and fc4_network_rules_replay; report abi=true for this lane's merge queue.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-netrules-finish5-r1-manifest.json.
