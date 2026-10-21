| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lotcfg-detail4: lotcfg DetailLevel host reads the city-detail preference (prop-lotconfig §B3.1/§B3.2) | `glue-lotcfg-detail4` / (engine lane) | ready: code 821caf1e; no ABI change; gate PASS (cargo 4099/0, clippy 0, ctest 65/65); lcd4-j1 (current38 + citydetail, turbo to 2032-01-01) clean, ref_warnings=0; pref 1 drives lotcfg catalogue level 1 (1909 configurations, 282 families) and citydetail reduce 3->1 (removed 8496, was 1324 with the hardcoded 3); PL22 filed | 1. answer PL22 (and CD6/CD7) for an in-game reload; 2. coordinator decides citydetail default promotion |

## Report

**Change** (engine/plugin/cFC4LotConfigManager.cpp only): the host callback `DetailLevel` returned a hardcoded 3. It now
returns `fc4::CityDetailSetting()`, the reader the city detail manager already uses: `cISC4App::GetPreferences`
(slot 38, borrowed, no Release; the cISC4App reference from QueryInterface is FC4_ACQUIRED/FC4_RELEASE) and the
public SDK field `SC4Preferences::videoPreferences.cityDetail` (+0xF16). No new raw address, no C ABI change, no core
change. The value is passed through unclamped (the spec names only 1..3; noted in a code comment and in PL22). The
core's reload and family-set semantics are untouched: the core reads `detail_level` at StaticInit and at each slot-5
load, as before. Those paths (rebuild on a detail change, family set kept) are covered by the existing core and
C ABI tests `t8_rebuild_on_detail_change` and `the_catalogue_and_its_rebuild_on_a_detail_change`, so I added no new
test.

**Instrumentation:** `cFC4LotConfigManager::Init` logs `lotcfg Init: pref cityDetail=P catalogue loaded a->b level
a->b configurations a->b families a->b reload=yes|no (inits=N reloads=M) ok=1`. It reads the process catalogue
state before and after `fc4_lot_config_init`, and keeps process-wide atomic counters.

**Gate** (`qbuild all`): cargo test 4099 passed / 0 failed, clippy 0 diagnostics, fc4verify build 197/197, ctest 65/65.

**Game run lcd4-j1** (pinned DLL build-runs/dll/lcd4, sha256 ede2e288; Konradshohe; turbo to 2032-01-01; 41 subsystems:
current38 + citydetail, plus simulator/budget; script build-runs/lcd4-script.txt: mark 2027-01-05, digest + full save
2028-01-05, digest + fast save 2031-06-01):
- Verdict clean: foreign 0, stalls 0, rtc 0, overruns 0, ref_warnings=0, date 2032-01-01, 168 s.
- `lotcfg Init: pref cityDetail=1 catalogue loaded 0->1 level 0->1 configurations 0->1909 families 0->282 reload=no
  (inits=1 reloads=0) ok=1`
- `citydetail Read -> 1 (level 3)`, then `citydetail Init: setting=1 level 3->1 pass=reduce removed=8496 created=0
  draws=0 network-lots=525`. In vcd4-j1 the same city and preference gave removed=1324 while the catalogue was the
  detail-3 one. The lot configurations now carry the level-1 object arrays, so the reduce pass removes what level 1
  drops. Both managers read the same preference value (1).
- Saves: full `ok=1` (13047 ms) and fast `ok=1` (468 ms), each with `citydetail Write -> 1 (level 1)`. All 5 script
  actions ran. `cFC4CityDetailManager destroyed (inits=1 ...)` at the end.
- No lotcfg shadow ran: the propmgr/lotcfg shadow needs the original class.

**Not observed in game:** a catalogue reload (§B3.1 step 2). A slot runs one city per process at a fixed preference
of 1. No published script action or harness key changes the preference or opens a second city, so the reload path is
verified only offline. Filed **PL22**, which also asks whether a preference outside 1..3 can occur. I make no claim
about a reload in game.

**Questions filed:** PL22 (docs/specs/questions/prop-lotconfig.md).
