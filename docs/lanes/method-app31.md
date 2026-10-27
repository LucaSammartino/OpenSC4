| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| method-app31: application method evidence (engine) | Codex / method-app31 / reserved detached HEAD; code c08a422d1 then corrected ba001c556 | wip; ABI unchanged. Final gate: cargo 8383 passed/0 failed/0 ignored, clippy 0 diagnostics, CTest 143/143, fmt 1/1; logs `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-ctest.log`. F = main `build-runs/game-results/method-app31-fixed-r3.log`: clean, end 2026-03-01, ref_warnings=0, supported 3/3 exact, cache 0/3 (AP18), resyncs=0; DLL 721948A4. Superseded DLL 8e413362: `suite-method-app31-house-r2-play-shadows.log` clean/19 supported samples/stopped 2027-07-12 before requested 2032; `suite-method-app31-gen-r1-gen27-konradshohe-s1-default-shadows.log` clean/25 supported samples/end 2028-01-28, ref_warnings=0; their string-method complete claims are INVALID (AP21). Table is F only; calls count observer invocations. New owner bytes verified: 0; not train-enqueued. | (1) Resolve AP21 in `docs/specs/questions/application.md`: SDK bool returns absent from spec; all string returns unobserved. Lazy caches additionally lack owner-write/base-call capture; AP18 differences are Documents versus slot base and Scripts/TestScripts versus Scripts. (2) Remaining methods need full return/out/owner/collaborator capture: startup resources/fonts/controls and Init; OnIdle 0x00449C70 (328 B) timers/effects (AP17); Shutdown 0x0044A520 (983 B) destruction (AP6); CloseCity/RequestQuit/Quit city/region/dialog/ref/queue calls. AP7-AP9 answers already implemented/tested. (3) Audit legacy 14 methods/882 B credit, bind remaining methods, rerun house through 2032, obtain checked method credit, then enqueue engine. Current generated owner is 321 methods/86846 B; house reach 113/30762 B, gen reach 121/34064 B (main `build-runs/suite/method-app31-{house-r2,gen-r1}/*.reach.tsv`; WT `build-runs/method-app31-owners.tsv`). |

| Method / spec address or interface slot (F) | Bytes | Calls | Complete | Mismatches |
|---|---:|---:|---:|---:|
| 0x00449DE0 cSC4App::GetDataDirectory | 18 | 3 | 0 | 0 |
| 0x00449E00 cSC4App::GetCDDataDirectory | 21 | 3 | 0 | 0 |
| 0x0044BF90 cSC4App::GetPluginDirectory (index -1) | 121 | 3 | 0 | 0 |
| 0x0044C010 cSC4App::GetCDPluginDirectory (index -1) | 121 | 3 | 0 | 0 |
| 0x00449E20 cSC4App::GetSkuSpecificDirectory | 21 | 3 | 0 | 0 |
| 0x00449E40 cSC4App::GetUserDataDirectory | 21 | 3 | 0 | 0 |
| 0x00449E60 cSC4App::GetDirectorySlot57 | 21 | 3 | 0 | 0 |
| 0x00449E80 cSC4App::GetRegionsDirectory (slot58 user base) | 21 | 3 | 0 | 0 |
| - cISC4App::slot28 GetAppIniFileName | 59 | 3 | 0 | 0 |
| - cISC4App::slot30 GetAppPreferencesFileName | 59 | 3 | 0 | 0 |
| 0x0080BC00 cSC4App::GetMode (folded traffic-owner body) | 4 | 3 | 3 | 0 |
| - cISC4App::slot59 GetUserPluginDirectory | 84 | 3 | 0 | 0 |
| - cISC4App::slot60 (regions) | unknown identity | 3 | 0 | 0 |
| - cISC4App::slot61 GetMySimDirectory | 84 | 3 | 0 | 0 |
| - cISC4App::slot62 GetAlbumDirectory | 84 | 3 | 0 | 0 |
| - cISC4App::slot63 GetHTTPCacheDirectory | 84 | 3 | 0 | 0 |
| - cISC4App::slot64 GetTestScriptDirectory | 84 | 3 | 0 | 0 |
| - cISC4App::slot65 GetExceptionReportsDirectory (AP18) | unknown identity | 3 | 0 | 3 |
| - cISC4App::slot66 (scripts; AP18) | unknown identity | 3 | 0 | 3 |
