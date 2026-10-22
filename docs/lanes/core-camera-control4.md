| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-camera-control4 (pure camera owners) | `core-camera-control4` / (engine lane) | in progress | Implement camera owners; run full gate. |

## Handoff
- Goal: complete pure-core cS3DCamera and cSC4CameraControl from camera spec sections 1, 2, 4, 6; no ABI/plugin changes.
- Worktree reserved, detached HEAD; commit normally, never change refs.
- Done: 399642e7 complete first bodies/inventory for both camera owners; reuse Xform unchanged; CA3–CA6 filed.
- Baseline qbuild all: cargo 4238/0, clippy clean, build 206/206; ctest 66/67 (cargo doctest saw incomplete module while edited; tree changed, baseline invalid).
- First quick: core 2400 passed / 4 failed; fixed independent golden arithmetic, threshold expectation and repeated parallel-edge crossing; added failed-ground/exhaustion cases.
- Second quick: PASS core 2406/0, dependent crates compiled, build 4/4; 34 camera tests pass. Removed unused helper after this gate; bounded malformed-save virtual-zoom arithmetic.
- New module fmt PASS (5 files, absolute paths); lib.rs fmt reports pre-existing module-order/export wrapping debt (126 diff lines), intentionally no mass reformat.
- Added four more discriminators for rotated perspective ray, rectangular-city foreign corner quirk, malformed save indices and literal viewport setter; 38 camera tests total.
- Candidate 1f72f8d1 full gate: cargo 4277/0, build 4/4, ctest 67/67; clippy failed only on two literal projection-offset assignments (assign_op_pattern).
- Added local lint exemption preserving specified operand order; moved only new module registration into alphabetical position. No other formatting changes.
- Next: commit final lint fix; qbuild all on committed unchanged code; DLL smoke via unique game label (no camera shadow seam).
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-camera-control4-r1-manifest.json.
- No game request outstanding; pure-core task has no shadow seam.
