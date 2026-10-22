| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-camera-contractfix4 (CA3–CA6 core corrections) | `core-camera-contractfix4` / (engine lane, detached worktree) | in progress | Correct public contracts, test, qualify whole owners, gate |

## Handoff
- Goal: correct camera pure core against independent CA4–CA6 and resolved CA3 constructor/look-at contracts.
- Scope: engine/core/src/camera/{s3d,geometry,control,tests,mod}.rs, README owner inventory; no ABI/plugin/CMake/default changes.
- Reused reserved detached worktree; initial status clean, no main.. commits.
- q.py cannot start Bash (E_ACCESSDENIED); use bounded PowerShell commands and required qbuild wrapper.
- Done: read public camera §§1–2, 5–7 and parent qualifications; reconciled defaults/yaw, partial Read, Init/detach, command order, synchronous live messages/reentry, directed clamp and ground-failure paths.
- Done: focused CA3–CA6 discriminator tests and per-native-body README audit; neither owner earns complete-owner credit (explicit missing bodies/cache/native lifecycle).
- First quick overlapped edits: 2689 passed/14 failed, dependents compiled, build 208/208; invalidated by tree changes. Next: stable quick, correct failures, absolute-path fmt, then one final all gate.
- Initial fmt wrapper needs absolute paths (relative file lookup failed); rustfmt applied only touched Rust files.
- No redundant full baseline: task-specific instruction requests one stable full cargo/clippy/native gate after correction; parent baseline/final gate are recorded in core-camera-control4.
- CA3 perspective setter/counter stays blocked. No game needed; native shadow/replacement deferred.
- Phase: answer-correction phase 2 (parent used 1); logical cap 3, no phase reset/fourth phase.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-camera-contractfix4-r1-manifest.json.
