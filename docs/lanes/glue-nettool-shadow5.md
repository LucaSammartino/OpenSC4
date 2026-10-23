| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-nettool-shadow5: scripted network tool shadow | Codex / glue-nettool-shadow5 (detached reserved lane) | **ready: measured diagnostics**, ABI unchanged; full gate PASS, cargo 6694/0, clippy clean, CTest 69/69; turbo and normal clean through 2032, both 5/14 exact, refs=0 | Answer NT27-NT29; complete piece/bridge/powerline adapters and resolve differences before coverage credit or profile enablement |

## Implementation and scope

`nettool_shadow=1` opts into one comparison for every scripted `network` action;
the default is off and there is no environment override. `FC4Script.cpp` runs
the production tool/geometry cores against the pre-placement city through a
whitelist of read callbacks. Terrain setters, occupant insertion/removal,
messages and commit callbacks are absent. The original tool then performs the
scripted real placement; public getters are sampled before Shutdown. Native
references retain the existing audited private-tool ownership path.

The compared outputs are ordered dragged cells, AL placement validity,
failure code, valid/straight/total step counts and integer solution cost.
Both cost getters read post-commit terrain. The script has 14 drags and two
terrain preparations, exercising straight/reversed/diagonal runs, a shared
endpoint bend, T-junction, single cell, clamping, rail/avenue conflicts,
powerline, a water/bridge attempt and a late-2031 placement.

This covers a **public result projection of cSC4NetworkTool**, exercising its
network_geometry core. The cSC4PowerLineTool probe compares inherited public
results against the current generic tool adapter; its pole/wire algorithms
are not compared. Solved network types/piece IDs/orientations, bridge models,
tunnels, slot-8 placement, masked extension and UI paths remain uncovered.
No entire owner is verified. The existing host has no bridge types/model
wiring and no powerline-specific placement wiring. NT27 requests the missing
native solved-result observation contract rather than inventing signatures.

## Gate

Final C++ source is commit `5b266ce51`; subsequent commits change records only.
`qbuild.py all` PASS: cargo workspace **6694 passed, 0 failed, 0 ignored**;
clippy **0 diagnostics**; queued native build **47/47**, CTest **69/69**, 78.9s.
Native request `qbuild-20261024-001410-build-ctest` completed in the same
operation (session 27094, exit 0); no duplicate standalone verify was launched.
Logs: `build-runs/qbuild-{test,clippy}.log`, `build-runs/fc4verify-{build,ctest}.log`.
The earlier full gate also passed (220/220 build, 69/69 CTest).

## Real-city evidence

All requests used Konradshohe, the original network tools, the shipped
`engine/tests/nettool-shadow.script`, and end_date=2032-01-01. All finished
clean with foreign_exceptions=0 and ref_warnings=0; no request remains open.

| Label | Pacing | Last date | Seconds | Exact/total | Record |
|---|---|---|---|---|---|
| nettool-shadow5-r1-20261024-a | turbo | 2032-01-02 | 30.860 | 2/14 | [337](../runs/nettool-shadow5-r1-20261024-a-337.md) |
| nettool-shadow5-r1-20261024-b | turbo | 2032-01-02 | 37.938 | 5/14 | [338](../runs/nettool-shadow5-r1-20261024-b-338.md) |
| nettool-shadow5-r1-20261024-c | normal | 2032-01-01 | 115.984 | 5/14 | [339](../runs/nettool-shadow5-r1-20261024-c-339.md) |

Run a preserves the initial diagnostic source: cached pre-commit replay cost
was compared with post-commit native cost. Commit `5b266ce51` corrects the
sampling phase. Runs b/c use the same DLL, sha256 prefix `a796132d`, and have
identical comparison lines after removing log timestamps/thread prefixes.
Remaining differences therefore persist at normal speed.

Exact log-derived count lines (the forbidden tools/ coverage scripts were
not run by this engine lane; the coordinator must perform its own credit audit):

```text
nettool-shadow5-r1-20261024-b 5/14
nettool-shadow5-r1-20261024-c 5/14
```

One exact line, verbatim from both corrected runs:

```text
nettool shadow (type=3 drag=(14,6)-(4,6) scope=drag/status/cost): mismatches=0 cells=11/11 cell_diffs=0 ok=1/1 failure=00000000/00000000 valid=11/11 straight=11/11 total=11/11 cost=55/55 uncovered=pieces,bridges,poles,wires question=NT27
```

All 14 outcomes, in script order, are identical at both speeds:

| Probe | Mismatches | Native/core difference | Question |
|---|---|---|---|
| road (4,4)-(14,4) | 1 | cost 110/113 | NT29 |
| reverse street (14,6)-(4,6) | 0 | exact | — |
| diagonal road (20,4)-(28,12) | 10 | cells 17/9; cost 170/102 | NT28, NT29 |
| bend road (14,4)-(18,8) | 6 | cells 9/5; cost 90/54 | NT28, NT29 |
| T-junction (9,4)-(9,12) | 0 | exact | — |
| single road (32,4) | 0 | exact | — |
| clamped street (4095,16)-(4095,20) | 0 | exact | — |
| rail (4,24)-(16,24) | 2 | ok 0/1; failure 1/0 | NT28 |
| avenue (4,28)-(16,28) | 28 | 26 ordered-cell differences; ok 0/1; failure 1/0 | NT28 |
| powerline (4,32)-(16,32) | 2 | valid 13/0; cost 65/13 | NT27 |
| rail crossing (9,20)-(9,30) | 2 | ok 0/1; failure 1/0 | NT28 |
| repeated road (4,4)-(14,4) | 0 | exact | — |
| bridge attempt (42,43)-(58,43) | 2 | valid 17/0; cost 974/0 | NT27 |
| late street (64,8)-(74,8) | 1 | cost 55/61 | NT29 |

Full per-drag log lines and native verdicts remain in
`build-runs/game-results/<label>.{log,verdict}`. Run numbers 337-339 were
allocated from this checkout's highest existing run 336.

## Questions and next steps

NT27-NT29 appended to `docs/specs/questions/network-tools.md`: solved-result
observation and adapter limitations; ordered cell/failure=1 contract; cost
and retained height inputs. The coordinator must regenerate the question
index (engine lanes cannot execute tools/ scripts).

READY is for the measured, default-off diagnostics, as instructed for partial
work with missing contracts. The shadow is **not exact**. No coverage tables
or regression profile were edited; do not enable it in the shadows profile
until every line is exact. Next: answer the questions, complete the missing
adapters and resolve the measured differences, then rerun through 2032 and
let the coordinator audit coverage. No C ABI exports/version changed.
