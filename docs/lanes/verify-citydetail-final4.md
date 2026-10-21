| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-citydetail-final4: audit cFC4CityDetail candidate; current38 default + citydetail joint run to 2032 | Claude lane, .claude/worktrees/verify-citydetail-final4 | in progress | see Handoff |

## Handoff
- Goal: audit citydetail candidate (spec city-detail.md, open CD6/CD7), validate factory/method hits, lifecycle, ref0, joint current38+citydetail run to 2032; report whether default promotion is justified (no promotion).
- current38 = aura,automata,citysit,civic,clock24,commercial,crime,demand,demolition,development,flammability,flora,history,industrial,landfill,landvalue,lotcfg,lotdev,neighbors,netlot,netmgr,nettool,occmgr,ordinance,parks,plumbing,pollution,power,propdev,propmgr,protection,residential,tract,traffic,weather,zonedev,zones,simgrids (derived: gex4-r5-union enable list minus tutorial/citydetail/exemplars, plus simgrids promoted train46).
- Done: baseline gate PASS (cargo 3980/0, clippy 0, build PASS, ctest 65/65). Audit: all 7+6 slots of §1/§6 implemented (QI 1/0x6AD4B9F1/0xE4FDA3D4, Init reduce/increase/network lots, Shutdown, GetCityDetail, Read/Write v0). 097db2da: destructor log (diagnostic).
- Finding: cFC4LotConfigManager.cpp DetailLevel() host returns constant 3 (prop-lotconfig §B3.1 says the preference); core lot_config_manager honours d. In the joint run (lotcfg replaced, pref 1, record 3) reduce keys on d=3 catalogue -> expect removed~0 vs 8496 with original lotcfg (cd4-repl-a).
- Outstanding: vcd4-j1 queued (pinned DLL build-runs/dll/vcd4, sha d3f65d454aa2, script build-runs/vcd4-script.txt). Final gate running (build-runs/final-all.txt).
- Next: qwait build-runs/game-results/vcd4-j1.verdict (no --fresh); grep log for citydetail lines, ref_warnings.
