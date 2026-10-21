| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-citydetail-final4: audit cFC4CityDetail candidate; current38 default + citydetail joint run to 2032 | Claude lane, .claude/worktrees/verify-citydetail-final4 | ready: gate PASS (cargo 3980/0, clippy 0, ctest 65/65); vcd4-j1 (current38 + citydetail, turbo to 2032-01-01) clean, ref_warnings=0, full lifecycle incl. destructor; promotion: hold (CD6/CD7 open, lotcfg detail gap) | lotcfg owner: DetailLevel host must read the preference (prop-lotconfig §B3.1); answer CD6/CD7; then coordinator decides citydetail default promotion |

## Report

**Audit (owner boundary, spec city-detail.md §1-§6):** complete. `cFC4CityDetailManager` implements every slot:
QueryInterface (1 / 0x6AD4B9F1 to +0, 0xE4FDA3D4 to +4, else false), AddRef/Release (last Release deletes), Init
(reduce/increase/none, then network lots §3.4, stored := cur every call, CD3), Shutdown (true), GetCityDetail,
serializable Write/Read (version 0, 8 bytes) and GetGZCLSID 0xEAD4BA36. Open questions on the spec: CD6 (no public
way to run the increase pass in game) and CD7 (no two-session reload path); both still open, not re-filed.

**What changed** (097db2da, plugin only, no ABI or Rust change): `cFC4CityDetail.cpp` destructor logs
`cFC4CityDetailManager(p) destroyed (inits, QueryInterface own/serializable)`: a diagnostic showing that the last Release
reaches the deleting destructor.

**Gate** (`qbuild all`, baseline at d68b254a and final): cargo test 3980 passed / 0 failed, clippy 0, CMake build PASS,
ctest 65/65.

**In-game run vcd4-j1** (pinned DLL build-runs/dll/vcd4, sha256 d3f65d45; Konradshohe; turbo to 2032-01-01; script
build-runs/vcd4-script.txt: full save + digest 2028-01-05, fast save + digest 2031-06-01; enable = current38 + citydetail,
41 subsystems enabled including simulator/budget). current38 = aura,automata,citysit,civic,clock24,commercial,crime,
demand,demolition,development,flammability,flora,history,industrial,landfill,landvalue,lotcfg,lotdev,neighbors,netlot,
netmgr,nettool,occmgr,ordinance,parks,plumbing,pollution,power,propdev,propmgr,protection,residential,tract,traffic,
weather,zonedev,zones,simgrids. I derived it from the gex4-r5-union request: its 42 keys minus tutorial/citydetail/exemplars,
plus simgrids, which train46 promoted. The coordinator should check this list against its candidate profile.
- Verdict clean: foreign 0, stalls 0, ref_warnings=0, date 2032-01-01, 205 s.
- Factory hit: `AddCls(0xEAD4BA36) registered`, then `cFC4CityDetailManager(051BB2F0) created`.
- Method hits: `Read -> 1 (level 3)`; `Init: setting=1 level 3->1 pass=reduce removed=1324 created=0 draws=0
  network-lots=525 (lot order: the lot list)`; `Write -> 1 (level 1)` at the full and the fast save (both ok=1);
  `Shutdown -> 1 (level 1, inits=1, QueryInterface own=1 serializable=3 refused=0, GetCityDetail=0)`; then
  `destroyed`, on the same object. That is the native lifecycle (create, Read, Init, Write x2, Shutdown, last Release),
  with balanced references.
- Evidence copied to build-runs/evidence/vcd4-j1.{log,verdict}.

**Finding (outside this owner):** the joint reduce removed 1324 objects. With the original lotcfg (cd4-repl-a) it
removed 8496. The cause is `engine/plugin/cFC4LotConfigManager.cpp` `DetailLevel()` (host for the lotcfg core), which
returns a constant 3 although its comment cites §B3.1. prop-lotconfig.md §B3.1 says d is the city detail preference.
The Rust core (lot_config_manager/manager.rs) honours whatever the host returns. So with lotcfg replaced, the
catalogue is always the detail-3 one, and the reduce pass at preference 1 keeps every level-0x10/0x20 object. The
spec is not ambiguous here, so I filed no question. This is an implementation gap for the lotcfg owner. It also affects
the original city detail manager under current38, so promoting citydetail does not change it.

**Promotion verdict:** not justified yet. The citydetail class is complete and was clean and balanced with the
current38 default through 2032. But (1) the increase pass and a cross-session reload are still unexercised in game
(CD6, CD7 open), and (2) the joint reduce result is wrong because of the lotcfg detail gap. Recommendation: fix lotcfg
`DetailLevel`, answer CD6/CD7 (harness keys), rerun this request (build-runs/game-queue/done/vcd4-j1.req), then
promote.

**Questions filed:** none (CD6, CD7 still open).
