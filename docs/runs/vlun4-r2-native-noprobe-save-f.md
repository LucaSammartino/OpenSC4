tag: vlun4-r2-native-noprobe-save-f
run: 265
date: 2026-10-22
lane: verify-language-native4
dll: build-runs/language-immutable2/FreeCity4.dll; SHA256 78ea9b099f55d8abd95ea76cea21f34ced2caf72fc68ec305f35da43185c9342
pacing: normal
verdict: stalled
switches: Konradshohe; seconds=600; harness timeout=750; end_date=2032-01-01; turbo=0; disable=all; language_native_verify=0; refaudit=1; script=build-runs/language-save.txt (2031-12-01 save); popup_dialogs=0; autotest_end=quit. Native original simulator/budget; request differs from run263 only in label and probe0 versus1.
result: FAILED native OFF full-save control: stalled (fallback) at2031-12-01; raw AUTOTEST watchdog_stalls1/foreign_exceptions0/ref_warnings0. Full save did not return and neither full shutdown hook completed. Probe disabled: no language BEGIN/END, constructed comparison or utility host callback executed; zero verification credit. Same immutable DLL and script reproduce the native save-boundary failure independently of the verifier; shared owned stack candidates and the save/watchdog setup remain evidence, not a proven internal cause. Do not fit language behavior or hide the earlier ON failure. Exact-label request/verdict/log retained under build-runs/game-queue/done/ and build-runs/game-results/. No full-owner/default/replaced, save or reload credit.
