| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netocc-replace17 | Codex / `glue-netocc-replace17` (engine) | continue: measured partial; cargo/clippy PASS; CTest 86/86; netocc-line-r17-20261025b clean, shadow 73/73 | Finalize measured partial queue; remaining factories and full-house trial still open. |

Implemented default-off `enable = netocc`, registering only cSC4PowerLineOccupant (0xC9C05C5D).
The adapter uses Rust PowerLine state and its v2 codec, preserves loaded cells and pole identities, and locks getters.
Pole ownership covers aliasing setters, counted getter outputs and partial-read cleanup; Read consults no city services.
Fresh lines use the unindexed box sentinel; loaded Init clears highlighting without posting.
The new factory CTest checks registration, interfaces, position/cell/bounds, removal counters and reference ownership.
Full gate: qbuild all PASS; workspace tests passed; clippy no diagnostics; native build 61/61; CTest 86/86.
Logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log and fc4verify-ctest.log.
The earlier focused compile used core before the sentinel fix and failed that assertion; build-runs/netocc-focused.log preserves it. The full gate passed the corrected test.
Exact tally: `netocc-line-r17-20261025b 73/73`; every shadow line has mismatches=0.
Run record: build-runs/game-results/netocc-line-r17-20261025b.verdict and corresponding .log.
Konradshohe, netocc plus netocc_shadow, turbo, clean 2032-01-10; ref_warnings=0, exceptions=0, panics=0.
Game factory creation is confirmed in the log; power-line samples were exact 73/73 over 1314 records.
Missing: the other twelve occupant factories, inherited lazy placeables and cross-module group enumeration (SPEC-GAP comments).
Full-house trial pending; the current list was requested because tools/regress.ps1 is forbidden to this lane.
Questions: none filed. Public C ABI unchanged. No tools files, coverage tables or run records edited.
This partial delivery does not complete the credit flip or authorize adding netocc to the house list.

## Handoff
Goal: land the measured power-line factory portion; complete netocc credit flip separately.
Done: 2fd03736d adds factory/transport/tests; b3c953421 fixes loaded Init. Full gate and game above are terminal PASS; no requests outstanding.
Next: finalize this report and write the READY partial merge-queue entry using actual HEAD; no queue file was written because the guard stopped finalization. No new build or game run is needed for unchanged source.
