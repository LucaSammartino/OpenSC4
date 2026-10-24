tag: adv-replace15-r2-full-2032-655
run: 655
date: 2026-10-24
lane: glue-advisors-replace15
dll: FreeCity4-glue-advisors-replace15 fb42fb2e8, sha256 prefix 8fd49163
pacing: turbo
verdict: clean
switches: Konradshohe; enable=landvalue,weather,ordinance,demand,power,plumbing,flammability,pollution,protection,crime,aura,flora,landfill,residential,neighbors,zones,development,tract,history,clock24,commercial,industrial,civic,parks,lotdev,zonedev,traffic,automata,netmgr,netlot,nettool,demolition,occmgr,propmgr,lotcfg,citysit,propdev,simgrids,citydetail,ports,dispatch,exemplars,span1555,span4444,span565,span8,span8888,tutorial,advisors; simulator/budget default-on; turbo=1; end_date=2032-01-01; seconds=600; timeout=900; popup_dialogs=0; autotest_end=quit
result: Current48 plus advisors (49 explicit keys) CLEAN through 2032-01-01, 110.766s simulation, 132s wall, 19.55 days/s. foreign_exceptions=0, ref_warnings=0, Rust panics=0, watchdog_stalls=0, rtc=0, overruns=0, structured exceptions=0, incomplete shadows=0, other warnings=0. advisors Init: ok=1, advisors=9, Lua acquired. Profile reconstructed from the explicit current40 list in s8n4-c40-45d931a9-20261022b.md and the eight additions in train210-643.md; no tools script read/run. Exact replace evidence: build-runs/game-results/adv-replace15-r2-full-2032.verdict and build-runs/game-results/adv-replace15-r2-full-2032.log. Source 8beddc753 subsequently restores only the return-channel review comment, without changing runtime behavior.
