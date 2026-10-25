# Round 3 scenario execution

The latest recorded run of each scenario uses its committed script (SHA-256 checked).
All runs use isolated region copies, the shadows profile and `close_dialogs=1`.

| Batch | Runs | Clean process | End reached | Actions OK / failed / pending | Exact shadow samples |
|---|---:|---:|---:|---|---|
| triggermap27-r3 | 9 | 9 | 7 | 28 / 14 / 33 | 3733 / 3772 |
| triggermap27-r3-current | 9 | 9 | 7 | 62 / 2 / 29 | 5758 / 5844 |
| triggermap27-r3-fixtures | 3 | 3 | 3 | 34 / 3 / 0 | 2310 / 2348 |
| final_selection | 9 | 9 | 9 | 88 / 3 / 0 | 7186 / 7291 |

The first batch used stale main-checkout DLL `767709b839eb1aca7c16b4d0673c214241f2c92546b579c102620d787d586d54`.
The current and fixture-correction batches pin train-integration DLL `495d3f939c197eab2a4aa23515aad9ad5c5ec8c498a282dbf0355efe9a9abd86`.
Default-DLL parser refusals, corrected-source runs and later fixture corrections remain separate.
Schonerlinde and Tempelhof did not advance beyond 2000-01-01 in either DLL batch; dated actions never ran.
The final suite uses advancing Kensington and Konradshohe fixtures for those scenarios.
Spandau built the first seaport candidate. Two already-built fallback skips were removed before the final replay.

## Log-supported target owners

The final selection provides named callback or explicit interface-route evidence for **20** of the 73 primary target owners.
This is observed activity during the run, not a gain over an idle control and not full method reach.
Interface routes and named returned simulator callbacks are recorded separately in `scenario-observations.tsv`.
Its evidence cells give original log line numbers and callback-event counts; unmatched shadow families receive no owner-entry credit.
No per-address reach file was produced. Function precision/recall, newly entered functions and newly reached bytes remain unmeasured.

| Final scenario | Primary target owners supported by logs |
|---|---|
| owner27-automata | - |
| owner27-coast | - |
| owner27-effects | - |
| owner27-growth | cSC4AuraSimulator,cSC4BuildingDevelopmentSimulator,cSC4CivicBuildingSimulator,cSC4DemandSimulator,cSC4GrowthDeveloper,cSC4LandValueSimulator,cSC4LotDeveloper,cSC4PropDeveloper,cSC4ResidentialSimulator,cSC4TractDeveloper,cSC4ZoneManager |
| owner27-networks | cSC4NetworkManager,cSC4NetworkTool |
| owner27-resources | - |
| owner27-terrain | cSC4FloraSimulator,cSC4WeatherSimulator,cSTESurfaceWater,cSTETerrain |
| owner27-traffic | cSC4TrafficSimulator |
| owner27-views | cSC424HourClock,cSC4LightingManager |

A null fire-instance return did not mean failed ignition: the effects scenario logged fire count 0 to 1 with active/ongoing set.
The growth scenario accepted all three 64-cell R/C/I rectangles. This does not certify subsequent buildings or all growth-owner methods.
Port construction alone does not establish navigable water, moving boats or every port-owner path.
Shadow differences/resyncs and script/observation findings are listed by key, scenario and first log line in `scenario-findings.tsv`.
No mismatch diagnosis or engine repair was attempted; function-compare counters are recorded as emitted, and no replace run was made.
Raw requests, staged scripts/DLL, hashes, JSON, summary.tsv, per_key.tsv, verdicts and complete logs remain under this worktree’s build-runs/suite/<tag>/.

