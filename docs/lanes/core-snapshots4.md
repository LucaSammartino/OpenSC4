| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-snapshots4 (portable snapshot album and taker) | `core-snapshots4` / `.claude/worktrees/core-snapshots4` (engine lane) | in progress | Audit owners; independent fixtures; portable core; focused and full gate. |

## Handoff
- Goal: implement snapshots.md sections 1-4/6 in portable core with typed host primitives.
- Worktree reused at detached HEAD; no branch/ref changes permitted.
- Done: audited all named spec methods and 157 ABI rows (151 taker / 6 album); inventory in engine/core/src/snapshots/OWNER_INVENTORY.md.
- Done: independent expected fixtures written before implementation; questions SN3-SN9 appended (SN1-SN2 already open).
- Milestone 757008ec: owner audit, questions and independent expected fixtures.
- Implemented: virtual filesystem/UI primitives, ten-field info codec, album workflows and taker state machine/capture pipeline.
- Baseline: cargo 4987 passed; clippy/build failed (logs preserved in build-runs/baseline); tree changed during later build phases, so baseline is not a clean final gate.
- Milestone d58b77cb: portable production owners committed.
- Added 16 total discriminating tests: byte fixtures, file scope/filter/order, copy/move failures, capture alpha/delay, pause/restore, drag/reuse, same-file dirty edits and UI hide/post/Lua ordering.
- Formatting of the six new Rust files passes (absolute paths required by qbuild fmt). q.py Bash transport unavailable; targeted rustfmt output retained in build-runs/snapshot-format.log.
- Milestone 91e3d071: strengthened 15-fixture suite and frozen production code.
- Focused quick Rust phase: 2965 passed / 0 failed, including the initial 6 snapshot fixtures (compiled before 9 later additions); final all must run all 15.
- New-file fmt passes; shared additive lib.rs fmt check fails on pre-existing ordering/layout (166 diff lines), left untouched as required.
- Quick PASS: core 2965/0 (initial 6 snapshot fixtures), dependents compiled, CMake 170/170; 1427.5 s total. Tree changed during this iterative run; not the final frozen-code gate.
- Review correction: typed on_key/on_capture_message now hide via the UI primitive before posting counter 0, own re-posts and raise the Lua event. A new independent effect-order fixture precedes this correction.
- Milestone 66f778c1: UI effect ordering correction, 16 total fixtures; new-file fmt passes.
- Next: stable final qbuild all on frozen source. Quick logs preserved in build-runs/quick-iteration; no source edits during final gate.
- No game requests: pure core only; no ABI changes or native-exact/full-owner claims.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-snapshots4-r1-manifest.json.
