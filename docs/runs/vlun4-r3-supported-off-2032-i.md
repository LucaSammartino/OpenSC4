tag: vlun4-r3-supported-off-2032-i
run: 268
date: 2026-10-22
lane: verify-language-native4
dll: build-runs/language-immutable2/FreeCity4.dll; SHA256 78ea9b099f55d8abd95ea76cea21f34ced2caf72fc68ec305f35da43185c9342
pacing: turbo
verdict: clean
switches: Konradshohe; seconds=420; timeout=600; end_date=2032-01-01; enable empty; disable=budget; language_native_verify=0; ref_audit=1; script=build-runs/evidence-run4/cmdline4-r3-supported-save.script (2031-12-01 save, 2031-12-15 mark persist-control-fullsave-complete); popup_dialogs=0; autotest_end=quit. Actual simulator-only1; budget original. ref_audit warning retained; actual refaudit:on is logged by default.
result: Supported-profile OFF control clean2032-01-04/ref0, foreign_exceptions0/watchdog_stalls0/rtc0/overruns0. Full save returned ok=1 in10953ms at22:10:06.574, executed2031-12-16; mark completed, PostAppShutdown22:10:07.222 and PostSystemServiceShutdown22:10:07.302. OFF completed before ON was queued22:11:55. Same immutable DLL/PDB and copied script as run269; pair differs only in label and probe0 versus1. No language comparison executed, zero verification credit. Four raw WARNING lines (two expected x87 and unknown language_native_verify/ref_audit) retained. This is an owned-simulator/budget-disabled turbo control, not a repaired all-original normal-speed save. All prior failures remain. Exact-label request/verdict/log retained under build-runs/game-queue/done/ and build-runs/game-results/. No full-owner/default/replaced or reload credit.
