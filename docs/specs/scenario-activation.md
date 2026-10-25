# Scenario activation and owner-level reach

Status: planning and observation contract, 2026-10-25. Round 3 registers nine
dated drivers in the shared `full` suite (section 5). The section-3 table is the
historical candidate list; its Lua drivers remain candidates, and four deliberately
stop with a prerequisite error. Do not count those as completed activation tests.

## 1. What activation means

A scenario activates an owner when positive entry counts identify at least one
of its methods during that scenario's measured interval. This does not establish
that every implemented method ran. Report observed method counts and body bytes
separately from the bytes assigned to the proposed trigger.

Record the actual region, city, initial date, action outcomes, observation window
and build settings. A loaded save, selected tool, parsed script or command return
alone does not establish the intended city change. Require, for example, actual
lot growth, an ignited building, a queried lot, a new resident, a started mission,
a moving boat, or allocation of the requested offscreen target format.

## 2. Candidate drivers and setup

All changes belong to isolated game-slot region copies. Preserve the original
Documents regions. These files add no default behavior: select `.script` files
with the existing opt-in `script` setting, and `.lua` files with the luacmd27
opt-in `lua_script` setting. The latter remains uncalibrated on this host.

The dated files put setup on 2000-01-01 so an already developed city executes it
on its first new day. Preflight that its saved date is later than this date and
observe four subsequent months. Resolve the actual end date from the save before
adding a row to the shared suite manifest. Lua frame waits are observation
intervals, not a claim that four simulated months elapsed. Budget, query, My Sim,
map and mission inputs need the specific additional driver described in the
scenario comments and the RE planning index. Modal closing is unverified.

City names and file sizes establish fixture availability only. A name suggesting
a port or an undeveloped tile does not establish water depth, port state, vacant
cells, funds, demand or utility service. Preflight those conditions independently.
Do not apply the terrain candidate until its small target area is confirmed
vacant. Its second flattening does not reconstruct the original terrain shape.

## 3. Top twenty candidate scenarios

The order is the current owner-gap planning priority. It is not a measured
ranking of additional functions reached. The candidate index is not a suite
manifest: end dates and remaining input prerequisites are still unresolved.

| Rank | Scenario | Region / city | Driver | State |
|---|---|---|---|---|
| 1 | offscreen | FC4 Test / Konradshohe | `engine/tests/scenarios/scripts/owner27-offscreen.lua` | blocked |
| 2 | dialogs | Maxisland / Letroy | `engine/tests/scenarios/scripts/owner27-dialogs.lua` | candidate |
| 3 | growth | Maxisland / West Weburg | `engine/tests/scenarios/scripts/owner27-growth.script` | candidate |
| 4 | networks | Maxisland / Wrensirstown | `engine/tests/scenarios/scripts/owner27-networks.script` | candidate |
| 5 | views | Maxisland / Downtown Weburg | `engine/tests/scenarios/scripts/owner27-views.lua` | candidate |
| 6 | driver | Maxisland / San Indersty | `engine/tests/scenarios/scripts/owner27-driver.lua` | candidate |
| 7 | city | Maxisland / West Weburg | `engine/tests/scenarios/scripts/owner27-city.lua` | candidate |
| 8 | budget | Maxisland / West Weburg | `engine/tests/scenarios/scripts/owner27-budget.lua` | candidate |
| 9 | automata | Maxisland / Downtown Weburg | `engine/tests/scenarios/scripts/owner27-automata.script` | candidate |
| 10 | effects | Maxisland / Wrensirstown | `engine/tests/scenarios/scripts/owner27-effects.script` | candidate |
| 11 | commands | FC4 Test / Konradshohe | `engine/tests/scenarios/scripts/owner27-commands.lua` | candidate |
| 12 | terrain | Maxisland / New City (2) | `engine/tests/scenarios/scripts/owner27-terrain.lua` | candidate |
| 13 | query | Maxisland / Bevopolis | `engine/tests/scenarios/scripts/owner27-query.lua` | candidate |
| 14 | mysim | Maxisland / North Weburg | `engine/tests/scenarios/scripts/owner27-mysim.lua` | candidate |
| 15 | web | FC4 Test / Konradshohe | `engine/tests/scenarios/scripts/owner27-web.lua` | blocked |
| 16 | traffic | Maxisland / Simicon Valley | `engine/tests/scenarios/scripts/owner27-traffic.script` | candidate |
| 17 | coast | Maxisland / Gavin's Port | `engine/tests/scenarios/scripts/owner27-coast.script` | candidate |
| 18 | missions | Maxisland / San Indersty | `engine/tests/scenarios/scripts/owner27-missions.lua` | blocked |
| 19 | resources | Maxisland / East Weburg | `engine/tests/scenarios/scripts/owner27-resources.lua` | candidate |
| 20 | text-effects | FC4 Test / Konradshohe | `engine/tests/scenarios/scripts/owner27-text-effects.lua` | blocked |

