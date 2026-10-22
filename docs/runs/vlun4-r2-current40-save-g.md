tag: vlun4-r2-current40-save-g
run: 266
date: 2026-10-22
lane: verify-language-native4
dll: build-runs/language-immutable2/FreeCity4.dll; SHA256 78ea9b099f55d8abd95ea76cea21f34ced2caf72fc68ec305f35da43185c9342
pacing: turbo
verdict: clean
switches: Konradshohe; seconds=600; harness timeout=750; end_date=2032-01-01; disable=budget; language_native_verify=1; refaudit=1; script=build-runs/language-save.txt; popup_dialogs=0; autotest_end=quit. enable=landvalue,weather,ordinance,demand,power,plumbing,flammability,pollution,protection,crime,aura,flora,landfill,residential,neighbors,zones,development,tract,history,clock24,commercial,industrial,civic,parks,lotdev,zonedev,traffic,automata,netmgr,netlot,nettool,demolition,occmgr,propmgr,lotcfg,citysit,propdev,simgrids,citydetail,ports. Exactly40 literal requested keys plus default simulator=41 actual; budget disabled, terrain/dispatch/effects absent.
result: Harness clean2032-01-01 but FAILED ref0 criterion: ref_warnings4, foreign_exceptions0/watchdog_stalls0. Full save returned ok=1 in11031ms on2031-12-01; both full shutdown hooks completed. All four warnings are class0xE990BFFC at airport/seaport DropObject:144, parks DropObject:126 and landfill ReleaseToken:465; retained without claiming a causal fix. Probe:46 objects, strict116235/116235, foreign LocaleHost2484/4140 (1656 differences), gaps2629/3043 (414 differences), refused0, natural0/0-not-exercised, resync0/fallback0, managerUnchanged1. Same immutable DLL; the matched OFF control also reports ref4. Exact-label request/verdict/log retained under build-runs/game-queue/done/ and build-runs/game-results/. LU1-LU4/SL1 open; no full-owner/default/replaced or reload credit.
