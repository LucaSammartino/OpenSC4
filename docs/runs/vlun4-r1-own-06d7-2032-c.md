tag: vlun4-r1-own-06d7-2032-c
run: 262
date: 2026-10-22
lane: verify-language-native4
dll: build-runs/language-immutable2/FreeCity4.dll; SHA256 78ea9b099f55d8abd95ea76cea21f34ced2caf72fc68ec305f35da43185c9342
pacing: turbo
verdict: clean
switches: Konradshohe; seconds=600; harness timeout=750; end_date=2032-01-01; enable=simulator; disable=budget; language_native_verify=1; ref_audit=1; script=build-runs/language-save.txt (2031-12-01 save); popup_dialogs=0; autotest_end=quit. Actual enabled owner set: simulator only; budget original. Unknown language_native_verify/ref_audit config warnings retained; the probe reads its flag directly and actual refaudit:on is logged.
result: Clean2032-01-08/ref0, foreign_exceptions0/watchdog_stalls0; full save returned ok=1 in11078ms on2031-12-08, then both full shutdown hooks completed. Corrected immutable artifact balances native creation audit holds and separates question observations:46 objects, strict116235/116235, foreign LocaleHost2484/4140 (1656 differences), gaps2629/3043 (414 differences), refused0, natural0/0-not-exercised, resync0, fallback0, managerUnchanged1. All raw differences and all earlier warnings/failures are preserved; production algorithm was not fitted to them. This proves the tested simulator-only/budget-disabled full-save profile, not an all-original simulation profile. Exact-label request/verdict/log retained under build-runs/game-queue/done/ and build-runs/game-results/. LU1-LU4/SL1 unresolved; no full-owner/default/replaced or reload credit.
