| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-nettool-shadow5: scripted network tool shadow, round 2 | Codex / `glue-nettool-shadow5` (detached reserved engine lane) | **ready: measured partial shadow**; NT27-NT29 merged; source `65082c135`; ABI exports unchanged; cargo 6760/0, clippy clean, native 50/50, CTest 69/69; runs 342-345 clean to 2032, refs=0, all 8/14 exact; NT30-NT31 filed | 1. Resolve four retained-height cost probes (NT30). 2. Wire powerline/bridge replay inputs (NT31). 3. Rerun to 2032; enable profile only at 14/14 exact. |

## Changes

Merged the answered contracts from `re-nettool-nt27` as `af12f558f`.
Implementation `1c9f287c8`, conformance fixtures `39c26e615`, native smoke
expectation `6ef6c766d`, and mutation-thread count publication `65082c135`
are committed. Round 1 measured 5/14 exact; round 2 measures 8/14.

- NT28: draw every axis-aligned staircase position, group major-axis rows,
  widen intervals with the two type widths, and emit increasing minor
  coordinates within each row. All fourteen ordered footprints now match.
- Normalize the public failure getter to 0/1, retaining internal reason
  codes in state. The plugin's validation callback now returns transport
  success even for a refused pass, preserving the supplied failure reason.
  Failed smoothing clears the solved price inventory while retaining the
  public footprint. Rail, avenue and crossing refusal probes are exact.
- NT29: retain network corners after successful smoothing and price the
  retained solved inventory against live terrain at each stored vertex ID.
  Placed state does not waive the cell charge; the intersection marker does.
  No additive corrections, fitted tolerances or native totals are used.
- NT27: the replacement coordinate getter replaces caller contents with
  its copy. Replacement step-count getters publish atomic values after
  mutation rather than entering the geometry RefCell on a query thread.
  These replacement-only paths are built/tested; the city runs select the
  original tools for the same-process shadow.
- Added focused tests for staircase rows/width ordering, retained heights,
  live terrain, intersection versus placed state, shared-vertex charging,
  solved-only inventory, maximum type charge, appended neighbours and repeat
  road charge. Updated two old conformance footprints and the native Boolean
  failure expectation to the answered contracts.

## Final gate

Final engine source: `65082c135`. `qbuild.py all` **PASS**:
workspace **6760 passed, 0 failed, 0 ignored**; clippy **0 diagnostics**;
native build **50/50**, CTest **69/69**, total native check **108.6 s**.
Request `qbuild-20261024-011752-build-ctest`, session 42992, collected to
terminal exit 0. Six explicitly owned Rust files pass formatting.
Final DLL SHA256:
`a980ec0bd6c0c529a5e4c0f724cf29f0650fb487cc24427769aa06b68229654c`.
Logs: `build-runs/qbuild-{test,clippy}.log` and
`build-runs/fc4verify-{build,ctest}.log`.

The first full attempt stopped at cargo 807/1 because two diagonal fixtures
encoded the superseded footprint; its native build passed 188/188 and CTest
68/69 exposed the superseded raw-failure smoke expectation. Its complete
logs remain under `build-runs/r2-initial-*.log`. After those input fixes a
full gate passed 6760/0 and 69/69; its logs remain under
`build-runs/r2-before-publication-*.log`. The final repeat above follows the
real diagnostic that new count getters must avoid RefCell access.
All asynchronous checks were collected; no duplicate request was launched.

## Real-city evidence

All runs used Konradshohe, original network tools, and
`engine/tests/nettool-shadow.script`, with end_date=2032-01-01. Every verdict
is **clean**, foreign_exceptions=0, ref_warnings=0. No request is outstanding.

| Label suffix (`nettool-shadow5-r2-20261024-`) | Pacing | Last date | Seconds | DLL | Exact/total | Record |
|---|---|---|---|---|---|---|
| a | turbo | 2032-01-04 | 50.172 | bc85eb39 | 8/14 | [342](../runs/nettool-shadow5-r2-20261024-a-365.md) |
| b | normal | 2032-01-01 | 133.250 | bc85eb39 | 8/14 | [343](../runs/nettool-shadow5-r2-20261024-b-366.md) |
| c | turbo | 2032-01-07 | 72.734 | 28231e1b | 8/14 | [344](../runs/nettool-shadow5-r2-20261024-c-367.md) |
| d | turbo, final artifact | 2032-01-15 | 40.468 | a980ec0b | 8/14 | [345](../runs/nettool-shadow5-r2-20261024-d-368.md) |

All fourteen comparison lines are identical across a/b/c/d after stripping
timestamps/thread prefixes. Each final relink was confirmed in game; run d
uses the final gated artifact. The differences persist at normal speed.
Exact log-derived count lines (the coordinator must perform its own coverage
audit; this lane did not access tools/ coverage scripts):

```text
nettool-shadow5-r2-20261024-a 8/14
nettool-shadow5-r2-20261024-b 8/14
nettool-shadow5-r2-20261024-c 8/14
nettool-shadow5-r2-20261024-d 8/14
```

One exact line, verbatim in all four runs:

```text
nettool shadow (type=6 drag=(4,28)-(16,28) scope=drag/status/cost): mismatches=0 cells=26/26 cell_diffs=0 ok=0/0 failure=00000001/00000001 valid=0/0 straight=13/13 total=13/13 cost=0/0 uncovered=pieces,bridges,poles,wires contract=NT27-NT29
```

## Six remaining probes

All rows below are original/core, identical at normal speed and turbo.
Per-probe mismatch counts in script order are
`1,0,1,1,0,0,0,0,0,4,0,0,4,1`.

| Probe | Remaining difference | Question |
|---|---|---|
| road (4,4)-(14,4) | cost 110/113 | NT30 |
| road (20,4)-(28,12) | cost 170/192; footprint now 17/17, ordered exact | NT30 |
| road (14,4)-(18,8) | cost 90/98; footprint now 9/9, ordered exact | NT30 |
| powerline (4,32)-(16,32) | ok 1/0, failure 0/1, valid 13/0, cost 65/0 | NT31 |
| bridge attempt (42,43)-(58,43) | ok 1/0, failure 0/1, valid 17/0, cost 974/0 | NT31 |
| street (64,8)-(74,8) | cost 55/61 | NT30 |

NT30 and NT31 were appended to
[questions/network-tools.md](../specs/questions/network-tools.md). NT30 asks
for admitted retained per-entry/corner fixtures, including commit-added
neighbours and refreshes; public totals cannot reveal those native inputs.
NT31 asks for concrete subclass resource/host fixtures. Existing bridge-type
enumeration and separate powerline placement wiring remain absent from the
generic replay. The coordinator must regenerate the question index.

## Owners, limits and landing

The comparison covers a **public result projection of cSC4NetworkTool**:
ordered coordinates, placement Boolean, public failure Boolean, valid,
straight and total step counts, and integer solution cost. It exercises
the network_geometry core. The cSC4PowerLineTool probe observes inherited
public results but replays the generic adapter; it does not compare the
powerline pole/wire implementation. Solved type masks, piece IDs,
orientations, bridge models, tunnels, UI, slot-8 and masked placement remain
outside this observation. **No entire owner is verified.** NT27 explicitly
provides no public solved-record getter, and none was invented.

READY is for this measured partial work, as requested. `nettool_shadow`
remains default-off. The shadows profile and coverage maps were not edited
because 14/14 is not exact. No C ABI exports or version changed. Next work
is the six probes above, followed by a new exact 2032 run before profile
enablement or coverage credit. The supervisor publishes the detached HEAD
to the reserved branch; this lane did not switch branches, update shared
refs, merge into main or push.
