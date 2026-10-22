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
- First request sgv4-r1-native39-20261022a returned; run-sgv4.ps1 stopped on clean-verdict validation. Next: inspect verdict/log, preserve save if clean, continue current40+native reload using unique labels and frozen DLL (no rebuild).
- Natural domain is all power-owner grid adapter calls; other owners outside scope. Native allocation failure and replacement-factory refusal cannot count as equality.
- No game request outstanding; no questions filed. SG1 boundaries unchanged.
- Only published contracts, public SDK and implementation may be read. No RE, tools, subagents or default changes.
