| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-simgrids-shadow4: independent native grid verification | `verify-simgrids-shadow4` (engine, detached worktree) | in progress; baseline gate running | Implement public-interface native versus independent Rust comparisons; gate; unique-label game save/reload/2032/ref0. |

## Handoff
- Goal: independent same-process native grid versus Rust verification; preserve SG1 replacement boundaries.
- Base: eaae6c54de3454c6a53205a9403c4d3389133b2a; detached HEAD, do not update refs.
- Baseline attempt: workspace cargo tests and clippy passed; delayed release build saw a temporary new-code placeholder and failed. Not a clean baseline (implementation began before completion).
- Implemented: private stateless Rust transport over independent core SimGrid; constructed native ten-format fixture and persistent native power-owner follow, no native-state resync. Shared public ABI table untouched.
- Milestone: 65ca724a substantive native/Rust verification committed by call 25.
- Focused quick build compiled/linked. Added typed high-bit/resize/version tests, geometry/coordinates/lifetime and literal Read fixtures. fmt new module passed; recursive lib.rs check encounters pre-existing formatting in unrelated child modules (no mass format).
- Stable full gate at e0522ffd: cargo 4940/0, clippy 0 diagnostics, build PASS, ctest 68/68. Prior overlapping retry failed DLL link while old CTest held DLL; final gate ran after old CTest completed.
- Frozen DLL/PDB: build-runs/dll-sgv4-r1; DLL SHA256 397C55ADE6EA4698799BF70BC50A18CE544D415128A19FF9F9FC47B249ED9EC5.
- First request sgv4-r1-native39-20261022a CORRUPT: 1 foreign exception immediately after original-grid probe (117 captures), before fixture result, no algorithm credit. No RE/dump investigation.
- Shadow-OFF control39 sgv4-r1-control39-off-20261022b CLEAN 2032-01-01/ref0; full SaveCity control executed. Failed ON run retained, not excused by control.
- Fixture now establishes Init+SetTractSize before first capture; SG2 parks constructor/pre-init Write contract (no causal assertion about exception). SG1 unchanged.
- Final code f5902bb7: quick PASS529; stable all cargo4940/clippy0/buildPASS/ctest68. New frozen DLL build-runs/dll-sgv4-r2 SHA256 CAD1864980992DAA5F352043EF456F3B25EC9D0323027DA9FF19E3381C5E8AFB.
- Native retry sgv4-r1-native39-20261022c queued/outstanding; qwait/script session86153. Script then preserves native save and submits current40-reload-20261022c, then native39-roundtrip-20261022c only if clean. At most one request outstanding. Inspect final counts including refusals/failures and all differences; file questions, no exact credit from bootstrap/digests.
- Natural domain is all power-owner grid adapter calls; other owners outside scope. Native allocation failure and replacement-factory refusal cannot count as equality.
- No game request outstanding; no questions filed. SG1 boundaries unchanged.
- Only published contracts, public SDK and implementation may be read. No RE, tools, subagents or default changes.
