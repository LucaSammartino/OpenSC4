| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-advisors-replace15 (advisors replacement registration) | `Codex` / `glue-advisors-replace15` (engine lane) | in progress | (1) Register factory and test; (2) gate; (3) run replacement games to 2032. |

## Handoff
- Goal: default-off advisors replacement factory, registration ctest, clean replacement games through 2032.
- Done: b592c83e4 adds default-off advisors registration and plugin tests for disabled enumeration and enabled factory/QI/release.
- Next: collect combined game adv-replace15-r2-full-2032; collect original all gate; run original advisors shadow through 2032.
- fb42fb2e8 fixes the object payload and Lua acquisition/GC balance. Native r2 build PASS 51/51; CTest 83/84 (only ret_channel_check failed: missing prior IsUserData [abi-ret] comment; restored). Bounded failure evidence build-runs/native-r2-failure.txt; queue label qbuild-20261024-233200-build-ctest. Functional tests, including new advisor_message_test, passed.
- Native gate: qbuild-20261024-231025-build-ctest PASS, build 269/269 edges, CTest 83/83. Logs build-runs/fc4verify-{build,ctest}.log. New registration/default-off tests pass.
- Gate: qbuild all still running (session 92127); workspace tests have completed, clippy now checking. Collect this same operation. No Rust edits.
- Full profile source: docs/runs/s8n4-c40-45d931a9-20261022b.md lists current40 plus span8; docs/runs/train210-643.md adds dispatch, exemplars, span1555, span4444, span565, span8, span8888, tutorial to current40 (48 keys). Simulator/budget default-on.
- Game adv-replace15-r1-only-2032: corrupt during Init, foreign_exceptions=1, ref_warnings=10. Exact verdict/log build-runs/game-results/adv-replace15-r1-only-2032.{verdict,log}. No outstanding request.
- Diagnosis: our PDB resolves DLL+0x1067440 to Deliver+0x160, called by SendCb and Advisor::base_init. §10 requires IGZUnknown; Deliver used SetVoid1, overwriting data1 and leaving GetIGZUnknown null. Fixed and added message payload/queue lifetime ctest. HostOf QI lacked FC4_ACQUIRED; TagGc leaked its QI on all paths; both fixed.
- Wait helper: build-runs/wait-verdict.py. Symbol helper only reads our DLL/PDB: build-runs/symbolize-plugin.py.
- Game adv-replace15-r2-only-2032: CLEAN through 2032-01-05, ref_warnings=0, foreign_exceptions=0, panics=0, stalls=0; 23.781s simulation, 38s wall. DLL sha256 prefix 8fd49163. Exact verdict/log build-runs/game-results/adv-replace15-r2-only-2032.{verdict,log}.
- Outstanding game: adv-replace15-r2-full-2032, current48 plus advisors, same corrected DLL.
- Constraints: clean-room; do not run/read tools; use lane build queue and game queue.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-advisors-replace15-r1-manifest.json.
