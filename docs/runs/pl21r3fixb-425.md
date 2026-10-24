tag: pl21r3fixb-425
run: 425
date: 2026-10-24
lane: glue-propmgr-pl21fix6
dll: main454438d47 plus PL21 capture and static hash-state cleanup dede60ae0, SHA256 8b3b9fefcd616b981c26b9c7f448a5605d99887d3b6ebf1ee08d41ff14d3c26f
pacing: turbo to 2032-01-01, standard replace and shadows profiles
verdict: PASS
switches: tools/regress.ps1 -Tag pl21r3fixb -Dll build-runs/dlls/pl21fix6-r3-fixed.dll -Queue; fixed-source repeat B, identical DLL and ini contents to repeat A
result: Both profiles clean2032, foreign_exceptions=0, ref_warnings=0, watchdog_stalls=0, rtc=0, overruns=0. Replace real141.031 s; shadows real705.250 s. Shadows propmgr lines220 exact220 mismatched0 incomplete0; lotcfg lines293 exact293 mismatched0 incomplete0; 73 query probes,1694 captures,64901 filter checks,input errors0. REGRESS PASS with 0 problems, wall744 s. Operation94349 collected to terminal exit0. Logs/verdicts build-runs/game-results/regress-pl21r3fixb-{replace,shadows}.*; console build-runs/pl21r3fixb-console.log. Final two repeats total propmgr440/440,lotcfg586/586 exact. No qwait or duplicate request; no game/build outstanding.
