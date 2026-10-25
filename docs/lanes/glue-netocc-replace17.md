| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netocc-replace17 (partial power-line replacement) | Codex / `glue-netocc-replace17` (engine) | ready partial: cargo 7527/7527; clippy clean; native 49/49; CTest 86/86; netocc-line-r17-20261025b clean, shadow 73/73 | 1. Implement remaining twelve factories. 2. Run netocc with the current house replace list before credit flip. |

Default-off `enable = netocc` registers only cSC4PowerLineOccupant (0xC9C05C5D).
The Rust-backed adapter preserves loaded cells and pole identities, uses the v2 codec and locks getters.
Ownership handles aliasing setters, counted getter outputs and partial-read cleanup; Read accesses no city services.
Fresh lines use the unindexed box sentinel; loaded Init clears highlighting without posting.
The shadow mapping hydrates live power-line state from v2 records.
Factory CTests cover registration, interfaces, cell/bounds, removal counters and reference ownership.
Tests leave twelve factories native, isolate factory instances and check destructor release of poles without Shutdown.
Shadow-only configuration registers none of the thirteen occupant factories.
Final qbuild all PASS: Rust 7527 passed, zero failed; clippy zero diagnostics; native build 49/49; CTest 86/86.
Native request: qbuild-20261025-030059-build-ctest; terminal PASS, no operation outstanding.
Logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log.
Exact tally: `netocc-line-r17-20261025b 73/73`; every shadow line has mismatches=0.
Run evidence: build-runs/game-results/netocc-line-r17-20261025b.verdict and corresponding .log.
Konradshohe, netocc plus netocc_shadow, turbo, clean 2032-01-10; ref_warnings=0, exceptions=0, panics=0.
Game factory creation confirmed; 18 power lines per sample, 1314 samples across 73 shadow lines.
Final round changes tests only; the completed unique game verdict covers unchanged production source.
Earlier failed focused assertions remain in build-runs/netocc-focused.log; corrected source passed the full gates.
Missing: twelve factories, inherited lazy placeables and cross-module group enumeration (SPEC-GAP comments).
Full-house trial has not run: current replace list was requested because tools/regress.ps1 is forbidden to this lane.
This partial delivery does not authorize appending netocc to the house list; no new owners or questions started.
Public C ABI unchanged; private static-link transport only. No questions filed or tools/coverage files edited.