## 4. Controls and verdicts

Capture startup, city load, steady-state play, save and shutdown as distinct
intervals. Compare an action scenario with the same city and settings without
that action. Never treat an aggregate reached-function count as an address list,
or a shadow counter as proof that all methods of its owner ran.

The offscreen, web, mission and text-authoring files deliberately fail before
doing work. Keep them out of automated smoke sets until their required drivers
exist. The unregistered candidates need action and scene verification before
admission. Retain failed placements, failed starts and partial runs distinctly.
Shadow or replacement differences are findings; record the scenario, key and
first differing log line without assigning a cause from temporal proximity.

## 5. Registered dated scenarios (round 3)

The shared suite now contains nine `owner27-*` rows in set `full`. These replace
the corresponding section-3 fixture choices and execution files. Six dated
candidates were completed, and terrain, visual-time and resource/save subsets
were translated into the existing dated grammar. The older Lua files are retained
as prescriptions for the additional controls; they are not suite inputs.

| Scenario suffix | Region / city | End date | Action scope |
|---|---|---|---|
| growth | London / Kensington | 2006-08-01 | R/C/I zoning, roads, price query, observation, save |
| networks | Berlin / Schonerlinde | 2000-05-09 | All thirteen network types, crossing, dezone, save |
| automata | Berlin / Tegel | 2029-12-21 | Developed city, daytime, observation |
| effects | Timbuktu / Big City Tutorial | 2016-01-18 | End tutorial, survey, fire attempt, day/night |
| coast | Berlin / Spandau | 2002-01-10 | Three candidate seaport sites, observation, save |
| traffic | London / Fulham | 2007-10-13 | Developed industrial city, observation, full saves |
| terrain | Berlin / Tempelhof | 2000-05-09 | Small height patch, lower/raise surface, save |
| views | Berlin / Konradshohe | 2026-05-28 | Visual time and explicit lighting hours |
| resources | London / Kensington | 2006-08-01 | Load interval, full/fast saves, observation |

Each requests `close_dialogs=1` when run. All source saves remain read-only;
only the queue's region copy may change. Setup dates are overdue so actions
run at the first new simulated day. If a save cannot enter running city mode,
no dated setup action can resolve that prerequisite; report the blocked actions
and actual last date rather than claiming the end window was observed.

Saved-date metadata supplies planned windows, not a guarantee of initial live
state. Native logs must confirm the actual starting date and final date. Record
failed site placements separately from successful save or speed actions.

The command names in the documented grammar must also be supported by the
selected DLL. A parser refusal invalidates the whole file. Keep results from a
stale DLL separate from a rerun with a current DLL; pin both DLL hashes and the
script hashes. A clean process verdict alone does not certify script completion,
successful city changes, exact shadows or target-owner entry coverage.

Native interface calls explicitly logged by a completed action can establish
that entry route. Shadow-state or digest observations support activity of a
family but do not identify every contributing original method. Until address
entry records are available, report these evidence levels separately and leave
function precision/recall and newly reached body bytes unmeasured.
