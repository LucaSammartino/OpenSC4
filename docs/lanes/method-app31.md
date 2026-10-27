| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| method-app31: application method outputs (engine) | Codex / method-app31 / reserved detached HEAD; merged main 4edc84efe; code cb374e29b | wip; C ABI unchanged, no seams. Added 16 scalar/pointer getter comparisons against core interface results from independent pre-call owner fields, with post-call field checks; every app-target delivery now checks its acknowledgment and mode-branch owner output, retaining incomplete evidence for missing effects. Focused tests 7/7; final qbuild all PASS: Rust 8385/0/0, clippy 0 diagnostics, CTest 143/143, fmt 1/1; logs WT build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-ctest.log. R4 = main build-runs/game-results/method-app31-scalar-house-r4: claimed slot 1 since 03:06:54; at 03:34:14 no completed log/verdict after three nine-minute qwait timeouts; target 2032-01-01, turbo=1, house shadows plus AP18 diagnostics/reach. DLL SHA256 560AB67E87D5BA7ABACA63E4805E7B7F927CFEEF2E4D201180C4061ADDAA1EED, unchanged after gate. New credited bytes 0; not train-enqueued. Existing questions AP6/AP17/AP18/AP21, no new questions. | Next: 1. Collect R4 via qwait on main build-runs/game-results/method-app31-scalar-house-r4.verdict, then method lines/log/reach; require requested end date, clean/ref_warnings=0, exact supported outputs, no resyncs, matching DLL bytes; never reuse its label. Keep the pending DLL fixed. 2. Resolve value-only AP21 return contracts in docs/specs/questions/application.md before interpreting string returns; capture actual lazy-cache writes/base calls. AP18 still differs on Documents versus slot58 base and Scripts/TestScripts versus Scripts. 3. Bind remaining native return/out/owner/collaborator outputs: prior house reach 113 methods/30762 B; largest SetupWinCtrlMgr 0044D810/4433 B, SetupResources 004572B0/3621 B, CreateFontStyleFromSpecification 005C11B0/2643 B, Init 0044C320/2474 B, SetupFontSet 00453A30/2333 B, PostFrameWorkInit 00453240/2015 B. OnIdle 00449C70/328 B needs pre-timer values/accessors and dispatch effects (AP17); Shutdown 0044A520/983 B needs helper destruction/order (AP6); CloseCity 00455330/528 B, RequestQuit 004546B0/332 B and Quit need city/region/dialog/ref/queue calls. All remain unverified; no exclusions or guessed hook signatures. Prior rank helper WT build-runs/rank-method-app31.py. 4. Obtain checked method credit and engine enqueue only after the whole reached-method acceptance passes. |

R4 counts below are pending, not zero or verified. Getter calls count observer invocations, not total game entries; DoMessage counts all forwarded app deliveries and remains complete=0 without GetType/collaborator tracing. String getters remain complete=0 (AP21; lazy effects also missing). Corrected previous F=method-app31-fixed-r3 was clean/ref_warnings=0, supported 3/3 exact, AP18 cache 0/3, resyncs=0 on old DLL 721948A4; superseded 8e413362 string-complete claims remain invalid. Byte sizes use the prior owner/ABI metadata; slot45/48 alias one function and folded getters belong to shared bodies.
| Method (address or interface slot; bytes) | R4 calls/complete/mismatches | Method (address or interface slot; bytes) | R4 calls/complete/mismatches |
|---|---|---|---|
| 0x00449DE0 GetDataDirectory; 18 | pending | - cISC4App::slot21; 7 | pending |
| 0x00449E00 GetCDDataDirectory; 21 | pending | - cISC4App::slot23; 7 | pending |
| 0x0044BF90 GetPluginDirectory(-1); 121 | pending | 0x0068D1B0 GetStored4C; 4 | pending |
| 0x0044C010 GetCDPluginDirectory(-1); 121 | pending | 0x006EFA20 GetFeatureManager; 7 | pending |
| 0x00449E20 GetSkuSpecificDirectory; 21 | pending | 0x00698CF0 GetRegionOwner; 4 | pending |
| 0x00449E40 GetUserDataDirectory; 21 | pending | - cISC4App::slot36; 53 | pending |
| 0x00449E60 GetDirectorySlot57; 21 | pending | 0x0081F910 GetCity; 4 | pending |
| 0x00449E80 GetRegionsDirectory; 21 | pending | - cISC4App::slot40; 7 | pending |
| - cISC4App::slot59; 84 | pending | - cISC4App::slot41; 7 | pending |
| - cISC4App::slot60; 84 | pending | - cISC4App::slot43; 7 | pending |
| - cISC4App::slot61; 84 | pending | - cISC4App::slot45; 7 | pending |
| - cISC4App::slot62; 84 | pending | - cISC4App::slot46; 7 | pending |
| - cISC4App::slot63; 84 | pending | - cISC4App::slot47; 7 | pending |
| - cISC4App::slot64; 84 | pending | - cISC4App::slot48; 7 | pending |
| - cISC4App::slot65 (AP18); 84 | pending | - cISC4App::slot49; 7 | pending |
| - cISC4App::slot66 (AP18); 114 | pending | - cISC4App::slot50; 7 | pending |
| - cISC4App::slot28; 59 | pending | 0x0080BC00 GetMode; 4 | pending |
| - cISC4App::slot30; 59 | pending | 0x00451ED0 DoMessage; 251 | pending |
