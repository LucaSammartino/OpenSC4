| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-camera-control4 (pure camera owners) | `core-camera-control4` / (engine lane) | in progress | Implement camera owners; run full gate. |

## Handoff
- Goal: complete pure-core cS3DCamera and cSC4CameraControl from camera spec sections 1, 2, 4, 6; no ABI/plugin changes.
- Worktree reserved, detached HEAD; commit normally, never change refs.
- Done: complete first bodies/inventory for both camera owners; reuse Xform unchanged; 32 independent tests added; CA3–CA6 filed.
- Baseline qbuild all: cargo 4238/0, clippy clean, build 206/206; ctest 66/67 (cargo doctest saw incomplete module while edited; tree changed, baseline invalid).
- First quick: core 2400 passed / 4 failed (two independent golden arithmetic mistakes and scroll clamp edge/outside handling); dependents compile. Formatting limited to new files.
- Next: inspect four failures, fix arithmetic expectations and scroll-edge handling; commit fixes; qbuild quick, fmt, final all on committed code.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-camera-control4-r1-manifest.json.
- No game request outstanding; pure-core task has no shadow seam.
